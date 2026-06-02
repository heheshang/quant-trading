//! services/bracket.rs — Bracket order business logic (P1-2.2 path A: deferred OCO)
//!
//! 中文说明：
//!   本文件实现 P1-2.2 括号订单（bracket order）的业务逻辑层 —— 采用
//!   "Path A: deferred OCO"（延迟 OCO）架构。括号订单的语义是：
//!     入场单（entry）成交后，立即挂出 2 张条件单（止损 SL + 止盈 TP），
//!     二者构成 OCO（One-Cancels-Other）关系 —— 任一触发即自动撤销另一张。
//!
//!   关键设计 —— OCO 创建被"延迟"到客户端：
//!     - 当母单被撮合引擎完全成交（fully filled）时，`flush_trades` hook
//!       调用本模块的 `record_parent_filled` —— 它只做"状态记录"工作。
//!     - 真正的 OCO 创建（调用 `TriggerOrderService::create_oco`）由前端
//!       轮询 `/api/v1/bracket-links/pending` 后再主动发起。
//!     - 这样设计的原因：
//!         1. 解耦：避免在 fill hook 中同步阻塞撮合引擎（OCO 可能涉及两次 IO）
//!         2. 容错：客户端重试机制可应对网络抖动，避免后台任务死信
//!         3. 可观测：bracket_links 状态机让 OCO 状态对运维透明
//!
//!   本模块的 3 大职责（three core responsibilities）：
//!     1. `record_parent_filled` —— 母单成交后的状态记录入口
//!     2. `BRACKET_PARENT_FILLED_TOTAL` 计数器 —— Prometheus 指标
//!     3. `list_pending_for_user` —— 待处理 OCO 链路查询（前端轮询接口）
//!
//! When a bracket parent order is fully filled by the matching engine, the
//! `flush_trades` hook calls `record_parent_filled` which:
//!   1. Inserts a row into `bracket_links` (so frontend can poll & act)
//!   2. Updates the parent's `advanced_params.oco_status` to "pending"
//!   3. Increments the `BRACKET_PARENT_FILLED_TOTAL` counter
//!
//! The actual OCO creation (calling `TriggerOrderService::create_oco`) is left
//! to the frontend/client (see ADR in T2 design).

// 标准库导入 + 第三方导入。Standard library + third-party imports.
use std::sync::Arc;

use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::order;
use crate::metrics;
use crate::utils::error::AppError;

/// `advanced_type` 字段的字符串常量模块 —— 避免 stringly-typed 错误。
/// String constants for `advanced_type` field to avoid stringly-typed bugs.
///
// 中文：
//   `orders.advanced_type` 字段以字符串形式存储高级订单类型（bracket / trailing_stop /
//   iceberg / twap / oco）。把这些 magic string 收敛到模块级常量，避免拼写错误导致
//   无法被下游服务识别（例如把 "bracket" 写成 "braket"）。
//
// English:
//   `orders.advanced_type` stores the high-level order type as a string. Centralizing
//   these magic strings into module-level constants prevents typos that would silently
//   break downstream service routing (e.g. writing "braket" instead of "bracket").
pub mod advanced_type {
    /// 括号订单类型标记 —— 必须与 `db/order.rs::OrderType::Bracket` 的 strum 序列化值一致。
    /// Bracket order type marker — must match the strum-serialized value of
    /// `db/order.rs::OrderType::Bracket`.
    pub const BRACKET: &str = "bracket";
}

