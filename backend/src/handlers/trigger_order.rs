//! handlers/trigger_order.rs — Trigger Order HTTP Handlers (条件触发单接口)
//! handlers/trigger_order.rs — HTTP handlers for trigger / conditional orders
//!
//! 中文：
//!   P1-F3 条件触发单 HTTP 接口层。覆盖四大类条件单的 REST 接口：
//!     - 止损单（stop-loss）：价格下穿 trigger_price 时市价卖出
//!     - 止盈单（take-profit）：价格上穿 trigger_price 时市价卖出
//!     - OCO 单（one-cancels-the-other）：止损+止盈成对，触发一个自动撤销另一个
//!     - TWAP 单（time-weighted average price）：按时间切片分批市价单
//!
//!   本文件 = handler（薄层），业务逻辑（轮询/触发/取消）全部下沉到
//!   `services::trigger_order::TriggerOrderService`，本层只做：
//!     1. 反序列化请求体
//!     2. 提取 `AuthenticatedUser`（JWT 中间件注入，`user_id` 用于防越权）
//!     3. 调用 service
//!     4. 用 `AppError` 统一映射成 HTTP 状态码
//!
//! English:
//!   P1-F3 HTTP layer for conditional / trigger orders. Exposes four families of REST
//!   endpoints:
//!     - stop-loss:    market-sell when price falls below `trigger_price`
//!     - take-profit:  market-sell when price rises above `trigger_price`
//!     - OCO:          stop-loss + take-profit pair; triggering one auto-cancels the other
//!     - TWAP:         time-sliced market orders to minimise market impact
//!
//!   This file is the thin HTTP layer. All business logic (polling, triggering, cancel
//!   cascade) lives in `services::trigger_order::TriggerOrderService`. Handlers only:
//!     1. deserialise the request body
//!     2. extract `AuthenticatedUser` injected by the JWT middleware (`user_id`
//!        prevents horizontal privilege escalation)
//!     3. delegate to the service
//!     4. convert errors to HTTP status codes via `AppError`

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::auth::AuthenticatedUser;
use crate::services::trigger_order::TriggerOrderService;
use crate::utils::error::AppError;

/// ==================== 请求/响应结构 ====================
/// ==================== Request / Response Schemas ====================
///
/// 创建止损单请求
/// Create stop-loss order request.
#[derive(Debug, Deserialize)]
pub struct CreateStopLossRequest {
    pub position_id: Uuid,
    pub symbol: String,
    pub trigger_price: f64,
    pub base_price: Option<f64>,
    pub quantity: f64,
}

/// 创建止盈单请求
/// Create take-profit order request.
#[derive(Debug, Deserialize)]
pub struct CreateTakeProfitRequest {
    pub position_id: Uuid,
    pub symbol: String,
    pub trigger_price: f64,
    pub base_price: Option<f64>,
    pub quantity: f64,
}

/// 创建OCO单请求
/// Create OCO (one-cancels-the-other) order request.
///
/// 中文：stop_loss / take_profit 必须分立两侧，否则 OCO 没有意义。
///   - stop_loss_price < take_profit_price：经典多头 OCO
///   - 两个价格相等被 service 层拒绝（防止"无意义 OCO"）
///
/// English: `stop_loss_price` and `take_profit_price` must lie on opposite sides of
///   the entry; equal prices are rejected by the service (a no-op OCO).
#[derive(Debug, Deserialize)]
pub struct CreateOcoRequest {
    pub position_id: Uuid,
    pub symbol: String,
    pub stop_loss_price: f64,
    pub take_profit_price: f64,
    pub base_price: Option<f64>,
    pub quantity: f64,
}

/// 创建TWAP单请求
/// Create TWAP (time-weighted average price) order request.
///
/// 中文：TWAP 不绑定 `position_id` — 它是开仓/平仓工具，由 `side` 决定方向。
///   - slice_quantity 决定每片下单量
///   - duration_secs / interval_secs 决定切片节奏（service 层会校验 interval > 0
///     且 `slice_quantity * max_slices ≈ quantity`）
///
/// English: TWAP is not tied to a position; direction is encoded in `side`. The
///   service validates `interval_secs > 0` and that the slice count roughly
///   accounts for the total `quantity`.
#[derive(Debug, Deserialize)]
pub struct CreateTwapRequest {
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub slice_quantity: f64,
    pub interval_secs: i32,
    pub duration_secs: i32,
}

