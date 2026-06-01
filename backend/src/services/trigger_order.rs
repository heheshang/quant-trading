//! services/trigger_order.rs — Trigger Order Service (条件触发单服务)
//!
//! P1-F3: 条件触发单 - 止损单/止盈单/OCO/TWAP
//! 依赖 P1-F2 实盘止盈止损

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
pub struct TriggerOrderService {
    db: Arc<DatabaseConnection>,
}

impl TriggerOrderService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建止损单
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

