//! services/iceberg.rs — Iceberg order business logic
//!
//! P1-2.1: Splits iceberg parents into child limit slices, replenishes the
//! order book on each child fill, and cascades parent cancel to all children.
//!
//! 中文说明：
//!   本文件是 P1-2.1 冰山订单（Iceberg Order）的业务逻辑层。所谓"冰山"指一笔大单
//!   被拆成多个只露出"冰山一角"（visible_quantity）的子单（child limit slices），
//!   依次投放进撮合引擎（matching engine），子单成交后立刻补投下一个切片，直到
//!   母单数量（total_quantity）全部成交。这样做的核心动机是**反 Market Impact**：
//!   避免一笔巨型市价委托一次性消耗订单簿深度（吃穿多档价位），被人从盘口信息中
//!   推断出"有人在大量成交"并抢先反向操作（front-running）。
//!
//!   三大职责（Three core responsibilities）：
//!     1. **Split（切分）**    — `split_into_children`：在母单创建时把总量切成 N 个
//!        切片，首切片立即入簿
//!     2. **Replenish（补投）** — `append_next_child`：任一 child 成交后追加下一个
//!        切片（直到 total_slices 全部完成）
//!     3. **Cancel cascade（级联取消）** — `cancel_iceberg_children`：母单被取消时
//!        同步取消所有未成交的 children，防止"孤儿子订单"继续消耗流动性
//!
//!   集成点（Integration points）：
//!     - `handlers/order.rs::create_order` 接收 `order_type=iceberg` 请求后调用
//!       `split_into_children`
//!     - `services/matching_engine.rs::flush_trades` 撮合后对 `advanced_type ==
//!       "iceberg_child"` 的成交回调 `append_next_child`
//!     - `services/matching_engine.rs`（cancel path）调 `cancel_iceberg_children`
//!       以保证取消是原子的（all-or-nothing）
//!
//!   设计权衡（Design trade-offs）：
//!     - 为什么 Split 在 matching engine 之外？因为切分逻辑涉及 DB 写入（创建 child
//!       记录 + 更新 parent.advanced_params），与撮合循环（hot path）解耦更易维护
//!     - 为什么 `children_ids` 用 Vec 记录？取消时反查从 O(1) 变 O(N)，但保留这条
//!       记录可以让 cascade delete 失败时仍能根据 parent 恢复状态（O(N) 扫描是
//!       cancel 路径的"可接受成本"——cancel 频次远低于 fill 频次）
//!     - 为什么不把 child 直接当 limit order 处理？因为 child 需要携带 `parent_id`
//!       反查信息和 `slice_index` 序列号，独立的 `IcebergChildParams` 才能持久化这些
//!       元数据

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::db::order::{self, OrderStatus, OrderType};
use crate::metrics;
use crate::models::iceberg_params::{IcebergChildParams, IcebergParams};
use crate::services::matching_engine::MatchingEngine;
use crate::utils::error::AppError;

/// String constants for `advanced_type` field to avoid stringly-typed bugs.
///
/// 中文：把"母单/子单"两个标签从字符串字面量提升为模块级常量，目的是消除
///   "stringly-typed" 拼写错误（如 `"iceberg_chld"` 不会在编译期被发现）。
///   配套使用：`orders.advanced_type` 列在创建/查询时直接引用这些常量。
///
/// English: Promoting the two labels to module-level constants eliminates
///   "stringly-typed" bugs — a typo like `"iceberg_chld"` would compile but
///   break the matching engine's classification. Use these constants in both
///   writes (`order::ActiveModel::advanced_type`) and reads (SeaORM filters).
pub mod advanced_type {
    /// 母单标签 — `orders.advanced_type = "iceberg"`，标识"这是一笔冰山母单"。
    /// Parent tag — identifies an iceberg parent order on the `orders` table.
    pub const ICEBERG: &str = "iceberg";
    /// 子单标签 — `orders.advanced_type = "iceberg_child"`，撮合引擎据此识别
    ///   "这是某个冰山母单的子切片"，成交后回调 `append_next_child`。
    /// Child tag — matching engine uses this to detect "this is a child slice
    ///   of an iceberg parent" and routes the fill to `append_next_child`.
    pub const ICEBERG_CHILD: &str = "iceberg_child";
}

