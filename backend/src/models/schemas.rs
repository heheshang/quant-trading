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
    pub sub: String,         // user id
    pub username: String,
    pub role: String,
    pub exp: usize,          // expiry timestamp
    pub iat: usize,          // issued at
    pub jti: String,         // token id (for revocation)
    pub token_type: String,  // "access" or "refresh"
}

// ============ Strategy ============

#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: String,
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
    pub template_type: String,
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

#[derive(Debug, Serialize)]
pub struct TemplateInfo {
    pub id: String,
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
        let params = PaginationParams { page: None, size: None };
        assert_eq!(params.page(), 1);
        assert_eq!(params.size(), 20);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_custom_values() {
        let params = PaginationParams { page: Some(3), size: Some(10) };
        assert_eq!(params.page(), 3);
        assert_eq!(params.size(), 10);
        assert_eq!(params.offset(), 20);
    }

    #[test]
    fn test_pagination_clamps_min_page() {
        let params = PaginationParams { page: Some(0), size: None };
        assert_eq!(params.page(), 1);
    }

    #[test]
    fn test_pagination_clamps_size_range() {
        let too_small = PaginationParams { page: None, size: Some(0) };
        assert_eq!(too_small.size(), 1);

        let too_large = PaginationParams { page: None, size: Some(200) };
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
