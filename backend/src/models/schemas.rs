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
    pub open_time: i64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub close_time: Option<i64>,
    pub quote_volume: Option<String>,
    pub trades: Option<i64>,
    pub source: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KlineListMeta {
    pub total: u64,
    pub page: u64,
    pub size: u64,
    pub gap_detected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct KlineListResponse {
    pub items: Vec<KlineResponse>,
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
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    #[serde(default)]
    pub close_time: Option<i64>,
    #[serde(default)]
    pub quote_volume: Option<String>,
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
