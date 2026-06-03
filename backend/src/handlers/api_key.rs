//! API Key 管理 Handler — P2-F1
//!
//! REST API for exchange API key CRUD + connectivity test + admin list

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::db::audit_log::{actions, target_types};
use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{PaginatedResponse, PaginationParams};
use crate::services::audit_log;
use crate::services::exchange::api_keys::ApiKeyStore;
use crate::services::exchange::signed_client::SignedBinanceClient;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ─── DTOs ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub exchange: String,
    pub api_key: String,
    pub secret: String,
    pub permissions: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApiKeyRequest {
    pub permissions: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub exchange: String,
    pub api_key: String, // masked: ***xxxx
    pub permissions: String,
    pub is_active: bool,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ApiKeyResponse {
    /// Mask the API key, showing only last 4 characters
    fn from_key(key: &crate::services::exchange::api_keys::ExchangeApiKey) -> Self {
        let masked = mask_api_key(&key.api_key);
        Self {
            id: key.id,
            user_id: key.user_id,
            exchange: key.exchange.clone(),
            api_key: masked,
            permissions: key.permissions.clone(),
            is_active: key.is_active,
            last_used_at: key.last_used_at,
            created_at: key.created_at,
        }
    }
}

fn mask_api_key(key: &str) -> String {
    if key.len() <= 4 {
        "****".to_string()
    } else {
        format!("***{}", &key[key.len() - 4..])
    }
}

#[derive(Debug, Serialize)]
pub struct ApiKeyTestResponse {
    pub success: bool,
    pub exchange: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyListResponse(pub PaginatedResponse<ApiKeyResponse>);

// ─── Handlers ────────────────────────────────────────────────────────────────

/// Helper: require admin role.
///
/// Currently unused — admin authorization is enforced at the router layer via
/// `require_admin_middleware` on the admin sub-router. Kept here for handlers
/// that need to perform an in-handler role check (e.g. self-service escalation).
#[allow(dead_code)]
fn require_admin(user: &AuthenticatedUser) -> Result<(), AppError> {
    if user.role != "admin" {
        return Err(AppError::Forbidden("Admin privileges required".into()));
    }
    Ok(())
}

/// GET /api/v1/api-keys
///
/// 列出当前用户的所有 API Key（脱敏：只返回后4位）
pub async fn list_api_keys(
    user: AuthenticatedUser,
    State(_db): State<Arc<DatabaseConnection>>,
    Extension(key_store): Extension<Arc<ApiKeyStore>>,
) -> Result<Json<ApiResponse<Vec<ApiKeyResponse>>>, AppError> {
    let keys = key_store.find_by_user(user.user_id).await?;

    let responses: Vec<ApiKeyResponse> = keys.iter().map(ApiKeyResponse::from_key).collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// POST /api/v1/api-keys
///
/// 创建新 API Key
pub async fn create_api_key(
    user: AuthenticatedUser,
    State(_db): State<Arc<DatabaseConnection>>,
    Extension(key_store): Extension<Arc<ApiKeyStore>>,
    headers: HeaderMap,
    Json(body): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ApiKeyResponse>>), AppError> {
    if body.exchange.is_empty() {
        return Err(AppError::Validation("exchange is required".into()));
    }
    if body.api_key.is_empty() {
        return Err(AppError::Validation("api_key is required".into()));
    }
    if body.secret.is_empty() {
        return Err(AppError::Validation("secret is required".into()));
    }

    let permissions = body.permissions.unwrap_or_else(|| "".to_string());

    let key = key_store
        .upsert(
            user.user_id,
            &body.exchange,
            &body.api_key,
            &body.secret,
            &permissions,
        )
        .await?;

    let resp = ApiKeyResponse::from_key(&key);

    tracing::info!(
        user_id = %user.user_id,
        exchange = %body.exchange,
        "API key created"
    );

    // P3-4: 写审计 —— 不脱敏的 exchange / permissions，方便事后回查。
    // 永远不写 api_key / secret 原文 / 密文。
    let (ip, ua, req_id) = audit_log::extract_audit_context(&headers, "0.0.0.0");
    let event = audit_log::AuditEvent {
        user_id: Some(user.user_id),
        action: actions::API_KEY_CREATED.to_string(),
        target_type: target_types::API_KEY.to_string(),
        target_id: key.id.to_string(),
        before: None,
        after: Some(serde_json::json!({
            "exchange": body.exchange,
            "permissions": permissions,
            "is_active": true,
        })),
        ip_address: ip,
        user_agent: ua,
        request_id: req_id,
    };
    audit_log::record(&_db, event).await;

    Ok((StatusCode::CREATED, Json(ApiResponse::success(resp))))
}

/// GET /api/v1/api-keys/{id}
///
/// 获取单个 Key 详情
pub async fn get_api_key(
    user: AuthenticatedUser,
    State(_db): State<Arc<DatabaseConnection>>,
    Extension(key_store): Extension<Arc<ApiKeyStore>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ApiKeyResponse>>, AppError> {
    let key = key_store
        .find_by_id_and_user(id, user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("API key not found".into()))?;

    Ok(Json(ApiResponse::success(ApiKeyResponse::from_key(&key))))
}

/// PUT /api/v1/api-keys/{id}
///
/// 更新 API Key（permissions / is_active）
pub async fn update_api_key(
    user: AuthenticatedUser,
    State(_db): State<Arc<DatabaseConnection>>,
    Extension(key_store): Extension<Arc<ApiKeyStore>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<UpdateApiKeyRequest>,
) -> Result<Json<ApiResponse<ApiKeyResponse>>, AppError> {
    // P3-4: 取 before 状态用于审计 diff（updated 类事件要写 {before, after}）。
    let before_state = key_store
        .find_by_id_and_user(id, user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("API key not found".into()))?;

    let updated = key_store
        .update_by_id(
            id,
            user.user_id,
            body.permissions.as_deref(),
            body.is_active,
        )
        .await?;

    tracing::info!(
        user_id = %user.user_id,
        key_id = %id,
        "API key updated"
    );

    // P3-4: 写审计 diff
    let (ip, ua, req_id) = audit_log::extract_audit_context(&headers, "0.0.0.0");
    let event = audit_log::AuditEvent {
        user_id: Some(user.user_id),
        action: actions::API_KEY_UPDATED.to_string(),
        target_type: target_types::API_KEY.to_string(),
        target_id: id.to_string(),
        before: Some(serde_json::json!({
            "exchange": before_state.exchange,
            "permissions": before_state.permissions,
            "is_active": before_state.is_active,
        })),
        after: Some(serde_json::json!({
            "exchange": updated.exchange,
            "permissions": updated.permissions,
            "is_active": updated.is_active,
        })),
        ip_address: ip,
        user_agent: ua,
        request_id: req_id,
    };
    audit_log::record(&_db, event).await;

    Ok(Json(ApiResponse::success(ApiKeyResponse::from_key(
        &updated,
    ))))
}

/// DELETE /api/v1/api-keys/{id}
///
/// 删除 API Key
pub async fn delete_api_key(
    user: AuthenticatedUser,
    State(_db): State<Arc<DatabaseConnection>>,
    Extension(key_store): Extension<Arc<ApiKeyStore>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // P3-4: 删除前先取一份"被删的是谁"——deleted 类事件也要记业务字段。
    let before = key_store
        .find_by_id_and_user(id, user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("API key not found".into()))?;

    key_store.delete_by_id(id, user.user_id).await?;

    tracing::info!(
        user_id = %user.user_id,
        key_id = %id,
        "API key deleted"
    );

    // P3-4: 写审计
    let (ip, ua, req_id) = audit_log::extract_audit_context(&headers, "0.0.0.0");
    let event = audit_log::AuditEvent {
        user_id: Some(user.user_id),
        action: actions::API_KEY_DELETED.to_string(),
        target_type: target_types::API_KEY.to_string(),
        target_id: id.to_string(),
        before: Some(serde_json::json!({
            "exchange": before.exchange,
            "permissions": before.permissions,
            "is_active": before.is_active,
        })),
        after: Some(serde_json::json!({ "deleted_id": id.to_string() })),
        ip_address: ip,
        user_agent: ua,
        request_id: req_id,
    };
    audit_log::record(&_db, event).await;

    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/api-keys/{id}/test
///
/// 测试连通性（调用 /exchange/ping 或 /api/v3/account）
pub async fn test_api_key(
    user: AuthenticatedUser,
    State(_db): State<Arc<DatabaseConnection>>,
    Extension(key_store): Extension<Arc<ApiKeyStore>>,
    Extension(signed_client): Extension<Arc<SignedBinanceClient>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ApiKeyTestResponse>>, AppError> {
    let key = key_store
        .find_by_id_and_user(id, user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("API key not found".into()))?;

    // Try to call the exchange via the signed client
    match signed_client.ping().await {
        Ok(resp) => {
            // Update last used time
            let _ = key_store
                .update_last_used(user.user_id, &key.exchange)
                .await;

            Ok(Json(ApiResponse::success(ApiKeyTestResponse {
                success: true,
                exchange: key.exchange,
                message: format!("Connected. Server time: {}", resp.server_time),
            })))
        }
        Err(e) => {
            tracing::warn!(
                user_id = %user.user_id,
                key_id = %id,
                error = %e,
                "API key connectivity test failed"
            );

            Ok(Json(ApiResponse::success(ApiKeyTestResponse {
                success: false,
                exchange: key.exchange,
                message: format!("Connection failed: {}", e),
            })))
        }
    }
}

/// GET /api/v1/admin/api-keys
///
/// 管理员查看所有 Key（分页）
pub async fn admin_list_api_keys(
    _user: AuthenticatedUser,
    State(_db): State<Arc<DatabaseConnection>>,
    Extension(key_store): Extension<Arc<ApiKeyStore>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<ApiKeyListResponse>>, AppError> {
    let page = params.page();
    let size = params.size();

    let (keys, total) = key_store.list_all_paginated(page, size).await?;

    let items: Vec<ApiKeyResponse> = keys.iter().map(ApiKeyResponse::from_key).collect();

    Ok(Json(ApiResponse::success(ApiKeyListResponse(
        PaginatedResponse {
            items,
            total,
            page,
            size,
        },
    ))))
}

// ─── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key_logic() {
        // 6+ chars: show last 4
        assert_eq!(mask_api_key("ABCDEFGH1234"), "***1234");
        assert_eq!(mask_api_key("abcdefghij"), "***ghij");
        // <=4 chars: fully masked
        assert_eq!(mask_api_key("1234"), "****");
        assert_eq!(mask_api_key("abcd"), "****");
        assert_eq!(mask_api_key(""), "****");
        assert_eq!(mask_api_key("a"), "****");
        assert_eq!(mask_api_key("ab"), "****");
        assert_eq!(mask_api_key("abc"), "****");
    }

    #[test]
    fn test_api_key_response_masking() {
        use crate::services::exchange::api_keys::ExchangeApiKey;

        // SAFETY: Test fixture only — masks the last-4-char logic of from_key().
        const TEST_API_KEY: &str = "test-api-key-fixture-abcd";
        let key = ExchangeApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            exchange: "binance".to_string(),
            api_key: TEST_API_KEY.to_string(),
            secret_encrypted: "encrypted-fixture".to_string(),
            nonce: "nonce-fixture".to_string(),
            permissions: "read,trade".to_string(),
            is_active: true,
            last_used_at: None,
            created_at: chrono::Utc::now(),
        };

        let resp = ApiKeyResponse::from_key(&key);
        assert_eq!(resp.api_key, "***abcd");
        assert_eq!(resp.exchange, "binance");
        assert_eq!(resp.permissions, "read,trade");
        assert!(resp.is_active);
    }

    #[test]
    fn test_require_admin_allows_admin() {
        let admin = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "admin".to_string(),
            role: "admin".to_string(),
            jti: "jti".to_string(),
        };
        assert!(require_admin(&admin).is_ok());
    }

    #[test]
    fn test_require_admin_rejects_non_admin() {
        let user = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "alice".to_string(),
            role: "user".to_string(),
            jti: "jti".to_string(),
        };
        let result = require_admin(&user);
        assert!(result.is_err());
        match result {
            Err(AppError::Forbidden(_)) => {}
            _ => panic!("Expected Forbidden error"),
        }
    }

    #[test]
    fn test_pagination_params_defaults() {
        let params = PaginationParams {
            page: None,
            size: None,
        };
        assert_eq!(params.page(), 1);
        assert_eq!(params.size(), 20);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_params_custom() {
        let params = PaginationParams {
            page: Some(3),
            size: Some(50),
        };
        assert_eq!(params.page(), 3);
        assert_eq!(params.size(), 50);
        assert_eq!(params.offset(), 100);
    }

    #[test]
    fn test_pagination_params_clamps_size() {
        let params = PaginationParams {
            page: Some(1),
            size: Some(200),
        };
        assert_eq!(params.size(), 100); // clamped to max 100
    }

    #[test]
    fn test_pagination_params_page_min_one() {
        let params = PaginationParams {
            page: Some(0),
            size: Some(20),
        };
        assert_eq!(params.page(), 1); // min 1
    }
}
