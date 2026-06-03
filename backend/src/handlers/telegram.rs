//! handlers/telegram.rs — P2-1 Telegram 通知 HTTP 端点
//!
//! 提供两个端点：
//!   POST /api/v1/notifications/telegram/bind  — 绑定 user 的 chat_id
//!   POST /api/v1/notifications/telegram/test  — 给已绑定的 user 发测试消息
//!
//! 中文：bind 接收前端从 Telegram 拿到的 chat_id，test 用 AlertNotificationService
//!   多播推送一条 info 级别消息以验证整条通路。
//! English: bind takes the chat_id obtained from the Telegram client, test pushes
//!   an info-level alert via AlertNotificationService to validate the full path.

use axum::{
    Extension, Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[cfg(test)]
use uuid::Uuid;

use crate::db::audit_log::{actions, target_types};
use crate::db::user::{ActiveModel as UserActive, Column as UserCol, Entity as UserEntity};
use crate::middleware::auth::AuthenticatedUser;
use crate::services::alert_notification_service::AlertNotificationService;
use crate::services::audit_log;
use crate::services::notification::AlertNotification;
use crate::state::TelegramChatIdCache;
use crate::utils::error::AppError;

// ─── Schemas ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BindRequest {
    /// Telegram chat_id（来自 @userinfobot 或前端登录流程）
    /// Telegram chat_id (from @userinfobot or front-end login flow).
    pub chat_id: String,
}

#[derive(Debug, Serialize)]
pub struct BindResponse {
    pub code: i32,
    pub data: BindData,
}