/// 查询条件单列表参数
/// Query parameters for listing trigger orders.
///
/// 中文：两个可选过滤器 — `status` 和 `symbol` 都为 `None` 时退化为"全量查询"。
///   - `status` 由 service 层做白名单校验（`pending|triggered|cancelled|failed`）
///   - `symbol` 直接做字符串精确匹配，不做大小写归一化（client 需自保证）
///
/// English: Both filters are optional. `None` on both falls back to "list all".
///   - `status` is whitelist-validated by the service
///   - `symbol` is exact-matched (no case folding; the client is responsible for
///     canonical casing, e.g. `"BTCUSDT"`)
#[derive(Debug, Deserialize)]
pub struct ListTriggerOrdersQuery {
    pub status: Option<String>,
    pub symbol: Option<String>,
}

/// 条件单响应
/// Trigger order response (unified shape for SL / TP / OCO / TWAP).
///
/// 中文：四大类条件单用同一个响应结构 — 字段集是四者的并集：
///   - SL/TP/OCO 用 `trigger_price` 单值
///   - 价格区间触发（暂未启用）用 `trigger_price_upper` / `trigger_price_lower`
///   - TWAP 用 `twap_*` 字段，其余类型这些字段为 0
///   - `oco_pair_id` 标识 OCO 对（SL/TP 配对；非 OCO 订单为 `None`）
///   - `triggered_order_id` 指向触发后下出的市价单（pending 时为 `None`）
///
/// English: One response struct for all four order types — the field set is the
///   union of the four shapes. SL/TP/OCO use the scalar `trigger_price`; range
///   triggers (not yet enabled) use `trigger_price_upper` / `_lower`; TWAP
///   populates the `twap_*` block and leaves it at 0 for non-TWAP orders.
///   `oco_pair_id` links the two legs of an OCO; `triggered_order_id` points to
///   the child market order once triggered.
#[derive(Debug, Serialize)]
pub struct TriggerOrderResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub position_id: Option<Uuid>,
    pub symbol: String,
    pub trigger_type: String,
    pub status: String,
    pub trigger_direction: String,
    pub trigger_price: f64,
    pub trigger_price_upper: Option<f64>,
    pub trigger_price_lower: Option<f64>,
    pub base_price: Option<f64>,
    pub side: String,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub avg_fill_price: Option<f64>,
    pub oco_pair_id: Option<Uuid>,
    pub triggered_order_id: Option<Uuid>,
    pub trigger_reason: Option<String>,
    pub triggered_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    // TWAP特定字段
    pub twap_slice_quantity: f64,
    pub twap_interval_secs: i32,
    pub twap_executed_slices: i32,
    pub twap_max_slices: i32,
}

