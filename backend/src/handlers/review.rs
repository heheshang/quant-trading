//! Strategy Review Handler — P2-F2
//!
//! REST API for strategy review workflow: submit, approve, reject, list pending

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::db::review::{Model as ReviewModel, ReviewStatus};
use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{PaginatedResponse, PaginationParams};
use crate::services::review as review_service;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ─── Request/Response DTOs ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SubmitReviewRequest {
    pub strategy_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ReviewDecisionRequest {
    pub strategy_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewResponse {
    pub id: Uuid,
    pub strategy_id: Uuid,
    pub review_status: String,
    pub rejection_reason: Option<String>,
    pub submitted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reviewed_by: Option<Uuid>,
}

impl From<ReviewModel> for ReviewResponse {
    fn from(r: ReviewModel) -> Self {
        Self {
            id: r.id,
            strategy_id: r.strategy_id,
            review_status: r.review_status,
            rejection_reason: r.rejection_reason,
            submitted_at: r.submitted_at,
            reviewed_at: r.reviewed_at,
            reviewed_by: r.reviewed_by,
        }
    }
}

// ─── Handlers ───────────────────────────────────────────────────────────────

/// POST /api/v1/reviews/submit — Submit a strategy for review
pub async fn submit_for_review(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<SubmitReviewRequest>,
) -> Result<Json<ApiResponse<ReviewResponse>>, AppError> {
    let review = review_service::submit_for_review(&db, body.strategy_id, user.user_id).await?;
    Ok(Json(ApiResponse::success(review.into())))
}

/// POST /api/v1/reviews/approve — Approve a strategy (admin)
pub async fn approve_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<ReviewDecisionRequest>,
) -> Result<Json<ApiResponse<ReviewResponse>>, AppError> {
    let review =
        review_service::approve_strategy(&db, body.strategy_id, user.user_id).await?;
    Ok(Json(ApiResponse::success(review.into())))
}

/// POST /api/v1/reviews/reject — Reject a strategy (admin)
pub async fn reject_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<ReviewDecisionRequest>,
) -> Result<Json<ApiResponse<ReviewResponse>>, AppError> {
    let review = review_service::reject_strategy(
        &db,
        body.strategy_id,
        user.user_id,
        body.reason,
    )
    .await?;
    Ok(Json(ApiResponse::success(review.into())))
}

/// GET /api/v1/reviews/pending — List all pending reviews (admin)
pub async fn list_pending_reviews(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<PaginatedResponse<ReviewResponse>>>, AppError> {
    let reviews = review_service::list_pending_reviews(&db).await?;
    let items: Vec<ReviewResponse> = reviews.into_iter().map(|r| r.into()).collect();
    let total = items.len() as u64;
    Ok(Json(ApiResponse::success(PaginatedResponse {
        items,
        total,
        page: 1,
        size: 50,
    })))
}

/// GET /api/v1/reviews/:strategy_id — Get review for a specific strategy
pub async fn get_strategy_review(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(strategy_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Option<ReviewResponse>>>, AppError> {
    let review = review_service::get_strategy_review(&db, strategy_id).await?;
    let response = review.map(|r| r.into());
    Ok(Json(ApiResponse::success(response)))
}
