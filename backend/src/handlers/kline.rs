use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{
    KlineCleanRequest, KlineCleanResponse, KlineCsvImportResponse, KlineExportParams,
    KlineFetchResponse, KlineImportHistoryResponse, KlineImportRequest, KlineImportResponse,
    KlineLatestResponse, KlineQualityResponse, KlineQueryParams, KlineQueryResponse,
    KlineRollbackResponse, KlineSymbolListResponse,
};
use crate::services::kline;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
    response::IntoResponse,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;

/// GET /api/v1/kline/latest
pub async fn get_latest_kline(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<KlineLatestParams>,
) -> Result<Json<ApiResponse<KlineLatestResponse>>, AppError> {
    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let result = kline::get_latest_kline(&db, user.user_id, symbol, interval).await?;
    Ok(Json(ApiResponse::success(KlineLatestResponse(result))))
}

#[derive(Debug, Deserialize)]
pub struct KlineLatestParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
}

/// GET /api/v1/kline/symbols
pub async fn list_symbols(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<KlineSymbolListResponse>>, AppError> {
    let result = kline::list_symbols(&db, user.user_id).await?;
    Ok(Json(ApiResponse::success(KlineSymbolListResponse(result))))
}

/// POST /api/v1/kline/fetch
pub async fn fetch_klines(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<KlineFetchRequest>,
) -> Result<Json<ApiResponse<KlineFetchResponse>>, AppError> {
    let result = kline::fetch_klines(&db, user.user_id, &body.symbol, &body.interval).await?;
    Ok(Json(ApiResponse::success(KlineFetchResponse(result))))
}

#[derive(Debug, Deserialize)]
pub struct KlineFetchRequest {
    pub symbol: String,
    pub interval: String,
}

/// DELETE /api/v1/kline/clean/rollback
pub async fn rollback_clean(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<KlineRollbackResponse>>, AppError> {
    let result = kline::rollback_clean(&db, user.user_id).await?;
    Ok(Json(ApiResponse::success(KlineRollbackResponse(result))))
}

/// POST /api/v1/kline/import/csv
pub async fn import_csv(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<ApiResponse<KlineCsvImportResponse>>, AppError> {
    // Extract fields and file from multipart
    let mut symbol = String::new();
    let mut interval = String::new();
    let mut csv_content = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read multipart field: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "symbol" => {
                symbol = field
                    .text()
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to read symbol: {}", e)))?;
            }
            "interval" => {
                interval = field
                    .text()
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to read interval: {}", e)))?;
            }
            "file" => {
                csv_content = field
                    .text()
                    .await
                    .map_err(|e| AppError::Internal(format!("Failed to read CSV file: {}", e)))?;
            }
            _ => {}
        }
    }

    if symbol.trim().is_empty() {
        return Err(AppError::Validation("symbol is required".into()));
    }
    if interval.trim().is_empty() {
        return Err(AppError::Validation("interval is required".into()));
    }
    if csv_content.trim().is_empty() {
        return Err(AppError::Validation("CSV file is required".into()));
    }

    let result = kline::import_csv(&db, user.user_id, &symbol, &interval, &csv_content).await?;
    Ok(Json(ApiResponse::success(KlineCsvImportResponse(result))))
}

/// GET /api/v1/kline/query
pub async fn query_klines(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<KlineQueryParams>,
) -> Result<Json<ApiResponse<KlineQueryResponse>>, AppError> {
    let result = kline::query_klines(&db, user.user_id, params).await?;
    Ok(Json(ApiResponse::success(KlineQueryResponse(result))))
}

/// POST /api/v1/kline/import
pub async fn import_klines(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<KlineImportRequest>,
) -> Result<Json<ApiResponse<KlineImportResponse>>, AppError> {
    let result = kline::import_klines(&db, user.user_id, body).await?;
    Ok(Json(ApiResponse::success(KlineImportResponse(result))))
}

/// GET /api/v1/kline/import-history
pub async fn import_history(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<KlineImportHistoryResponse>>, AppError> {
    let result = kline::import_history(&db, user.user_id).await?;
    Ok(Json(ApiResponse::success(KlineImportHistoryResponse(
        result,
    ))))
}

/// GET /api/v1/kline/quality
pub async fn quality_report(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<KlineQueryParams>,
) -> Result<Json<ApiResponse<KlineQualityResponse>>, AppError> {
    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let result = kline::quality_report(&db, user.user_id, symbol, interval).await?;
    Ok(Json(ApiResponse::success(KlineQualityResponse(result))))
}

/// POST /api/v1/kline/clean
pub async fn clean_klines(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<KlineCleanRequest>,
) -> Result<Json<ApiResponse<KlineCleanResponse>>, AppError> {
    let result = kline::clean_klines(&db, user.user_id, body).await?;
    Ok(Json(ApiResponse::success(KlineCleanResponse(result))))
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
