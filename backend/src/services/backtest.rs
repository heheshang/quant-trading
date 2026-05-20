//! Backtest business logic service — orchestrates backtest execution, result queries,
//! pagination, and lifecycle management.
//!
//! This service layer sits between the API handlers and the data/engine layers.
//! Handlers are responsible only for request extraction and response formatting;
//! all business decisions live here.

use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, LazyLock, Mutex};

use sea_orm::DatabaseConnection;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::db::backtest as backtest_db;
use crate::middleware::auth::AuthenticatedUser;
use crate::models::backtest::{
    BacktestConfig, BacktestResultResponse, BacktestRunRequest, BacktestRunResponse, EquityPoint,
    TradeRecord,
};
use crate::models::schemas::PaginatedResponse;
use crate::services::backtest_engine::{BacktestEngine, build_kline_query, sample_equity_curve};
use crate::services::exchange::ws_hub::{HubMessage, WsHub};
use crate::services::strategy;
use crate::utils::error::AppError;

/// Global concurrency limiter for backtest runs.
/// Allows at most 5 simultaneous backtest executions.
static BACKTEST_SEMAPHORE: Semaphore = Semaphore::const_new(5);

/// Global registry of CancellationTokens for running backtests.
/// Keyed by backtest result UUID — allows cancel/delete to signal running tasks.
static CANCEL_TOKENS: LazyLock<Mutex<HashMap<Uuid, CancellationToken>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Backtest business logic service.
///
/// Provides methods for the full backtest lifecycle:
/// submit → track → query results/trades/equity → list history → delete/cancel.
pub struct BacktestService;

