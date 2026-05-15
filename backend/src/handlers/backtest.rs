//! Backtest API handlers — async backtest task submission, status tracking,
//! result queries, and deletion.
//!
//! API endpoints (per PRD and ADR-007):
//!   POST   /api/v1/backtest          — run backtest
//!   GET    /api/v1/backtest/{id}     — get full result
//!   GET    /api/v1/backtest/{id}/trades  — get trades (paginated)
//!   GET    /api/v1/backtest/{id}/equity  — get equity curve (paginated)
//!   GET    /api/v1/backtest/history  — list history (query: strategy_id, page, size)
//!   DELETE /api/v1/backtest/{id}     — delete backtest record
//!
//! Handlers are thin wrappers: they extract request params, delegate to
//! [`BacktestService`](crate::services::backtest::BacktestService), and format
//! the response. No business logic lives here.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::middleware::auth::AuthenticatedUser;
use crate::models::backtest::{
    BacktestResultResponse, BacktestRunRequest, BacktestRunResponse, EquityPoint, TradeRecord,
};
use crate::models::schemas::PaginatedResponse;
use crate::services::backtest::BacktestService;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// Query parameters for trades/equity pagination.
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    /// Page number (1-indexed). Defaults to 1.
    pub page: Option<u64>,
    /// Page size. Defaults to 50, max 200.
    pub size: Option<u64>,
}

impl PaginationQuery {
    /// Return the validated page number (clamped to >= 1).
    pub fn page(&self) -> u64 {
        self.page.unwrap_or(1).max(1)
    }

    /// Return the validated page size (clamped to 1..=200).
    pub fn size(&self) -> u64 {
        self.size.unwrap_or(50).clamp(1, 200)
    }
}

// ============ POST /api/v1/backtest ============

/// Run a new backtest.
///
/// Validates the request, verifies strategy ownership, and spawns the
/// backtest engine asynchronously. Returns immediately with a pending status.
pub async fn run_backtest(
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<BacktestRunRequest>,
) -> Result<Json<ApiResponse<BacktestRunResponse>>, AppError> {
    let response = BacktestService::run_backtest(&db, &user, &req).await?;
    Ok(Json(ApiResponse::success(response)))
}

// ============ GET /api/v1/backtest/{id} ============

/// Get a full backtest result by ID.
///
/// Returns the complete result including config, metrics, trades, and equity curve.
pub async fn get_backtest(
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<BacktestResultResponse>>, AppError> {
    let result = BacktestService::get_backtest(&db, id).await?;
    Ok(Json(ApiResponse::success(result)))
}

// ============ GET /api/v1/backtest/{id}/trades ============

/// Get trade records for a backtest with pagination.
///
/// Query params: `page` (default 1), `size` (default 50, max 200).
pub async fn get_backtest_trades(
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<TradeRecord>>>, AppError> {
    let result = BacktestService::get_backtest_trades(&db, id, pagination.page(), pagination.size()).await?;
    Ok(Json(ApiResponse::success(result)))
}

// ============ GET /api/v1/backtest/{id}/equity ============

/// Get equity curve for a backtest with pagination.
///
/// Applies sampling (max 2000 points) before paginating.
/// Query params: `page` (default 1), `size` (default 50, max 200).
pub async fn get_backtest_equity(
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<EquityPoint>>>, AppError> {
    let result = BacktestService::get_backtest_equity(&db, id, pagination.page(), pagination.size()).await?;
    Ok(Json(ApiResponse::success(result)))
}

// ============ GET /api/v1/backtest/history ============

/// Query parameters for the backtest history list endpoint.
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    /// Filter by strategy ID (optional).
    pub strategy_id: Option<Uuid>,
    /// Page number (default 1).
    pub page: Option<u64>,
    /// Page size (default 20, max 100).
    pub size: Option<u64>,
}

/// List backtest history with optional strategy filter and pagination.
///
/// Returns full result objects for the requested page.
pub async fn list_backtest_history(
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<BacktestResultResponse>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(20).clamp(1, 100);

    let result = BacktestService::list_backtest_history(&db, query.strategy_id, page, size).await?;
    Ok(Json(ApiResponse::success(result)))
}

// ============ DELETE /api/v1/backtest/{id} ============

/// Delete a backtest record by ID.
///
/// If the backtest is currently running, cancels it first.
pub async fn delete_backtest(
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    BacktestService::delete_backtest(&db, id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============ POST /api/v1/backtest/{id}/cancel ============

/// Cancel a running backtest.
///
/// Signals the engine to stop and updates the result status to "failed".
pub async fn cancel_backtest(
    State(db): State<std::sync::Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    BacktestService::cancel_backtest(&db, id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({}))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_query_defaults() {
        let pq = PaginationQuery {
            page: None,
            size: None,
        };
        assert_eq!(pq.page(), 1);
        assert_eq!(pq.size(), 50);
    }

    #[test]
    fn test_pagination_query_custom() {
        let pq = PaginationQuery {
            page: Some(3),
            size: Some(100),
        };
        assert_eq!(pq.page(), 3);
        assert_eq!(pq.size(), 100);
    }

    #[test]
    fn test_pagination_query_clamps() {
        let pq = PaginationQuery {
            page: Some(0),
            size: Some(999),
        };
        assert_eq!(pq.page(), 1);
        assert_eq!(pq.size(), 200);
    }

    #[test]
    fn test_history_query_defaults() {
        let hq = HistoryQuery {
            strategy_id: None,
            page: None,
            size: None,
        };
        assert!(hq.strategy_id.is_none());
        assert!(hq.page.is_none());
        assert!(hq.size.is_none());
    }
}