impl From<crate::db::trigger_order::Model> for TriggerOrderResponse {
    fn from(m: crate::db::trigger_order::Model) -> Self {
        // Model → DTO 转换：纯映射，无业务逻辑。
        // 关键边界处理：
        //   - SeaORM 枚举 → 字符串（前端拿到的是 `"pending"` 而非数字）
        //   - `DateTime<Utc>` → RFC3339 字符串（前后端统一使用 ISO-8601 字符串）
        //   - `Option<DateTime>` 用 `map(...).to_rfc3339()`，None 保持 None
        //
        // Model → DTO conversion: pure mapping, no business logic.
        // Key boundary handling:
        //   - SeaORM enums → strings (frontend gets `"pending"`, not a numeric code)
        //   - `DateTime<Utc>` → RFC3339 string (ISO-8601 end-to-end)
        //   - `Option<DateTime>` uses `map(...).to_rfc3339()`; `None` stays `None`
        Self {
            id: m.id,
            user_id: m.user_id,
            position_id: m.position_id,
            symbol: m.symbol,
            // SeaORM 枚举 → 字符串（前端友好） / enum → string (frontend-friendly)
            trigger_type: m.trigger_type.to_string(),
            status: m.status.to_string(),
            trigger_direction: m.trigger_direction.to_string(),
            trigger_price: m.trigger_price,
            trigger_price_upper: m.trigger_price_upper,
            trigger_price_lower: m.trigger_price_lower,
            base_price: m.base_price,
            // side 枚举 → 字符串 / side enum → string
            side: m.side.to_string(),
            quantity: m.quantity,
            filled_quantity: m.filled_quantity,
            avg_fill_price: m.avg_fill_price,
            oco_pair_id: m.oco_pair_id,
            triggered_order_id: m.triggered_order_id,
            trigger_reason: m.trigger_reason,
            // Option<DateTime> → Option<String>（ISO-8601） / Option<DateTime> → Option<String>
            triggered_at: m.triggered_at.map(|t| t.to_rfc3339()),
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
            // TWAP 专用字段：非 TWAP 订单这些值是 0（schema 默认为 0）。
            // TWAP-only fields: 0 for non-TWAP orders (schema default 0).
            twap_slice_quantity: m.twap_slice_quantity,
            twap_interval_secs: m.twap_interval_secs,
            twap_executed_slices: m.twap_executed_slices,
            twap_max_slices: m.twap_max_slices,
        }
    }
}

/// OCO订单对响应
/// OCO pair response — both legs returned together.
///
/// 中文：客户端拿到 OCO 创建响应后，可直接通过 `stop_loss.id` / `take_profit.id`
///   轮询两个独立的状态。`stop_loss.oco_pair_id == take_profit.oco_pair_id`，
///   这是客户端识别"这俩是一对 OCO"的依据。
///
/// English: After OCO creation, the client polls each leg independently by id.
///   Equality of `oco_pair_id` on both legs is the canonical way to recognise a
///   freshly created OCO pair.
#[derive(Debug, Serialize)]
pub struct OcoPairResponse {
    pub stop_loss: TriggerOrderResponse,
    pub take_profit: TriggerOrderResponse,
}

/// 取消条件单请求
/// Cancel trigger order request body.
///
/// 中文：`reason` 留空时由 handler 兜底为 `"user_cancelled"`，并写入数据库的
///   `trigger_reason` 字段，便于审计。`reason` 长度由 service 层限制为 ≤ 256 字符。
///
/// English: When `reason` is `None`, the handler falls back to `"user_cancelled"`
///   which is persisted in `trigger_reason` for audit. The service caps length at
///   256 chars.
#[derive(Debug, Deserialize)]
pub struct CancelTriggerOrderRequest {
    pub reason: Option<String>,
}

/// ==================== HTTP Handlers ====================
/// ==================== HTTP Handlers ====================
///
/// 创建止损单
/// POST /api/v1/trigger-orders/stop-loss
///
/// 中文：返回 `201 Created` + 完整 `TriggerOrderResponse`，客户端据此轮询状态。
///   - `user.user_id` 来自 JWT 中间件（`AuthenticatedUser` extractor），
///     service 层据此做行级权限校验（防越权写入他人账户）
///   - HTTP 层不做价格校验（保持薄），由 service 层负责
///
/// English: Returns `201 Created` with the full `TriggerOrderResponse`; the client
///   polls the returned `id` for status updates.
///   - `user.user_id` comes from the JWT middleware; the service uses it as a
///     row-level ownership guard (prevents cross-tenant writes)
///   - The handler stays thin; price validation lives in the service.
pub async fn create_stop_loss(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<CreateStopLossRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 薄层：构造 service → 透传参数 → 把结果包装成 HTTP 响应。
    // Thin layer: build the service, forward fields as-is, wrap the result.
    // 错误路径走 `AppError`，由 `IntoResponse` 统一映射为 4xx/5xx（详见 `utils/error.rs`）。
    // Errors propagate as `AppError`, mapped to 4xx/5xx via `IntoResponse`
    // (see `utils/error.rs`).
    let service = TriggerOrderService::new(db);
    let order = service
        .create_stop_loss(
            user.user_id, // 来自 JWT 中间件，防止越权写入他人账户 / from JWT, ownership guard
            req.position_id,
            &req.symbol,
            req.trigger_price,
            req.base_price,
            req.quantity,
        )
        .await?;

    // 201 Created：新建资源。客户端用返回的 `id` 轮询 SL 状态。
    // 201 Created: a new resource was created. The client polls the returned `id`.
    Ok((StatusCode::CREATED, Json(TriggerOrderResponse::from(order))))
}