#[derive(Debug, Serialize)]
pub struct BindData {
    pub user_id: String,
    pub chat_id: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct TestRequest {
    /// 可选：自定义测试消息内容（不填则使用默认）
    /// Optional: custom test message body (default if omitted).
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TestResponse {
    pub code: i32,
    pub data: TestData,
}

#[derive(Debug, Serialize)]
pub struct TestData {
    pub user_id: String,
    pub chat_id: Option<String>,
    pub sent: bool,
    /// 0 = OK, 1 = skipped (no chat_id bound), 2 = at least one channel errored
    pub status: i32,
    /// Human-readable message
    pub message: String,
    /// Aggregated per-channel error messages (if any)
    pub errors: Vec<String>,
}

// ─── Handlers ──────────────────────────────────────────────────

/// POST /api/v1/notifications/telegram/bind
///
/// 中文：将当前 authenticated user 的 `telegram_chat_id` 更新为请求体里的值。
///   chat_id 必须非空；存入前做基础 trim。绑定成功后写 `TelegramChatIdCache`
///   缓存（通过 Extension 注入），让 risk_manager 触发的告警能直接通过 per-user
///   解析器找到目标。
/// English: Updates the authenticated user's `telegram_chat_id` to the value in
///   the request body. chat_id must be non-empty; basic trim is applied.
///   On success the value is written to `TelegramChatIdCache` (injected via
///   Extension) so that risk_manager-driven alerts can find the target via
///   the per-user resolver without re-reading the DB on every send.
pub async fn bind_chat_id(
    State(db): State<Arc<DatabaseConnection>>,
    Extension(chat_cache): Extension<Arc<TelegramChatIdCache>>,
    user: AuthenticatedUser,
    headers: HeaderMap,
    Json(req): Json<BindRequest>,
) -> Result<Response, AppError> {
    let chat_id = req.chat_id.trim().to_string();
    if chat_id.is_empty() {
        return Err(AppError::BadRequest(
            "chat_id must be non-empty".to_string(),
        ));
    }
    // 长度限制：Telegram chat_id 是 int64，ASCII 表达 ≤ 20 字符；保留余量。
    // Length cap: Telegram chat_id is int64, ASCII ≤ 20 chars; keep a buffer.
    if chat_id.len() > 64 {
        return Err(AppError::BadRequest(
            "chat_id too long (max 64 chars)".to_string(),
        ));
    }

    // 查找 user
    let user_model = UserEntity::find()
        .filter(UserCol::Id.eq(user.user_id))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {} not found", user.user_id)))?;

    // P3-4: 拿 before 状态（是否已绑 / 旧 chat_id 长度）
    let was_bound = user_model.telegram_chat_id.is_some();
    let old_chat_id_len = user_model.telegram_chat_id.as_ref().map(|s| s.len());

    let mut active: UserActive = user_model.into();
    active.telegram_chat_id = Set(Some(chat_id.clone()));
    active.updated_at = Set(chrono::Utc::now());
    UserEntity::update(active).exec(db.as_ref()).await?;

    // 写缓存：让 TelegramChannel 解析器在 send() 时能命中。
    // Write cache: the TelegramChannel resolver can hit on the next send().
    chat_cache.set(user.user_id, chat_id.clone()).await;

    // P3-4: 写审计 —— 通知渠道绑定是敏感操作 (攻击者若拿到 token 可劫持告警)。
    // 仅记 user_id / new chat_id 长度 / 是否首次绑定 —— 不写明文 chat_id (PII 防御)。
    let (ip, ua, req_id) = audit_log::extract_audit_context(&headers, "0.0.0.0");
    let event = audit_log::AuditEvent {
        user_id: Some(user.user_id),
        action: actions::TELEGRAM_BOUND.to_string(),
        target_type: target_types::TELEGRAM.to_string(),
        target_id: user.user_id.to_string(),
        before: Some(serde_json::json!({
            "was_bound": was_bound,
            "old_chat_id_len": old_chat_id_len,
        })),
        after: Some(serde_json::json!({
            "new_chat_id_len": chat_id.len(),
            "bound": true,
        })),
        ip_address: ip,
        user_agent: ua,
        request_id: req_id,
    };
    audit_log::record(&db, event).await;

    Ok((
        StatusCode::OK,
        Json(BindResponse {
            code: 0,
            data: BindData {
                user_id: user.user_id.to_string(),
                chat_id,
                message: "telegram_chat_id 已绑定".to_string(),
            },
        }),
    )
        .into_response())
}

/// POST /api/v1/notifications/telegram/test
///
/// 中文：用 AlertNotificationService 多播一条 info 级别测试消息以验证
///   绑定 + 推送全链路。无 chat_id 绑定时返回 status=1；至少一个渠道失败
///   时 status=2 + errors 非空；全部成功时 status=0。
/// English: Multicasts an info-level test message via AlertNotificationService
///   to validate the bind + push pipeline. status=1 if no chat_id bound,
///   status=2 + non-empty `errors` if any channel failed, status=0 on full success.
pub async fn send_test(
    State(db): State<Arc<DatabaseConnection>>,
    Extension(notifier): Extension<Arc<AlertNotificationService>>,
    user: AuthenticatedUser,
    Json(req): Json<TestRequest>,
) -> Result<Response, AppError> {
    // 读出 user 的 chat_id（让操作员知道测试会发到哪里）
    // Read the user's chat_id so operators can see where the test will land.
    let user_model = UserEntity::find()
        .filter(UserCol::Id.eq(user.user_id))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {} not found", user.user_id)))?;
    let bound_chat_id = user_model.telegram_chat_id.clone();
    let user_id_str = user.user_id.to_string();

    // 没绑 → 返回 status=1（不是 4xx — bind 是前端独立流程）
    // Not bound → return status=1 (not 4xx — bind is a separate front-end flow).
    if bound_chat_id.is_none() {
        return Ok((
            StatusCode::OK,
            Json(TestResponse {
                code: 0,
                data: TestData {
                    user_id: user_id_str,
                    chat_id: None,
                    sent: false,
                    status: 1,
                    message: "用户尚未绑定 telegram_chat_id，请先 POST /telegram/bind".to_string(),
                    errors: vec![],
                },
            }),
        )
            .into_response());
    }

    // 构造通知：metadata.user_id 让 per-user resolver 能查到 chat_id
    // Build notification: metadata.user_id lets the per-user resolver find the chat_id.
    let content = req.message.unwrap_or_else(|| {
        "这是一条来自量化交易系统的 Telegram 通知测试消息。".to_string()
    });
    let mut n = AlertNotification::new(
        "Telegram 通知测试".to_string(),
        content,
        "test_alert".to_string(),
        "info".to_string(),
        None,
    );
    n = n.with_metadata("user_id", &user_id_str);

    // 走 notifier 多播
    // Multicast via the notifier.
    let send_result = notifier.send_alert_no_dedup(&n).await;
    match send_result {
        Ok(()) => Ok((
            StatusCode::OK,
            Json(TestResponse {
                code: 0,
                data: TestData {
                    user_id: user_id_str,
                    chat_id: bound_chat_id,
                    sent: true,
                    status: 0,
                    message: "测试消息已通过所有渠道发送".to_string(),
                    errors: vec![],
                },
            }),
        )
            .into_response()),
        Err(err_str) => {
            // err_str 是 "telegram: ...; wechat: ..." 这类用 "; " 拼接的字符串
            // err_str is a "; "-joined string like "telegram: ...; wechat: ..."
            let errors: Vec<String> = err_str
                .split("; ")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok((
                StatusCode::OK,
                Json(TestResponse {
                    code: 0,
                    data: TestData {
                        user_id: user_id_str,
                        chat_id: bound_chat_id,
                        sent: false,
                        status: 2,
                        message: "部分或全部渠道失败".to_string(),
                        errors,
                    },
                }),
            )
                .into_response())
        }
    }
}

// ─── Router ────────────────────────────────────────────────────

/// 中文：返回挂载在 `/api/v1/notifications/telegram/*` 的子路由。State 类型
///   用项目约定的 `DbPool = Arc<DatabaseConnection>`（与 portfolio/risk
///   等路由一致）。`TelegramChatIdCache` 与 `AlertNotificationService`
///   通过 Extension 注入。
/// English: Returns the sub-router mounted at `/api/v1/notifications/telegram/*`.
///   State type is the project convention `DbPool = Arc<DatabaseConnection>`
///   (matches portfolio/risk routes). `TelegramChatIdCache` and
///   `AlertNotificationService` are injected via Extension.
pub fn router() -> Router<Arc<DatabaseConnection>> {
    Router::new()
        .route("/notifications/telegram/bind", post(bind_chat_id))
        .route("/notifications/telegram/test", post(send_test))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_request_trim() {
        let req: BindRequest = serde_json::from_str(r#"{"chat_id":"  12345  "}"#).unwrap();
        assert_eq!(req.chat_id.trim(), "12345");
    }

    #[test]
    fn test_bind_request_rejects_empty() {
        let s = BindRequest {
            chat_id: "".to_string(),
        };
        assert!(s.chat_id.trim().is_empty());
    }

    #[test]
    fn test_test_request_default() {
        let req: TestRequest = serde_json::from_str("{}").unwrap();
        assert!(req.message.is_none());
    }

    #[test]
    fn test_user_id_str_format() {
        let uid: Uuid = Uuid::new_v4();
        let s = uid.to_string();
        assert_eq!(s.len(), 36);
    }
}
