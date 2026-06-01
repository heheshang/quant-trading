//! Backtest database operations — SeaORM CRUD for backtest_results table
//! and kline data access.
//!
//! Provides:
//!   - [`create_backtest_run`] (INSERT with status=running)
//!   - [`update_completed`] (UPDATE metrics/trades/equity_curve)
//!   - [`update_failed`] (UPDATE status=failed + error)
//!   - [`find_backtest_result`] (SELECT by id)
//!   - [`get_backtest_progress`] (SELECT status + progress)
//!   - [`list_backtest_history`] (paginated SELECT by strategy_id)
//!   - [`delete_backtest_record`] (DELETE by id)
//!   - [`load_klines`] (SELECT from kline_data by symbol/interval/time)
//!   - [`find_strategy`] (SELECT from strategies by id)

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

use crate::db::backtest_results;
use crate::db::strategy;
use crate::models::backtest::{
    BacktestConfig, BacktestMetrics, BacktestProgressResponse, BacktestResultResponse,
    BacktestSummary, EquityPoint, Kline, MetricsPreview, TradeRecord,
};
use crate::models::schemas::PaginatedResponse;
use crate::utils::error::AppError;

// ============ Backtest Results CRUD ============

/// Create a new backtest run record with status=running.
pub async fn create_backtest_run(
    db: &DatabaseConnection,
    id: Uuid,
    user_id: Uuid,
    strategy_id: Uuid,
    config: &BacktestConfig,
) -> Result<(), AppError> {
    let now = chrono::Utc::now();
    backtest_results::ActiveModel {
        id: Set(id),
        user_id: Set(user_id),
        strategy_id: Set(strategy_id),
        config: Set(serde_json::to_value(config).map_err(|e| AppError::Internal(e.to_string()))?),
        status: Set("running".to_string()),
        progress: Set(0),
        metrics: Set(None),
        trades: Set(None),
        equity_curve: Set(None),
        start_time: Set(Some(now)),
        end_time: Set(None),
        duration_ms: Set(None),
        error: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;
    tracing::info!(result_id = %id, user_id = %user_id, strategy_id = %strategy_id, "Created backtest run (DB)");
    Ok(())
}

/// Update backtest result with completed metrics.
pub async fn update_completed(
    db: &DatabaseConnection,
    id: Uuid,
    metrics: &BacktestMetrics,
    trades: &[TradeRecord],
    equity_curve: &[EquityPoint],
    duration_ms: i64,
) -> Result<(), AppError> {
    let now = chrono::Utc::now();
    backtest_results::ActiveModel {
        id: Set(id),
        status: Set("completed".to_string()),
        progress: Set(100),
        metrics: Set(Some(
            serde_json::to_value(metrics).map_err(|e| AppError::Internal(e.to_string()))?,
        )),
        trades: Set(Some(
            serde_json::to_value(trades).map_err(|e| AppError::Internal(e.to_string()))?,
        )),
        equity_curve: Set(Some(
            serde_json::to_value(equity_curve).map_err(|e| AppError::Internal(e.to_string()))?,
        )),
        end_time: Set(Some(now)),
        duration_ms: Set(Some(duration_ms)),
        updated_at: Set(now),
        ..Default::default()
    }
    .update(db)
    .await?;
    tracing::info!("Backtest completed: {}", id);
    Ok(())
}

/// Update backtest result with failure.
pub async fn update_failed(
    db: &DatabaseConnection,
    id: Uuid,
    error_msg: &str,
) -> Result<(), AppError> {
    let now = chrono::Utc::now();
    backtest_results::ActiveModel {
        id: Set(id),
        status: Set("failed".to_string()),
        progress: Set(0),
        end_time: Set(Some(now)),
        error: Set(Some(error_msg.to_string())),
        updated_at: Set(now),
        ..Default::default()
    }
    .update(db)
    .await?;
    tracing::error!("Backtest failed: {} - {}", id, error_msg);
    Ok(())
}

/// Find a backtest result by ID.
///
/// Returns `None` if no record exists for the given UUID.
pub async fn find_backtest_result(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<BacktestResultResponse>, AppError> {
    tracing::debug!(result_id = %id, "Finding backtest result");
    let m = backtest_results::Entity::find_by_id(id).one(db).await?;
    match m {
        Some(model) => Ok(Some(model_to_result_response(model)?)),
        None => Ok(None),
    }
}

/// Get backtest progress (used by polling API).
pub async fn get_backtest_progress(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<BacktestProgressResponse>, AppError> {
    let m = backtest_results::Entity::find_by_id(id).one(db).await?;
    match m {
        Some(model) => {
            let start = model.start_time.unwrap_or(model.created_at);
            let elapsed_ms = (chrono::Utc::now() - start).num_milliseconds();
            Ok(Some(BacktestProgressResponse {
                id: model.id,
                status: model.status,
                progress: model.progress,
                current_bar: model.progress as i64,
                total_bars: 100,
                elapsed_ms,
            }))
        }
        None => Ok(None),
    }
}

/// List backtest history for a strategy with pagination.
///
/// Returns [`BacktestSummary`] items sorted by `created_at` DESC.
pub async fn list_backtest_history(
    db: &DatabaseConnection,
    strategy_id: Option<Uuid>,
    page: u64,
    page_size: u64,
) -> Result<PaginatedResponse<BacktestSummary>, AppError> {
    let mut query =
        backtest_results::Entity::find().order_by_desc(backtest_results::Column::CreatedAt);

    if let Some(sid) = strategy_id {
        query = query.filter(backtest_results::Column::StrategyId.eq(sid));
    }

    let paginator = query.paginate(db, page_size);
    let items: Vec<backtest_results::Model> = paginator.fetch_page(page - 1).await?;
    let total = paginator.num_items().await? as u64;

    let summaries: Vec<BacktestSummary> = items
        .into_iter()
        .map(|m| {
            let config: BacktestConfig =
                serde_json::from_value(m.config).unwrap_or_else(|_| BacktestConfig {
                    symbol: "unknown".into(),
                    interval: "1h".into(),
                    start_date: String::new(),
                    end_date: String::new(),
                    initial_capital: 0.0,
                    fee_rate: 0.001,
                    slippage_rate: 0.0005,
                    ..Default::default()
                });
            let metrics_preview = m.metrics.as_ref().and_then(|v| {
                serde_json::from_value::<BacktestMetrics>(v.clone())
                    .ok()
                    .map(|metrics| MetricsPreview {
                        total_return_pct: metrics.total_return_pct,
                        sharpe_ratio: metrics.sharpe_ratio,
                        max_drawdown_pct: metrics.max_drawdown_pct,
                        total_trades: metrics.total_trades,
                    })
            });
            BacktestSummary {
                id: m.id,
                status: m.status,
                config,
                metrics: metrics_preview,
                start_time: m.start_time,
                duration_ms: m.duration_ms,
                created_at: m.created_at,
            }
        })
        .collect();

    Ok(PaginatedResponse {
        items: summaries,
        total,
        page,
        size: page_size,
    })
}

/// Delete a backtest record by ID.
pub async fn delete_backtest_record(db: &DatabaseConnection, id: Uuid) -> Result<(), AppError> {
    backtest_results::Entity::delete_by_id(id).exec(db).await?;
    tracing::info!(result_id = %id, "Deleted backtest record (DB)");
    Ok(())
}

// ============ Kline Data Access ============

/// Load kline data for backtest simulation.
/// Returns the Klines sorted by open_time ASC.
pub async fn load_klines(
    db: &DatabaseConnection,
    symbol: &str,
    interval: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Option<Vec<Kline>>, AppError> {
    use crate::db::kline;
    use crate::db::kline::Column as K;

    let results = kline::Entity::find()
        .filter(K::Symbol.eq(symbol))
        .filter(K::Interval.eq(interval))
        .filter(K::OpenTime.gte(start_ms))
        .filter(K::OpenTime.lte(end_ms))
        .order_by_asc(K::OpenTime)
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to load klines: {}", e)))?;

    if results.is_empty() {
        tracing::debug!(
            "No klines found for {} {} from {} to {}",
            symbol,
            interval,
            start_ms,
            end_ms
        );
        return Ok(None);
    }

    let klines: Vec<Kline> = results
        .into_iter()
        .map(|r| Kline {
            open_time: r.open_time,
            open: r.open.parse().unwrap_or(0.0),
            high: r.high.parse().unwrap_or(0.0),
            low: r.low.parse().unwrap_or(0.0),
            close: r.close.parse().unwrap_or(0.0),
            volume: r.volume.parse().unwrap_or(0.0),
        })
        .collect();

    tracing::info!(
        "Loaded {} klines for {} {} from {} to {}",
        klines.len(),
        symbol,
        interval,
        start_ms,
        end_ms
    );
    Ok(Some(klines))
}

/// Find a strategy by ID (for validation).
pub async fn find_strategy(
    db: &DatabaseConnection,
    strategy_id: Uuid,
) -> Result<Option<serde_json::Value>, AppError> {
    let m = strategy::Entity::find_by_id(strategy_id).one(db).await?;
    match m {
        Some(model) => {
            let value =
                serde_json::to_value(&model).map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

// ============ Mapping Helpers ============

fn model_to_result_response(
    m: backtest_results::Model,
) -> Result<BacktestResultResponse, AppError> {
    let config: BacktestConfig = serde_json::from_value(m.config)
        .map_err(|e| AppError::Internal(format!("Invalid config JSON: {}", e)))?;
    let metrics = m
        .metrics
        .and_then(|v| serde_json::from_value::<BacktestMetrics>(v).ok());
    let trades = m
        .trades
        .and_then(|v| serde_json::from_value::<Vec<TradeRecord>>(v).ok());
    let equity_curve = m
        .equity_curve
        .and_then(|v| serde_json::from_value::<Vec<EquityPoint>>(v).ok());

    Ok(BacktestResultResponse {
        id: m.id,
        user_id: m.user_id,
        strategy_id: m.strategy_id,
        status: m.status,
        progress: m.progress,
        config,
        metrics,
        trades,
        equity_curve,
        start_time: m.start_time,
        end_time: m.end_time,
        duration_ms: m.duration_ms,
        error: m.error,
    })
}