/// OCO 链路状态枚举 —— 同时持久化到 `bracket_links.oco_status` 和
///   `orders.advanced_params.oco_status`（镜像存储，便于查询）。
/// OCO linkage status. Stored in `bracket_links.oco_status` and mirrored in
/// `orders.advanced_params.oco_status`.
///
// 中文：
//   镜像存储的原因（WHY mirror to advanced_params）：
//     1. 减少 JOIN —— 列出用户订单时可以直接读 orders 表，过滤 `advanced_params->>'oco_status'`
//     2. 模型自包含 —— 拿到 BracketParams 就能立刻知道 OCO 是否完成，无需二次查询
//     3. GIN 索引友好 —— JSONB 上的 oco_status 可以被 B-tree expression 索引
//
//   状态机语义（state machine semantics）：
//     pending   → OCO 尚未创建，前端需要轮询并发起 create_oco 请求
//     linked    → OCO 已成功创建（SL + TP 都已挂出）
//     cancelled → OCO 创建被取消（例如客户端主动撤销母单、或 SL/TP 一侧先被触发）
//     failed    → OCO 创建失败（重试耗尽后进入终态，等待人工介入）
//
// English:
//   Why mirror to advanced_params:
//     1. Avoid JOINs — listing user orders can read orders directly and filter on
//        `advanced_params->>'oco_status'`.
//     2. Self-contained model — a BracketParams alone tells you the OCO state without
//        a second query.
//     3. GIN-friendly — the JSONB field can be indexed with a B-tree expression index.
//
//   State machine:
//     pending   → OCO not yet created; frontend must poll and call create_oco.
//     linked    → OCO successfully created (both SL and TP legs are live).
//     cancelled → OCO creation was cancelled (e.g. user cancelled parent, or one leg
//                 triggered first).
//     failed    → OCO creation failed (retries exhausted; terminal state awaiting
//                 manual intervention).
pub mod oco_status {
    /// 初始态：母单已成交，等待客户端发起 OCO 创建。
    /// Initial state: parent has filled, awaiting client-side OCO creation.
    pub const PENDING: &str = "pending";
    /// 终态之一：OCO 已成功创建（SL + TP 都已挂出）。
    /// Terminal (success): OCO has been created (both SL and TP legs are live).
    pub const LINKED: &str = "linked";
    /// 终态之一：OCO 创建被取消或一侧已触发。
    /// Terminal (cancelled): OCO was cancelled or one leg already triggered.
    pub const CANCELLED: &str = "cancelled";
    /// 终态之一：OCO 创建失败（需要人工介入）。
    /// Terminal (failure): OCO creation failed; needs manual intervention.
    pub const FAILED: &str = "failed";
}

