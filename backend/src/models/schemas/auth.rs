// Re-export market schemas for convenience
pub use crate::models::market_schemas::*;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============ Auth Request/Response ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
}

// ============ User ============

#[derive(Debug, Serialize, Deserialize, ToSchema)]
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AdminUpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: Option<bool>,
    pub role_id: Option<Uuid>,
}

// ============ Role ============

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RoleResponse {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
}

// ============ Permission ============

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PermissionResponse {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub resource: String,
    pub action: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePermissionRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub resource: String,
    pub action: String,
}

// ============ Pagination ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationParams {
    pub page: Option<u64>,
    pub size: Option<u64>,
}

// `PaginatedResponse<T>` is generic; the OpenAPI spec emits it as a schema
// with no typed `items`. We intentionally skip `ToSchema` here because utoipa's
// derive macro for generics requires the `T: utoipa::ToSchema` bound, which we
// don't want to impose project-wide. The two response wrappers that use it
// (`UserListResponse`, `StrategyListResponse`) are typed as the `response.rs`
// newtype; their `ToSchema` impls reference `PaginatedResponse<T>` and would
// fail to compile. The follow-up task t_<batch-A> covers adding the bound +
// composed schema. For now, those two endpoints will appear in the OpenAPI
// spec but their response body will be marked as `object` (untyped), which is
// still usable from `openapi-fetch` with manual `as any` casts on the FE.
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

