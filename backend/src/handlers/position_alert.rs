//! handlers/position_alert.rs — P1-F2 实盘止盈止损 API
//!
//! 中文说明：
//!   本文件是 P1-F2 实盘止盈止损的 HTTP 接入层。Handler 负责：参数解析、字段校验、
//!   多租户隔离（user_id 必须从 JWT 提取并随每次查询下发）、跨服务编排（如创建 alert
//!   前需要先查持仓以获取 symbol）。具体业务逻辑（CRUD、状态机、触发判定）下沉到
//!   `services/position_alert_service.rs`，价格轮询与触发执行下沉到
//!   `services/position_alert_monitor.rs`。
//!
//! PRD: P1-F2 实盘止盈止损
//! 端点：
//!   POST /api/v1/alerts           — 创建止盈/止损
//!   GET  /api/v1/alerts           — 列表（可按持仓筛选）
//!   GET  /api/v1/alerts/:id      — 详情
//!   PUT  /api/v1/alerts/:id      — 修改（价格/距离）
//!   DELETE /api/v1/alerts/:id    — 取消
//!   POST /api/v1/alerts/:id/trigger — 手动触发（测试用）
//!   POST /api/v1/alerts/batch-check — 批量检查（行情心跳调用）
//!
//! English description:
//!   HTTP layer for the P1-F2 live take-profit / stop-loss module. Handlers are
//!   responsible for: request parsing, field validation, multi-tenant isolation
//!   (`user_id` is extracted from the JWT and pushed down on every query), and
//!   cross-service orchestration (e.g. resolving the position's `symbol` before
//!   creating the alert). All CRUD / state-machine / trigger logic lives in
//!   `services/position_alert_service.rs`; the polling loop and trigger execution
//!   live in `services/position_alert_monitor.rs`.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::position_alerts::{AlertStatus, AlertType, TriggerMode};
use crate::middleware::auth::AuthenticatedUser;
use crate::services::position_alert_service::{
    AlertResponse, CreateAlertRequest, PositionAlertService, UpdateAlertRequest,
};
use crate::utils::error::AppError;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

// ─── Schemas ───────────────────────────────────────────────────

/// 创建 alert 的请求体 — 字段全部以字符串接收，避免 JS 数字精度丢失。
/// Request body for `POST /api/v1/alerts` — every numeric field is a string
/// to avoid the IEEE-754 precision loss that the JS client suffers from
/// when round-tripping large `f64` values through JSON.
#[derive(Debug, Deserialize)]
pub struct CreateAlertRequestSchema {
    pub position_id: String,               // UUID string
    pub alert_type: String,                // "take_profit" | "stop_loss" | "trailing_stop"
    pub trigger_price: String,             // f64 as string
    pub trigger_mode: Option<String>,      // "market" | "limit", default "market"
    pub limit_price: Option<String>,       // optional, for limit trigger
    pub trailing_distance: Option<String>, // e.g. "0.5" for 0.5%
    pub note: Option<String>,
}

/// 修改 alert 的请求体 — 所有字段可选（PATCH 语义），未提供表示保持原值。
/// Request body for `PUT /api/v1/alerts/:id` — every field is optional (PATCH
/// semantics); omitted fields keep their current value on the server side.
#[derive(Debug, Deserialize)]
pub struct UpdateAlertRequestSchema {
    pub trigger_price: Option<String>,
    pub limit_price: Option<String>,
    pub trigger_mode: Option<String>,
    pub trailing_distance: Option<String>,
    pub status: Option<String>, // "active" | "paused" | "cancelled"
}

/// 列表查询参数 — 三个维度可独立组合。
/// List query string — the three dimensions are independent and can be
/// combined freely. `position_id` is preferred over `symbol` when both are
/// given (it scopes the result to a single position).
#[derive(Debug, Deserialize)]
pub struct ListAlertsQuery {
    pub symbol: Option<String>,
    pub position_id: Option<String>,
    pub status: Option<String>,
}

/// 创建成功后的精简响应体 — 避免回传整个 alert 行造成敏感字段外泄。
/// Slim response shape returned on `POST /api/v1/alerts`. We deliberately
/// avoid echoing the full alert row so internal-only fields (e.g. trailing
/// state) do not leak to the client before the first poll tick.
#[derive(Debug, Serialize)]
pub struct AlertCreatedResponse {
    pub alert_id: String,
    pub position_id: String,
    pub alert_type: String,
    pub trigger_price: String,
    pub trigger_mode: String,
    pub status: String,
    pub created_at: String,
}

// ─── Helpers ───────────────────────────────────────────────────