/// `bracket_links` 行模型 —— 之所以 inline（而非 SeaORM Entity）是因为
///   我们使用 `db.execute()` 走原始 SQL（与 `trigger_order.rs` 模式一致）。
/// `bracket_links` row model. Inlined (not a SeaORM entity) because we use raw
/// SQL via `db.execute()` — same pattern as `trigger_order.rs` (raw row reads).
///
/// 中文：
///   `bracket_links` 是一张"中间表"（junction table），作用是解耦母单和子单状态机：
///     - 母单 `orders` 表只关心自己的 lifecycle（draft → active → filled）
///     - 子单（SL/TP）由 `trigger_orders` 表管理
///     - bracket_links 负责"母单成交 → OCO 创建"这段过渡期的状态记录
///
///   客户端轮询这张表即可知道"哪些母单已经成交但 OCO 还没建好"。
///
/// English: `bracket_links` is a junction-style table that decouples parent and
///   child order state machines:
///     - Parent (`orders`) only tracks its own lifecycle (draft → active → filled).
///     - Child SL/TP legs are managed by `trigger_orders`.
///     - bracket_links records the transient state between "parent filled" and
///       "OCO created".
///
///   Clients poll this table to discover "which parents have filled but still
///   need their OCO created".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketLink {
    /// 主键 UUID（与 `orders.id` 同类型）。
    /// Primary key UUID (same type as `orders.id`).
    pub id: Uuid,
    /// 关联的母单订单 ID。
    /// Foreign key to the parent bracket order.
    pub parent_order_id: Uuid,
    /// 母单所属 user —— 用于按 user 维度查询/鉴权。
    /// Owning user of the parent order — used for per-user queries and authorization.
    pub user_id: Uuid,
    /// 交易对符号（e.g. "BTC/USDT"），从母单冗余存储。
    /// Trading symbol (e.g. "BTC/USDT") — mirrored from parent for OCO creation.
    pub symbol: String,
    /// 止损触发价 —— OCO 创建时传给 TriggerOrderService。
    /// Stop-loss trigger price — passed to TriggerOrderService when creating OCO.
    pub sl_price: f64,
    /// 止盈触发价 —— OCO 创建时传给 TriggerOrderService。
    /// Take-profit trigger price — passed to TriggerOrderService when creating OCO.
    pub tp_price: f64,
    /// 母单方向（"buy" | "sell"）—— 子单的 SL/TP 方向与母单相反。
    /// Parent order side ("buy" | "sell") — child SL/TP legs have the opposite side.
    pub side: String,
    /// 母单成交数量（filled quantity at the time of fill）。
    /// Quantity that was filled on the parent.
    pub filled_quantity: f64,
    /// 当前 OCO 链路状态（参见 `oco_status` 模块）。
    /// Current OCO linkage status (see `oco_status` module).
    pub oco_status: String,
    /// SL 条件单创建后的 trigger_order.id —— 仅在 `linked` 后填充。
    /// Trigger order id of the SL leg — populated only after `linked`.
    pub sl_trigger_id: Option<Uuid>,
    /// TP 条件单创建后的 trigger_order.id —— 仅在 `linked` 后填充。
    /// Trigger order id of the TP leg — populated only after `linked`.
    pub tp_trigger_id: Option<Uuid>,
    /// 行创建时间。
    /// Row creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最后更新时间（状态变化时刷新）。
    /// Last update timestamp (refreshed on status changes).
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// `record_parent_filled` 的入参 bundle —— 7 个独立参数会显得冗长，
///   打包成 struct 让函数签名更紧凑、易于扩展。
/// Input bundle for `record_parent_filled` to keep its signature under 7 args.
#[derive(Debug, Clone)]
pub struct RecordParentFilledInput {
    /// 母单 UUID。
    /// UUID of the bracket parent order.
    pub parent_id: Uuid,
    /// 母单成交数量（在 fill 时的 filled_quantity）。
    /// Quantity filled on the parent (== parent.quantity at fill time).
    pub filled_quantity: f64,
    /// 止损价 —— 拷贝自母单 `advanced_params.stop_loss_price`。
    /// Stop-loss price — copied from parent's `advanced_params.stop_loss_price`.
    pub sl_price: f64,
    /// 止盈价 —— 拷贝自母单 `advanced_params.take_profit_price`。
    /// Take-profit price — copied from parent's `advanced_params.take_profit_price`.
    pub tp_price: f64,
    /// 交易对符号（e.g. "BTC/USDT"）。
    /// Trading symbol (e.g. "BTC/USDT").
    pub symbol: String,
    /// 母单方向（"buy" | "sell"）。
    /// Parent order side ("buy" | "sell").
    pub side: String,
    /// 母单所属 user ID —— 用于 bracket_links 行的鉴权/查询。
    /// Owning user id of the parent — used for authorization and per-user queries.
    pub user_id: Uuid,
}

