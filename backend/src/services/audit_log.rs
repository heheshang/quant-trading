//! services/audit_log.rs — 审计日志写入与查询
//!
//! 写盘策略：
//!   - `record(...)`        : 同步直写。简单、可观测、出错就 log。
//!   - `record_via_mq(...)` : 入队到 P3-2 `audit_logs` 队列，由消费者批量写。
//!                            适合「不影响主链路延迟」的场景（已记录但未必
//!                            立即可见；admin 查询时多了一个消费延迟）。
//!
//! 默认入口是 `record`。P0 阶段不强制走 MQ；调用方按需选。
//!
//! 查询接口：
//!   - `query(...)`   : 多条件 + 分页（user_id / action / target_type / 时间范围）
//!   - `find_by_id`   : 单条详情

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::db::audit_log::{ActiveModel, Entity, Model};
use crate::mq::queues::JobKind;
use crate::mq::publisher::MqClient;
use crate::utils::error::AppError;

/// 写入审计日志所需的事件载荷。
///
/// 由调用方（hook handler / service）填好；service 层负责落库或入队。
///
/// 设计要点：
///   - `user_id` 是 `Option` —— 系统任务（无登录态）也能写。
///   - `target_id` 是 `String`（不限定 UUID/i64）—— 跨表关联灵活。
///   - `before` / `after` 任意 `Serialize` —— 自动序列化为 JSONB。
///     不想区分时把 `before` 留 None、`after` 填整个对象也行（约定不是强制的）。
///   - 整个 struct 派生 `Serialize`：要 publish 到 MQ 时整个 struct 是 envelope payload。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub user_id: Option<Uuid>,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub before: Option<JsonValue>,
    pub after: Option<JsonValue>,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

impl AuditEvent {
    /// 构造一个 `created` 类事件：`{created: after}`。
    pub fn created(
        user_id: Option<Uuid>,
        action: impl Into<String>,
        target_type: impl Into<String>,
        target_id: impl Into<String>,
        after: impl Serialize,
        ip_address: impl Into<String>,
        user_agent: Option<String>,
        request_id: Option<String>,
    ) -> Self {
        Self {
            user_id,
            action: action.into(),
            target_type: target_type.into(),
            target_id: target_id.into(),
            before: None,
            after: Some(
                serde_json::json!({ "created": serde_json::to_value(after).unwrap_or(JsonValue::Null) }),
            ),
            ip_address: ip_address.into(),
            user_agent,
            request_id,
        }
    }

    /// 构造一个 `deleted` 类事件：`{deleted_id}`。
    pub fn deleted(
        user_id: Option<Uuid>,
        action: impl Into<String>,
        target_type: impl Into<String>,
        target_id: impl Into<String>,
        ip_address: impl Into<String>,
        user_agent: Option<String>,
        request_id: Option<String>,
    ) -> Self {
        let tid_string: String = target_id.into();
        Self {
            user_id,
            action: action.into(),
            target_type: target_type.into(),
            target_id: tid_string.clone(),
            before: None,
            after: Some(serde_json::json!({ "deleted_id": tid_string })),
            ip_address: ip_address.into(),
            user_agent,
            request_id,
        }
    }

    /// 构造一个 `updated` 类事件：`{before, after}`。
    pub fn updated(
        user_id: Option<Uuid>,
        action: impl Into<String>,
        target_type: impl Into<String>,
        target_id: impl Into<String>,
        before: impl Serialize,
        after: impl Serialize,
        ip_address: impl Into<String>,
        user_agent: Option<String>,
        request_id: Option<String>,
    ) -> Self {
        Self {
            user_id,
            action: action.into(),
            target_type: target_type.into(),
            target_id: target_id.into(),
            before: Some(serde_json::to_value(before).unwrap_or(JsonValue::Null)),
            after: Some(serde_json::to_value(after).unwrap_or(JsonValue::Null)),
            ip_address: ip_address.into(),
            user_agent,
            request_id,
        }
    }
}

/// 同步写一条审计日志。
///
/// 错误处理：失败不向调用方抛（审计是旁路）；只 log warn + metrics counter。
/// 这是有意的：审计写失败不应阻塞用户的 API key 创建等主操作。
/// 但为了单元测试可控，提供一个返回 `Result` 的版本（`record_strict`）。
pub async fn record(db: &DatabaseConnection, event: AuditEvent) {
    if let Err(e) = record_strict(db, event).await {
        tracing::warn!(error = %e, "audit_log::record failed (non-fatal)");
    }
}