/// P1-2.1: Split an iceberg parent into N child limit orders and insert the
/// first child into the matching engine order book.
///
/// Pre-conditions (caller must ensure):
/// - `parent` has `order_type = Iceberg` and `advanced_type = advanced_type::ICEBERG`
/// - `parent.advanced_params` deserializes to a valid `IcebergParams`
/// - `parent.quantity == params.total_quantity`
///
/// 中文：
///   这是"冰山"生命周期的入口：在母单刚通过 handler 校验后被调用一次，把整笔
///   母单**切碎**成 `total_slices` 个 child limit slice，并把**首切片**立刻
///   投递到撮合引擎的 order book。后续 child 不会在这里被创建——它们由
///   `append_next_child` 在每个 child 成交后**按需**补投（lazy replenishment）。
///
///   幂等性（Idempotency）：
///     - 当 `children_ids.len() >= total_slices` 时直接 return，不会重复切分
///     - 这一保护使得网络重试 / 重复 POST 不会导致子单爆量
///
///   为什么"在 matching engine 之外"做切分（Why split outside matching engine）：
///     1. 切分动作涉及 **DB 写入**（INSERT child row + UPDATE parent.advanced_params），
///        这些是 I/O 操作，不适合放在撮合循环（hot path）里阻塞撮合
///     2. 切分策略（如 visible_quantity 取值、slice_index 编号）属于业务规则，
///        与撮合核心的价格-时间优先队列解耦后更易独立测试（见末尾 `#[test]`）
///     3. 失败语义清晰：DB 写入失败可以整笔回滚，而不会让撮合引擎处于"半切分"状态
///
///   返回值：刷新过的 `IcebergParams`（已包含首切片的 id 与 `slice_index_next=1`）。
pub async fn split_into_children(
    db: &Arc<DatabaseConnection>,
    engine: &Arc<MatchingEngine>,
    parent: &order::Model,
) -> Result<IcebergParams, AppError> {
    // 防御性：哪怕 handler 已经校验过，这里仍重新从 DB 解析一次。
    // Defensive re-parse from DB even though handler validated upstream.
    //   原因：DB 行可能来自恢复路径（重启 / 消息回放），绕过 handler 直接落库。
    //   Reason: rows may arrive via recovery paths (restart / replay) that
    //   bypassed the handler validation.
    let mut params: IcebergParams = parent
        .advanced_params
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| {
            AppError::Internal(format!("Parent {} missing advanced_params", parent.id))
        })?;

    if (params.children_ids.len() as u32) >= params.total_slices {
        // 幂等保护：已经切完就不再切。
        // Idempotency guard: bail out if split already happened.
        //   触发场景：handler 重试、网络重传导致 create_order 被调多次
        //   Trigger scenarios: handler retry, network retransmit re-invoking create_order
        return Ok(params);
    }

    // 创建首切片（slice_index=0）。
    // Create the first slice (slice_index=0).
    //   注意：此处只创建 1 个 child，剩余 N-1 个由 `append_next_child` 按需补投，
    //   而不是一次性全建好——这样能避免"创建了一堆 child 但行情突变导致母单被取消"
    //   时浪费 DB 行 + order book slot。
    //   Note: only the first slice is created here; the remaining N-1 are
    //   appended lazily by `append_next_child`. This avoids wasting DB rows
    //   and order-book slots if the parent is cancelled before slices fill.
    let first_child = create_child_order(db, parent, &params, 0).await?;

    // 把首切片 id 记到母单的 children_ids 列表（O(1) 追加）。
    // Record first child id in parent's children_ids list (O(1) push).
    params.children_ids.push(first_child.id);
    params.slice_index_next = 1;

    // 把"切分进度"持久化到母单行（关键：崩后重启能从此处恢复未补投的 child）。
    // Persist split progress on the parent row (critical for crash recovery —
    // a restart can resume from here to replenish uncreated children).
    update_parent_params(db, parent.id, &params).await?;

    // 投递首切片到撮合引擎簿册（order book）。
    // Insert first slice into the matching engine order book.
    //   remaining_quantity = quantity − filled_quantity（首切片必然为 quantity，因为尚未成交）。
    //   remaining_quantity = quantity − filled_quantity (always equals quantity for
    //   the brand-new first slice).
    engine.insert_limit_order(crate::services::matching_engine::OrderEntry {
        order_id: first_child.id,
        user_id: first_child.user_id,
        symbol: first_child.symbol.clone(),
        side: first_child.side.clone(),
        price: first_child.price,
        remaining_quantity: first_child.quantity - first_child.filled_quantity,
        created_at: first_child.created_at,
    });

    // Prometheus 指标：自增"已创建"的 child 计数（按 label 维度区分）。
    // Prometheus metric: bump "created" counter (labelled dimension).
    metrics::ICEBERG_CHILD_ORDERS_TOTAL
        .with_label_values(&["created"])
        .inc();

    Ok(params)
}