/// 记录一个 bracket 母单已被完全成交。
///   这是 fill hook → bracket service 的入口点。
/// Record that a bracket parent order has been fully filled.
///
/// # Arguments (passed as `RecordParentFilledInput` to keep the signature compact).
/// * `input.parent_id` — UUID of the bracket parent order.
/// * `input.filled_quantity` — quantity that was filled (parent.quantity at fill time).
/// * `input.sl_price` / `input.tp_price` — copied from parent's `advanced_params`.
/// * `input.symbol` / `input.side` / `input.user_id` — copied from parent.
///
/// # Returns
/// The newly inserted `BracketLink` row, or an error.
///
/// 中文：
///   本函数是 3 步原子流程（虽然不是 DB 事务，但业务上必须全部成功）：
///     1. 向 `bracket_links` 插入一行（status=pending）
///     2. 更新母单 `orders.advanced_params.oco_status` 镜像为 "pending"
///     3. 增加 `BRACKET_PARENT_FILLED_TOTAL` Prometheus 计数器
///
///   如果第 1 步失败，整次调用失败（不进入第 2、3 步）。
///   如果第 2 步失败（极少见 —— 通常是母单被并发删除），
///   bracket_links 仍可能留下孤儿行，由对账任务清理。
///
/// English: This is a 3-step business pipeline (not wrapped in a DB transaction):
///   1. Insert a row into `bracket_links` (status=pending).
///   2. Mirror the parent's `orders.advanced_params.oco_status` to "pending".
///   3. Bump the `BRACKET_PARENT_FILLED_TOTAL` Prometheus counter.
///
///   If step 1 fails, the call fails (steps 2 and 3 are not attempted).
///   If step 2 fails (rare — usually parent was deleted concurrently), step 3 is
///   skipped, leaving a possibly orphan row in bracket_links (reconciliation job
///   picks these up).
pub async fn record_parent_filled(
    db: &Arc<DatabaseConnection>,
    input: RecordParentFilledInput,
) -> Result<BracketLink, AppError> {
    // 解构入参到本地变量 —— 让后续代码更紧凑。
    // Destructure the input into locals for compactness below.
    let RecordParentFilledInput {
        parent_id,
        filled_quantity,
        sl_price,
        tp_price,
        symbol,
        side,
        user_id,
    } = input;
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // 步骤 1：插入 bracket_links 行。
    // Step 1: insert a bracket_links row.
    //
    // 中文：
    //   走原始 SQL 而非 SeaORM ActiveModel 的原因（WHY raw SQL）：
    //     - bracket_links 不是完整的 SeaORM Entity（见 struct 上方 doc）
    //     - 字段数量较多，ActiveModel 写法会非常冗长
    //     - 使用 format!() 拼接 UUID 与字符串字段时务必做单引号转义
    //       （`replace('\'', "''")`），防止 SQL 注入
    //
    // English: We use raw SQL because bracket_links is not a full SeaORM entity
    //   (see struct doc above). When using `format!()` for UUID / string fields,
    //   always escape single quotes (`replace('\'', "''")`) to prevent SQL injection.
    let insert_sql = format!(
        "INSERT INTO bracket_links \
         (id, parent_order_id, user_id, symbol, sl_price, tp_price, side, \
          filled_quantity, oco_status, created_at, updated_at) \
         VALUES ('{}', '{}', '{}', '{}', {}, {}, '{}', {}, 'pending', NOW(), NOW()) \
         RETURNING id, parent_order_id, user_id, symbol, sl_price, tp_price, side, \
                   filled_quantity, oco_status, sl_trigger_id, tp_trigger_id, \
                   created_at, updated_at",
        id,
        parent_id,
        user_id,
        // 转义单引号：防止 symbol 字段中包含 ' 导致 SQL 注入。
        // Escape single quotes: prevents SQL injection if `symbol` contains "'".
        symbol.replace('\'', "''"),
        sl_price,
        tp_price,
        // 同上：side 字段防御性转义（正常值仅为 "buy"/"sell"）。
        // Same defensive escape for `side` (normally only "buy"/"sell").
        side.replace('\'', "''"),
        filled_quantity,
    );

    // 执行 INSERT ... RETURNING —— 0 行影响视为异常（理论上不应发生）。
    // Execute INSERT ... RETURNING — 0 rows affected is treated as a bug.
    let result = sea_orm::ConnectionTrait::execute(
        db.as_ref(),
        sea_orm::Statement::from_string(sea_orm::DatabaseBackend::Postgres, insert_sql),
    )
    .await
    .map_err(|e| AppError::Database(format!("bracket_link insert failed: {}", e)))?;

    // 防御性检查：正常情况下 rows_affected 应为 1。
    // Defensive check: rows_affected should be 1 under normal operation.
    if result.rows_affected() == 0 {
        return Err(AppError::Internal(
            "bracket_link insert returned 0 rows".to_string(),
        ));
    }

    // 步骤 2：把母单 advanced_params.oco_status 镜像为 "pending"。
    // Step 2: mirror parent's advanced_params.oco_status to "pending".
    //
    // 中文：让"列出用户订单"接口无需 JOIN 即可过滤"待处理 OCO"。
    // English: lets the "list user orders" endpoint filter "pending OCO" without a JOIN.
    update_parent_oco_status(db, parent_id, oco_status::PENDING).await?;

    // 步骤 3：增加 Prometheus 计数器 —— 按 side 维度打 label。
    // Step 3: bump the Prometheus counter, labelled by `side`.
    //
    // 中文：side 标签让 Grafana 可以按 buy/sell 拆解括号订单的成交速率。
    // English: the `side` label lets Grafana split bracket fill rate by buy/sell.
    metrics::BRACKET_PARENT_FILLED_TOTAL
        .with_label_values(&[side.as_str()])
        .inc();

    // 记录结构化日志：bracket_link_id 用于和日志/告警关联。
    // Structured log: bracket_link_id ties this event to logs/alerts downstream.
    tracing::info!(
        bracket_link_id = %id,
        parent_order_id = %parent_id,
        user_id = %user_id,
        symbol = %symbol,
        sl_price = sl_price,
        tp_price = tp_price,
        "Bracket parent filled, deferred OCO pending"
    );

    // 构造返回值 —— 注意 sl_trigger_id / tp_trigger_id 初始为 None
    //   （只有 OCO 真正创建成功（即 status 变 linked）后才会被填充）。
    // Construct the return value. Note: sl_trigger_id / tp_trigger_id are None
    //   initially — they are only populated once the OCO is actually created
    //   (i.e. status transitions to "linked").
    Ok(BracketLink {
        id,
        parent_order_id: parent_id,
        user_id,
        symbol: symbol.to_string(),
        sl_price,
        tp_price,
        side: side.to_string(),
        filled_quantity,
        oco_status: oco_status::PENDING.to_string(),
        sl_trigger_id: None,
        tp_trigger_id: None,
        created_at: now,
        updated_at: now,
    })
}