impl BacktestService {
    /// Submit a new backtest run.
    ///
    /// Validates the request config, verifies strategy ownership, loads kline data
    /// (or generates synthetic data), acquires a concurrency permit, and spawns
    /// the backtest engine on a blocking thread. Returns immediately with a
    /// pending status and the new result UUID.
    pub async fn run_backtest(
        db: &Arc<DatabaseConnection>,
        user: &AuthenticatedUser,
        req: &BacktestRunRequest,
        ws_hub: Arc<WsHub>,
    ) -> Result<BacktestRunResponse, AppError> {
        // 1. Validate config
        req.config.validate().map_err(AppError::Validation)?;

        // 2. Verify strategy exists
        let strategy_model = backtest_db::find_strategy(db, req.strategy_id)
            .await?
            .ok_or_else(|| AppError::NotFound("strategy not found".into()))?;

        // 3. Verify strategy ownership: strategy must belong to the authenticated user
        let strategy_user_id = strategy_model
            .get("user_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::nil);
        if strategy_user_id != user.user_id {
            tracing::warn!(
                user_id = %user.user_id,
                strategy_id = %req.strategy_id,
                owner_id = %strategy_user_id,
                "Backtest run rejected: strategy ownership mismatch"
            );
            return Err(AppError::Forbidden(
                "you do not have permission to run this strategy's backtest".into(),
            ));
        }

        // 4. Load template
        let template_type = strategy_model
            .get("template_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let template = strategy::get_template(template_type)
            .ok_or_else(|| AppError::Validation("invalid template type".into()))?;

        // 5. Load kline data (or generate synthetic data for demo)
        let (symbol, interval, start_ms, end_ms) = build_kline_query(&req.config);
        let klines = backtest_db::load_klines(db, &symbol, &interval, start_ms, end_ms)
            .await?
            .unwrap_or_else(|| generate_synthetic_klines(&req.config));

        if klines.is_empty() {
            return Err(AppError::Validation(
                "no kline data for the specified range".into(),
            ));
        }

        // 6. Try to acquire semaphore permit
        let permit = BACKTEST_SEMAPHORE.try_acquire().map_err(|_| {
            AppError::TooManyRequests("backtest concurrency limit reached (5)".into())
        })?;

        // 7. Use auth_user.user_id for the backtest run
        let user_id = user.user_id;

        // 8. Get strategy params
        let strategy_params = strategy_model
            .get("parameters")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // 9. Create result record
        let result_id = Uuid::new_v4();
        backtest_db::create_backtest_run(db, result_id, user_id, req.strategy_id, &req.config)
            .await?;

        tracing::info!(
            result_id = %result_id,
            strategy_id = %req.strategy_id,
            user_id = %user_id,
            symbol = %req.config.symbol,
            "Backtest run submitted"
        );

        // 10. Set up progress tracking and cancellation
        let progress = Arc::new(AtomicU32::new(0));
        let cancel_token = CancellationToken::new();
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(result_id, cancel_token.clone());

        // 11. Spawn backtest engine in blocking task
        let mut engine = BacktestEngine::new(
            req.config.clone(),
            klines,
            template,
            strategy_params,
            progress.clone(),
            cancel_token,
        );
        let db_clone = db.clone();
        let result_id_clone = result_id;
        let ws_hub_clone = ws_hub.clone();
        let progress_clone = progress.clone();

        // Spawn progress monitor — polls AtomicU32 every 200ms and broadcasts to WS
        let _monitor_handle = tokio::spawn(async move {
            let mut last_pct: u32 = 0;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                let current = progress_clone.load(std::sync::atomic::Ordering::Relaxed);
                if current != last_pct && current > 0 {
                    last_pct = current;
                    let _ = ws_hub_clone.broadcast(HubMessage::BacktestProgress {
                        backtest_id: result_id_clone,
                        progress: current,
                        status: if current >= 100 {
                            "completed".to_string()
                        } else {
                            "running".to_string()
                        },
                    });
                }
                if current >= 100 {
                    break;
                }
            }
        });

        tokio::task::spawn_blocking(move || {
            let _permit = permit; // hold permit until engine completes
            let start = std::time::Instant::now();

            match engine.run() {
                Ok((metrics, trades, equity)) => {
                    let duration_ms = start.elapsed().as_millis() as i64;
                    tracing::info!(
                        result_id = %result_id_clone,
                        duration_ms,
                        total_trades = metrics.total_trades,
                        return_pct = %format!("{:.2}", metrics.total_return_pct),
                        "Backtest engine completed"
                    );
                    let rt = tokio::runtime::Handle::current();
                    let _ = rt.block_on(async {
                        backtest_db::update_completed(
                            &db_clone,
                            result_id_clone,
                            &metrics,
                            &trades,
                            &equity,
                            duration_ms,
                        )
                        .await
                    });
                }
                Err(e) => {
                    tracing::error!(
                        result_id = %result_id_clone,
                        error = %e,
                        "Backtest engine failed"
                    );
                    let rt = tokio::runtime::Handle::current();
                    let _ = rt.block_on(async {
                        backtest_db::update_failed(&db_clone, result_id_clone, &e).await
                    });
                }
            }
        });

        // 12. Return immediately with pending status
        Ok(BacktestRunResponse {
            id: result_id,
            status: "running".into(),
            progress: 0,
            created_at: chrono::Utc::now(),
        })
    }

    /// Query a single backtest result by ID.
    ///
    /// Returns the full result including config, metrics, trades, and equity curve.
    pub async fn get_backtest(
        db: &Arc<DatabaseConnection>,
        id: Uuid,
    ) -> Result<BacktestResultResponse, AppError> {
        tracing::info!(result_id = %id, "Querying backtest result");
        let result = backtest_db::find_backtest_result(db, id)
            .await?
            .ok_or_else(|| AppError::NotFound("backtest result not found".into()))?;
        Ok(result)
    }

    /// Query backtest trade records with pagination.
    ///
    /// Extracts the trades array from the backtest result and applies
    /// server-side pagination (slice). Default page=1, size=50, max size=200.
    pub async fn get_backtest_trades(
        db: &Arc<DatabaseConnection>,
        id: Uuid,
        page: u64,
        size: u64,
    ) -> Result<PaginatedResponse<TradeRecord>, AppError> {
        tracing::info!(
            result_id = %id,
            page,
            size,
            "Querying backtest trades with pagination"
        );
        let result = backtest_db::find_backtest_result(db, id)
            .await?
            .ok_or_else(|| AppError::NotFound("backtest result not found".into()))?;

        let all_trades = result.trades.unwrap_or_default();
        let total = all_trades.len() as u64;
        let paginated = paginate_slice(&all_trades, page, size);

        Ok(PaginatedResponse {
            items: paginated,
            total,
            page,
            size,
        })
    }

    /// Query backtest equity curve with pagination.
    ///
    /// Applies sampling (max 2000 points) before pagination to keep responses
    /// manageable. Default page=1, size=50, max size=200.
    pub async fn get_backtest_equity(
        db: &Arc<DatabaseConnection>,
        id: Uuid,
        page: u64,
        size: u64,
    ) -> Result<PaginatedResponse<EquityPoint>, AppError> {
        tracing::info!(
            result_id = %id,
            page,
            size,
            "Querying backtest equity curve with pagination"
        );
        let result = backtest_db::find_backtest_result(db, id)
            .await?
            .ok_or_else(|| AppError::NotFound("backtest result not found".into()))?;

        let equity = result.equity_curve.unwrap_or_default();
        let sampled = sample_equity_curve(&equity, 2000);
        let total = sampled.len() as u64;
        let paginated = paginate_slice(&sampled, page, size);

        Ok(PaginatedResponse {
            items: paginated,
            total,
            page,
            size,
        })
    }

    /// List backtest history with optional strategy filter and pagination.
    ///
    /// Returns summaries; then enriches each with full result data for the
    /// paginated page only.
    pub async fn list_backtest_history(
        db: &Arc<DatabaseConnection>,
        strategy_id: Option<Uuid>,
        page: u64,
        size: u64,
    ) -> Result<PaginatedResponse<BacktestResultResponse>, AppError> {
        tracing::info!(
            strategy_id = ?strategy_id,
            page,
            size,
            "Listing backtest history"
        );
        let results = backtest_db::list_backtest_history(db, strategy_id, page, size).await?;

        let mut items = Vec::new();
        for summary in &results.items {
            if let Some(full) = backtest_db::find_backtest_result(db, summary.id).await? {
                items.push(full);
            }
        }

        Ok(PaginatedResponse {
            items,
            total: results.total,
            page: results.page,
            size: results.size,
        })
    }

    /// Delete a backtest record by ID.
    ///
    /// If the backtest is currently running, cancels it first via the
    /// CANCEL_TOKENS registry.
    pub async fn delete_backtest(db: &Arc<DatabaseConnection>, id: Uuid) -> Result<(), AppError> {
        // Cancel if running
        if let Some(token) = CANCEL_TOKENS.lock().unwrap().remove(&id) {
            token.cancel();
            tracing::info!(result_id = %id, "Cancelled running backtest during delete");
        }

        backtest_db::delete_backtest_record(db, id).await?;
        tracing::info!(result_id = %id, "Deleted backtest record");
        Ok(())
    }

    /// Cancel a running backtest by ID.
    ///
    /// Removes the cancellation token from the global registry and signals
    /// the engine to stop. Returns an error if no running backtest is found.
    pub async fn cancel_backtest(db: &Arc<DatabaseConnection>, id: Uuid) -> Result<(), AppError> {
        // Drop MutexGuard before any .await to keep future Send-compatible
        let token = CANCEL_TOKENS.lock().unwrap().remove(&id);
        match token {
            Some(token) => {
                token.cancel();
                let _ = backtest_db::update_failed(db, id, "cancelled").await;
                tracing::info!(result_id = %id, "Backtest cancelled by user");
                Ok(())
            }
            None => {
                tracing::warn!(result_id = %id, "Cancel requested but backtest not running");
                Err(AppError::NotFound(
                    "backtest not running or not found".into(),
                ))
            }
        }
    }
}

/// Paginate an in-memory slice.
///
/// Returns the subset of `items` for the given `page` (1-indexed) and `size`.
/// Clamps out-of-range pages to empty results rather than panicking.
fn paginate_slice<T: Clone>(items: &[T], page: u64, size: u64) -> Vec<T> {
    let size = size.clamp(1, 200) as usize;
    let start = (page.max(1) - 1) as usize * size;
    if start >= items.len() {
        return Vec::new();
    }
    let end = (start + size).min(items.len());
    items[start..end].to_vec()
}

/// Generate synthetic kline data for demo/testing when no real kline data is available.
///
/// Produces random-walk price data for the date range specified in `config`,
/// respecting the configured interval (1m through 1w).
fn generate_synthetic_klines(config: &BacktestConfig) -> Vec<crate::models::backtest::Kline> {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    use rand::Rng;

    let start = NaiveDate::parse_from_str(&config.start_date, "%Y-%m-%d")
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    let end = NaiveDate::parse_from_str(&config.end_date, "%Y-%m-%d")
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());

