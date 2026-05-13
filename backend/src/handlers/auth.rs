use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{LoginRequest, RefreshTokenRequest, RegisterRequest};
use crate::services::auth;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{extract::State, Json};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// POST /api/v1/auth/register
pub async fn register(
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = auth::register(&db, body).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "user": result.user,
        "access_token": result.access_token,
        "refresh_token": result.refresh_token,
        "expires_in": result.expires_in,
    }))))
}

/// POST /api/v1/auth/login
pub async fn login(
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = auth::login(&db, body).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "user": result.user,
        "access_token": result.access_token,
        "refresh_token": result.refresh_token,
        "expires_in": result.expires_in,
    }))))
}

/// POST /api/v1/auth/refresh
pub async fn refresh(
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<RefreshTokenRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = auth::refresh_token(&db, &body.refresh_token, &crate::CONFIG.jwt_secret).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "access_token": result.access_token,
        "refresh_token": result.refresh_token,
        "expires_in": result.expires_in,
    }))))
}

/// POST /api/v1/auth/logout
pub async fn logout(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    auth::logout(&db, user.user_id).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "Logged out successfully"
    }))))
}

/// GET /api/v1/auth/me - alias for /users/me
pub async fn me_route(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // Delegate to users handler
    super::users::get_me(user, State(db)).await
}