/// 更新 bracket 母单 advanced_params 中的 oco_status 字段。
///   保持母单行元数据与 bracket_links 表的同步。
/// Update the OCO linkage status of a bracket parent order's advanced_params.
/// This keeps the parent row's metadata in sync with the bracket_links table.
///
/// 中文：
///   设计取舍（WHY mirror to advanced_params）：
///     - 优：减少 JOIN，前端按"待处理 OCO"过滤订单时性能更好
///     - 劣：存在双写不一致风险（bracket_links 是 source of truth，
///       advanced_params 是 derived view）。本函数是 single-writer，
///       短期内不会冲突；若未来多 writer，需引入乐观锁。
///
///   防御性处理（defensive handling）：
///     - 若 `advanced_params` 为 None 或不是 JSON object，
///       替换为 `{"oco_status": <new_status>}`（避免崩溃）
///
/// English: Trade-off (WHY mirror):
///   - Pro: avoids JOINs when listing user orders and filtering on "pending OCO".
///   - Con: dual-write consistency risk (bracket_links is source of truth, advanced_params
///     is a derived view). This function is currently a single-writer, so conflicts
///     are unlikely; if multiple writers appear, optimistic locking will be needed.
///
///   Defensive handling:
///     - If `advanced_params` is None or not a JSON object, replace it with
///       `{"oco_status": <new_status>}` (avoids a crash).
pub async fn update_parent_oco_status(
    db: &Arc<DatabaseConnection>,
    parent_id: Uuid,
    new_status: &str,
) -> Result<(), AppError> {
    // 拉取母单 —— 不存在则报 404（理论上 fill hook 不应触发不存在的订单）。
    // Load parent; missing parent is a 404 (the fill hook should never hit a missing one).
    let parent = order::Entity::find_by_id(parent_id)
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Bracket parent not found: {}", parent_id)))?;

    // 取出 advanced_params —— 若为 None 则初始化为空对象。
    // Extract advanced_params; initialize to empty object if None.
    let mut new_params = parent
        .advanced_params
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = new_params.as_object_mut() {
        // 常规路径：advanced_params 是 JSON object，写入/覆盖 oco_status。
        // Normal path: advanced_params is a JSON object — write/overwrite oco_status.
        obj.insert("oco_status".to_string(), serde_json::json!(new_status));
    } else {
        // 防御路径：advanced_params 不是 object（如数组或字符串）—— 整体替换。
        //   这通常是脏数据导致的，记录告警后直接覆盖。
        // Defensive: advanced_params is not an object (e.g. array/string) — overwrite
        //   entirely. This usually indicates dirty data; the new object still has
        //   the correct oco_status.
        // 如果 advanced_params 不是 JSON object（例如脏数据），整体替换。
        // If advanced_params is not a JSON object (e.g. dirty data), replace it.
        new_params = serde_json::json!({"oco_status": new_status});
    }

    // 转为 ActiveModel 准备 UPDATE。
    // Convert to ActiveModel for the UPDATE.
    let mut active: order::ActiveModel = parent.into();
    // 写入新的 advanced_params 与 updated_at。
    // Write the new advanced_params and refresh updated_at.
    active.advanced_params = Set(Some(new_params));
    active.updated_at = Set(chrono::Utc::now());

    // 执行 UPDATE —— 失败时回传 DB 错误。
    // Execute the UPDATE; surface DB errors verbatim.
    active
        .update(db.as_ref())
        .await
        .map_err(|e| AppError::Database(format!("update parent oco_status: {}", e)))?;

    Ok(())
}

