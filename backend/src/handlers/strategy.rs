use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{
    BulkUpdateStatusRequest, CreateStrategyRequest, ExportParams, ImportBatchRequest,
    ImportStrategyRequest, PaginationParams, StrategyBulkUpdateResponse, StrategyCreateResponse,
    StrategyExportResponse, StrategyImportResponse, StrategyListResponse, StrategyUpdateResponse,
    TemplateListResponse, UpdateStatusRequest, UpdateStrategyRequest,
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
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// POST /api/v1/strategies/bulk/delete — Delete multiple strategies
pub async fn bulk_delete_strategies(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<BulkDeleteRequest>,
) -> Result<Json<ApiResponse<strategy::BulkDeleteResponse>>, AppError> {
    let deleted = strategy::bulk_delete_strategies(&db, user.user_id, &body.ids).await?;
    Ok(Json(ApiResponse::success(deleted)))
}

#[derive(Debug, Deserialize)]
pub struct BulkDeleteRequest {
    pub ids: Vec<Uuid>,
}

/// POST /api/v1/strategies/code/upload — Upload strategy code file
pub async fn upload_strategy_code(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<ApiResponse<UploadCodeResponse>>, AppError> {
    let mut file_name = String::new();
    let mut file_content = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Internal(format!("Failed to read multipart field: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            file_name = field.file_name().unwrap_or("strategy.py").to_string();
            file_content = field.text().await.map_err(|e| {
                AppError::Internal(format!("Failed to read file content: {}", e))
            })?;
        }
    }

    if file_content.trim().is_empty() {
        return Err(AppError::Validation("File content is empty".into()));
    }

    let path = strategy::store_strategy_code(&db, user.user_id, &file_name, &file_content).await?;

    Ok(Json(ApiResponse::success(UploadCodeResponse { path })))
}

#[derive(Debug, Serialize)]
pub struct UploadCodeResponse {
    pub path: String,
}

/// GET /api/v1/strategies/templates
pub async fn list_templates(
    _user: AuthenticatedUser,
) -> Result<Json<ApiResponse<TemplateListResponse>>, AppError> {
    let templates = strategy::list_templates().await?;
    Ok(Json(ApiResponse::success(TemplateListResponse(templates))))
}

/// GET /api/v1/strategies
pub async fn list_strategies(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<StrategyListResponse>>, AppError> {
    let result = strategy::list_strategies(&db, user.user_id, params).await?;
    Ok(Json(ApiResponse::success(StrategyListResponse(result))))
}

/// POST /api/v1/strategies
pub async fn create_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<CreateStrategyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<StrategyCreateResponse>>), AppError> {
    let result = strategy::create_strategy(&db, user.user_id, body).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(StrategyCreateResponse(result)))))
}

/// GET /api/v1/strategies/{id}
pub async fn get_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(strategy_id): Path<Uuid>,
) -> Result<Json<ApiResponse<StrategyUpdateResponse>>, AppError> {
    let result = strategy::get_strategy(&db, user.user_id, strategy_id).await?;
    Ok(Json(ApiResponse::success(StrategyUpdateResponse(result))))
}

/// PUT /api/v1/strategies/{id}
pub async fn update_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(strategy_id): Path<Uuid>,
    Json(body): Json<UpdateStrategyRequest>,
) -> Result<Json<ApiResponse<StrategyUpdateResponse>>, AppError> {
    let result = strategy::update_strategy(&db, user.user_id, strategy_id, body).await?;
    Ok(Json(ApiResponse::success(StrategyUpdateResponse(result))))
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
) -> Result<Json<ApiResponse<StrategyUpdateResponse>>, AppError> {
    let result =
        strategy::update_strategy_status(&db, user.user_id, strategy_id, body.status).await?;
    Ok(Json(ApiResponse::success(StrategyUpdateResponse(result))))
}

/// POST /api/v1/strategies/bulk/status
pub async fn bulk_update_status(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<BulkUpdateStatusRequest>,
) -> Result<Json<ApiResponse<StrategyBulkUpdateResponse>>, AppError> {
    let result = strategy::bulk_update_status(&db, user.user_id, body).await?;
    Ok(Json(ApiResponse::success(StrategyBulkUpdateResponse(result))))
}

/// GET /api/v1/strategies/export
pub async fn export_strategies(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<ExportParams>,
) -> Result<Json<ApiResponse<StrategyExportResponse>>, AppError> {
    let result = strategy::export_strategies(&db, user.user_id, params.status.as_deref()).await?;
    Ok(Json(ApiResponse::success(StrategyExportResponse(result))))
}

/// POST /api/v1/strategies/import
pub async fn import_strategy(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<ImportStrategyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<StrategyCreateResponse>>), AppError> {
    let result = strategy::import_strategy(&db, user.user_id, body).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(StrategyCreateResponse(result)))))
}

/// POST /api/v1/strategies/import (batch, JSON array body)
/// Handles frontend importStrategies(data: CreateStrategyPayload[])
pub async fn import_strategies_batch(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<ImportBatchRequest>,
) -> Result<Json<ApiResponse<StrategyImportResponse>>, AppError> {
    let result = strategy::import_batch(&db, user.user_id, body).await?;
    Ok(Json(ApiResponse::success(StrategyImportResponse {
        imported: result.imported,
        errors: result.errors,
    })))
}
