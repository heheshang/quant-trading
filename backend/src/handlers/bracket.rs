//! handlers/bracket.rs — Bracket order HTTP handlers (P1-2.2)
//!
//! Path A: deferred OCO. The bracket parent order is created via the existing
//! `/api/v1/orders` endpoint with `order_type=bracket` (handler logic in
//! `handlers/order.rs`). This module exposes a single GET endpoint for
//! frontend polling of pending bracket links.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::auth::AuthenticatedUser;
use crate::services::bracket::{self, BracketLink};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// GET /api/v1/bracket-links/pending
///
/// Returns all `pending` bracket links for the current user. The frontend
/// polls this endpoint and, for each link, decides whether to call
/// `POST /api/v1/trigger-orders/oco` to actually create the OCO.
pub async fn list_pending_bracket_links(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
) -> Result<impl IntoResponse, AppError> {
    let links = bracket::list_pending_for_user(&db, user.user_id).await?;
    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(PendingBracketLinksResponse {
            user_id: user.user_id,
            count: links.len(),
            links,
        })),
    ))
}

#[derive(serde::Serialize)]
pub struct PendingBracketLinksResponse {
    pub user_id: Uuid,
    pub count: usize,
    pub links: Vec<BracketLink>,
}

pub fn router() -> Router<Arc<DatabaseConnection>> {
    Router::new().route("/bracket-links/pending", get(list_pending_bracket_links))
}