    let interval_minutes = match config.interval.as_str() {
        "1m" => 1,
        "5m" => 5,
        "15m" => 15,
        "30m" => 30,
        "1h" => 60,
        "2h" => 120,
        "4h" => 240,
        "1d" => 1440,
        "1w" => 10080,
        _ => 60,
    };

    let days = (end - start).num_days().max(7) as usize;
    let bars_per_day = 24 * 60 / interval_minutes.max(1);
    let total_bars = days * bars_per_day;

    let mut rng = rand::thread_rng();
    let mut klines = Vec::with_capacity(total_bars);

    let start_dt = NaiveDateTime::new(start, NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let start_ms = start_dt.and_utc().timestamp_millis();
    let interval_ms = interval_minutes as i64 * 60_000;

    let mut price = 100.0f64;
    for i in 0..total_bars {
        let change: f64 = rng.gen_range(-2.0..2.5);
        price = if price + change > 1.0 {
            price + change
        } else {
            1.0
        };
        let open = price;
        let high = open + rng.gen_range(0.0..3.0);
        let low = open - rng.gen_range(0.0..3.0);
        let close = rng.gen_range(low..high);

        klines.push(crate::models::backtest::Kline {
            open_time: start_ms + i as i64 * interval_ms,
            open,
            high,
            low,
            close,
            volume: rng.gen_range(500.0..5000.0),
        });
    }

    tracing::info!(
        symbol = %config.symbol,
        interval = %config.interval,
        bars = total_bars,
        "Generated synthetic kline data"
    );

    klines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::backtest::{BacktestConfig, EquityPoint, TradeRecord};
    use rstest::rstest;

    // ============ paginate_slice tests ============

    #[rstest]
    #[case(vec![1, 2, 3, 4, 5], 1, 2, vec![1, 2])]
    #[case(vec![1, 2, 3, 4, 5], 2, 2, vec![3, 4])]
    #[case(vec![1, 2, 3, 4, 5], 3, 2, vec![5])]
    #[case(vec![1, 2, 3, 4, 5], 4, 2, vec![])]
    #[case(vec![1, 2, 3, 4, 5], 1, 10, vec![1, 2, 3, 4, 5])]
    #[case(vec![1, 2, 3], 0, 2, vec![1, 2])] // page 0 clamps to 1
    #[case(Vec::<i32>::new(), 1, 10, vec![])]
    fn test_paginate_slice(
        #[case] items: Vec<i32>,
        #[case] page: u64,
        #[case] size: u64,
        #[case] expected: Vec<i32>,
    ) {
        let result = paginate_slice(&items, page, size);
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_paginate_slice_clamps_max_size() {
        let items: Vec<i32> = (0..500).collect();
        let result = paginate_slice(&items, 1, 9999);
        // size is clamped to 200
        assert_eq!(result.len(), 200);
        assert_eq!(result[0], 0);
        assert_eq!(result[199], 199);
    }

    // ============ BacktestConfig validation tests ============

    #[rstest]
    fn test_config_valid() {
        let config = BacktestConfig {
            symbol: "BTC/USDT".into(),
            interval: "1h".into(),
            start_date: "2024-01-01".into(),
            end_date: "2024-12-31".into(),
            initial_capital: 10000.0,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
        };
        assert!(config.validate().is_ok());
    }

    #[rstest]
    #[case(
        "2024-06-01",
        "2024-01-01",
        10000.0,
        0.001,
        0.0005,
        "start_date must be before end_date"
    )]
    #[case(
        "2024-01-01",
        "2024-01-03",
        10000.0,
        0.001,
        0.0005,
        "date range must be at least 7 days"
    )]
    #[case(
        "2024-01-01",
        "2024-12-31",
        50.0,
        0.001,
        0.0005,
        "initial_capital must be >= 100"
    )]
    #[case(
        "2024-01-01",
        "2024-12-31",
        10000.0,
        0.02,
        0.0005,
        "fee_rate must be 0..0.01"
    )]
    #[case(
        "2024-01-01",
        "2024-12-31",
        10000.0,
        0.001,
        0.02,
        "slippage_rate must be 0..0.01"
    )]
    #[case(
        "not-a-date",
        "2024-12-31",
        10000.0,
        0.001,
        0.0005,
        "start_date must be YYYY-MM-DD"
    )]
    fn test_config_validation_errors(
        #[case] start: &str,
        #[case] end: &str,
        #[case] capital: f64,
        #[case] fee: f64,
        #[case] slippage: f64,
        #[case] expected_msg: &str,
    ) {
        let config = BacktestConfig {
            symbol: "BTC/USDT".into(),
            interval: "1h".into(),
            start_date: start.into(),
            end_date: end.into(),
            initial_capital: capital,
            fee_rate: fee,
            slippage_rate: slippage,
        };
        let err = config.validate().unwrap_err();
        assert!(
            err.contains(expected_msg),
            "Expected error containing '{}', got '{}'",
            expected_msg,
            err
        );
    }

    // ============ TradeRecord pagination integration test ============

    #[rstest]
    fn test_trades_pagination_page_1() {
        let trades: Vec<TradeRecord> = (0..5)
            .map(|i| TradeRecord {
                entry_time: i * 1000,
                exit_time: i * 1000 + 500,
                direction: "long".into(),
                entry_price: 100.0 + i as f64,
                exit_price: 105.0 + i as f64,
                quantity: 1.0,
                pnl_usdt: 5.0,
                pnl_pct: 5.0,
                holding_period_ms: 500,
                exit_reason: "signal".into(),
                fee: 0.1,
                slippage: 0.05,
            })
            .collect();

        let result = paginate_slice(&trades, 1, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].entry_price, 100.0);
        assert_eq!(result[1].entry_price, 101.0);
    }

    #[rstest]
    fn test_equity_pagination() {
        let equity: Vec<EquityPoint> = (0..100)
            .map(|i| EquityPoint {
                time: i * 60000,
                equity: 10000.0 + i as f64,
                drawdown_pct: 0.0,
            })
            .collect();

        // Page 1, size 50
        let page1 = paginate_slice(&equity, 1, 50);
        assert_eq!(page1.len(), 50);
        assert_eq!(page1[0].time, 0);
        assert_eq!(page1[49].time, 49 * 60000);

        // Page 2, size 50
        let page2 = paginate_slice(&equity, 2, 50);
        assert_eq!(page2.len(), 50);
        assert_eq!(page2[0].time, 50 * 60000);
        assert_eq!(page2[49].time, 99 * 60000);

        // Page 3 should be empty
        let page3 = paginate_slice(&equity, 3, 50);
        assert!(page3.is_empty());
    }

    #[rstest]
    fn test_empty_trades_pagination() {
        let trades: Vec<TradeRecord> = vec![];
        let result = paginate_slice(&trades, 1, 10);
        assert!(result.is_empty());
    }
}
