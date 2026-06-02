//! services/trigger_order.rs — Trigger Order Service (条件触发单服务)
//! services/trigger_order.rs — Trigger Order Service (conditional trigger order service)
//!
//! P1-F3: 条件触发单 - 止损单/止盈单/OCO/TWAP
//! 依赖 P1-F2 实盘止盈止损
//!
//! English:
//! P1-F3 — Conditional trigger orders: stop-loss / take-profit / OCO / TWAP.
//! Depends on P1-F2 (live take-profit / stop-loss infrastructure).
//!
//! 中文四大职责：
//! 1. create_stop_loss / create_take_profit — 单边触发单
//! 2. create_oco — 双触发单 + 互斥（一方触发立即取消另一方）
//! 3. create_twap — 时间加权分片下单（避免大单 market impact）
//! 4. check_trigger / trigger_order — 价格触发的统一入口
//!
//! English — four core responsibilities:
//! 1. create_stop_loss / create_take-profit — single-leg trigger orders
//! 2. create_oco — paired triggers (mutual exclusion: one firing cancels the other)
//! 3. create_twap — time-weighted slicing to avoid market impact on large orders
//! 4. check_trigger / trigger_order — single entry point for price-based firing

use chrono::{Duration as ChronoDuration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    EntityTrait, QueryFilter, Set, Statement,
};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

use crate::db::order::positions::Entity as PositionEntity;
use crate::db::order::{OrderSide, OrderStatus, OrderType, TimeInForce, TradeMode};
use crate::db::trigger_order::{
    Entity as TriggerOrderEntity, Model as TriggerOrder, TriggerDirection, TriggerStatus,
    TriggerType, TwapSide,
};
use crate::utils::error::AppError;

// ────────────────────────── enum string-value helpers ──────────────────────────
//
// SeaORM 1.1.x's `DeriveActiveEnum` ships a generated `IntoActiveValue<TriggerType>`
// that inserts the variant's **Rust name** (e.g. `"StopLoss"`) as `text`, but
// our Postgres `trigger_orders` columns are real enums (e.g. `trigger_type`).
// The result is a `text → trigger_type` type-mismatch on every `INSERT` and
// `text → trigger_status` on every `SELECT … WHERE` (`sea-orm#2500`).
//
// Workaround: every write path bypasses SeaORM and emits the `string_value`
// directly (`'stop_loss'::trigger_type`). These helpers centralize the mapping
// so the raw SQL stays in sync with the entity definition.
fn trigger_type_str(v: &TriggerType) -> &'static str {
    match v {
        TriggerType::StopLoss => "stop_loss",
        TriggerType::TakeProfit => "take_profit",
        TriggerType::Oco => "oco",
        TriggerType::Twap => "twap",
    }
}
// Currently used only by future service methods (e.g. cancel/status
// updates); suppress the dead-code warning for now.
#[allow(dead_code)]
fn trigger_status_str(v: &TriggerStatus) -> &'static str {
    match v {
        TriggerStatus::Pending => "pending",
        TriggerStatus::Triggered => "triggered",
        TriggerStatus::Cancelled => "cancelled",
        TriggerStatus::Expired => "expired",
        TriggerStatus::Failed => "failed",
    }
}
fn trigger_direction_str(v: &TriggerDirection) -> &'static str {
    match v {
        TriggerDirection::Up => "up",
        TriggerDirection::Down => "down",
    }
}
fn parse_trigger_type(s: &str) -> TriggerType {
    match s {
        "stop_loss" => TriggerType::StopLoss,
        "take_profit" => TriggerType::TakeProfit,
        "oco" => TriggerType::Oco,
        "twap" => TriggerType::Twap,
        _ => TriggerType::StopLoss,
    }
}
fn parse_trigger_status(s: &str) -> TriggerStatus {
    match s {
        "pending" => TriggerStatus::Pending,
        "triggered" => TriggerStatus::Triggered,
        "cancelled" => TriggerStatus::Cancelled,
        "expired" => TriggerStatus::Expired,
        "failed" => TriggerStatus::Failed,
        _ => TriggerStatus::Pending,
    }
}
fn parse_trigger_direction(s: &str) -> TriggerDirection {
    match s {
        "up" => TriggerDirection::Up,
        "down" => TriggerDirection::Down,
        _ => TriggerDirection::Down,
    }
}

