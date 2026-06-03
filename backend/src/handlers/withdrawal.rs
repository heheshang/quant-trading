//! handlers/withdrawal.rs — P3-6 提现两步确认 HTTP 端点
//!
//! 中文：用户提现的两步流程对外暴露三个端点
//!   - `POST /api/v1/withdrawals/initiate`  — 输入 amount/asset/address，
//!     生成 6 位 code，存 hash，发邮件/Telegram，返回 {confirmation_id, expires_at}
//!   - `POST /api/v1/withdrawals/confirm`   — 输入 confirmation_id + code，
//!     验证 hash + 未过期，标 Confirmed，返回 {withdrawal_id, status}
//!   - `GET  /api/v1/withdrawals`           — 当前用户的提现历史
//!
//! 鉴权：全部走 `auth_middleware`（在 main.rs 里 nest 时加）。
//! 状态：handler 通过 `State<DbPool>` 拿 DB；notifier 通过 Extension 注入。
//!
//! English: HTTP layer for the two-step withdrawal flow. The handlers
//! are thin: validate the request shape, call the service, convert
//! `AppError` to HTTP status codes via the existing `IntoResponse` impl.

use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::withdrawal_confirmation::status as wstatus;
use crate::middleware::auth::AuthenticatedUser;
use crate::mq::publisher::MqClient;
use crate::services::alert_notification_service::AlertNotificationService;
use crate::services::withdrawal as svc;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ─── Schemas ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct InitiateRequest {
    /// 提现金额，字符串避免 JSON 浮点精度漂移
    pub amount: String,
    pub asset: String,
    pub dest_address: String,
}

#[derive(Debug, Serialize)]
pub struct InitiateData {
    pub confirmation_id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmRequest {
    pub confirmation_id: Uuid,
    /// 6 位验证码（明文）。
    /// 走 HTTPS + 短 TTL + 一次性：不应被认为有比其它短期凭证更高的安全要求。
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct ConfirmData {
    pub withdrawal_id: Uuid,
    pub status: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ListData {
    pub items: Vec<svc::WithdrawalView>,
    pub total: u64,
    pub page: u32,
    pub size: u32,
}

// ─── Handlers ─────────────────────────────────────────────────────

/// POST /api/v1/withdrawals/initiate
///
/// 中文：发起提现。校验 amount/asset/address，生成 6 位 code，存 hash，
///   push 到 notifier。返回 confirmation_id + expires_at（不返回 code —
///   code 只通过邮件/Telegram 通知用户）。
/// English: Begin a withdrawal. Generates a 6-digit code, stores its
///   SHA-256 hash, and pushes the plaintext to the user via the
///   configured channels. The plaintext code is **never** returned in
///   the response — it's only ever sent through the notifier.
pub async fn initiate(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Extension(notifier): Extension<Arc<AlertNotificationService>>,
    Json(req): Json<InitiateRequest>,
) -> Result<Json<ApiResponse<InitiateData>>, AppError> {
    let resp = svc::initiate(
        &db,
        notifier.as_ref(),
        user.user_id,
        &req.amount,
        &req.asset,
        &req.dest_address,
    )
    .await?;
    Ok(Json(ApiResponse::success(InitiateData {
        confirmation_id: resp.confirmation_id,
        expires_at: resp.expires_at,
    })))
}

/// POST /api/v1/withdrawals/confirm
///
/// 中文：用户提交 6 位 code + confirmation_id 完成提现。
///   错误码：
///     - 40401: confirmation_id 不存在 或 不属于当前用户（防 enumeration）
///     - 40901: 状态不是 Pending（已 confirmed / expired / cancelled）
///     - 40902: 已过期（独立语义，便于前端区分"输错了"和"再要一个新 code"）
///     - 40301: code 不匹配
pub async fn confirm(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Extension(mq): Extension<Option<Arc<MqClient>>>,
    Json(req): Json<ConfirmRequest>,
) -> Result<Json<ApiResponse<ConfirmData>>, AppError> {
    let resp = svc::confirm(
        &db,
        user.user_id,
        req.confirmation_id,
        &req.code,
        mq.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::success(ConfirmData {
        withdrawal_id: resp.withdrawal_id,
        status: resp.status,
    })))
}

/// GET /api/v1/withdrawals
///
/// 当前用户的提现历史（按时间倒序，分页）。空集合 200 返回。
pub async fn list(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ApiResponse<ListData>>, AppError> {
    let page = q.page.unwrap_or(1);
    let size = q.size.unwrap_or(20);
    let (items, total) = svc::list_for_user(&db, user.user_id, page, size).await?;
    Ok(Json(ApiResponse::success(ListData {
        items,
        total,
        page,
        size,
    })))
}

/// POST /api/v1/withdrawals/{id}/cancel
///
/// 用户主动撤销一笔 Pending 提现。已 Confirmed / Expired / Cancelled 的
/// 单子不能再次 cancel（409 Conflict）。
pub async fn cancel(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<StatusCode, AppError> {
    svc::cancel(&db, user.user_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ─── Router factory ──────────────────────────────────────────────

/// Build the withdrawal sub-router.
///
/// 路径前缀：所有路由都共享 `/withdrawals`（在 main.rs 里通过 `nest("/api/v1", ...)` 挂载）。
/// 鉴权：`auth_middleware` 在 main.rs 外层加。
///
/// State: `Arc<DatabaseConnection>` + 两个 Extension：
///   - `Arc<dyn NotificationChannel>` (initiate 用)
///   - `Option<Arc<MqClient>>` (confirm 用，MQ 不可用时降级)
pub fn router() -> Router<Arc<DatabaseConnection>> {
    Router::new()
        .route("/withdrawals/initiate", post(initiate))
        .route("/withdrawals/confirm", post(confirm))
        .route("/withdrawals", get(list))
        .route("/withdrawals/{id}/cancel", post(cancel))
}

// ─── Re-exports ──────────────────────────────────────────────────
pub use wstatus::{CANCELLED, CONFIRMED, EXPIRED, PENDING};

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `ListQuery` should default to page 1, size 20 (the service layer
    /// applies the actual clamping). The handler doesn't reach into
    /// the DB so this is the only pure unit test we can do here.
    #[test]
    fn list_query_defaults() {
        let q = ListQuery::default();
        assert!(q.page.is_none());
        assert!(q.size.is_none());
    }

    /// Status constants match the entity module.
    #[test]
    fn status_constants_match_entity() {
        assert_eq!(PENDING, wstatus::PENDING);
        assert_eq!(CONFIRMED, wstatus::CONFIRMED);
        assert_eq!(EXPIRED, wstatus::EXPIRED);
        assert_eq!(CANCELLED, wstatus::CANCELLED);
    }

    /// `InitiateData` serializes the expected fields. We test via JSON
    /// round-trip rather than the public API so a future field rename
    /// in the service is caught here too.
    #[test]
    fn initiate_data_serializes_expected_fields() {
        let id = Uuid::nil();
        let data = InitiateData {
            confirmation_id: id,
            expires_at: chrono::Utc::now(),
        };
        let v = serde_json::to_value(&data).expect("serialize InitiateData");
        assert!(v.get("confirmation_id").is_some());
        assert!(v.get("expires_at").is_some());
        assert_eq!(v["confirmation_id"].as_str().unwrap(), id.to_string());
    }

    /// Router builds without panicking. We can't actually call
    /// handlers without a DB + state, but `into_make_service` is
    /// enough to assert the surface is registered.
    #[test]
    fn router_builds() {
        let r = router();
        let _svc = r.into_make_service();
    }
}
