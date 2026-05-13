use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{
    KlineCleanRequest, KlineExportParams, KlineImportRequest, KlineQueryParams,
};
use crate::services::kline;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// GET /api/v1/kline/query
pub async fn query_klines(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<KlineQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = kline::query_klines(&db, user.user_id, params).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result).map_err(
        |e| AppError::Internal(format!("serialization error: {}", e)),
    )?)))
}

/// POST /api/v1/kline/import
pub async fn import_klines(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<KlineImportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = kline::import_klines(&db, user.user_id, body).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result).map_err(
        |e| AppError::Internal(format!("serialization error: {}", e)),
    )?)))
}

/// GET /api/v1/kline/import-history
pub async fn import_history(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = kline::import_history(&db, user.user_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result).map_err(
        |e| AppError::Internal(format!("serialization error: {}", e)),
    )?)))
}

/// GET /api/v1/kline/quality
pub async fn quality_report(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<KlineQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let result = kline::quality_report(&db, user.user_id, symbol, interval).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result).map_err(
        |e| AppError::Internal(format!("serialization error: {}", e)),
    )?)))
}

/// POST /api/v1/kline/clean
pub async fn clean_klines(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<KlineCleanRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = kline::clean_klines(&db, user.user_id, body).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result).map_err(
        |e| AppError::Internal(format!("serialization error: {}", e)),
    )?)))
}

/// GET /api/v1/kline/export
pub async fn export_klines(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<KlineExportParams>,
) -> Result<impl IntoResponse, AppError> {
    let csv_content = kline::export_klines(&db, user.user_id, params).await?;

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "text/csv".parse().unwrap(),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        "attachment; filename=\"klines.csv\"".parse().unwrap(),
    );

    Ok((headers, csv_content).into_response())
}