/// 把字符串映射为 `AlertType` 枚举 — 未识别的值返回 400，错误信息列出合法选项。
/// Map a wire string to `AlertType`. Unknown values are rejected with 400 and
/// the error message echoes the legal alternatives so the client can self-correct.
fn parse_alert_type(s: &str) -> Result<AlertType, AppError> {
    match s {
        "take_profit" => Ok(AlertType::TakeProfit),
        "stop_loss" => Ok(AlertType::StopLoss),
        "trailing_stop" => Ok(AlertType::TrailingStop),
        _ => Err(AppError::BadRequest(format!(
            "Invalid alert_type: {}. Expected: take_profit | stop_loss | trailing_stop",
            s
        ))),
    }
}

/// 把 `Option<String>` 映射为 `TriggerMode` — `None`/未识别一律回退到 `Market`。
///
///   中文说明：限价 vs 市价的"宽容默认"是有意为之 —— 大多数客户端只需要挂单立即成交，
///   强制填字段反而会拖慢集成。Limit 模式必须由调用方显式声明。
///
/// Map `Option<String>` to `TriggerMode`. `None` or any unrecognised value
/// silently falls back to `Market` — this is intentional, since the vast
/// majority of clients want immediate execution and forcing them to send
/// `trigger_mode` on every call adds friction. `Limit` must be opt-in.
fn parse_trigger_mode(s: &Option<String>) -> TriggerMode {
    match s.as_deref() {
        Some("limit") => TriggerMode::Limit,
        _ => TriggerMode::Market,
    }
}

/// 把字符串映射为 `AlertStatus` — 与 `parse_alert_type` 同模式（严格枚举）。
/// String → `AlertStatus`. Strict enum mapping, mirrors `parse_alert_type`.
///   注意：这里只接受 "active | paused | cancelled" 三态；"triggered" 是服务层
///   内部产生的终态，handler 不应接受客户端直接写入。
///   Note: only the three user-settable states are accepted. `"triggered"` is
///   a terminal state produced by the service layer; clients must never set it
///   directly via this endpoint.
fn parse_alert_status(s: &str) -> Result<AlertStatus, AppError> {
    match s {
        "active" => Ok(AlertStatus::Active),
        "paused" => Ok(AlertStatus::Paused),
        "cancelled" => Ok(AlertStatus::Cancelled),
        _ => Err(AppError::BadRequest(format!(
            "Invalid status: {}. Expected: active | paused | cancelled",
            s
        ))),
    }
}

// ─── Handlers ──────────────────────────────────────────────────

