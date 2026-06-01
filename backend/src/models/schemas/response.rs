//! 各类 Handler 响应体 DTO

use serde::Serialize;

// `JwtClaims` and `PaginationParams` are referenced only from the
// `#[cfg(test)] mod tests` block in handlers (e.g. test fixtures that
// construct an auth token). They are not used by the production code path,
// so gate them behind `#[cfg(test)]` to keep `cargo check` clean.
#[cfg(test)]
use super::auth::{JwtClaims, PaginationParams};
use super::auth::{PaginatedResponse, RoleResponse, UserResponse};
use super::market::{
    KlineCleanResult, KlineImportLogResponse, KlineImportResult, KlineListResponse,
    KlineQualityReport, KlineResponse,
};
use super::strategy::{StrategyResponse, TemplateInfo};

// `TickerResponse` is defined in `crate::models::market_schemas` (legacy
// module). Re-import here so handler response types can reference it.
use crate::models::market_schemas::TickerResponse;

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

