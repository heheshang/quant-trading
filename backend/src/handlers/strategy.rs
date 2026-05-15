use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{
    BulkUpdateStatusRequest, CreateStrategyRequest, ExportParams, ImportStrategyRequest,
    PaginationParams, UpdateStatusRequest, UpdateStrategyRequest,
};
use crate::services::strategy;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use uuid::Uuid;

/// GET /api/v1/strategies/templates
pub async fn list_templates(
    _user: AuthenticatedUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let templates = strategy::list_templates().await?;
    Ok(Json(ApiResponse::success(serde_json::json!(templates))))
}

/// GET /api/v1/strategies
pub async fn list_strategies(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = strategy::list_strategies(&db, user.user_id, params).await?;
    Ok(Json(ApiResponse::success(serde_json::json!(result))))
}

/// POST /api/v1/strategies
pub async fn create_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<CreateStrategyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    let result = strategy::create_strategy(&db, user.user_id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(serde_json::json!(result))),
    ))
}

/// GET /api/v1/strategies/{id}
pub async fn get_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(strategy_id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = strategy::get_strategy(&db, user.user_id, strategy_id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!(result))))
}

/// PUT /api/v1/strategies/{id}
pub async fn update_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(strategy_id): Path<Uuid>,
    Json(body): Json<UpdateStrategyRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = strategy::update_strategy(&db, user.user_id, strategy_id, body).await?;
    Ok(Json(ApiResponse::success(serde_json::json!(result))))
}

/// DELETE /api/v1/strategies/{id}
pub async fn delete_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(strategy_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    strategy::delete_strategy(&db, user.user_id, strategy_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/strategies/{id}/status
pub async fn update_status(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(strategy_id): Path<Uuid>,
    Json(body): Json<UpdateStatusRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result =
        strategy::update_strategy_status(&db, user.user_id, strategy_id, body.status).await?;
    Ok(Json(ApiResponse::success(serde_json::json!(result))))
}

/// POST /api/v1/strategies/bulk/status
pub async fn bulk_update_status(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<BulkUpdateStatusRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = strategy::bulk_update_status(&db, user.user_id, body).await?;
    Ok(Json(ApiResponse::success(serde_json::json!(result))))
}

/// GET /api/v1/strategies/export
pub async fn export_strategies(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<ExportParams>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let result = strategy::export_strategies(&db, user.user_id, params.status.as_deref()).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// POST /api/v1/strategies/import
pub async fn import_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<ImportStrategyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<serde_json::Value>>), AppError> {
    let result = strategy::import_strategy(&db, user.user_id, body).await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(serde_json::json!(result))),
    ))
}