/// 创建止盈单
/// POST /api/v1/trigger-orders/take-profit
///
/// 中文：与 create_stop_loss 形态完全对称 — 业务上 SL/TP 是同一类"价格触发"单，
///   只是触发方向不同。SL 在 trigger 之下击穿，TP 在 trigger 之上击穿。
///   共享同一份 service 代码路径，仅在落库时写入不同的 `trigger_direction` 字段。
///
/// English: Symmetric to `create_stop_loss` — SL and TP are the same shape of
///   "price-trigger" order with opposite directions (SL fires on a downward
///   cross, TP on an upward cross). They share the same service path and only
///   differ in the persisted `trigger_direction` enum.
pub async fn create_take_profit(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<CreateTakeProfitRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 与 create_stop_loss 结构对称 — 唯一差异是 service 调用不同。
    // Mirror of create_stop_loss; only the service call differs.
    let service = TriggerOrderService::new(db);
    let order = service
        .create_take_profit(
            user.user_id, // JWT 注入，防越权 / injected by JWT middleware
            req.position_id,
            &req.symbol,
            req.trigger_price,
            req.base_price,
            req.quantity,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(TriggerOrderResponse::from(order))))
}

/// 创建OCO单
/// POST /api/v1/trigger-orders/oco
///
/// 中文：OCO 在 service 层做原子事务 — 两腿要么一起创建，要么都不创建。
///   响应同时返回 `stop_loss` + `take_profit` 两个完整记录。
///   - 客户端通过比对 `stop_loss.oco_pair_id == take_profit.oco_pair_id`
///     确认 OCO 配对成功
///   - 触发任一腿后，service 会自动撤销另一腿（cancellation cascade），
///     客户端需轮询状态变更
///
/// English: OCO is created in a single atomic transaction — both legs or neither.
///   The response returns both full records so the client can verify pairing
///   by comparing `oco_pair_id`. The service auto-cancels the other leg on
///   trigger (cancellation cascade); the client must poll for that transition.
pub async fn create_oco(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<CreateOcoRequest>,
) -> Result<impl IntoResponse, AppError> {
    // OCO = 同时创建两条腿 (SL + TP)。service 在一个事务里完成，保证原子性。
    // OCO = create both legs (SL + TP) in one service transaction (atomicity).
    let service = TriggerOrderService::new(db);
    let (stop_loss, take_profit) = service
        .create_oco(
            user.user_id, // JWT 防越权 / JWT ownership guard
            req.position_id,
            &req.symbol,
            // 两侧价格必须分立（SL < TP 或 SL > TP），否则 service 拒绝。
            // Both prices must lie on opposite sides of entry, else service rejects.
            req.stop_loss_price,
            req.take_profit_price,
            req.base_price,
            req.quantity,
        )
        .await?;

    // 同时返回两腿完整记录，客户端可立即拿到 `oco_pair_id` 用于关联。
    // Return both legs so the client has `oco_pair_id` for correlation.
    Ok((
        StatusCode::CREATED,
        Json(OcoPairResponse {
            stop_loss: TriggerOrderResponse::from(stop_loss),
            take_profit: TriggerOrderResponse::from(take_profit),
        }),
    ))
}

