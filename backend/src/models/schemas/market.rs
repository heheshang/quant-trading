//! K线与 WebSocket 消息 DTO

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============ K-line ============

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KlineResponse {
    pub id: i64,
    pub user_id: Uuid,
    pub symbol: String,
    pub interval: String,
    pub timestamp: i64,
    pub open_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub close_time: Option<i64>,
    pub quote_volume: Option<f64>,
    pub trades: Option<i64>,
    pub source: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KlineListMeta {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub gap_detected: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KlineListResponse {
    pub data: Vec<KlineResponse>,
    pub meta: KlineListMeta,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct KlineQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct KlineImportRequest {
    pub symbol: String,
    pub interval: String,
    pub source: String,
    pub data: Vec<KlineImportItem>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct KlineImportItem {
    pub open_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    #[serde(default)]
    pub close_time: Option<i64>,
    #[serde(default)]
    pub quote_volume: Option<f64>,
    #[serde(default)]
    pub trades: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KlineImportResult {
    pub imported_rows: i64,
    pub duplicate_rows: i64,
    pub failed_rows: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KlineImportLogResponse {
    pub id: i64,
    pub user_id: Uuid,
    pub symbol: String,
    pub interval: String,
    pub source: String,
    pub total_rows: i64,
    pub imported_rows: i64,
    pub duplicate_rows: i64,
    pub failed_rows: i64,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KlineQualityAnomaly {
    pub open_time: i64,
    pub anomaly_type: String,
    pub open: Option<String>,
    pub high: Option<String>,
    pub low: Option<String>,
    pub close: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KlineQualityReport {
    pub symbol: String,
    pub interval: String,
    pub total_rows: i64,
    pub valid_rows: i64,
    pub coverage_pct: f64,
    pub gap_count: i64,
    pub anomaly_count: i64,
    pub duplicate_count: i64,
    pub anomalies: Vec<KlineQualityAnomaly>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct KlineCleanRequest {
    pub symbol: String,
    pub interval: String,
    pub clean_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct KlineCleanResult {
    pub removed_count: i64,
    pub backup_count: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct KlineExportParams {
    pub symbol: String,
    pub interval: String,
    pub format: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

// ============ WS ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct WsQueryParams {
    pub token: String,
}