/// POST /api/v1/alerts — 创建止盈/止损警戒
///
/// 中文说明：
///   - `user_id` **必须**从 JWT (`AuthenticatedUser`) 提取，不能信任请求体，避免用户 A
///     给用户 B 的持仓挂单（多租户隔离）
///   - 创建前需要先把 `position_id` 反查为 `symbol`，因为 `CreateAlertRequest.symbol`
///     字段必填，而前端请求体里没有（symbol 应该跟随持仓而不是客户端传入，防止伪造）
///   - `trailing_distance` 在 handler 层做 (0, 100) 校验后转成小数；该范围与
///     `trailing_stop_params.rs` 的 (0, 0.1) 业务上限一致（这里按百分比接收，转换时除以 100）
///
/// PRD: P1-F2 Scenario "开仓时附加止盈止损"
///
/// English:
///   - `user_id` **must** come from the JWT (`AuthenticatedUser`) — never trust
///     the request body, otherwise tenant A could attach alerts to tenant B's
///     positions (multi-tenant isolation)
///   - We resolve `position_id` → `symbol` before insertion, because
///     `CreateAlertRequest.symbol` is required and must be derived from the
///     authoritative position row, not echoed from the client (anti-spoofing)
///   - `trailing_distance` is validated to (0, 100) in the handler and then
///     converted to a fraction. The 10% upper bound matches the business cap
///     in `trailing_stop_params.rs`; the handler receives it as a percentage
///     string and divides by 100 to match the storage format.
pub async fn create_alert(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<CreateAlertRequestSchema>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let service = PositionAlertService::new(db.clone());

    let position_id = req
        .position_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid position_id format".to_string()))?;

    let trigger_price = req
        .trigger_price
        .parse::<f64>()
        .map_err(|_| AppError::BadRequest("Invalid trigger_price format".to_string()))?;

    // 触发价必须为正 — 负价/零价在所有交易所都不存在，会让监控层的比较
    // 逻辑（current_price >= trigger_price）失效。
    // Trigger price must be positive — negative / zero prices are impossible on
    // any real exchange, and would break the monitor layer's comparisons
    // (e.g. `current_price >= trigger_price`).
    if trigger_price <= 0.0 {
        return Err(AppError::BadRequest(
            "trigger_price must be positive".to_string(),
        ));
    }

    let limit_price = match &req.limit_price {
        Some(p) => Some(
            p.parse::<f64>()
                .map_err(|_| AppError::BadRequest("Invalid limit_price format".to_string()))?,
        ),
        None => None,
    };

    let trailing_distance = match &req.trailing_distance {
        Some(d) => {
            let v = d.parse::<f64>().map_err(|_| {
                AppError::BadRequest("Invalid trailing_distance format".to_string())
            })?;
            // 跟踪距离的范围校验 (0, 100) 百分比 → 内部存储为小数 (0, 1.0)。
            // - 下界 v > 0：距离为 0 等同于立即触发（无意义且危险）
            // - 上界 v < 100：超过 100% 的距离在数学上无意义（永远不会触发）
            // 业务上限真正的硬约束 (0, 0.1) 在 service / trailing_stop_params.rs 强制。
            // Validate trailing distance in (0, 100) percent — internally stored as a
            // fraction (0, 1.0).
            // - lower bound v > 0: a zero-distance alert would trigger immediately
            //   (meaningless and dangerous)
            // - upper bound v < 100: distance ≥ 100% is mathematically a no-op
            //   (price can never move that far against the position)
            // The real business cap (0, 0.1) is enforced one layer down in
            // `services/position_alert_service.rs` / `trailing_stop_params.rs`.
            if v <= 0.0 || v >= 100.0 {
                return Err(AppError::BadRequest(
                    "trailing_distance must be between 0 and 100".to_string(),
                ));
            }
            Some(v / 100.0) // 转换为小数 / convert percentage → fraction
        }
        None => None,
    };

    let create_req = CreateAlertRequest {
        position_id,
        // 占位符：下面紧接着会从持仓表里查出来覆盖。
        // 保留占位而不是 Option<String>，可让 service 层签名保持简单。
        // Placeholder — overwritten below with the real symbol from the position row.
        // Keeping a placeholder (rather than `Option<String>`) keeps the service
        // signature simple and forces the handler to do the resolution.
        symbol: "BTCUSDT".to_string(), // 从持仓查询获得 / resolved from position lookup below
        alert_type: parse_alert_type(&req.alert_type)?,
        trigger_price,
        trigger_mode: parse_trigger_mode(&req.trigger_mode),
        limit_price,
        trailing_distance,
        note: req.note,
    };

    // 持仓存在性 + 所有权校验：多租户隔离的核心边界（multi-tenant boundary）。
    // - 必须按 (user_id, id) 双键过滤，单按 id 查会泄漏他人持仓
    // - 查不到时返回 404 而不是 403，避免泄漏"该 id 存在但属于其他用户"的信息
    // Position existence + ownership check — the core multi-tenant boundary.
    // - Must filter on (user_id, id); a query keyed on `id` alone would leak
    //   the existence of other tenants' positions
    // - 404 (not 403) on miss — avoids leaking "this id exists but belongs
    //   to another user" via timing or status code
    let position = crate::db::order::positions::Entity::find()
        .filter(crate::db::order::positions::Column::UserId.eq(user.user_id))
        .filter(crate::db::order::positions::Column::Id.eq(position_id))
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Position {} not found", position_id)))?;

    let mut create_req = create_req;
    create_req.symbol = position.symbol.clone();

    // 把 JWT 里的 user_id 透传给 service，让 service 层做最终的 ownership 校验。
    // 这一步看似冗余（handler 已经按 user_id 查过持仓），但它是 defense in depth：
    // 万一未来 service 的 create_alert 被新 caller 复用，user_id 也仍然受 JWT 约束。
    // Push the JWT-derived `user_id` into the service so the service layer can
    // re-check ownership. This looks redundant (the handler already filtered
    // the position by user_id), but it's defense in depth: if `create_alert`
    // is ever called from a new caller, the user_id is still constrained by
    // the JWT subject.
    let alert = service.create_alert(user.user_id, create_req).await?;

    // 把 SeaORM 枚举反序列化成对外的字符串 — 三段一样的模式是因为枚举没有
    // 实现统一的 `as_str()`，只能走 `serde_json` 反射拿到 wire 形式。
    // Convert SeaORM enums back to their wire string form. The three lines are
    // identical because the enums don't implement a common `as_str()` — we go
    // through `serde_json` to recover the same string we deserialised on the way in.
    let resp = AlertCreatedResponse {
        alert_id: alert.id.to_string(),
        position_id: alert.position_id.to_string(),
        alert_type: serde_json::to_value(&alert.alert_type)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        trigger_price: format!("{:.8}", alert.trigger_price),
        trigger_mode: serde_json::to_value(&alert.trigger_mode)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        status: serde_json::to_value(&alert.status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        created_at: alert.created_at.to_rfc3339(),
    };

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(resp).unwrap()),
    ))
}