/// 创建TWAP单
/// POST /api/v1/trigger-orders/twap
///
/// 中文：TWAP 与 SL/TP/OCO 的最大差异：
///   - 不绑定 `position_id`（它是开仓/平仓工具，方向由 `side` 决定）
///   - 创建后由后台 worker 按 `interval_secs` 节奏分批下市价单，
///     客户端通过轮询 `twap_executed_slices` 观察进度
///   - 校验重点在 `slice_quantity` 与 `quantity` 的整除性、
///     `interval_secs` 严格 > 0（service 层负责）
///
/// English: TWAP differs from SL/TP/OCO in two key ways:
///   - no `position_id` (it's an entry/exit tool; direction comes from `side`)
///   - a background worker slices the order on `interval_secs` cadence; the
///     client polls `twap_executed_slices` to observe progress
///   - service validates `slice_quantity` divides `quantity` and that
///     `interval_secs > 0`
pub async fn create_twap(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<CreateTwapRequest>,
) -> Result<impl IntoResponse, AppError> {
    // TWAP 校验比 SL/TP 严格 — service 会拒绝 quantity/slice_quantity 整除不彻底、
    // interval_secs <= 0、duration_secs < interval_secs 等异常组合。
    // TWAP has stricter validation than SL/TP — service rejects non-divisible
    // quantity/slice_quantity, non-positive intervals, duration < interval, etc.
    let service = TriggerOrderService::new(db);
    let order = service
        .create_twap(
            user.user_id, // JWT 防越权 / JWT ownership guard
            &req.symbol,
            &req.side,
            req.quantity,
            req.slice_quantity,
            req.interval_secs,
            req.duration_secs,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(TriggerOrderResponse::from(order))))
}

/// 查询条件单列表
/// GET /api/v1/trigger-orders
///
/// 中文：列表查询自动按 `user_id` 隔离 — service 强制 `WHERE user_id = ?`，
///   客户端无法越权看到他人订单。两个可选 query 字段在 service 层做白名单校验。
///
/// English: List is automatically partitioned by `user_id`; the service enforces
///   `WHERE user_id = ?` so cross-tenant reads are impossible. The two optional
///   query fields are whitelist-validated by the service.
pub async fn list_trigger_orders(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Query(query): Query<ListTriggerOrdersQuery>,
) -> Result<impl IntoResponse, AppError> {
    // 关键：service 必须接收 `user.user_id` 作为强约束 — 禁止在 service 内
    // 出现"按 user_id 过滤"被绕过（e.g. SELECT * 暴露他人订单）。
    // KEY: service MUST take `user.user_id` as a hard constraint. Never call
    // list endpoints without this filter (would leak other users' orders).
    let service = TriggerOrderService::new(db);
    let orders = service
        .list_trigger_orders(user.user_id, query.status, query.symbol)
        .await?;

    // Model → DTO 转换：把 SeaORM Model 转成 HTTP 响应结构（枚举 → 字符串、
    // datetime → RFC3339 字符串），详见下方 `From` impl。
    // Model → DTO conversion: enum → string, datetime → RFC3339 (see `From` impl).
    let response: Vec<TriggerOrderResponse> =
        orders.into_iter().map(TriggerOrderResponse::from).collect();

    Ok(Json(response))
}

