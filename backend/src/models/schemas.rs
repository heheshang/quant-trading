// Re-export market schemas for convenience
pub use crate::models::market_schemas::*;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============ Auth Request/Response ============

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

// ============ User ============

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub role: RoleResponse,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminUpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: Option<bool>,
    pub role_id: Option<Uuid>,
}

// ============ Role ============

#[derive(Debug, Serialize, Deserialize)]
pub struct RoleResponse {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
}

// ============ Permission ============

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionResponse {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub resource: String,
    pub action: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePermissionRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub resource: String,
    pub action: String,
}

// ============ Pagination ============

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u64>,
    pub size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
}

impl PaginationParams {
    pub fn page(&self) -> u64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn size(&self) -> u64 {
        self.size.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> u64 {
        (self.page() - 1) * self.size()
    }
}

// ============ JWT Claims ============

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String, // user id
    pub username: String,
    pub role: String,
    pub exp: usize,         // expiry timestamp
    pub iat: usize,         // issued at
    pub jti: String,        // token id (for revocation)
    pub token_type: String, // "access" or "refresh"
}

// ============ Strategy ============

#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: String,
    pub symbol: String,
    pub timeframe: String,
    pub strategy_type: String,
    pub template_id: Uuid,
    pub template_type: String,
    pub parameters: serde_json::Value,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateStrategyRequest {
    pub name: String,
    pub description: Option<String>,
    pub symbol: String,
    pub timeframe: String,
    pub strategy_type: String,
    pub template_id: Uuid,
    #[serde(alias = "template_type")]
    pub template_type: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStrategyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct BulkUpdateStatusRequest {
    pub ids: Vec<uuid::Uuid>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ExportParams {
    pub status: Option<String>,
    #[allow(dead_code)]
    pub format: Option<String>, // currently only json is supported
}

#[derive(Debug, Deserialize)]
pub struct ImportStrategyRequest {
    pub name: String,
    pub description: Option<String>,
    pub symbol: String,
    pub timeframe: String,
    pub strategy_type: String,
    pub template_id: Uuid,
    #[serde(alias = "template_type")]
    pub template_type: Option<String>,
    pub parameters: serde_json::Value,
    /// Optional: override status on import (default: "draft")
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportBatchRequest {
    pub strategies: Vec<ImportStrategyRequest>,
}

#[derive(Debug, Serialize)]
pub struct ImportBatchResponse {
    pub imported: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TemplateInfo {
    pub id: String,
    pub template_id: Uuid, // ADR D6: deterministic UUID for this template
    pub name: String,
    pub description: String,
    pub category: String,
    pub default_parameters: serde_json::Value,
    pub parameter_schema: Vec<ParameterDef>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParameterDef {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub label: String,
    pub description: String,
    pub default: serde_json::Value,
    pub min: Option<serde_json::Value>,
    pub max: Option<serde_json::Value>,
    pub options: Option<Vec<String>>,
}

// ============ K-line ============

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct KlineListMeta {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub gap_detected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct KlineListResponse {
    pub data: Vec<KlineResponse>,
    pub meta: KlineListMeta,
}

#[derive(Debug, Deserialize)]
pub struct KlineQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct KlineImportRequest {
    pub symbol: String,
    pub interval: String,
    pub source: String,
    pub data: Vec<KlineImportItem>,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct KlineImportResult {
    pub imported_rows: i64,
    pub duplicate_rows: i64,
    pub failed_rows: i64,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct KlineQualityAnomaly {
    pub open_time: i64,
    pub anomaly_type: String,
    pub open: Option<String>,
    pub high: Option<String>,
    pub low: Option<String>,
    pub close: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Deserialize)]
pub struct KlineCleanRequest {
    pub symbol: String,
    pub interval: String,
    pub clean_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KlineCleanResult {
    pub removed_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct KlineExportParams {
    pub symbol: String,
    pub interval: String,
    pub format: Option<String>,
    #[allow(dead_code)]
    pub start_time: Option<i64>,
    #[allow(dead_code)]
    pub end_time: Option<i64>,
}

// ============ WS ============

#[derive(Debug, Deserialize)]
pub struct WsQueryParams {
    pub token: String,
}

// ============ Handler Response Types ============

// --- Auth ---
#[derive(Debug, Serialize)]
pub struct AuthResponseBody {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize)]
pub struct TokenResponseBody {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

// --- Strategy ---
#[derive(Debug, Serialize)]
pub struct TemplateListResponse(pub Vec<TemplateInfo>);

#[derive(Debug, Serialize)]
pub struct StrategyListResponse(pub PaginatedResponse<StrategyResponse>);

#[derive(Debug, Serialize)]
pub struct StrategyBulkUpdateResponse(pub Vec<StrategyResponse>);

#[derive(Debug, Serialize)]
pub struct StrategyCreateResponse(pub StrategyResponse);

#[derive(Debug, Serialize)]
pub struct StrategyUpdateResponse(pub StrategyResponse);

#[derive(Debug, Serialize)]
pub struct StrategyExportResponse(pub Vec<StrategyResponse>);

#[derive(Debug, Serialize)]
pub struct StrategyImportResponse {
    pub imported: usize,
    pub errors: Vec<String>,
}

// --- Kline ---
#[derive(Debug, Clone, Serialize)]
pub struct KlineSymbolOverview {
    pub symbol: String,
    pub interval: String,
    pub data_points: i64,
    pub coverage_start: i64,
    pub coverage_end: i64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub quality: String,
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KlineLatestResponse(pub Option<KlineResponse>);

#[derive(Debug, Serialize)]
pub struct KlineSymbolListResponse(pub Vec<crate::services::kline::KlineSymbolOverview>);

#[derive(Debug, Serialize)]
pub struct KlineFetchResponse(pub KlineImportResult);

#[derive(Debug, Serialize)]
pub struct KlineRollbackResponse(pub KlineCleanResult);

#[derive(Debug, Serialize)]
pub struct KlineCsvImportResponse(pub KlineImportResult);

#[derive(Debug, Serialize)]
pub struct KlineQueryResponse(pub KlineListResponse);

#[derive(Debug, Serialize)]
pub struct KlineImportResponse(pub KlineImportResult);

#[derive(Debug, Serialize)]
pub struct KlineImportHistoryResponse(pub Vec<KlineImportLogResponse>);

#[derive(Debug, Serialize)]
pub struct KlineQualityResponse(pub KlineQualityReport);

#[derive(Debug, Serialize)]
pub struct KlineCleanResponse(pub KlineCleanResult);

// --- User ---
#[derive(Debug, Serialize)]
pub struct UserListResponse(pub PaginatedResponse<UserResponse>);

#[derive(Debug, Serialize)]
pub struct UserMeResponse(pub UserResponse);

#[derive(Debug, Serialize)]
pub struct UserUpdateResponse(pub UserResponse);

#[derive(Debug, Serialize)]
pub struct ChangePasswordResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct UserDeleteResponse {
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RoleListResponse(pub Vec<RoleResponse>);

// --- Market ---
#[derive(Debug, Serialize)]
pub struct TickerListResponse(pub Vec<TickerResponse>);

#[derive(Debug, Serialize)]
pub struct TickerSingleResponse(pub TickerResponse);

#[derive(Debug, Serialize)]
pub struct TickerHistoryResponse(pub serde_json::Value);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_defaults() {
        let params = PaginationParams {
            page: None,
            size: None,
        };
        assert_eq!(params.page(), 1);
        assert_eq!(params.size(), 20);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_custom_values() {
        let params = PaginationParams {
            page: Some(3),
            size: Some(10),
        };
        assert_eq!(params.page(), 3);
        assert_eq!(params.size(), 10);
        assert_eq!(params.offset(), 20);
    }

    #[test]
    fn test_pagination_clamps_min_page() {
        let params = PaginationParams {
            page: Some(0),
            size: None,
        };
        assert_eq!(params.page(), 1);
    }

    #[test]
    fn test_pagination_clamps_size_range() {
        let too_small = PaginationParams {
            page: None,
            size: Some(0),
        };
        assert_eq!(too_small.size(), 1);

        let too_large = PaginationParams {
            page: None,
            size: Some(200),
        };
        assert_eq!(too_large.size(), 100);
    }

    #[test]
    fn test_jwt_claims_roundtrip() {
        let claims = JwtClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".into(),
            username: "alice".into(),
            role: "admin".into(),
            exp: 9999999999,
            iat: 1000000000,
            jti: "unique-id".into(),
            token_type: "access".into(),
        };

        let json = serde_json::to_value(&claims).unwrap();
        assert_eq!(json["sub"], "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(json["username"], "alice");
        assert_eq!(json["role"], "admin");
        assert_eq!(json["token_type"], "access");

        // Deserialize back
        let deserialized: JwtClaims = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.sub, claims.sub);
        assert_eq!(deserialized.jti, claims.jti);
    }

    #[test]
    fn test_paginated_response_serialization() {
        let resp: PaginatedResponse<String> = PaginatedResponse {
            items: vec!["a".into(), "b".into()],
            total: 2,
            page: 1,
            size: 20,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["items"].as_array().unwrap().len(), 2);
        assert_eq!(json["total"], 2);
        assert_eq!(json["page"], 1);
        assert_eq!(json["size"], 20);
    }
}

// ============ KDJ Indicator ============

/// KDJ signal type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum KdjSignal {
    GoldenCross,
    DeathCross,
    Overbought,
    Oversold,
    #[default]
    None,
}

/// Single KDJ bar result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdjBar {
    pub open_time: i64,
    pub k: f64,
    pub d: f64,
    pub j: f64,
    pub signal: KdjSignal,
}

/// KDJ calculation parameters
#[derive(Debug, Clone, Serialize)]
pub struct KdjParams {
    pub n: usize,
    pub m1: usize,
    pub m2: usize,
}

/// KDJ API response
#[derive(Debug, Clone, Serialize)]
pub struct KdjResponse {
    pub data: Vec<KdjBar>,
    pub params: KdjParams,
    pub symbol: String,
    pub interval: String,
}

/// KDJ query parameters (incoming from HTTP)
#[derive(Debug, Deserialize)]
pub struct KdjQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub n: Option<usize>,
    pub m1: Option<usize>,
    pub m2: Option<usize>,
}

// ─── MA (Moving Average) ────────────────────────────────────────────────────

/// Single MA line result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaBar {
    pub open_time: i64,
    pub ma: f64,
}

/// MA API response
#[derive(Debug, Clone, Serialize)]
pub struct MaResponse {
    pub data: Vec<MaBar>,
    pub period: usize,
    pub symbol: String,
    pub interval: String,
}

/// MA query parameters
#[derive(Debug, Deserialize)]
pub struct MaQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub period: Option<usize>,
}

// ─── MACD ─────────────────────────────────────────────────────────────────────

/// Single MACD bar result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacdBar {
    pub open_time: i64,
    pub macd: f64,      // MACD line value
    pub signal: f64,    // Signal line value
    pub histogram: f64, // MACD - Signal
}

/// MACD API response
#[derive(Debug, Clone, Serialize)]
pub struct MacdResponse {
    pub data: Vec<MacdBar>,
    pub params: MacdParams,
    pub symbol: String,
    pub interval: String,
}

/// MACD calculation parameters
#[derive(Debug, Clone, Serialize)]
pub struct MacdParams {
    pub fast_period: usize,
    pub slow_period: usize,
    pub signal_period: usize,
}

/// MACD query parameters
#[derive(Debug, Deserialize)]
pub struct MacdQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub fast_period: Option<usize>,
    pub slow_period: Option<usize>,
    pub signal_period: Option<usize>,
}

// ─── RSI ─────────────────────────────────────────────────────────────────────

/// Single RSI bar result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RsiBar {
    pub open_time: i64,
    pub rsi: f64,
}

/// RSI API response
#[derive(Debug, Clone, Serialize)]
pub struct RsiResponse {
    pub data: Vec<RsiBar>,
    pub period: usize,
    pub symbol: String,
    pub interval: String,
}

/// RSI query parameters
#[derive(Debug, Deserialize)]
pub struct RsiQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub period: Option<usize>,
}

// ─── Bollinger Bands ────────────────────────────────────────────────────────

/// Single Bollinger Bands bar result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BollingerBar {
    pub open_time: i64,
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

/// Bollinger Bands API response
#[derive(Debug, Clone, Serialize)]
pub struct BollingerResponse {
    pub data: Vec<BollingerBar>,
    pub params: BollingerParams,
    pub symbol: String,
    pub interval: String,
}

/// Bollinger Bands parameters
#[derive(Debug, Clone, Serialize)]
pub struct BollingerParams {
    pub period: usize,
    pub std_dev: f64,
}

/// Bollinger Bands query parameters
#[derive(Debug, Deserialize)]
pub struct BollingerQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub period: Option<usize>,
    pub std_dev: Option<f64>,
}

