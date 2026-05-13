// ============ Backtest API Handlers ============
//
// API endpoints (per PRD and ADR-007):
//   POST   /api/v1/backtest          — run backtest
//   GET    /api/v1/backtest/{id}     — get full result
//   GET    /api/v1/backtest/{id}/trades  — get trades only
//   GET    /api/v1/backtest/{id}/equity  — get equity curve only
//   GET    /api/v1/backtest/history  — list history (query: strategy_id, page, size)
//   DELETE /api/v1/backtest/{id}     — delete backtest record
//
// Concurrency limit: 5 simultaneous backtest runs (Semaphore)

use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, LazyLock, Mutex};

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
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
use crate::services::backtest_engine::{build_kline_query, sample_equity_curve, BacktestEngine};
use crate::services::strategy;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// Global concurrency limiter
static BACKTEST_SEMAPHORE: Semaphore = Semaphore::const_new(5);

/// Global registry of CancellationTokens for running backtests.
/// Keyed by backtest result UUID.
static CANCEL_TOKENS: LazyLock<Mutex<HashMap<Uuid, CancellationToken>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ============ POST /api/v1/backtest ============

pub async fn run_backtest(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<BacktestRunRequest>,
) -> Result<Json<ApiResponse<BacktestRunResponse>>, AppError> {
    // 1. Validate config
    req.config.validate().map_err(AppError::Validation)?;

    // 2. Verify strategy exists
    let strategy_model = backtest_db::find_strategy(&db, req.strategy_id)
        .await?
        .ok_or_else(|| AppError::NotFound("strategy not found".into()))?;

    // 3. Verify strategy ownership: strategy must belong to the authenticated user
    let strategy_user_id = strategy_model
        .get("user_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::nil);
    if strategy_user_id != user.user_id {
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

    // 4. Load kline data (or generate synthetic data for demo)
    let (symbol, interval, start_ms, end_ms) = build_kline_query(&req.config);
    let klines = backtest_db::load_klines(&db, &symbol, &interval, start_ms, end_ms)
        .await?
        .unwrap_or_else(|| generate_synthetic_klines(&req.config));

    if klines.is_empty() {
        return Err(AppError::Validation(
            "no kline data for the specified range".into(),
        ));
    }

    // 5. Try to acquire semaphore permit
    let permit = BACKTEST_SEMAPHORE
        .try_acquire()
        .map_err(|_| AppError::TooManyRequests("backtest concurrency limit reached (5)".into()))?;

    // 6. Use auth_user.user_id for the backtest run (ownership already verified above)
    let user_id = user.user_id;

    // 7. Get strategy params
    let strategy_params = strategy_model
        .get("parameters")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    // 8. Create result record
    let result_id = Uuid::new_v4();
    backtest_db::create_backtest_run(&db, result_id, user_id, req.strategy_id, &req.config).await?;

    // 9. Set up progress tracking and cancellation
    let progress = Arc::new(AtomicU32::new(0));
    let cancel_token = CancellationToken::new();
    CANCEL_TOKENS
        .lock()
        .unwrap()
        .insert(result_id, cancel_token.clone());

    // 10. Spawn backtest engine in blocking task
    let mut engine = BacktestEngine::new(
        req.config.clone(),
        klines,
        template,
        strategy_params,
        progress,
        cancel_token,
    );
    let db_clone = db.clone();
    let result_id_clone = result_id;

    tokio::task::spawn_blocking(move || {
        let _permit = permit; // hold permit until engine completes
        let start = std::time::Instant::now();

        match engine.run() {
            Ok((metrics, trades, equity)) => {
                let duration_ms = start.elapsed().as_millis() as i64;
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
                let rt = tokio::runtime::Handle::current();
                let _ = rt.block_on(async {
                    backtest_db::update_failed(&db_clone, result_id_clone, &e).await
                });
            }
        }
    });

    // 11. Return immediately with pending status
    Ok(Json(ApiResponse::success(BacktestRunResponse {
        id: result_id,
        status: "running".into(),
        progress: 0,
        created_at: chrono::Utc::now(),
    })))
}

// ============ GET /api/v1/backtest/{id} ============

pub async fn get_backtest(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<BacktestResultResponse>>, AppError> {
    let result = backtest_db::find_backtest_result(&db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("backtest result not found".into()))?;

    Ok(Json(ApiResponse::success(result)))
}

// ============ GET /api/v1/backtest/{id}/trades ============

pub async fn get_backtest_trades(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<TradeRecord>>>, AppError> {
    let result = backtest_db::find_backtest_result(&db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("backtest result not found".into()))?;

    let trades = result.trades.unwrap_or_default();
    Ok(Json(ApiResponse::success(trades)))
}

// ============ GET /api/v1/backtest/{id}/equity ============

pub async fn get_backtest_equity(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<EquityPoint>>>, AppError> {
    let result = backtest_db::find_backtest_result(&db, id)
        .await?
        .ok_or_else(|| AppError::NotFound("backtest result not found".into()))?;

    let equity = result.equity_curve.unwrap_or_default();
    let sampled = sample_equity_curve(&equity, 2000);
    Ok(Json(ApiResponse::success(sampled)))
}

// ============ GET /api/v1/backtest/history ============

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub strategy_id: Option<Uuid>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

pub async fn list_backtest_history(
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<BacktestResultResponse>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(20).clamp(1, 100);

    let results = backtest_db::list_backtest_history(&db, query.strategy_id, page, size).await?;

    let mut items = Vec::new();
    for summary in &results.items {
        if let Some(full) = backtest_db::find_backtest_result(&db, summary.id).await? {
            items.push(full);
        }
    }

    Ok(Json(ApiResponse::success(PaginatedResponse {
        items,
        total: results.total,
        page: results.page,
        size: results.size,
    })))
}

// ============ DELETE /api/v1/backtest/{id} ============

pub async fn delete_backtest(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Cancel if running
    if let Some(token) = CANCEL_TOKENS.lock().unwrap().remove(&id) {
        token.cancel();
    }

    backtest_db::delete_backtest_record(&db, id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============ Cancel Endpoint (POST /api/v1/backtest/{id}/cancel) ============

pub async fn cancel_backtest(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // Drop MutexGuard before any .await to keep future Send-compatible
    let token = CANCEL_TOKENS.lock().unwrap().remove(&id);
    match token {
        Some(token) => {
            token.cancel();
            let _ = backtest_db::update_failed(&db, id, "cancelled").await;
            Ok(Json(ApiResponse::success(serde_json::json!({}))))
        }
        None => Err(AppError::NotFound(
            "backtest not running or not found".into(),
        )),
    }
}

// ============ Synthetic Kline Generator ============

/// Generate synthetic kline data for demo/testing when no real kline data is available.
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

    klines
}