/// 查询单个条件单
/// GET /api/v1/trigger-orders/:id
///
/// 中文：service 内部同时检查 `id = ?` 与 `user_id = ?` — UUID 存在但属于
///   他人时返回 `AppError::NotFound`（不是 `Forbidden`，避免泄露"该 UUID 存在"）。
///
/// English: The service checks `id = ?` AND `user_id = ?`. A UUID that exists
///   but belongs to another user returns `AppError::NotFound` (NOT `Forbidden`)
///   to avoid leaking the existence of someone else's order.
pub async fn get_trigger_order(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Path(order_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // service 的 get_trigger_order 双重过滤：id + user_id。
    // 命中但属于他人 → 404 (NotFound)；不存在 → 404。两种情况对外不可区分。
    // Service filters on both id and user_id. Owned-by-someone-else → 404,
    // not found → 404. Indistinguishable from the outside (no information leak).
    let service = TriggerOrderService::new(db);
    let order = service.get_trigger_order(user.user_id, order_id).await?;

    Ok(Json(TriggerOrderResponse::from(order)))
}

/// 取消条件单
/// DELETE /api/v1/trigger-orders/:id
///
/// 中文：cancel = 显式的两步流程：
///   1. 先 `get_trigger_order(user_id, order_id)` 做"读时鉴权"（防止直接 cancel
///      他人订单时留下审计痕迹或绕过 ownership check）
///   2. 再 `cancel_trigger_order(order_id, reason)` 真正改状态
///
///   OCO 联动取消：service 在 cancel 任一腿时自动取消另一腿（如果还 pending），
///   client 需通过轮询另一腿的状态变更来感知联动效果。
///
/// English: Cancel is intentionally a two-step flow:
///   1. First `get_trigger_order(user_id, order_id)` performs a read-time ownership
///      check (prevents audit-trail pollution and ownership-bypass attacks)
///   2. Then `cancel_trigger_order(order_id, reason)` flips the status
///
///   OCO cascade: the service auto-cancels the other leg (if still pending) when
///   one is cancelled; the client must poll the other leg to observe the cascade.
pub async fn cancel_trigger_order(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Path(order_id): Path<Uuid>,
    Json(req): Json<CancelTriggerOrderRequest>,
) -> Result<impl IntoResponse, AppError> {
    let service = TriggerOrderService::new(db);

    // 先验证权限（"读时鉴权"）。如果 order_id 不属于当前 user，service 返回
    // AppError::NotFound → 客户端拿到 404，无法区分"不存在"与"无权"，
    // 避免泄露订单存在性。
    // Read-time ownership check. If the order doesn't belong to `user.user_id`,
    // service returns AppError::NotFound → client gets 404. Indistinguishable
    // from "doesn't exist" — no existence-leak.
    service.get_trigger_order(user.user_id, order_id).await?;

    // 取消订单：`reason` 缺省时兜底为 "user_cancelled"，写入 trigger_reason 列。
    // Cancel: fall back to "user_cancelled" if reason is missing, persisted in
    // `trigger_reason` for audit.
    let reason = req.reason.unwrap_or_else(|| "user_cancelled".to_string());
    service.cancel_trigger_order(order_id, &reason).await?;

    // 204 No Content：DELETE 成功且无响应体（符合 REST 习惯）。
    // 204 No Content: DELETE succeeded with no body (REST convention).
    Ok(StatusCode::NO_CONTENT)
}

/// ==================== 路由注册 ====================
/// ==================== Router ====================
/// 注册触发订单路由
/// Register trigger-order routes onto an Axum `Router`.
///
/// 中文：所有路由共享 `/trigger-orders` 前缀（在 `main.rs` 中通过 `nest` 挂载）。
///   鉴权由外层 `middleware::from_fn(auth::jwt_auth)` 统一提供，handler 内
///   只需 `AuthenticatedUser` extractor 即可拿到 `user_id`。
///
/// English: All routes share the `/trigger-orders` prefix (mounted via `nest` in
///   `main.rs`). Auth is applied externally via `middleware::from_fn(auth::jwt_auth)`;
///   each handler simply declares an `AuthenticatedUser` extractor to get `user_id`.
pub fn router() -> Router<Arc<DatabaseConnection>> {
    // 路由表：4 个 POST 创建 + 1 个 GET 列表 + 1 个 GET 单个 + 1 个 DELETE 取消。
    // Router table: 4 POST create + 1 GET list + 1 GET single + 1 DELETE cancel.
    // - 列表与单个 GET 都已通过 `user.user_id` 做行级隔离。
    // - List and single GET are both row-partitioned by `user.user_id`.
    // - DELETE 是 "先读再写"，读时鉴权；详见 cancel_trigger_order 注释。
    // - DELETE is read-then-write; see cancel_trigger_order doc.
    Router::new()
        .route("/trigger-orders/stop-loss", post(create_stop_loss))
        .route("/trigger-orders/take-profit", post(create_take_profit))
        .route("/trigger-orders/oco", post(create_oco))
        .route("/trigger-orders/twap", post(create_twap))
        .route("/trigger-orders", get(list_trigger_orders))
        .route("/trigger-orders/{id}", get(get_trigger_order))
        .route("/trigger-orders/{id}", delete(cancel_trigger_order))
}