/// EMA bar response
#[derive(Debug, Clone, Serialize)]
pub struct EmaResponse {
    pub data: Vec<MaBar>,
    pub period: usize,
    pub symbol: String,
    pub interval: String,
}

/// EMA query parameters
#[derive(Debug, Deserialize)]
pub struct EmaQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub period: Option<usize>,
}

/// ATR bar response
#[derive(Debug, Clone, Serialize)]
pub struct AtrResponse {
    pub data: Vec<RsiBar>,
    pub period: usize,
    pub symbol: String,
    pub interval: String,
}

/// ATR query parameters
#[derive(Debug, Deserialize)]
pub struct AtrQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub period: Option<usize>,
}

/// Stochastic bar
#[derive(Debug, Clone, Serialize)]
pub struct StochasticBar {
    pub open_time: i64,
    pub k: f64,
    pub d: f64,
}

/// Stochastic params
#[derive(Debug, Clone, Serialize)]
pub struct StochasticParams {
    pub k_period: usize,
    pub d_period: usize,
    pub smooth_k: usize,
}

/// Stochastic API response
#[derive(Debug, Clone, Serialize)]
pub struct StochasticResponse {
    pub data: Vec<StochasticBar>,
    pub params: StochasticParams,
    pub symbol: String,
    pub interval: String,
}

/// Stochastic query parameters
#[derive(Debug, Deserialize)]
pub struct StochasticQueryParams {
    pub symbol: Option<String>,
    pub interval: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub k_period: Option<usize>,
    pub d_period: Option<usize>,
    pub smooth_k: Option<usize>,
}
