//! handlers/dashboard.rs — Dashboard REST API handlers
//!
//! Endpoints:
//!   GET /api/v1/dashboard/stats
//!   GET /api/v1/dashboard/pnl?range=7d|30d|90d

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth::AuthenticatedUser;
use crate::services::dashboard::{self, PnLHistory, PnLQuery};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

// ─── Request Types ──────────────────────────────────────────────

// GET /dashboard/stats has no query params
// GET /dashboard/pnl uses PnLQuery

// ─── Handlers ───────────────────────────────────────────────────

/// GET /api/v1/dashboard/stats — Dashboard statistics
pub async fn get_stats(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<dashboard::DashboardStats>>, AppError> {
    let stats = dashboard::get_stats(&db, user.user_id).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// GET /api/v1/dashboard/pnl?range=7d|30d|90d — PnL time series
pub async fn get_pnl(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<PnLQuery>,
) -> Result<Json<ApiResponse<PnLHistory>>, AppError> {
    let pnl = dashboard::get_pnl_history(&db, _user.user_id, &params).await?;
    Ok(Json(ApiResponse::success(pnl)))
}

// ─── Unit Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pnl_query_range_parsing() {
        let json = serde_json::json!({ "range": "7d" });
        let query: PnLQuery = serde_json::from_value(json).unwrap();
        assert_eq!(query.range, "7d");
    }

    #[test]
    fn test_pnl_query_default() {
        let json = serde_json::json!({ "range": "30d" });
        let query: PnLQuery = serde_json::from_value(json).unwrap();
        assert_eq!(query.range, "30d");
    }
}