/// Decode a single `QueryResult` row (post `::text` cast) into a typed
/// `TriggerOrder` model. Returned error is a database string for upstream
/// wrapping into `AppError::Database`.
fn row_to_trigger_order(row: sea_orm::QueryResult) -> Result<TriggerOrder, String> {
    Ok(TriggerOrder {
        id: row.try_get_by::<Uuid, _>("id").map_err(|e| e.to_string())?,
        user_id: row
            .try_get_by::<Uuid, _>("user_id")
            .map_err(|e| e.to_string())?,
        position_id: row
            .try_get_by::<Option<Uuid>, _>("position_id")
            .map_err(|e| e.to_string())?,
        symbol: row
            .try_get_by::<String, _>("symbol")
            .map_err(|e| e.to_string())?,
        trigger_type: parse_trigger_type(
            &row.try_get_by::<String, _>("trigger_type").map_err(|e| e.to_string())?,
        ),
        status: parse_trigger_status(
            &row.try_get_by::<String, _>("status").map_err(|e| e.to_string())?,
        ),
        trigger_direction: parse_trigger_direction(
            &row.try_get_by::<String, _>("trigger_direction").map_err(|e| e.to_string())?,
        ),
        trigger_price: row
            .try_get_by::<f64, _>("trigger_price")
            .map_err(|e| e.to_string())?,
        trigger_price_upper: row
            .try_get_by::<Option<f64>, _>("trigger_price_upper")
            .map_err(|e| e.to_string())?,
        trigger_price_lower: row
            .try_get_by::<Option<f64>, _>("trigger_price_lower")
            .map_err(|e| e.to_string())?,
        base_price: row
            .try_get_by::<Option<f64>, _>("base_price")
            .map_err(|e| e.to_string())?,
        side: match row
            .try_get_by::<String, _>("side")
            .map_err(|e| e.to_string())?
            .as_str()
        {
            "sell" => TwapSide::Sell,
            _ => TwapSide::Buy,
        },
        quantity: row
            .try_get_by::<f64, _>("quantity")
            .map_err(|e| e.to_string())?,
        filled_quantity: row
            .try_get_by::<f64, _>("filled_quantity")
            .map_err(|e| e.to_string())?,
        avg_fill_price: row
            .try_get_by::<Option<f64>, _>("avg_fill_price")
            .map_err(|e| e.to_string())?,
        twap_slice_quantity: row
            .try_get_by::<f64, _>("twap_slice_quantity")
            .map_err(|e| e.to_string())?,
        twap_interval_secs: row
            .try_get_by::<i32, _>("twap_interval_secs")
            .map_err(|e| e.to_string())?,
        twap_start_time: row
            .try_get_by::<Option<chrono::DateTime<Utc>>, _>("twap_start_time")
            .map_err(|e| e.to_string())?,
        twap_end_time: row
            .try_get_by::<Option<chrono::DateTime<Utc>>, _>("twap_end_time")
            .map_err(|e| e.to_string())?,
        twap_executed_slices: row
            .try_get_by::<i32, _>("twap_executed_slices")
            .map_err(|e| e.to_string())?,
        twap_max_slices: row
            .try_get_by::<i32, _>("twap_max_slices")
            .map_err(|e| e.to_string())?,
        oco_pair_id: row
            .try_get_by::<Option<Uuid>, _>("oco_pair_id")
            .map_err(|e| e.to_string())?,
        triggered_order_id: row
            .try_get_by::<Option<Uuid>, _>("triggered_order_id")
            .map_err(|e| e.to_string())?,
        trigger_reason: row
            .try_get_by::<Option<String>, _>("trigger_reason")
            .map_err(|e| e.to_string())?,
        triggered_at: row
            .try_get_by::<Option<chrono::DateTime<Utc>>, _>("triggered_at")
            .map_err(|e| e.to_string())?,
        expire_at: row
            .try_get_by::<Option<chrono::DateTime<Utc>>, _>("expire_at")
            .map_err(|e| e.to_string())?,
        created_at: row
            .try_get_by::<chrono::DateTime<Utc>, _>("created_at")
            .map_err(|e| e.to_string())?,
        updated_at: row
            .try_get_by::<chrono::DateTime<Utc>, _>("updated_at")
            .map_err(|e| e.to_string())?,
        cancelled_at: row
            .try_get_by::<Option<chrono::DateTime<Utc>>, _>("cancelled_at")
            .map_err(|e| e.to_string())?,
    })
}

/// Trigger Order Service - 条件触发单服务
/// Trigger Order Service — owns all `trigger_orders` DB access and the
/// price-firing pipeline. Stateless besides the `Arc<DatabaseConnection>`.
pub struct TriggerOrderService {
    db: Arc<DatabaseConnection>,
}

