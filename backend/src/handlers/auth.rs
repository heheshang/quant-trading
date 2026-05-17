use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{AuthResponseBody, LoginRequest, LogoutResponse, RefreshTokenRequest, RegisterRequest, TokenResponseBody, UserMeResponse};
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
) -> Result<Json<ApiResponse<AuthResponseBody>>, AppError> {
    let result = auth::register(&db, body).await?;

    Ok(Json(ApiResponse::success(AuthResponseBody {
        user: result.user,
        access_token: result.access_token,
        refresh_token: result.refresh_token,
        expires_in: result.expires_in,
    })))
}

/// POST /api/v1/auth/login
pub async fn login(
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthResponseBody>>, AppError> {
    let result = auth::login(&db, body).await?;

    Ok(Json(ApiResponse::success(AuthResponseBody {
        user: result.user,
        access_token: result.access_token,
        refresh_token: result.refresh_token,
        expires_in: result.expires_in,
    })))
}

/// POST /api/v1/auth/refresh
pub async fn refresh(
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<RefreshTokenRequest>,
) -> Result<Json<ApiResponse<TokenResponseBody>>, AppError> {
    let result = auth::refresh_token(&db, &body.refresh_token, &crate::CONFIG.jwt_secret).await?;

    Ok(Json(ApiResponse::success(TokenResponseBody {
        access_token: result.access_token,
        refresh_token: result.refresh_token,
        expires_in: result.expires_in,
    })))
}

/// POST /api/v1/auth/logout
pub async fn logout(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<LogoutResponse>>, AppError> {
    auth::logout(&db, user.user_id).await?;

    Ok(Json(ApiResponse::success(LogoutResponse {
        message: "Logged out successfully".into(),
    })))
}

/// GET /api/v1/auth/me - alias for /users/me
pub async fn me_route(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<UserMeResponse>>, AppError> {
    // Delegate to users handler
    let resp = super::users::get_me_internal(&db, user.user_id).await?;
    Ok(Json(ApiResponse::success(UserMeResponse(resp))))
}