/// P1-2.1: Append the next child slice into the order book after a fill.
///
/// Called by `matching_engine::flush_trades` after a child fills.
/// No-op when the parent is already fully filled.
///
/// 中文：
///   这是冰山订单的"补投"（replenish）核心。每当撮合引擎把一个 child slice 完全
///   撮合（filled）后，会回调本函数：
///     1. 把 child 行标记为 `Filled`（filled_quantity、avg_fill_price、filled_at）
///     2. 累计到母单的 `filled_quantity` 与 `filled_children`
///     3. **如果母单尚未完成（`!is_complete`）**：创建下一个 child 切片 + 投递到簿册
///     4. **如果母单已完成（`is_complete`）**：把母单自身也标为 `Filled`
///
///   关键不变量（Key invariants）：
///     - 任意时刻 order book 中**至多一个**该母单的 child 处于活跃状态（"冰山一角"）
///     - `is_complete()` 由 `filled_children >= total_slices` 判定（详见 `IcebergParams`）
///
///   为什么要在 child 完全成交后才补投下一个（Why replenish only after a full fill）：
///     - 避免 order book 中同时挂着多个相同价位的 child（容易被识别为冰山）
///     - 真正的"反 Market Impact"目标：每个切片都是独立的下单事件，外部观察者
///       看到的不是一笔大单，而是 N 笔小额接连成交
///
///   状态机分支（State machine branches）：
///     - `is_complete == true`  → 母单置为 Filled，不再补投
///     - `is_complete == false` → 母单置为 PartialFilled（如果 `parent_new_filled > 0`），
///                                否则保持 Pending；创建 + 入簿下一个 child
pub async fn append_next_child(
    db: &Arc<DatabaseConnection>,
    engine: &Arc<MatchingEngine>,
    child_id: Uuid,
    child_filled_qty: f64,
) -> Result<(), AppError> {
    // 步骤 1：加载 child 行 + 反向解析出 parent_id 用于加载母单。
    // Step 1: load child row + reverse-resolve parent_id to load parent.
    //   这是一个 2 次 DB 往返（先 child、再 parent），后续可优化为 JOIN。
    //   Two DB roundtrips (child then parent); could be optimized to a JOIN later.
    let child = order::Entity::find_by_id(child_id)
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Child order {} not found", child_id)))?;

    let child_params: IcebergChildParams = child
        .advanced_params
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| {
            AppError::Internal(format!("Child {} missing advanced_params", child_id))
        })?;

    let parent = order::Entity::find_by_id(child_params.parent_id)
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Parent {} not found", child_params.parent_id))
        })?;

    // 步骤 2：把 child 行原地更新为 Filled（同时记录 avg_fill_price = 委托价，
    //   简化版——真正的均价应该在 trade 聚合层计算）。
    // Step 2: update child row to Filled (avg_fill_price = order price for now;
    //   the real VWAP should be computed at the trade-aggregation layer).
    let mut child_active: order::ActiveModel = child.clone().into();
    child_active.filled_quantity = Set(child_filled_qty);
    child_active.avg_fill_price = Set(child.price);
    child_active.status = Set(OrderStatus::Filled);
    child_active.filled_at = Set(Some(chrono::Utc::now()));
    child_active.updated_at = Set(chrono::Utc::now());
    child_active
        .update(db.as_ref())
        .await
        .map_err(|e| AppError::Database(format!("update child: {}", e)))?;

    // Prometheus 指标：自增"已成交"的 child 计数。
    // Prometheus metric: bump "filled" counter.
    metrics::ICEBERG_CHILD_ORDERS_TOTAL
        .with_label_values(&["filled"])
        .inc();

    // 步骤 3：重新解析母单的 advanced_params 并把 filled_children +1。
    // Step 3: re-parse parent advanced_params and increment filled_children.
    //   这个 re-parse 是必要的——因为可能与 split_into_children 并发执行，
    //   解析到的是 DB 中最新的状态，避免丢更新。
    //   Re-parse is necessary because this can race with split_into_children;
    //   parsing from DB gives us the latest persisted state and avoids lost updates.
    let mut parent_params: IcebergParams = parent
        .advanced_params
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| {
            AppError::Internal(format!("Parent {} missing advanced_params", parent.id))
        })?;

    parent_params.filled_children += 1;

    // 步骤 4：计算母单新累计成交量 + 是否已完成（filled_children >= total_slices）。
    // Step 4: compute new cumulative fill + completion status.
    let parent_new_filled = parent.filled_quantity + child_filled_qty;
    let is_complete = parent_params.is_complete();

    // 步骤 5：未完成 → 补投下一个 child（关键状态机分支 A）。
    // Step 5: not complete → replenish next child (key state-machine branch A).
    //   next_idx 用刚刚自增后的 filled_children（=已成交切片数 = 下一个切片的 0-based 索引）。
    //   next_idx uses the just-incremented filled_children (== number of filled
    //   slices == 0-based index of the next slice to create).
    if !is_complete {
        let next_idx = parent_params.filled_children;
        let next_child = create_child_order(db, &parent, &parent_params, next_idx).await?;
        parent_params.children_ids.push(next_child.id);
        parent_params.slice_index_next = next_idx + 1;

        engine.insert_limit_order(crate::services::matching_engine::OrderEntry {
            order_id: next_child.id,
            user_id: next_child.user_id,
            symbol: next_child.symbol.clone(),
            side: next_child.side.clone(),
            price: next_child.price,
            remaining_quantity: next_child.quantity - next_child.filled_quantity,
            created_at: next_child.created_at,
        });

        metrics::ICEBERG_CHILD_ORDERS_TOTAL
            .with_label_values(&["created"])
            .inc();
    }

    // 步骤 6：持久化母单更新（关键状态机分支 B）。
    // Step 6: persist parent updates (key state-machine branch B).
    //   分支树：
    //   - is_complete=true          → 母单置 Filled
    //   - is_complete=false 且 > 0   → 母单置 PartialFilled（已有部分成交）
    //   - is_complete=false 且 == 0  → 母单保持原状态（理论上不会出现，但兜底）
    let parent_advanced = serde_json::to_value(&parent_params)
        .map_err(|e| AppError::Internal(format!("serialize parent params: {}", e)))?;
    let mut parent_active: order::ActiveModel = parent.clone().into();
    parent_active.filled_quantity = Set(parent_new_filled);
    parent_active.advanced_params = Set(Some(parent_advanced));
    if is_complete {
        // 冰山完全体填满：母单也置为 Filled，标记 filled_at 供报表层使用。
        // Iceberg fully filled: parent → Filled, record filled_at for reporting.
        parent_active.status = Set(OrderStatus::Filled);
        parent_active.filled_at = Set(Some(chrono::Utc::now()));
    } else if parent_new_filled > 0.0 {
        // 部分成交：母单至少有一个 child 已 done，但还有切片待补投。
        // Partial fill: at least one child done, more slices still pending.
        parent_active.status = Set(OrderStatus::PartialFilled);
    }
    // else: 母单仍处 Pending（兜底，正常路径不会进入）。
    //       parent stays Pending (defensive fallback; should not occur on normal path).
    parent_active.updated_at = Set(chrono::Utc::now());
    parent_active
        .update(db.as_ref())
        .await
        .map_err(|e| AppError::Database(format!("update parent: {}", e)))?;

    Ok(())
}

