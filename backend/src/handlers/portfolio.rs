//! handlers/portfolio.rs — Portfolio REST API handlers
//!
//! ADR: ADR-009 §3.3 Handler 层
//! Endpoints:
//!   GET /api/v1/portfolio/summary
//!   GET /api/v1/portfolio/positions
//!   GET /api/v1/portfolio/performance
//!   GET /api/v1/portfolio/equity_curve

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::middleware::auth::AuthenticatedUser;
use crate::services::portfolio::{
    self, EquityCurveQuery, PortfolioPerformance, PortfolioPositionsQuery, PortfolioSummary,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

// ─── Request Types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PortfolioSummaryQuery {
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct PortfolioPerformanceQuery {
    pub user_id: Option<Uuid>,
}

// ─── Helpers ────────────────────────────────────────────────────

/// Check if the requesting user can access the target user_id's data.
/// Only self or admin can access.
fn check_access(user: &AuthenticatedUser, target_user_id: Uuid) -> Result<(), AppError> {
    if target_user_id != user.user_id && user.role != "admin" {
        return Err(AppError::Forbidden("无权限查看该组合".to_string()));
    }
    Ok(())
}

// ─── Handlers ───────────────────────────────────────────────────

/// GET /api/v1/portfolio/summary — 组合权益汇总
pub async fn get_portfolio_summary(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<PortfolioSummaryQuery>,
) -> Result<Json<ApiResponse<PortfolioSummary>>, AppError> {
    let user_id = params.user_id.unwrap_or(user.user_id);
    check_access(&user, user_id)?;

    let summary = portfolio::get_summary(&db, user_id).await?;
    Ok(Json(ApiResponse::success(summary)))
}

/// GET /api/v1/portfolio/positions — 持仓汇总
pub async fn list_portfolio_positions(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<PortfolioPositionsQuery>,
) -> Result<Json<ApiResponse<portfolio::Paginated<portfolio::PortfolioPosition>>>, AppError> {
    let user_id = params.user_id.unwrap_or(user.user_id);
    check_access(&user, user_id)?;

    let positions = portfolio::list_positions(&db, user_id, &params).await?;
    Ok(Json(ApiResponse::success(positions)))
}

/// GET /api/v1/portfolio/performance — 多策略绩效率对比
pub async fn get_portfolio_performance(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<PortfolioPerformanceQuery>,
) -> Result<Json<ApiResponse<PortfolioPerformance>>, AppError> {
    let user_id = params.user_id.unwrap_or(user.user_id);
    check_access(&user, user_id)?;

    let performance = portfolio::get_performance(&db, user_id).await?;
    Ok(Json(ApiResponse::success(performance)))
}

/// GET /api/v1/portfolio/equity_curve — 权益曲线数据
pub async fn get_equity_curve(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<EquityCurveQuery>,
) -> Result<Json<ApiResponse<portfolio::EquityCurve>>, AppError> {
    let user_id = params.user_id.unwrap_or(user.user_id);
    check_access(&user, user_id)?;

    let curve = portfolio::get_equity_curve(&db, user_id, &params).await?;
    Ok(Json(ApiResponse::success(curve)))
}

// ─── Unit Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_access_self() {
        let user = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "testuser".to_string(),
            role: "user".to_string(),
            jti: "jti".to_string(),
        };
        assert!(check_access(&user, user.user_id).is_ok());
    }

    #[test]
    fn test_check_access_admin() {
        let admin = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "admin".to_string(),
            role: "admin".to_string(),
            jti: "jti".to_string(),
        };
        let other_user_id = Uuid::new_v4();
        assert!(check_access(&admin, other_user_id).is_ok());
    }

    #[test]
    fn test_check_access_forbidden() {
        let user = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "testuser".to_string(),
            role: "user".to_string(),
            jti: "jti".to_string(),
        };
        let other_user_id = Uuid::new_v4();
        let result = check_access(&user, other_user_id);
        assert!(result.is_err());
        if let Err(AppError::Forbidden(msg)) = result {
            assert_eq!(msg, "无权限查看该组合");
        } else {
            panic!("Expected Forbidden error");
        }
    }

    #[test]
    fn test_portfolio_summary_query_deserialization() {
        let user_id = Uuid::new_v4();
        let json = serde_json::json!({ "user_id": user_id.to_string() });
        let query: PortfolioSummaryQuery = serde_json::from_value(json).unwrap();
        assert_eq!(query.user_id, Some(user_id));
    }

    #[test]
    fn test_portfolio_summary_query_none() {
        let json = serde_json::json!({});
        let query: PortfolioSummaryQuery = serde_json::from_value(json).unwrap();
        assert!(query.user_id.is_none());
    }

    #[test]
    fn test_portfolio_performance_query_deserialization() {
        let user_id = Uuid::new_v4();
        let json = serde_json::json!({ "user_id": user_id.to_string() });
        let query: PortfolioPerformanceQuery = serde_json::from_value(json).unwrap();
        assert_eq!(query.user_id, Some(user_id));
    }
}