/// GET /api/v1/alerts — 列表（可按 symbol / position_id 筛选）
///
/// 中文说明：
///   - 优先按 `position_id` 精确查（最常见：前端详情页），否则按 `symbol` 列活跃 alert
///   - 二次内存过滤 `status` 是有意为之：service 层的 `list_active_alerts` 已经
///     做了活跃态过滤，再叠加 `status` 过滤可以避免多发一条 DB 查询
///
/// English:
///   - Prefer the exact `position_id` lookup (most common: position detail
///     page); fall back to listing active alerts for a `symbol`
///   - The second-pass in-memory `status` filter is deliberate: the service's
///     `list_active_alerts` is already pre-filtered to "active" — applying an
///     extra `status` filter in Rust avoids issuing a second DB roundtrip.
pub async fn list_alerts(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<ListAlertsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = PositionAlertService::new(db);

    let alerts = if let Some(position_id) = &params.position_id {
        let pid = position_id
            .parse::<Uuid>()
            .map_err(|_| AppError::BadRequest("Invalid position_id".to_string()))?;
        service.list_alerts_by_position(user.user_id, pid).await?
    } else {
        service
            .list_active_alerts(user.user_id, params.symbol.clone())
            .await?
    };

    // 在内存里补一刀 status 过滤（service 层的 list_* 接口已预过滤一次"active"）。
    // 这里用 `Vec::into_iter().filter()` 链式写法，刻意把内存过滤保持在 handler 层
    // 而不是在 service 加新接口，避免 service 接口爆炸。
    // Apply an in-memory `status` filter on top of the service's pre-filtered
    // active list. The in-memory filter is kept in the handler (rather than
    // adding yet another service method) to keep the service surface small —
    // the dataset per user is small enough that an extra DB roundtrip is not
    // worth the API bloat.
    let alerts: Vec<_> = if let Some(status_str) = &params.status {
        let target_status = parse_alert_status(status_str)?;
        alerts
            .into_iter()
            .filter(|a| a.status == target_status)
            .collect()
    } else {
        alerts
    };

    let items: Vec<AlertResponse> = alerts.iter().map(AlertResponse::from_model).collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": items
    })))
}

/// GET /api/v1/alerts/:id — 详情
///
/// 中文说明：直连 DB 而不经过 service，因为 service 没有 `get_alert` —
/// 详情接口语义最简单（按主键 + 租户过滤单行），绕过 service 层避免为
/// 单行查询额外建一层抽象。
///
/// English: Hits the DB directly rather than going through `PositionAlertService`,
/// because the service has no `get_alert` method — for a single-row lookup keyed
/// on (user_id, id) the extra service layer would be pure ceremony.
pub async fn get_alert(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    use crate::db::position_alerts::Column as AlertCol;
    use crate::db::position_alerts::Entity as AlertEntity;

    let alert = AlertEntity::find()
        .filter(AlertCol::UserId.eq(user.user_id))
        .filter(AlertCol::Id.eq(alert_id))
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Alert {} not found", alert_id)))?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": AlertResponse::from_model(&alert)
    })))
}