impl TriggerOrderService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建止损单
    /// Create a stop-loss trigger order for an existing position.
    ///
    /// 中文：
    ///   - 校验持仓存在（user_id 二次过滤防越权）
    ///   - 触发方向由 position.side 决定：long → 价格下跌触发 (Down) / short → 上涨触发 (Up)
    ///   - 平仓方向与持仓相反：long → 卖出平仓 (Sell) / short → 买入平仓 (Buy)
    ///   - 通过 raw SQL 写入以绕开 SeaORM enum bug（参考 `insert_trigger_order_raw`）
    ///   - 递增 `TRIGGER_ORDERS_CREATED_TOTAL{type="stop_loss"}` 指标
    ///
    /// English:
    ///   - Validates position exists; secondary `user_id` filter prevents cross-tenant access
    ///   - Trigger direction derives from `position.side`: long → Down on price drop; short → Up on price rise
    ///   - Closing direction is opposite to the position
    ///   - Inserts via raw SQL to bypass the SeaORM enum bug (see `insert_trigger_order_raw`)
    ///   - Increments `TRIGGER_ORDERS_CREATED_TOTAL{type="stop_loss"}` metric
    pub async fn create_stop_loss(
        &self,
        user_id: Uuid,
        position_id: Uuid,
        symbol: &str,
        trigger_price: f64,
        base_price: Option<f64>,
        quantity: f64,
    ) -> Result<TriggerOrder, AppError> {
        // 验证持仓存在
        let position = PositionEntity::find_by_id(position_id)
            .filter(crate::db::order::positions::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Position not found: {}", position_id)))?;

        // 确定触发方向 (多头持仓止损: 价格下跌触发 = down; 空头持仓止损: 价格上涨触发 = up)
        let trigger_direction = if position.side == crate::db::order::PositionSide::Long {
            TriggerDirection::Down
        } else {
            TriggerDirection::Up
        };

        // 确定方向 (止损卖空)
        let side = if position.side == crate::db::order::PositionSide::Long {
            TwapSide::Sell
        } else {
            TwapSide::Buy
        };

        let now = Utc::now();
        let new_id = Uuid::new_v4();
        self.insert_trigger_order_raw(
            new_id,
            user_id,
            Some(position_id),
            symbol,
            TriggerType::StopLoss,
            trigger_direction,
            Some(trigger_price),
            None,
            None,
            base_price,
            side,
            quantity,
            0.0,
            60,
            None,
            None,
            0,
            0,
            None,
            None,
            None,
            None,
            now,
        )
        .await?;

        let result = self
            .get_trigger_order_via_raw_text(new_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Trigger order {} not found", new_id)))?;

        info!(
            user_id = %user_id,
            position_id = %position_id,
            symbol = %symbol,
            trigger_price = trigger_price,
            "Stop loss order created"
        );

        // P0-3: count trigger creation by type
        crate::metrics::TRIGGER_ORDERS_CREATED_TOTAL
            .with_label_values(&["stop_loss"])
            .inc();

        Ok(result)
    }

    /// 创建止盈单
    /// Create a take-profit trigger order for an existing position.
    ///
    /// 中文：与 stop_loss 镜像逻辑，但触发方向相反（long → Up / short → Down）。
    /// 同样递增 `TRIGGER_ORDERS_CREATED_TOTAL{type="take_profit"}` 指标。
    ///
    /// English: Mirror of `create_stop_loss` but with the trigger direction inverted
    /// (long → Up on price rise; short → Down on price drop). Increments
    /// `TRIGGER_ORDERS_CREATED_TOTAL{type="take_profit"}`.
    pub async fn create_take_profit(
        &self,
        user_id: Uuid,
        position_id: Uuid,
        symbol: &str,
        trigger_price: f64,
        base_price: Option<f64>,
        quantity: f64,
    ) -> Result<TriggerOrder, AppError> {
        // 验证持仓存在
        let position = PositionEntity::find_by_id(position_id)
            .filter(crate::db::order::positions::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Position not found: {}", position_id)))?;

        // 确定触发方向 (多头持仓止盈: 价格上涨触发 = up; 空头持仓止盈: 价格下跌触发 = down)
        let trigger_direction = if position.side == crate::db::order::PositionSide::Long {
            TriggerDirection::Up
        } else {
            TriggerDirection::Down
        };

        // 确定方向 (止盈平仓)
        let side = if position.side == crate::db::order::PositionSide::Long {
            TwapSide::Sell
        } else {
            TwapSide::Buy
        };

        let now = Utc::now();
        let new_id = Uuid::new_v4();
        self.insert_trigger_order_raw(
            new_id,
            user_id,
            Some(position_id),
            symbol,
            TriggerType::TakeProfit,
            trigger_direction,
            Some(trigger_price),
            None,
            None,
            base_price,
            side,
            quantity,
            0.0,
            60,
            None,
            None,
            0,
            0,
            None,
            None,
            None,
            None,
            now,
        )
        .await?;

        let result = self
            .get_trigger_order_via_raw_text(new_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Trigger order {} not found", new_id)))?;

        info!(
            user_id = %user_id,
            position_id = %position_id,
            symbol = %symbol,
            trigger_price = trigger_price,
            "Take profit order created"
        );

        // P0-3: count trigger creation by type
        crate::metrics::TRIGGER_ORDERS_CREATED_TOTAL
            .with_label_values(&["take_profit"])
            .inc();

        Ok(result)
    }

    /// 创建OCO单 (One-Cancels-Other)
    /// 当一个触发时，另一个自动取消
    /// Create an OCO (One-Cancels-Other) pair: stop-loss + take-profit linked
    /// so that firing either side auto-cancels the other.
    ///
    /// 中文：
    ///   - 多头：SL 在下 (Down 触发) / TP 在上 (Up 触发)
    ///   - 空头：SL 在上 (Up 触发) / TP 在下 (Down 触发)
    ///   - 两个子单通过 `oco_pair_id` 互指
    ///   - atomic 写入：两个 `insert_trigger_order_raw` 都成功才返回 Ok，失败时 caller 需自行补偿
    ///     （当前未实现回滚，后续可加事务或 saga）
    ///   - 指标 `TRIGGER_ORDERS_CREATED_TOTAL{type="oco"}` 一次 inc 2（因为产生 2 个 trigger 单）
    ///
    /// English:
    ///   - Long: SL below (Down), TP above (Up)
    ///   - Short: SL above (Up), TP below (Down)
    ///   - Two child rows reference each other via `oco_pair_id`
    ///   - Atomicity caveat: both `insert_trigger_order_raw` calls must succeed;
    ///     a failure between them leaves an orphan (no transaction / saga yet)
    ///   - `TRIGGER_ORDERS_CREATED_TOTAL{type="oco"}` is incremented by 2 (one pair)
    #[allow(clippy::too_many_arguments)]
    pub async fn create_oco(
        &self,
        user_id: Uuid,
        position_id: Uuid,
        symbol: &str,
        stop_loss_price: f64,   // 止损价格
        take_profit_price: f64, // 止盈价格
        base_price: Option<f64>,
        quantity: f64,
    ) -> Result<(TriggerOrder, TriggerOrder), AppError> {
        // 验证持仓存在
        let position = PositionEntity::find_by_id(position_id)
            .filter(crate::db::order::positions::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Position not found: {}", position_id)))?;

        // 确定方向 (平仓方向)
        let side = if position.side == crate::db::order::PositionSide::Long {
            TwapSide::Sell
        } else {
            TwapSide::Buy
        };

        // 多头: 止损在下(价格下跌触发=down), 止盈在上(价格上涨触发=up)
        // 空头: 止损在上(价格上涨触发=up), 止盈在下(价格下跌触发=down)
        let (stop_trigger_dir, profit_trigger_dir) =
            if position.side == crate::db::order::PositionSide::Long {
                (TriggerDirection::Down, TriggerDirection::Up)
            } else {
                (TriggerDirection::Up, TriggerDirection::Down)
            };

        let now = Utc::now();

        // 创建止损单
        let stop_loss_id = Uuid::new_v4();
        // 创建止盈单
        let take_profit_id = Uuid::new_v4();

        self.insert_trigger_order_raw(
            stop_loss_id,
            user_id,
            Some(position_id),
            symbol,
            TriggerType::Oco,
            stop_trigger_dir,
            Some(stop_loss_price),
            Some(take_profit_price),
            None,
            base_price,
            side.clone(),
            quantity,
            0.0,
            60,
            None,
            None,
            0,
            0,
            Some(take_profit_id), // oco_pair_id
            None,
            None,
            None,
            now,
        )
        .await?;
        self.insert_trigger_order_raw(
            take_profit_id,
            user_id,
            Some(position_id),
            symbol,
            TriggerType::Oco,
            profit_trigger_dir,
            Some(take_profit_price),
            None,
            Some(stop_loss_price),
            base_price,
            side,
            quantity,
            0.0,
            60,
            None,
            None,
            0,
            0,
            Some(stop_loss_id), // oco_pair_id
            None,
            None,
            None,
            now,
        )
        .await?;

        let stop_loss = self
            .get_trigger_order_via_raw_text(stop_loss_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Stop loss order {} not found", stop_loss_id))
            })?;
        let take_profit = self
            .get_trigger_order_via_raw_text(take_profit_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "Take profit order {} not found",
                    take_profit_id
                ))
            })?;

        info!(
            position_id = %position_id,
            symbol = %symbol,
            stop_loss_price = stop_loss_price,
            take_profit_price = take_profit_price,
            "OCO order pair created"
        );

        // P0-3: an OCO pair creates two trigger orders (sl + tp)
        crate::metrics::TRIGGER_ORDERS_CREATED_TOTAL
            .with_label_values(&["oco"])
            .inc_by(2);

        Ok((stop_loss, take_profit))
    }

    /// 创建TWAP单 (Time-Weighted Average Price)
    /// Create a TWAP order: time-weighted slicing to spread a large order
    /// across `duration_secs`, with `slice_quantity` per `interval_secs` tick.
    ///
    /// 中文：
    ///   - 与 SL/TP/OCO 不同：TWAP 是**时间驱动**而非价格驱动（`check_trigger` 中显式返回 false）
    ///   - 由 `process_twap_slice` 按 `interval_secs` 节拍推进
    ///   - `twap_max_slices = duration_secs / interval_secs` — 上限由 caller 校验
    ///   - 递增 `TRIGGER_ORDERS_CREATED_TOTAL{type="twap"}` 指标（按"一个 TWAP 计划"为单位）
    ///
    /// English:
    ///   - Unlike SL/TP/OCO, TWAP is **time-driven**, not price-driven
    ///     (see `check_trigger` returning `false` for `TriggerType::Twap`)
    ///   - `process_twap_slice` advances the schedule on each `interval_secs` tick
    ///   - `twap_max_slices = duration_secs / interval_secs` — caller is expected to validate
    ///   - Metric `TRIGGER_ORDERS_CREATED_TOTAL{type="twap"}` increments once per TWAP plan
    #[allow(clippy::too_many_arguments)]
    pub async fn create_twap(
        &self,
        user_id: Uuid,
        symbol: &str,
        side: &str, // "buy" or "sell"
        quantity: f64,
        slice_quantity: f64,
        interval_secs: i32,
        duration_secs: i32,
    ) -> Result<TriggerOrder, AppError> {
        let now = Utc::now();
        let end_time = now + ChronoDuration::seconds(duration_secs as i64);

        let twap_side = match side {
            "buy" => TwapSide::Buy,
            "sell" => TwapSide::Sell,
            _ => return Err(AppError::BadRequest(format!("Invalid side: {}", side))),
        };

        let trigger_direction = if side == "buy" {
            TriggerDirection::Up
        } else {
            TriggerDirection::Down
        };
        let new_id = Uuid::new_v4();
        self.insert_trigger_order_raw(
            new_id,
            user_id,
            None,
            symbol,
            TriggerType::Twap,
            trigger_direction,
            None,
            None,
            None,
            None,
            twap_side,
            quantity,
            slice_quantity,
            interval_secs,
            Some(now),
            Some(end_time),
            0,
            duration_secs / interval_secs,
            None,
            None,
            None,
            Some(end_time),
            now,
        )
        .await?;

        let result = self
            .get_trigger_order_via_raw_text(new_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Trigger order {} not found", new_id)))?;
        info!(
            new_id = %new_id,
            user_id = %user_id,
            symbol = %symbol,
            side = %side,
            quantity = quantity,
            slice_quantity = slice_quantity,
            interval_secs = interval_secs,
            duration_secs = duration_secs,
            "TWAP order created"
        );

        // P0-3: count trigger creation by type
        crate::metrics::TRIGGER_ORDERS_CREATED_TOTAL
            .with_label_values(&["twap"])
            .inc();

        Ok(result)
    }

    /// 检查价格是否触发条件单
    /// Check all pending triggers for a symbol against the current price.
    ///
    /// 中文：
    ///   - **唯一入口**：上游只应通过本方法检查价格触发（避免重复触发）
    ///   - 触发判定：SL/TP/OCO 用 trigger_direction 决定 ≤ 或 ≥
    ///   - TWAP 显式不参与价格检查（时间驱动在 `process_twap_slice`）
    ///   - 触发后调用 `trigger_order` 创建市价 close 单 + 更新状态
    ///   - 用 raw SQL 读取（绕开 SeaORM enum bug）
    ///
    /// English:
    ///   - **Single entry point**: callers should funnel all price-trigger checks
    ///     through this method to avoid duplicate firing
    ///   - SL/TP/OCO use `trigger_direction` to choose ≤ or ≥
    ///   - TWAP is explicitly excluded (time-driven path in `process_twap_slice`)
    ///   - On fire, calls `trigger_order` which creates a market close + status update
    ///   - Reads via raw SQL (SeaORM enum workaround)
    pub async fn check_trigger(&self, symbol: &str, current_price: f64) -> Result<(), AppError> {
        // Bypass SeaORM's enum-aware Select because the generated
        // `DeriveActiveEnum` decode path fails on the real Postgres
        // enum type. Use raw SQL with `::text` cast on read and
        // string-value literal on compare.
        let pending_orders: Vec<TriggerOrder> = self
            .list_pending_trigger_orders_raw(symbol)
            .await
            .map_err(AppError::Database)?;

        for order in pending_orders {
            let should_trigger = match order.trigger_type {
                TriggerType::StopLoss | TriggerType::TakeProfit | TriggerType::Oco => {
                    match order.trigger_direction {
                        TriggerDirection::Up => current_price >= order.trigger_price,
                        TriggerDirection::Down => current_price <= order.trigger_price,
                    }
                }
                TriggerType::Twap => {
                    // TWAP由时间驱动，不是价格触发
                    false
                }
            };

            if should_trigger {
                self.trigger_order(&order, current_price).await?;
            }
        }

        Ok(())
    }

    /// 触发条件单
    /// Activate a trigger order: create a market close + advance status.
    ///
    /// 中文：
    ///   - OCO 联动：触发一方时自动 cancel 另一方（互斥语义）
    ///   - 创建 IOC 限价/市价 close 单（price = base_price 优先，否则 current_price）
    ///   - raw SQL UPDATE 状态（参考 `insert_trigger_order_raw` 的 why）
    ///   - 递增 `TRIGGER_ORDERS_FIRED_TOTAL{type=...}` 指标
    ///
    /// English:
    ///   - OCO cascade: auto-cancel the paired order (mutual-exclusion semantics)
    ///   - Creates a market close (IOC); price prefers `base_price`, falls back to `current_price`
    ///   - Status UPDATE via raw SQL (see `insert_trigger_order_raw` for the why)
    ///   - Increments `TRIGGER_ORDERS_FIRED_TOTAL{type=...}` metric
    async fn trigger_order(
        &self,
        order: &TriggerOrder,
        current_price: f64,
    ) -> Result<(), AppError> {
        // 如果是OCO单，先取消关联的另一单
        if let Some(oco_pair_id) = order.oco_pair_id {
            self.cancel_trigger_order(oco_pair_id, "OCO pair triggered")
                .await?;
        }

        // 创建实际的市场委托
        let order_side = match order.side {
            TwapSide::Buy => OrderSide::Buy,
            TwapSide::Sell => OrderSide::Sell,
        };

        let now = Utc::now();
        let new_order_id = Uuid::new_v4();

        let market_order = crate::db::order::ActiveModel {
            id: Set(new_order_id),
            user_id: Set(order.user_id),
            strategy_id: Set(None),
            symbol: Set(order.symbol.clone()),
            side: Set(order_side.clone()),
            order_type: Set(OrderType::Market),
            price: Set(order.base_price.or(Some(current_price))),
            quantity: Set(order.quantity),
            filled_quantity: Set(0.0),
            avg_fill_price: Set(None),
            status: Set(OrderStatus::Pending),
            mode: Set(TradeMode::Paper),
            fee: Set(0.0),
            reject_reason: Set(None),
            time_in_force: Set(TimeInForce::IOC),
            expire_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            cancelled_at: Set(None),
            filled_at: Set(None),
            // P1-2: advanced order type fields
            advanced_type: Set(None),
            advanced_params: Set(None),
        };

        market_order.insert(self.db.as_ref()).await.map_err(|e| {
            error!("Failed to create triggered order: {:?}", e);
            AppError::Database(e.to_string())
        })?;

        // 更新条件单状态 (raw SQL — see comment on cancel_trigger_order)
        let reason = format!("price_triggered_at_{}", current_price);
        let upd_sql = format!(
            "UPDATE trigger_orders SET \
                status = 'triggered'::trigger_status, \
                triggered_at = '{now}'::timestamptz, \
                triggered_order_id = '{oid}'::uuid, \
                trigger_reason = '{rsn}', \
                updated_at = '{now}'::timestamptz \
             WHERE id = '{id}'::uuid",
            now = now.to_rfc3339(),
            oid = new_order_id,
            rsn = reason.replace('\'', "''"),
            id = order.id
        );
        self.db
            .as_ref()
            .execute(Statement::from_string(DatabaseBackend::Postgres, upd_sql))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        info!(
            order_id = %order.id,
            user_id = %order.user_id,
            symbol = %order.symbol,
            trigger_type = ?order.trigger_type,
            trigger_price = order.trigger_price,
            current_price = current_price,
            "Trigger order activated"
        );

        // P0-3: count fired trigger orders by type
        crate::metrics::TRIGGER_ORDERS_FIRED_TOTAL
            .with_label_values(&[trigger_type_str(&order.trigger_type)])
            .inc();

        Ok(())
    }

    /// 取消条件单
    /// Cancel a trigger order. OCO pair is cancelled automatically.
    ///
    /// 中文：
    ///   - 终态（triggered/cancelled/expired/failed）的 order 不可再 cancel
    ///   - OCO 联动：如果该 order 是 OCO 的一边，另一边（仍 pending）一并 cancel
    ///   - 原因（reason）写入 `trigger_reason` 字段供审计
    ///   - raw SQL 绕开 SeaORM enum bug
    ///
    /// English:
    ///   - Terminal-status orders (triggered / cancelled / expired / failed) cannot be cancelled
    ///   - OCO cascade: if this is one leg of an OCO, the still-pending pair is cancelled too
    ///   - `reason` is written to `trigger_reason` for audit
    ///   - raw SQL bypass for SeaORM enum bug
    pub async fn cancel_trigger_order(&self, order_id: Uuid, reason: &str) -> Result<(), AppError> {
        // Bypass SeaORM's enum decode on read; cancel uses raw UPDATE too
        // to avoid the same variant-name-as-text issue.
        let order = self
            .get_trigger_order_via_raw_text(order_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Trigger order not found: {}", order_id)))?;

        if order.status.is_terminal() {
            return Err(AppError::Conflict(format!(
                "Trigger order status is {:?}, cannot cancel",
                order.status
            )));
        }

        let now = Utc::now();
        let sql = format!(
            "UPDATE trigger_orders SET \
                status = 'cancelled'::trigger_status, \
                cancelled_at = '{now}'::timestamptz, \
                trigger_reason = '{rsn}', \
                updated_at = '{now}'::timestamptz \
             WHERE id = '{oid}'::uuid",
            now = now.to_rfc3339(),
            rsn = reason.replace('\'', "''"),
            oid = order_id
        );
        self.db
            .as_ref()
            .execute(Statement::from_string(DatabaseBackend::Postgres, sql))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 如果是OCO单，也取消关联的另一单
        if let Some(oco_pair_id) = order.oco_pair_id {
            let pair = self
                .get_trigger_order_via_raw_text(oco_pair_id)
                .await?;
            if let Some(pair_order) = pair
                && pair_order.status == TriggerStatus::Pending
            {
                let pair_reason = format!("OCO cancelled by pair: {}", reason);
                let pair_sql = format!(
                    "UPDATE trigger_orders SET \
                        status = 'cancelled'::trigger_status, \
                        cancelled_at = '{now}'::timestamptz, \
                        trigger_reason = '{rsn}', \
                        updated_at = '{now}'::timestamptz \
                     WHERE id = '{oid}'::uuid",
                    now = now.to_rfc3339(),
                    rsn = pair_reason.replace('\'', "''"),
                    oid = oco_pair_id
                );
                self.db
                    .as_ref()
                    .execute(Statement::from_string(DatabaseBackend::Postgres, pair_sql))
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;
            }
        }

        info!(
            order_id = %order_id,
            reason = reason,
            "Trigger order cancelled"
        );

        Ok(())
    }

    /// 获取用户的条件单列表
    /// List a user's trigger orders, filtered by `status` and/or `symbol`.
    ///
    /// 中文：
    ///   - 必须带 `user_id` 过滤（多租户隔离）
    ///   - 按 `created_at DESC` 倒序（最新在前）
    ///   - raw SQL + `::text` cast 读取（enum workaround）
    ///   - 无过滤时 `status_clause` / `symbol_clause` 为空串，不会引入无效 WHERE
    ///
    /// English:
    ///   - Always filtered by `user_id` (multi-tenant isolation)
    ///   - Ordered `created_at DESC` (newest first)
    ///   - Raw SQL with `::text` cast (enum workaround)
    ///   - Empty `status` / `symbol` filter produces an empty clause (no spurious WHERE)
    pub async fn list_trigger_orders(
        &self,
        user_id: Uuid,
        status: Option<String>,
        symbol: Option<String>,
    ) -> Result<Vec<TriggerOrder>, AppError> {
        // Bypass SeaORM filter chain because the generated enum decode
        // fails on real Postgres enum columns.
        let status_clause = match status.as_deref() {
            Some("pending") => "AND status = 'pending'::trigger_status",
            Some("triggered") => "AND status = 'triggered'::trigger_status",
            Some("cancelled") => "AND status = 'cancelled'::trigger_status",
            Some("expired") => "AND status = 'expired'::trigger_status",
            _ => "",
        };
        let symbol_clause = match symbol.as_deref() {
            Some(s) if !s.is_empty() => format!("AND symbol = '{}'", s),
            _ => String::new(),
        };
        let sql = format!(
            "SELECT id, user_id, position_id, symbol, \
                    trigger_type::text      AS trigger_type, \
                    status::text            AS status, \
                    trigger_direction::text AS trigger_direction, \
                    trigger_price, trigger_price_upper, trigger_price_lower, \
                    base_price, side, quantity, filled_quantity, avg_fill_price, \
                    twap_slice_quantity, twap_interval_secs, twap_start_time, \
                    twap_end_time, twap_executed_slices, twap_max_slices, \
                    oco_pair_id, triggered_order_id, trigger_reason, \
                    triggered_at, expire_at, created_at, updated_at, cancelled_at \
             FROM trigger_orders \
             WHERE user_id = '{uid}'::uuid {st} {sy} \
             ORDER BY created_at DESC",
            uid = user_id,
            st = status_clause,
            sy = symbol_clause
        );
        let rows = self
            .db
            .as_ref()
            .query_all(Statement::from_string(DatabaseBackend::Postgres, sql))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        rows.into_iter()
            .map(row_to_trigger_order)
            .collect::<Result<Vec<_>, _>>()
            .map_err(AppError::Database)
    }

    /// 获取单个条件单详情
    /// Fetch a single trigger order with ownership check.
    ///
    /// 中文：
    ///   - 二次校验 `order.user_id == user_id`（防越权）
    ///   - 不存在返回 `AppError::NotFound`，越权返回 `AppError::Forbidden`
    ///
    /// English:
    ///   - Secondary `user_id` ownership check prevents cross-tenant access
    ///   - Returns `NotFound` if missing, `Forbidden` if owned by another user
    pub async fn get_trigger_order(
        &self,
        user_id: Uuid,
        order_id: Uuid,
    ) -> Result<TriggerOrder, AppError> {
        let order = self
            .get_trigger_order_via_raw_text(order_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Trigger order not found: {}", order_id)))?;

        if order.user_id != user_id {
            return Err(AppError::Forbidden(
                "No permission to access this trigger order".to_string(),
            ));
        }

        Ok(order)
    }

    /// Internal: insert a single trigger_order row using raw SQL.
    /// Bypasses SeaORM's generated `DeriveActiveEnum` `IntoActiveValue`
    /// (which sends the variant's Rust name as `text`, failing the
    /// real Postgres enum column).
    ///
    /// 中文：用 raw SQL INSERT 写入单行 trigger_order。
    /// 绕开 SeaORM 1.1.x `DeriveActiveEnum` 生成的 `IntoActiveValue`
    /// （该实现把变体的 Rust 名称当 `text` 发送，与真正的 Postgres enum 类型不匹配，
    /// 参考 `sea-orm#2500`）。所有 4 个 enum helpers（trigger_type_str 等）
    /// 集中维护 string-value 映射以保证 raw SQL 与 entity 定义同步。
    #[allow(clippy::too_many_arguments, clippy::result_large_err)]
    pub(crate) async fn insert_trigger_order_raw(
        &self,
        new_id: Uuid,
        user_id: Uuid,
        position_id: Option<Uuid>,
        symbol: &str,
        trigger_type: TriggerType,
        trigger_direction: TriggerDirection,
        trigger_price: Option<f64>,
        trigger_price_upper: Option<f64>,
        trigger_price_lower: Option<f64>,
        base_price: Option<f64>,
        side: TwapSide,
        quantity: f64,
        twap_slice_quantity: f64,
        twap_interval_secs: i32,
        twap_start_time: Option<chrono::DateTime<Utc>>,
        twap_end_time: Option<chrono::DateTime<Utc>>,
        twap_executed_slices: i32,
        twap_max_slices: i32,
        oco_pair_id: Option<Uuid>,
        triggered_order_id: Option<Uuid>,
        trigger_reason: Option<&str>,
        expire_at: Option<chrono::DateTime<Utc>>,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), AppError> {
        let opt_str = |v: Option<chrono::DateTime<Utc>>| match v {
            Some(t) => format!("'{}'::timestamptz", t.to_rfc3339()),
            None => "NULL".to_string(),
        };
        let opt_uuid = |v: Option<Uuid>| match v {
            Some(u) => format!("'{}'::uuid", u),
            None => "NULL".to_string(),
        };
        let opt_f64 = |v: Option<f64>| match v {
            Some(f) => f.to_string(),
            None => "NULL".to_string(),
        };
        let opt_text = |v: Option<&str>| match v {
            Some(s) => format!("'{}'", s.replace('\'', "''")),
            None => "NULL".to_string(),
        };
        let sql = format!(
            "INSERT INTO trigger_orders (\
                id, user_id, position_id, symbol, trigger_type, status, \
                trigger_direction, trigger_price, trigger_price_upper, \
                trigger_price_lower, base_price, side, quantity, \
                filled_quantity, avg_fill_price, twap_slice_quantity, \
                twap_interval_secs, twap_start_time, twap_end_time, \
                twap_executed_slices, twap_max_slices, oco_pair_id, \
                triggered_order_id, trigger_reason, triggered_at, \
                expire_at, created_at, updated_at, cancelled_at\
             ) VALUES (\
                '{id}'::uuid, '{uid}'::uuid, {pid}, '{sym}', \
                '{tt}'::trigger_type, 'pending'::trigger_status, \
                '{td}'::trigger_direction, {tp}, {tpu}, {tpl}, {bp}, '{sd}', \
                {qty}, 0.0, NULL, {tsq}, {tis}, {tst}, {tet}, \
                {tes}, {tms}, {oco}, {trid}, {trsn}, NULL, \
                {exp}, '{now}'::timestamptz, '{now}'::timestamptz, NULL\
             )",
            id = new_id,
            uid = user_id,
            pid = opt_uuid(position_id),
            sym = symbol,
            tt = trigger_type_str(&trigger_type),
            td = trigger_direction_str(&trigger_direction),
            tp = opt_f64(trigger_price),
            tpu = opt_f64(trigger_price_upper),
            tpl = opt_f64(trigger_price_lower),
            bp = opt_f64(base_price),
            sd = match side {
                TwapSide::Buy => "buy",
                TwapSide::Sell => "sell",
            },
            qty = quantity,
            tsq = twap_slice_quantity,
            tis = twap_interval_secs,
            tst = opt_str(twap_start_time),
            tet = opt_str(twap_end_time),
            tes = twap_executed_slices,
            tms = twap_max_slices,
            oco = opt_uuid(oco_pair_id),
            trid = opt_uuid(triggered_order_id),
            trsn = opt_text(trigger_reason),
            exp = opt_str(expire_at),
            now = now.to_rfc3339(),
        );
        self.db
            .as_ref()
            .execute(Statement::from_string(DatabaseBackend::Postgres, sql))
            .await
            .map_err(|e| {
                error!("Failed to insert trigger order: {:?}", e);
                AppError::Database(e.to_string())
            })?;
        Ok(())
    }

    /// Internal: append a TWAP slice execution: increment `filled_quantity`
    /// and `twap_executed_slices`, and (optionally) mark the order as
    /// `triggered` when the final slice completes.
    ///
    /// 中文：推进 TWAP 状态机：累加 filled_quantity / twap_executed_slices，
    /// 最后一刀时把 status 升为 triggered。raw SQL 同样为绕开 SeaORM enum bug。
    ///
    /// English raw-SQL reason: same SeaORM enum workaround as `insert_trigger_order_raw`.
    pub(crate) async fn update_twap_slice_raw(
        &self,
        order_id: Uuid,
        new_filled: f64,
        new_slices: i32,
        final_slice: bool,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), AppError> {
        let status_clause = if final_slice {
            ", status = 'triggered'::trigger_status"
        } else {
            ""
        };
        let sql = format!(
            "UPDATE trigger_orders SET \
                filled_quantity = {nf}, \
                twap_executed_slices = {ns}, \
                updated_at = '{n}'::timestamptz \
                {st} \
             WHERE id = '{id}'::uuid",
            nf = new_filled,
            ns = new_slices,
            n = now.to_rfc3339(),
            st = status_clause,
            id = order_id
        );
        self.db
            .as_ref()
            .execute(Statement::from_string(DatabaseBackend::Postgres, sql))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Internal: update the `status` column of a trigger_order to one of the
    /// known enum values, using raw SQL. Required because the SeaORM
    /// `DeriveActiveEnum` `IntoActiveValue` cannot encode the variant name
    /// as the corresponding Postgres enum value.
    ///
    /// 中文：用 raw SQL 把 status 列更新为已知 enum 值之一。
    /// 同样为绕开 SeaORM 1.1.x 的 enum bug（参考 `insert_trigger_order_raw` 的注释）。
    pub(crate) async fn update_trigger_status_raw(
        &self,
        order_id: Uuid,
        new_status: TriggerStatus,
    ) -> Result<(), AppError> {
        let status_str = match new_status {
            TriggerStatus::Pending => "pending",
            TriggerStatus::Triggered => "triggered",
            TriggerStatus::Cancelled => "cancelled",
            TriggerStatus::Expired => "expired",
            TriggerStatus::Failed => "failed",
        };
        let sql = format!(
            "UPDATE trigger_orders SET status = '{s}'::trigger_status, updated_at = '{n}'::timestamptz WHERE id = '{id}'::uuid",
            s = status_str,
            n = Utc::now().to_rfc3339(),
            id = order_id
        );
        self.db
            .as_ref()
            .execute(Statement::from_string(DatabaseBackend::Postgres, sql))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// Internal: list all pending trigger_orders for the given symbol using
    /// raw SQL with `::text` casts. Required because SeaORM 1.1.x's enum
    /// decode on the real Postgres enum type fails.
    ///
    /// 中文：用 raw SQL + `::text` cast 列出某 symbol 的所有 pending trigger 单。
    /// 同样为绕开 SeaORM 1.1.x enum decode 在真 Postgres enum 类型上的失败。
    pub(crate) async fn list_pending_trigger_orders_raw(
        &self,
        symbol: &str,
    ) -> Result<Vec<TriggerOrder>, String> {
        let sql = format!(
            "SELECT id, user_id, position_id, symbol, \
                    trigger_type::text      AS trigger_type, \
                    status::text            AS status, \
                    trigger_direction::text AS trigger_direction, \
                    trigger_price, trigger_price_upper, trigger_price_lower, \
                    base_price, side, quantity, filled_quantity, avg_fill_price, \
                    twap_slice_quantity, twap_interval_secs, twap_start_time, \
                    twap_end_time, twap_executed_slices, twap_max_slices, \
                    oco_pair_id, triggered_order_id, trigger_reason, \
                    triggered_at, expire_at, created_at, updated_at, cancelled_at \
             FROM trigger_orders \
             WHERE symbol = '{sym}' \
               AND status = 'pending'::trigger_status",
            sym = symbol
        );
        let rows = self
            .db
            .as_ref()
            .query_all(Statement::from_string(DatabaseBackend::Postgres, sql))
            .await
            .map_err(|e| e.to_string())?;
        rows.into_iter()
            .map(row_to_trigger_order)
            .collect::<Result<Vec<_>, _>>()
    }

    /// Internal: read a single trigger_order row using a raw SELECT that
    /// casts the enum columns to text. Required because SeaORM 1.1.x's
    /// generated `FromQueryResult` for a `DeriveActiveEnum` enum column
    /// decodes the value as TEXT, which fails against the real Postgres
    /// enum type on the wire.
    ///
    /// 中文：用 raw SELECT + enum→text cast 读单行 trigger_order。
    /// 原因：SeaORM 1.1.x `DeriveActiveEnum` 生成的 `FromQueryResult` 走 TEXT 解码，
    /// 遇到真 Postgres enum 类型在 wire 上失败。返回 None 表示行不存在（区别于 DB error）。
    ///
    /// English: returns `Ok(None)` if the row does not exist (distinct from DB error).
    async fn get_trigger_order_via_raw_text(
        &self,
        order_id: Uuid,
    ) -> Result<Option<TriggerOrder>, AppError> {
        let sql = format!(
            "SELECT id, user_id, position_id, symbol, \
                    trigger_type::text      AS trigger_type, \
                    status::text            AS status, \
                    trigger_direction::text AS trigger_direction, \
                    trigger_price, trigger_price_upper, trigger_price_lower, \
                    base_price, side, quantity, filled_quantity, avg_fill_price, \
                    twap_slice_quantity, twap_interval_secs, twap_start_time, \
                    twap_end_time, twap_executed_slices, twap_max_slices, \
                    oco_pair_id, triggered_order_id, trigger_reason, \
                    triggered_at, expire_at, created_at, updated_at, cancelled_at \
             FROM trigger_orders WHERE id = '{id}'::uuid",
            id = order_id
        );
        let row = self
            .db
            .as_ref()
            .query_one(Statement::from_string(DatabaseBackend::Postgres, sql))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let Some(row) = row else { return Ok(None) };
        // Map the (post-cast) text columns to the strongly-typed Model.
        row_to_trigger_order(row)
            .map(Some)
            .map_err(AppError::Database)
    }

    /// 处理TWAP订单切片
    /// Process one TWAP tick: create a market slice order, update fill counters.
    ///
    /// 中文：
    ///   - 由 `position_alert_monitor` 或 `matching_engine` 周期调度（每 interval_secs）
    ///   - 校验：order 必须存在 / 类型为 TWAP / 状态为 pending
    ///   - 早退：超过 `twap_end_time` → 标 Expired / 已达 `twap_max_slices` → 标 Triggered / 数量耗尽 → 标 Triggered
    ///   - 切片量 = min(remaining, slice_quantity) — 最后一刀自然吸收余数
    ///   - raw SQL 推进 filled_quantity + twap_executed_slices
    ///   - 返回 `Ok(true)` 表示本 tick 下了单，`Ok(false)` 表示推进到终态
    ///
    /// English:
    ///   - Invoked by the periodic scheduler (every `interval_secs`)
    ///   - Validates: order exists / type is TWAP / status is pending
    ///   - Early returns: past `twap_end_time` → Expired; reached `twap_max_slices` → Triggered;
    ///     quantity exhausted → Triggered
    ///   - Slice size = `min(remaining, slice_quantity)` — last slice naturally absorbs the remainder
    ///   - Raw SQL to advance `filled_quantity` and `twap_executed_slices`
    ///   - Returns `Ok(true)` if a slice order was placed, `Ok(false)` if it reached a terminal state
    pub async fn process_twap_slice(
        &self,
        order_id: Uuid,
        current_price: f64,
    ) -> Result<bool, AppError> {
        let order = TriggerOrderEntity::find_by_id(order_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("TWAP order not found: {}", order_id)))?;

        if order.trigger_type != TriggerType::Twap {
            return Err(AppError::BadRequest(
                "Order is not a TWAP order".to_string(),
            ));
        }

        if order.status != TriggerStatus::Pending {
            return Ok(false); // 已完成或已取消
        }

        // 检查是否已超过结束时间
        if let Some(end_time) = order.twap_end_time
            && Utc::now() > end_time
        {
            self.update_trigger_status_raw(order.id, TriggerStatus::Expired)
                .await?;
            return Ok(false);
        }

        // 检查是否达到最大切片数
        if order.twap_executed_slices >= order.twap_max_slices {
            self.update_trigger_status_raw(order.id, TriggerStatus::Triggered)
                .await?;
            return Ok(false);
        }

        // 计算本次成交数量
        let remaining_qty = order.quantity - order.filled_quantity;
        let slice_qty = remaining_qty.min(order.twap_slice_quantity);

        if slice_qty <= 1e-12 {
            self.update_trigger_status_raw(order.id, TriggerStatus::Triggered)
                .await?;
            return Ok(false);
        }

        // 创建市价单
        let order_side = match order.side {
            TwapSide::Buy => OrderSide::Buy,
            TwapSide::Sell => OrderSide::Sell,
        };

        let now = Utc::now();
        let new_order_id = Uuid::new_v4();

        let market_order = crate::db::order::ActiveModel {
            id: Set(new_order_id),
            user_id: Set(order.user_id),
            strategy_id: Set(None),
            symbol: Set(order.symbol.clone()),
            side: Set(order_side),
            order_type: Set(OrderType::Market),
            price: Set(Some(current_price)),
            quantity: Set(slice_qty),
            filled_quantity: Set(0.0),
            avg_fill_price: Set(None),
            status: Set(OrderStatus::Pending),
            mode: Set(TradeMode::Paper),
            fee: Set(0.0),
            reject_reason: Set(None),
            time_in_force: Set(TimeInForce::IOC),
            expire_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            cancelled_at: Set(None),
            filled_at: Set(None),
            // P1-2: advanced order type fields
            advanced_type: Set(None),
            advanced_params: Set(None),
        };

        market_order
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 更新TWAP订单状态 (raw SQL — see comment on insert_trigger_order_raw)
        let new_filled = order.filled_quantity + slice_qty;
        let current_slices = order.twap_executed_slices + 1;
        let final_slice =
            new_filled >= order.quantity - 1e-12 || current_slices >= order.twap_max_slices;
        self.update_twap_slice_raw(order.id, new_filled, current_slices, final_slice, now)
            .await?;

        info!(
            order_id = %order_id,
            slice_qty = slice_qty,
            filled_qty = new_filled,
            total_qty = order.quantity,
            "TWAP slice executed"
        );

        Ok(true)
    }
}