/// 严格版：失败向上抛 `AppError::Internal`。给测试 / 关键路径用。
pub async fn record_strict(db: &DatabaseConnection, event: AuditEvent) -> Result<(), AppError> {
    let diff = match (event.before, event.after) {
        (None, None) => serde_json::json!({}),
        (Some(b), None) => serde_json::json!({ "before": b }),
        (None, Some(a)) => serde_json::json!({ "after": a }),
        (Some(b), Some(a)) => serde_json::json!({ "before": b, "after": a }),
    };

    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(event.user_id),
        action: Set(event.action),
        target_type: Set(event.target_type),
        target_id: Set(event.target_id),
        diff: Set(diff),
        ip_address: Set(event.ip_address),
        user_agent: Set(event.user_agent),
        request_id: Set(event.request_id),
        created_at: Set(Utc::now()),
    };

    model
        .insert(db)
        .await
        .map_err(|e| AppError::Internal(format!("audit_log insert failed: {e}")))?;
    Ok(())
}

/// 异步批量 flush：通过 P3-2 RabbitMQ 入队 `audit_logs` 队列。
///
/// 行为：
///   1. 优先 publish 到 `JobKind::AuditLog`（routing_key = `audit_logs`）。
///   2. publish 失败 → 降级为同步 `record()`（直写 DB）。这样**永远不丢数据**，
///      只是没有吞吐优化。
///   3. publish 成功 → 函数立刻返回。consumer 由 `workers::audit_log` 拉
///      批量刷盘（典型批 50 条 / 1s tick）。
///
/// `mq` 为 `None` 时（`MQ_ENABLED=false` 配置）走纯同步路径。
pub async fn record_via_mq(
    db: &DatabaseConnection,
    mq: Option<&MqClient>,
    event: AuditEvent,
) {
    if let Some(client) = mq {
        match client.publish(JobKind::AuditLog, &event).await {
            Ok(job_id) => {
                tracing::debug!(
                    action = %event.action,
                    target_type = %event.target_type,
                    job_id,
                    "audit_log::record_via_mq — enqueued for batch flush"
                );
                return;
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    action = %event.action,
                    "audit_log::record_via_mq — MQ publish failed, falling back to direct write"
                );
            }
        }
    }
    // 兜底：直写 DB。即使 MQ 完全不可用也保证不丢数据。
    record(db, event).await;
}