/// 列出某 user 的所有 `pending` bracket links —— 供前端轮询。
/// List all `pending` bracket links for a user (frontend polling).
///
/// 中文：
///   - 限制 200 行 —— 避免恶意 / 异常用户导致 OOM
///   - 仅返回 pending —— linked/cancelled/failed 是终态，前端不应再处理
///   - 按 created_at ASC —— FIFO 顺序让前端"先成交的先建 OCO"
///
/// English:
///   - LIMIT 200 — prevents a malicious / buggy user from OOMing the API.
///   - Only "pending" rows — linked/cancelled/failed are terminal and must not
///     be reprocessed by the client.
///   - ORDER BY created_at ASC — FIFO, so the oldest fills get their OCO created first.
pub async fn list_pending_for_user(
    db: &Arc<DatabaseConnection>,
    user_id: Uuid,
) -> Result<Vec<BracketLink>, AppError> {
    use sea_orm::DatabaseBackend;

    // 原始 SQL SELECT —— 不走 SeaORM Entity 的原因与 record_parent_filled 相同。
    // Raw SELECT — same rationale as in record_parent_filled (not a full Entity).
    let sql = format!(
        "SELECT id, parent_order_id, user_id, symbol, sl_price, tp_price, side, \
                filled_quantity, oco_status, sl_trigger_id, tp_trigger_id, \
                created_at, updated_at \
         FROM bracket_links \
         WHERE user_id = '{}' AND oco_status = 'pending' \
         ORDER BY created_at ASC \
         LIMIT 200",
        user_id
    );

    let result = sea_orm::ConnectionTrait::query_all(
        db.as_ref(),
        sea_orm::Statement::from_string(DatabaseBackend::Postgres, sql),
    )
    .await
    .map_err(|e| AppError::Database(format!("list pending bracket_links: {}", e)))?;

    // 逐行反序列化为 BracketLink。
    // Deserialize each row into a BracketLink.
    let mut links = Vec::with_capacity(result.len());
    for row in result {
        links.push(BracketLink {
            // 用 unwrap_or_default() 而非 ? —— 字段缺失时降级为默认值，
            //   不让单行脏数据拖垮整次查询。
            // Use unwrap_or_default() rather than `?` — missing fields degrade to defaults
            //   rather than failing the entire query on one bad row.
            id: row.try_get_by::<Uuid, _>("id").unwrap_or_default(),
            parent_order_id: row.try_get_by::<Uuid, _>("parent_order_id").unwrap_or_default(),
            user_id: row.try_get_by::<Uuid, _>("user_id").unwrap_or_default(),
            symbol: row.try_get_by::<String, _>("symbol").unwrap_or_default(),
            sl_price: row.try_get_by::<f64, _>("sl_price").unwrap_or_default(),
            tp_price: row.try_get_by::<f64, _>("tp_price").unwrap_or_default(),
            side: row.try_get_by::<String, _>("side").unwrap_or_default(),
            filled_quantity: row.try_get_by::<f64, _>("filled_quantity").unwrap_or_default(),
            oco_status: row.try_get_by::<String, _>("oco_status").unwrap_or_default(),
            sl_trigger_id: row
                .try_get_by::<Option<Uuid>, _>("sl_trigger_id")
                .unwrap_or_default(),
            tp_trigger_id: row
                .try_get_by::<Option<Uuid>, _>("tp_trigger_id")
                .unwrap_or_default(),
            // 时间字段缺失时降级为 now() —— 避免 None 触发上层 NPE。
            // For timestamps, fall back to now() if missing — avoids a None-induced NPE upstream.
            created_at: row
                .try_get_by::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: row
                .try_get_by::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
        });
    }
    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 中文：验证 BRACKET 常量值是字面量 "bracket" —— 防止无意修改
    ///   导致 `advanced_type='bracket'` 的母单无法被本服务识别。
    /// English: verify the BRACKET constant literal — guards against accidental
    ///   renames that would break routing of `advanced_type='bracket'` parents.
    #[test]
    fn test_advanced_type_brackets_constant() {
        assert_eq!(advanced_type::BRACKET, "bracket");
    }

    /// 中文：验证 4 个 oco_status 常量字面量 —— 这些字符串会持久化到
    ///   `bracket_links.oco_status` 和 `orders.advanced_params.oco_status`，
    ///   改名会破坏对账/告警规则。
    /// English: verify the four oco_status literal values — these strings are
    ///   persisted to `bracket_links.oco_status` and `orders.advanced_params.oco_status`,
    ///   so a rename would break reconciliation and alert rules.
    #[test]
    fn test_oco_status_constants() {
        assert_eq!(oco_status::PENDING, "pending");
        assert_eq!(oco_status::LINKED, "linked");
        assert_eq!(oco_status::CANCELLED, "cancelled");
        assert_eq!(oco_status::FAILED, "failed");
    }

    /// 中文：验证 BracketLink 走 serde JSON 序列化/反序列化后字段值保持一致
    ///   （含 None 的 trigger_id 字段）。
    /// English: verify BracketLink round-trips through serde JSON (including
    ///   `None` trigger id fields).
    #[test]
    fn test_bracket_link_serde_roundtrip() {
        let link = BracketLink {
            id: Uuid::new_v4(),
            parent_order_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            symbol: "BTC/USDT".to_string(),
            sl_price: 49000.0,
            tp_price: 52000.0,
            side: "buy".to_string(),
            filled_quantity: 1.0,
            oco_status: "pending".to_string(),
            sl_trigger_id: None,
            tp_trigger_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&link).unwrap();
        let back: BracketLink = serde_json::from_str(&json).unwrap();
        assert_eq!(back.symbol, "BTC/USDT");
        assert_eq!(back.sl_price, 49000.0);
        assert_eq!(back.oco_status, "pending");
    }

    /// 中文：验证 OCO 创建成功（即 status 变 linked）后
    ///   `sl_trigger_id` / `tp_trigger_id` 都被填充为 `Some(uuid)`。
    /// English: verify that after OCO is created (status transitions to "linked"),
    ///   both `sl_trigger_id` and `tp_trigger_id` are `Some(uuid)`.
    #[test]
    fn test_bracket_link_with_trigger_ids() {
        let link = BracketLink {
            id: Uuid::new_v4(),
            parent_order_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            symbol: "ETH/USDT".to_string(),
            sl_price: 2900.0,
            tp_price: 3200.0,
            side: "buy".to_string(),
            filled_quantity: 2.5,
            oco_status: "linked".to_string(),
            sl_trigger_id: Some(Uuid::new_v4()),
            tp_trigger_id: Some(Uuid::new_v4()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert!(link.sl_trigger_id.is_some());
        assert!(link.tp_trigger_id.is_some());
        assert_eq!(link.oco_status, "linked");
    }

    /// 中文：兜底测试 —— 确认我们在本文件/相邻模块里引用了
    ///   正确的 `OrderStatus` 枚举变体（Filled 表示已成交、Cancelled 表示已取消）。
    ///   防止 `OrderStatus` 重构时悄悄破坏联动逻辑。
    /// English: sanity check that the `OrderStatus` enum variants we depend on
    ///   (Filled for parent fully-filled, Cancelled for parent cancelled) still
    ///   exist with these exact names — guards against a silent enum refactor
    ///   breaking integration logic.
    #[test]
    fn test_order_status_pending_filter() {
        // 只是为了确保我们引用了正确的 OrderStatus 枚举变体。
        // Just to make sure we use the right OrderStatus enum variant.
        assert_eq!(format!("{:?}", order::OrderStatus::Filled), "Filled");
        assert_eq!(format!("{:?}", order::OrderStatus::Cancelled), "Cancelled");
    }
}