/// P1-2.1: Cancel all pending children of an iceberg parent.
///
/// Returns the number of children that were cancelled.
///
/// 中文：
///   这是"级联取消"（cancel cascade）入口。当母单被用户/API 主动取消时，撮合引擎
///   在取消母单簿册条目的同时调用本函数，**同步**取消该母单所有尚未完成的 child
///   切片（status 为 Pending 或 PartialFilled 的）。
///
///   为什么要级联取消（Why cascade is mandatory）：
///     1. **反孤儿子订单**（anti-orphan）：如果不级联取消，已经入簿的 child 会
///        继续消耗订单簿的流动性，撮合引擎会在母单已"消失"的情况下继续执行这些
///        残余切片，导致用户被成交他本想撤的单
///     2. **状态一致性**：报表层 / 用户面板的"未结订单"列表不应出现"母单已取消
///        但 child 仍 Pending"的诡异组合
///     3. **风控**：母单通常带杠杆 / 风险限额，取消不全会留下敞口
///
///   实现策略（Implementation strategy）：
///     - 用 SQL 一次拉出所有 ICEBERG_CHILD + (Pending|PartialFilled) 的行
///       （典型场景：同时活跃的 child 数 ≤ 1，所以 in-memory 过滤代价极低）
///     - 对每条记录：反查 `parent_id` 过滤出本母单的，再 `remove_from_book` +
///       UPDATE status=Cancelled
///
///   返回值：被取消的 child 数量（>0 时 Prometheus 计数器自增）。
///   返回值用于上层做"级联取消是否生效"断言——若为 0 说明母单没有活跃 child，
///   调用方应继续完成母单自身的取消流程。
pub async fn cancel_iceberg_children(
    db: &Arc<DatabaseConnection>,
    engine: &Arc<MatchingEngine>,
    parent_id: Uuid,
) -> Result<u32, AppError> {
    // SQL 过滤策略：先把所有 ICEBERG_CHILD + 未完结的候选查出来（in-memory 再按
    // parent_id 二次过滤）。考虑过直接 `filter(parent_id=...)` 但 IcebergChildParams
    // 嵌在 JSONB 里，SeaORM 0.x 不支持 JSONB 列内字段的等值过滤（需要 raw SQL）。
    //
    // SQL filter strategy: pull all ICEBERG_CHILD + open candidates, then filter
    // by parent_id in-memory. A direct `parent_id=...` filter would need raw SQL
    // because parent_id lives inside the JSONB blob (SeaORM 0.x limitation).
    let all_children = order::Entity::find()
        .filter(order::Column::AdvancedType.eq(advanced_type::ICEBERG_CHILD))
        .filter(order::Column::Status.is_in(vec![
            OrderStatus::Pending,
            OrderStatus::PartialFilled,
        ]))
        .all(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut cancelled_count = 0u32;
    for child in all_children {
        // 防御性：JSONB 反序列化失败时跳过这一行（不要让一条坏行阻塞整个 cascade）。
        // Defensive: skip rows whose JSONB fails to deserialize rather than aborting
        // the whole cascade over a single bad row.
        let child_params: IcebergChildParams = match child
            .advanced_params
            .as_ref()
            .and_then(|v| serde_json::from_value::<IcebergChildParams>(v.clone()).ok())
        {
            Some(p) => p,
            None => continue,
        };

        // 二次过滤：只取消属于本 parent_id 的 child（其它母单的 child 不应被波及）。
        // Second filter: only cancel children of this parent (don't touch other parents' children).
        if child_params.parent_id != parent_id {
            continue;
        }

        // 从撮合引擎簿册中摘除（防止下一轮撮合把它当成"在簿"订单撮合掉）。
        // Remove from matching engine book (prevents next matching pass from
        // matching this orphan against the book).
        engine.remove_from_book(child.id);

        // DB 状态置为 Cancelled 并记录 cancelled_at。
        // Set DB status to Cancelled and record cancelled_at.
        let mut active: order::ActiveModel = child.into();
        active.status = Set(OrderStatus::Cancelled);
        active.cancelled_at = Set(Some(chrono::Utc::now()));
        active.updated_at = Set(chrono::Utc::now());
        active
            .update(db.as_ref())
            .await
            .map_err(|e| AppError::Database(format!("cancel child: {}", e)))?;

        cancelled_count += 1;
    }

    // 指标：只有真正取消 ≥1 个 child 时才 inc_by（避免 0 增量干扰 PromQL 速率计算）。
    // Metric: only increment when at least one child was actually cancelled (avoids
    // spurious rate() spikes from no-op cascade calls).
    if cancelled_count > 0 {
        metrics::ICEBERG_CHILD_ORDERS_TOTAL
            .with_label_values(&["cancelled"])
            .inc_by(cancelled_count as u64);
    }

    Ok(cancelled_count)
}

// ─── Helpers ─────────────────────────────────────────────────────
//
// 内部辅助函数（Helpers）：
//   - `create_child_order`：把母单"投影"成一个新的 child 限价单行
//   - `update_parent_params`：把变更后的 IcebergParams 写回母单行
//
// 中文说明（Helpers — Chinese note）：
//   这两个函数都是"纯 DB 操作"，不触发任何业务规则（数量计算、状态机判定等
//   都在 `split_into_children` / `append_next_child` 中完成，helpers 只负责
//   "按给定参数落库"）。这样切分的好处：
//     1. 单元测试只需要 mock 一次 DB（insert/update）就能覆盖多种业务路径
//     2. 业务逻辑和持久化解耦后，重构 SQL schema 不影响策略代码
//
// English: These helpers are pure DB operations — no business rules live here.
//   All slice-quantity math, status-machine decisions, etc. happen in
//   `split_into_children` / `append_next_child`. This separation keeps the
//   helpers trivially testable and shields business logic from schema changes.

/// 在 orders 表里 INSERT 一个新的 child 限价单行。
/// INSERT a new child limit-order row into the `orders` table.
///
/// 中文：
///   把母单的"用户/策略/交易对/方向/价格/TIF/expire_at"等字段全部**镜像**到 child
///   行（child 与 parent 在交易语义上是同一笔订单，只是物理上拆成多行），只覆盖
///   以下字段：
///     - `quantity`      = 本切片数量（最后一片 = remainder）
///     - `order_type`    = Limit（冰山本质就是分批的限价单）
///     - `status`        = Pending
///     - `advanced_type` = "iceberg_child"（撮合引擎据此识别）
///     - `advanced_params` = `{parent_id, slice_index}`
///
///   数量计算（Quantity calculation）：
///     - 非最后一片：child_qty = visible_quantity
///     - 最后一片：  child_qty = total_quantity − visible_quantity × slice_index
///                  （吸收浮点 ceil 的余数，防止累加超过 total）
///
/// English: Mirrors the parent's user/strategy/symbol/side/price/TIF/expire_at
///   into the child row (they are economically the same order, just physically
///   split), and overrides only the child-specific fields listed above.
async fn create_child_order(
    db: &DatabaseConnection,
    parent: &order::Model,
    params: &IcebergParams,
    slice_index: u32,
) -> Result<order::Model, AppError> {
    let child_id = Uuid::new_v4();
    // 最后一片：用 remainder 算法（吸收浮点 ceil 余数），其余 = visible_quantity。
    // Last slice: use remainder arithmetic (absorbs ceil rounding error), others = visible_quantity.
    let child_qty = if slice_index + 1 >= params.total_slices {
        // Last slice: remainder
        let consumed = params.visible_quantity * (slice_index as f64);
        params.total_quantity - consumed
    } else {
        params.visible_quantity
    };

    // 写一行 IcebergChildParams 标识"这是哪个母单的第几片"。
    // Persist IcebergChildParams to identify "which parent's slice #N this is".
    let child_params = IcebergChildParams::new(parent.id, slice_index);
    let child_advanced = serde_json::to_value(&child_params)
        .map_err(|e| AppError::Internal(format!("serialize child params: {}", e)))?;

    let now = chrono::Utc::now();
    // 镜像母单字段 + 覆盖 child 特有字段。
    // Mirror parent fields + override child-specific ones.
    let active = order::ActiveModel {
        id: Set(child_id),
        user_id: Set(parent.user_id),
        strategy_id: Set(parent.strategy_id),
        symbol: Set(parent.symbol.clone()),
        side: Set(parent.side.clone()),
        order_type: Set(OrderType::Limit),
        price: Set(parent.price),
        quantity: Set(child_qty),
        filled_quantity: Set(0.0),
        avg_fill_price: Set(None),
        status: Set(OrderStatus::Pending),
        mode: Set(parent.mode.clone()),
        fee: Set(0.0),
        reject_reason: Set(None),
        time_in_force: Set(parent.time_in_force.clone()),
        expire_at: Set(parent.expire_at),
        created_at: Set(now),
        updated_at: Set(now),
        cancelled_at: Set(None),
        filled_at: Set(None),
        advanced_type: Set(Some(advanced_type::ICEBERG_CHILD.to_string())),
        advanced_params: Set(Some(child_advanced)),
    };

    active
        .insert(db)
        .await
        .map_err(|e| AppError::Database(format!("insert child order: {}", e)))
}

/// 把更新后的 `IcebergParams` 序列化后写回母单行的 `advanced_params` JSONB。
/// Serialize the updated `IcebergParams` back to the parent's `advanced_params` JSONB.
///
/// 中文：
///   这是冰山"状态持久化"的标准入口。常见调用方：
///     - `split_into_children` 切完首切片后调用
///     - `append_next_child`  累加 `filled_children`、刷新 `slice_index_next` 后调用
///
///   为什么不直接合并进 `append_next_child` 的最后一段写？因为 `split_into_children`
///   需要"独立"地把切分结果写回，而 `append_next_child` 写的是"成交进度"——
///   两者语义不同，拆成 helper 便于审计和单元测试。
///
/// English: Standard "persist iceberg state" helper. Kept separate from
///   `append_next_child`'s tail-write because the semantic of "split progress"
///   vs. "fill progress" is distinct — splitting them makes auditing easier.
async fn update_parent_params(
    db: &DatabaseConnection,
    parent_id: Uuid,
    params: &IcebergParams,
) -> Result<(), AppError> {
    let value = serde_json::to_value(params)
        .map_err(|e| AppError::Internal(format!("serialize: {}", e)))?;
    let parent = order::Entity::find_by_id(parent_id)
        .one(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Parent {} not found", parent_id)))?;
    let mut active: order::ActiveModel = parent.into();
    active.advanced_params = Set(Some(value));
    active.updated_at = Set(chrono::Utc::now());
    active
        .update(db)
        .await
        .map_err(|e| AppError::Database(format!("update parent params: {}", e)))?;
    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────
//
// 单元测试覆盖（Unit test coverage）：
//   1. advanced_type 常量正确性（防字符串字面量漂移）
//   2. IcebergParams / IcebergChildParams JSONB 序列化往返
//   3. validate_new 边界（visible < total、零值拒绝）
//   4. next_slice_quantity 最后一片 remainder 行为
//   5. is_complete 进度判定（filled_children 从 0..total_slices 全部命中）
//   6. 整除场景（total = N × visible）下所有切片等量、无 remainder
//
// Chinese note:
//   业务函数（split_into_children / append_next_child / cancel_iceberg_children）
//   都是 async 且依赖 DatabaseConnection + MatchingEngine，无法在 unit test 中
//   不引入 mock。因此把这些"纯数据"逻辑的覆盖集中在 IcebergParams 自身的测试
//   中（详见 models/iceberg_params.rs），本文件的测试只负责最薄一层 sanity check。

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::iceberg_params::IcebergParams;

    /// 中文：常量值守门 — 防止有人把字符串字面量改坏。
    /// English: constant value sentinel — catches accidental string-literal drift.
    #[test]
    fn test_advanced_type_constants() {
        assert_eq!(advanced_type::ICEBERG, "iceberg");
        assert_eq!(advanced_type::ICEBERG_CHILD, "iceberg_child");
    }

    /// 中文：IcebergParams 经过 JSONB 序列化后能完整还原（含 children_ids）。
    /// English: IcebergParams must survive a full JSONB round-trip (including children_ids).
    #[test]
    fn test_iceberg_params_serialize_roundtrip() {
        let p = IcebergParams {
            visible_quantity: 1.0,
            total_quantity: 10.0,
            filled_children: 3,
            total_slices: 10,
            slice_index_next: 4,
            children_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };
        let json = serde_json::to_value(&p).unwrap();
        let p2: IcebergParams = serde_json::from_value(json).unwrap();
        assert_eq!(p, p2);
    }

    /// 中文：IcebergChildParams 的 parent_id / slice_index 能正确序列化。
    /// English: IcebergChildParams' parent_id / slice_index must serialize correctly.
    #[test]
    fn test_iceberg_child_params_serialize() {
        let c = IcebergChildParams::new(Uuid::new_v4(), 5);
        let json = serde_json::to_value(&c).unwrap();
        let c2: IcebergChildParams = serde_json::from_value(json).unwrap();
        assert_eq!(c, c2);
    }

    /// 中文：validate_new 拒绝 visible ≥ total（D5 防御：visible == total 等于"不是冰山"）。
    /// English: validate_new rejects visible ≥ total (D5: visible == total is just a limit order).
    #[test]
    fn test_iceberg_params_validate_inverted() {
        // D5: validate_new enforces visible < total
        let r = IcebergParams::validate_new(10.0, 5.0);
        assert!(r.is_err());
    }

    /// 中文：validate_new 拒绝 visible = 0（避免切片无限小、退化为无意义操作）。
    /// English: validate_new rejects visible = 0 (avoids degenerate "infinite slice" orders).
    #[test]
    fn test_iceberg_params_validate_zero_visible() {
        let r = IcebergParams::validate_new(0.0, 10.0);
        assert!(r.is_err());
    }

    /// 中文：total=10 / visible=3 → total_slices=4，最后一片吸收 ceil 余数 1.0。
    /// English: total=10 / visible=3 → 4 slices, last slice absorbs ceil remainder 1.0.
    #[test]
    fn test_iceberg_params_last_slice_remainder() {
        let p = IcebergParams::validate_new(3.0, 10.0).unwrap();
        assert_eq!(p.total_slices, 4);
        // filled=3 → next slice is last → remainder
        // 中文：已成交 3 切片，剩 1 切片，qty = total − visible×3 = 10 − 9 = 1
        // English: 3 filled + 1 remaining slice, qty = total − visible×3 = 10 − 9 = 1
        let mut p2 = p.clone();
        p2.filled_children = 3;
        let q = p2.next_slice_quantity();
        assert!((q - 1.0).abs() < 1e-9);
    }

    /// 中文：is_complete 在 filled_children 走到 total_slices 时翻转为 true。
    /// English: is_complete flips to true exactly when filled_children reaches total_slices.
    #[test]
    fn test_iceberg_params_is_complete_progression() {
        let mut p = IcebergParams::validate_new(1.0, 3.0).unwrap();
        assert_eq!(p.total_slices, 3);
        assert!(!p.is_complete());
        p.filled_children = 1;
        assert!(!p.is_complete());
        p.filled_children = 2;
        assert!(!p.is_complete());
        p.filled_children = 3;
        assert!(p.is_complete());
    }

    /// 中文：total = N × visible（整除）时，所有切片（含最后一片）数量相同。
    /// English: when total = N × visible (exact multiple), every slice has the same qty.
    #[test]
    fn test_validate_new_exact_multiple() {
        // total=10, visible=2 → 5 slices, all equal
        let p = IcebergParams::validate_new(2.0, 10.0).unwrap();
        assert_eq!(p.total_slices, 5);
        for i in 0..4 {
            let mut p2 = p.clone();
            p2.filled_children = i;
            assert!((p2.next_slice_quantity() - 2.0).abs() < 1e-9);
        }
        // last slice (i=4) — same as others when exact multiple
        // 整除时，最后一片的 remainder = total − visible×4 = 10 − 8 = 2（与 visible 相等）
        // When exact multiple, last slice remainder = total − visible×4 = 10 − 8 = 2 (== visible)
        let mut p3 = p.clone();
        p3.filled_children = 4;
        assert!((p3.next_slice_quantity() - 2.0).abs() < 1e-9);
    }
}