// ─── Query ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuditQuery {
    pub user_id: Option<Uuid>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub request_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AuditLogView {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub diff: JsonValue,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Model> for AuditLogView {
    fn from(m: Model) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            action: m.action,
            target_type: m.target_type,
            target_id: m.target_id,
            diff: m.diff,
            ip_address: m.ip_address,
            user_agent: m.user_agent,
            request_id: m.request_id,
            created_at: m.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AuditLogPage {
    pub items: Vec<AuditLogView>,
    pub total: u64,
    pub page: u32,
    pub size: u32,
}

/// 多条件分页查询。
///
/// 默认 `page=1, size=20`；size 上限 200 防止 admin 误请求 10MB 响应。
pub async fn query(db: &DatabaseConnection, q: AuditQuery) -> Result<AuditLogPage, AppError> {
    let page = q.page.unwrap_or(1).max(1);
    let size = q.size.unwrap_or(20).clamp(1, 200);

    use crate::db::audit_log::Column;
    let mut find = Entity::find();

    if let Some(uid) = q.user_id {
        find = find.filter(Column::UserId.eq(uid));
    }
    if let Some(action) = q.action.as_deref() {
        find = find.filter(Column::Action.eq(action));
    }
    if let Some(tt) = q.target_type.as_deref() {
        find = find.filter(Column::TargetType.eq(tt));
    }
    if let Some(tid) = q.target_id.as_deref() {
        find = find.filter(Column::TargetId.eq(tid));
    }
    if let Some(req_id) = q.request_id.as_deref() {
        find = find.filter(Column::RequestId.eq(req_id));
    }
    if let Some(from) = q.from {
        find = find.filter(Column::CreatedAt.gte(from));
    }
    if let Some(to) = q.to {
        find = find.filter(Column::CreatedAt.lte(to));
    }

    // 先 count 再按页取。
    // 注意：`paginate()` 在 SeaORM 0.12 内部会先 count 再 fetch，所以合并到一行。
    let paginator = find
        .order_by_desc(Column::CreatedAt)
        .into_model::<Model>()
        .paginate(db, size as u64);

    let total = paginator
        .num_items()
        .await
        .map_err(|e| AppError::Internal(format!("audit_log num_items failed: {e}")))?;

    let items = paginator
        .fetch_page(page as u64 - 1)
        .await
        .map_err(|e| AppError::Internal(format!("audit_log fetch_page failed: {e}")))?;

    Ok(AuditLogPage {
        items: items.into_iter().map(AuditLogView::from).collect(),
        total,
        page,
        size,
    })
}

/// 单条详情。
pub async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> Result<Option<AuditLogView>, AppError> {
    Entity::find_by_id(id)
        .one(db)
        .await
        .map(|opt| opt.map(AuditLogView::from))
        .map_err(|e| AppError::Internal(format!("audit_log find_by_id failed: {e}")))
}

// ─── Header extractors ─────────────────────────────────────────────
//
// 不同的 handler 签名不一样（有的拿 `Request`，有的拿 `HeaderMap`），
// 这里给一种统一入口：调用方拿 IP（一般通过 `axum::extract::ConnectInfo<SocketAddr>`），
// 再加上 `HeaderMap` 就能补齐 UA / request_id。
//
// 返回 `(ip_address, user_agent, request_id)` 三元组，调用方塞进 `AuditEvent`。
//
// 设计取舍：不把 `Request` 接进来是为了让 hook 站点的签名改动最小
// （handlers 大多已经拿 `HeaderMap` 或 `Parts` 的一部分），IP 在中间件层
// 之后已经通过 `ConnectInfo` 拿到。

/// 从 `HeaderMap` + 调用方拿到的 IP 提取 IP / UA / request_id。
pub fn extract_audit_context(
    headers: &axum::http::HeaderMap,
    client_ip: &str,
) -> (String, Option<String>, Option<String>) {
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());

    // P0-3 request_id 中间件通过响应头 X-Request-Id 注入；
    // 直接读 header 适用于 handler 已拿到 HeaderMap 的场景。
    let request_id = headers
        .get(crate::middleware::request_id::X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());

    (client_ip.to_owned(), user_agent, request_id)
}

/// 从 `axum::http::request::Parts` 提取 audit context（用于拿 `Extensions` 的场景）。
pub fn extract_audit_context_from_parts(
    parts: &axum::http::request::Parts,
    client_ip: &str,
) -> (String, Option<String>, Option<String>) {
    let (ip, ua, hdr_req_id) = extract_audit_context(&parts.headers, client_ip);
    // 优先拿中间件注入的 RequestId extension（最权威）
    let req_id = parts
        .extensions
        .get::<crate::middleware::request_id::RequestId>()
        .map(|r| r.0.clone())
        .or(hdr_req_id);
    (ip, ua, req_id)
}

// ─── Helper macro ────────────────────────────────────────────────────

/// 一行调用式写审计：`audit!(db, "user.role.changed", user_id, target_type, target_id, before, after, ip, ua, req_id)`
///
/// 字段多、调用方懒；为减少 boilerplate 提供这个 macro。
/// 调用方仍可选择直接构造 `AuditEvent` 以获得更多控制（例如 `created` / `deleted` 变体）。
///
/// 用法（任一即可）：
///   - `audit!(db, event)`                       — 直接传 `AuditEvent`
///   - `audit!(db, "user.role.changed", uid, "user", tid, before, after, ip)`
///   - `audit!(db, "api_key.created", uid, "api_key", tid, after, ip, ua, req_id)`
///
/// 因为参数数可变，macro 内部用 `$($arg:expr),+` 匹配，再根据 arity 分支。
/// 为简单起见，支持以下三种 arity：1、7、9。
///
/// 调用方需要先 `use crate::services::audit_log::audit;`。
macro_rules! audit {
    // 1 个参数：直接传 AuditEvent
    ($db:expr, $event:expr) => {{
        let ev: $crate::services::audit_log::AuditEvent = $event;
        $crate::services::audit_log::record($db, ev).await;
    }};

    // 7 个参数：action, user_id, target_type, target_id, after, ip
    ($db:expr, $action:expr, $user_id:expr, $target_type:expr, $target_id:expr, $after:expr, $ip:expr) => {{
        let ev = $crate::services::audit_log::AuditEvent::created(
            $user_id,
            $action,
            $target_type,
            $target_id,
            $after,
            $ip,
            None,
            None,
        );
        $crate::services::audit_log::record($db, ev).await;
    }};

    // 9 个参数：action, user_id, target_type, target_id, before, after, ip, ua, req_id
    ($db:expr, $action:expr, $user_id:expr, $target_type:expr, $target_id:expr,
     $before:expr, $after:expr, $ip:expr, $ua:expr, $req_id:expr) => {{
        let ev = $crate::services::audit_log::AuditEvent::updated(
            $user_id,
            $action,
            $target_type,
            $target_id,
            $before,
            $after,
            $ip,
            $ua,
            $req_id,
        );
        $crate::services::audit_log::record($db, ev).await;
    }};
}
pub(crate) use audit;