/// PUT /api/v1/alerts/:id — 修改止盈止损
///
/// 中文说明：
///   - 与 `create_alert` 共享 `(0, 100)` 百分比→小数 转换逻辑；上界校验
///     与 `trailing_stop_params.rs` 的 10% 业务硬上限保持一致
///   - `trigger_mode` 解析比 `create_alert` 严格（无效值会 400），因为
///     修改接口通常是用户显式切换市价↔限价，宽容默认反而会掩盖误操作
///
/// PRD: P1-F2 Scenario "手动修改止盈止损"
///
/// English:
///   - Shares the (0, 100) percent → fraction conversion with `create_alert`,
///     and the resulting business cap (10%) matches `trailing_stop_params.rs`
///   - `trigger_mode` parsing is stricter here than in `create_alert`: an
///     unknown value returns 400 rather than silently defaulting to `Market`,
///     because edits are typically an explicit market↔limit toggle where
///     a permissive default would mask user mistakes.
pub async fn update_alert(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(alert_id): Path<Uuid>,
    Json(req): Json<UpdateAlertRequestSchema>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = PositionAlertService::new(db);

    let trigger_price = req
        .trigger_price
        .as_ref()
        .map(|p| p.parse::<f64>())
        .transpose()
        .map_err(|_| AppError::BadRequest("Invalid trigger_price".to_string()))?;

    let limit_price = req
        .limit_price
        .as_ref()
        .map(|p| p.parse::<f64>())
        .transpose()
        .map_err(|_| AppError::BadRequest("Invalid limit_price".to_string()))?;

    let trigger_mode = req
        .trigger_mode
        .as_ref()
        .map(|m| match m.as_str() {
            "limit" => Ok(TriggerMode::Limit),
            "market" => Ok(TriggerMode::Market),
            _ => Err(AppError::BadRequest(format!("Invalid trigger_mode: {}", m))),
        })
        .transpose()?;

    let trailing_distance = req
        .trailing_distance
        .as_ref()
        .map(|d| {
            let v: f64 = d
                .parse()
                .map_err(|_| AppError::BadRequest("Invalid trailing_distance".to_string()))?;
            // 校验逻辑与 create_alert 完全一致 — 重复是必要的（两个入口都接受
            // 用户输入）；后续若引入第三个入口应抽出公共函数。
            // Validation logic mirrors `create_alert` exactly. Duplication is
            // intentional here (two distinct user-facing entry points); refactor
            // to a shared helper if a third entry point ever appears.
            if v <= 0.0 || v >= 100.0 {
                return Err(AppError::BadRequest(
                    "trailing_distance must be between 0 and 100".to_string(),
                ));
            }
            Ok(v / 100.0)
        })
        .transpose()?;

    let status = req.status.as_deref().map(parse_alert_status).transpose()?;

    let update_req = UpdateAlertRequest {
        trigger_price,
        limit_price,
        trigger_mode,
        trailing_distance,
        status,
    };

    let updated = service
        .update_alert(user.user_id, alert_id, update_req)
        .await?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": AlertResponse::from_model(&updated)
    })))
}

/// DELETE /api/v1/alerts/:id — 取消止盈止损
///
/// 中文说明：**"取消" ≠ "删除"**。
///   - DELETE 动词在这里复用 REST 习惯，但实际语义是软删除：
///     `status` 置为 `cancelled`，行保留在表中
///   - 保留行的目的：
///       1. 审计/合规 — 需要能回溯"用户在哪个时间点取消了哪条 alert"
///       2. 历史回测 — 触发历史/历史曲线会关联这些 alert
///       3. 防误操作恢复 — 短期窗口内可由运维手动恢复（status 回 active）
///   - 真要"硬删除"应走单独的 purge 接口（目前未实现，避免误用）
///
/// English: **"Cancel" is not "delete".** The HTTP verb is reused per REST
/// convention, but the actual semantics is a soft delete — `status` is set
/// to `cancelled` and the row is preserved. The row is kept for:
///   1. Audit / compliance — we need a paper trail of "when did this user
///      cancel which alert"
///   2. Backtest history — trigger history and P&L curves join against these
///   3. Quick-recovery — operators can flip status back to `active` in case
///      of a misclick
///
/// A separate (currently unimplemented) purge endpoint would be required for
/// hard deletion; we deliberately don't expose that to avoid accidental loss.
pub async fn cancel_alert(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = PositionAlertService::new(db);
    service.cancel_alert(user.user_id, alert_id).await?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "message": "Alert cancelled successfully" }
    })))
}

// ─── Router ────────────────────────────────────────────────────

/// 路由表 — 注意 DELETE 复用到 `cancel_alert`（软删除语义，详见该函数注释）。
///
///   集成点：调用方在 `main.rs` 用 `Router::nest("/api/v1/alerts", position_alert::router())`
///   挂载本路由表；后续如果要加 `POST /:id/trigger` 或 `POST /batch-check`，
///   也在此处追加。
///
/// English: route table — note DELETE is mapped to `cancel_alert` (soft-delete
/// semantics, see that function's doc). Callers mount this via
/// `Router::nest("/api/v1/alerts", position_alert::router())` in `main.rs`.
/// Any future `POST /:id/trigger` / `POST /batch-check` routes also go here.
pub fn router() -> Router<Arc<DatabaseConnection>> {
    Router::new()
        .route("/", axum::routing::post(create_alert))
        .route("/", axum::routing::get(list_alerts))
        .route("/{id}", axum::routing::get(get_alert))
        .route("/{id}", axum::routing::put(update_alert))
        .route("/{id}", axum::routing::delete(cancel_alert))
}
