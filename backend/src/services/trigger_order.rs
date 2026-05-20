//! services/trigger_order.rs — Trigger Order Service (条件触发单服务)
//!
//! P1-F3: 条件触发单 - 止损单/止盈单/OCO/TWAP
//! 依赖 P1-F2 实盘止盈止损

use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use tracing::{info, warn, error};

use crate::db::trigger_order::{
    Entity as TriggerOrderEntity, Model as TriggerOrder, ActiveModel as TriggerOrderActive,
    TriggerType, TriggerStatus, TriggerDirection, TwapSide,
};
use crate::db::order::{Entity as OrderEntity, Model as Order, OrderSide, OrderType, TradeMode, OrderStatus, TimeInForce};
use crate::db::order::positions::Entity as PositionEntity;
use crate::services::matching_engine::MatchingEngine;
use crate::utils::error::AppError;

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
        let model = TriggerOrderActive {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            position_id: Set(Some(position_id)),
            symbol: Set(symbol.to_string()),
            trigger_type: Set(TriggerType::StopLoss),
            status: Set(TriggerStatus::Pending),
            trigger_direction: Set(trigger_direction),
            trigger_price: Set(trigger_price),
            trigger_price_upper: Set(None),
            trigger_price_lower: Set(None),
            base_price: Set(base_price),
            side: Set(side),
            quantity: Set(quantity),
            filled_quantity: Set(0.0),
            avg_fill_price: Set(None),
            twap_slice_quantity: Set(0.0),
            twap_interval_secs: Set(60),
            twap_start_time: Set(None),
            twap_end_time: Set(None),
            twap_executed_slices: Set(0),
            twap_max_slices: Set(0),
            oco_pair_id: Set(None),
            triggered_order_id: Set(None),
            trigger_reason: Set(None),
            triggered_at: Set(None),
            expire_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            cancelled_at: Set(None),
        };

        let result = model.insert(self.db.as_ref()).await
            .map_err(|e| {
                error!("Failed to create stop loss order: {:?}", e);
                AppError::Database(e.to_string())
            })?;

        info!(
            user_id = %user_id,
            position_id = %position_id,
            symbol = %symbol,
            trigger_price = trigger_price,
            "Stop loss order created"
        );

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
        let model = TriggerOrderActive {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            position_id: Set(Some(position_id)),
            symbol: Set(symbol.to_string()),
            trigger_type: Set(TriggerType::TakeProfit),
            status: Set(TriggerStatus::Pending),
            trigger_direction: Set(trigger_direction),
            trigger_price: Set(trigger_price),
            trigger_price_upper: Set(None),
            trigger_price_lower: Set(None),
            base_price: Set(base_price),
            side: Set(side),
            quantity: Set(quantity),
            filled_quantity: Set(0.0),
            avg_fill_price: Set(None),
            twap_slice_quantity: Set(0.0),
            twap_interval_secs: Set(60),
            twap_start_time: Set(None),
            twap_end_time: Set(None),
            twap_executed_slices: Set(0),
            twap_max_slices: Set(0),
            oco_pair_id: Set(None),
            triggered_order_id: Set(None),
            trigger_reason: Set(None),
            triggered_at: Set(None),
            expire_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            cancelled_at: Set(None),
        };

        let result = model.insert(self.db.as_ref()).await
            .map_err(|e| {
                error!("Failed to create take profit order: {:?}", e);
                AppError::Database(e.to_string())
            })?;

        info!(
            user_id = %user_id,
            position_id = %position_id,
            symbol = %symbol,
            trigger_price = trigger_price,
            "Take profit order created"
        );

        Ok(result)
    }

    /// 创建OCO单 (One-Cancels-Other)
    /// 当一个触发时，另一个自动取消
    pub async fn create_oco(
        &self,
        user_id: Uuid,
        position_id: Uuid,
        symbol: &str,
        stop_loss_price: f64,    // 止损价格
        take_profit_price: f64,  // 止盈价格
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
        let (stop_trigger_dir, profit_trigger_dir) = if position.side == crate::db::order::PositionSide::Long {
            (TriggerDirection::Down, TriggerDirection::Up)
        } else {
            (TriggerDirection::Up, TriggerDirection::Down)
        };

        let now = Utc::now();

        // 创建止损单
        let stop_loss_id = Uuid::new_v4();
        let stop_loss_model = TriggerOrderActive {
            id: Set(stop_loss_id),
            user_id: Set(user_id),
            position_id: Set(Some(position_id)),
            symbol: Set(symbol.to_string()),
            trigger_type: Set(TriggerType::Oco),
            status: Set(TriggerStatus::Pending),
            trigger_direction: Set(stop_trigger_dir),
            trigger_price: Set(stop_loss_price),
            trigger_price_upper: Set(Some(take_profit_price)),
            trigger_price_lower: Set(None),
            base_price: Set(base_price),
            side: Set(side.clone()),
            quantity: Set(quantity),
            filled_quantity: Set(0.0),
            avg_fill_price: Set(None),
            twap_slice_quantity: Set(0.0),
            twap_interval_secs: Set(60),
            twap_start_time: Set(None),
            twap_end_time: Set(None),
            twap_executed_slices: Set(0),
            twap_max_slices: Set(0),
            oco_pair_id: Set(None), // 稍后更新
            triggered_order_id: Set(None),
            trigger_reason: Set(None),
            triggered_at: Set(None),
            expire_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            cancelled_at: Set(None),
        };

        // 创建止盈单
        let take_profit_id = Uuid::new_v4();
        let take_profit_model = TriggerOrderActive {
            id: Set(take_profit_id),
            user_id: Set(user_id),
            position_id: Set(Some(position_id)),
            symbol: Set(symbol.to_string()),
            trigger_type: Set(TriggerType::Oco),
            status: Set(TriggerStatus::Pending),
            trigger_direction: Set(profit_trigger_dir),
            trigger_price: Set(take_profit_price),
            trigger_price_upper: Set(None),
            trigger_price_lower: Set(Some(stop_loss_price)),
            base_price: Set(base_price),
            side: Set(side),
            quantity: Set(quantity),
            filled_quantity: Set(0.0),
            avg_fill_price: Set(None),
            twap_slice_quantity: Set(0.0),
            twap_interval_secs: Set(60),
            twap_start_time: Set(None),
            twap_end_time: Set(None),
            twap_executed_slices: Set(0),
            twap_max_slices: Set(0),
            oco_pair_id: Set(None), // 稍后更新
            triggered_order_id: Set(None),
            trigger_reason: Set(None),
            triggered_at: Set(None),
            expire_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            cancelled_at: Set(None),
        };

        // 插入两个订单
        let stop_loss = stop_loss_model.insert(self.db.as_ref()).await
            .map_err(|e| {
                error!("Failed to create stop loss order: {:?}", e);
                AppError::Database(e.to_string())
            })?;

        let take_profit = take_profit_model.insert(self.db.as_ref()).await
            .map_err(|e| {
                error!("Failed to create take profit order: {:?}", e);
                AppError::Database(e.to_string())
            })?;

        // 更新互相关联
        let mut stop_loss_update: TriggerOrderActive = stop_loss.clone().into();
        stop_loss_update.oco_pair_id = Set(Some(take_profit_id));
        stop_loss_update.updated_at = Set(Utc::now());
        stop_loss_update.update(self.db.as_ref()).await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut take_profit_update: TriggerOrderActive = take_profit.clone().into();
        take_profit_update.oco_pair_id = Set(Some(stop_loss_id));
        take_profit_update.updated_at = Set(Utc::now());
        take_profit_update.update(self.db.as_ref()).await
            .map_err(|e| AppError::Database(e.to_string()))?;

        info!(
            user_id = %user_id,
            position_id = %position_id,
            symbol = %symbol,
            stop_loss_price = stop_loss_price,
            take_profit_price = take_profit_price,
            "OCO order pair created"
        );

        Ok((stop_loss, take_profit))
    }

    /// 创建TWAP单 (Time-Weighted Average Price)
    pub async fn create_twap(
        &self,
        user_id: Uuid,
        symbol: &str,
        side: &str,  // "buy" or "sell"
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

        let model = TriggerOrderActive {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            position_id: Set(None),
            symbol: Set(symbol.to_string()),
            trigger_type: Set(TriggerType::Twap),
            status: Set(TriggerStatus::Pending),
            trigger_direction: Set(if side == "buy" { TriggerDirection::Up } else { TriggerDirection::Down }),
            trigger_price: Set(0.0), // TWAP不需要触发价格监控
            trigger_price_upper: Set(None),
            trigger_price_lower: Set(None),
            base_price: Set(None),
            side: Set(twap_side),
            quantity: Set(quantity),
            filled_quantity: Set(0.0),
            avg_fill_price: Set(None),
            twap_slice_quantity: Set(slice_quantity),
            twap_interval_secs: Set(interval_secs),
            twap_start_time: Set(Some(now)),
            twap_end_time: Set(Some(end_time)),
            twap_executed_slices: Set(0),
            twap_max_slices: Set((duration_secs / interval_secs) as i32),
            oco_pair_id: Set(None),
            triggered_order_id: Set(None),
            trigger_reason: Set(None),
            triggered_at: Set(None),
            expire_at: Set(Some(end_time)),
            created_at: Set(now),
            updated_at: Set(now),
            cancelled_at: Set(None),
        };

        let result = model.insert(self.db.as_ref()).await
            .map_err(|e| {
                error!("Failed to create TWAP order: {:?}", e);
                AppError::Database(e.to_string())
            })?;

        info!(
            user_id = %user_id,
            symbol = %symbol,
            side = %side,
            quantity = quantity,
            slice_quantity = slice_quantity,
            interval_secs = interval_secs,
            duration_secs = duration_secs,
            "TWAP order created"
        );

        Ok(result)
    }

    /// 检查价格是否触发条件单
    pub async fn check_trigger(
        &self,
        symbol: &str,
        current_price: f64,
    ) -> Result<(), AppError> {
        // 查询所有待触发的条件单
        let pending_orders = TriggerOrderEntity::find()
            .filter(crate::db::trigger_order::Column::Symbol.eq(symbol))
            .filter(crate::db::trigger_order::Column::Status.eq(TriggerStatus::Pending))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

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
    async fn trigger_order(&self, order: &TriggerOrder, current_price: f64) -> Result<(), AppError> {
        // 如果是OCO单，先取消关联的另一单
        if let Some(oco_pair_id) = order.oco_pair_id {
            self.cancel_trigger_order(oco_pair_id, "OCO pair triggered").await?;
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

        market_order.insert(self.db.as_ref()).await
            .map_err(|e| {
                error!("Failed to create triggered order: {:?}", e);
                AppError::Database(e.to_string())
            })?;

        // 更新条件单状态
        let mut active: TriggerOrderActive = order.clone().into();
        active.status = Set(TriggerStatus::Triggered);
        active.triggered_at = Set(Some(now));
        active.triggered_order_id = Set(Some(new_order_id));
        active.trigger_reason = Set(Some(format!("price_triggered_at_{}", current_price)));
        active.updated_at = Set(now);
        active.update(self.db.as_ref()).await
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

        Ok(())
    }

    /// 取消条件单
    pub async fn cancel_trigger_order(&self, order_id: Uuid, reason: &str) -> Result<(), AppError> {
        let order = TriggerOrderEntity::find_by_id(order_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Trigger order not found: {}", order_id)))?;

        if order.status.is_terminal() {
            return Err(AppError::Conflict(format!(
                "Trigger order status is {:?}, cannot cancel",
                order.status
            )));
        }

        let now = Utc::now();
        let mut active: TriggerOrderActive = order.clone().into();
        active.status = Set(TriggerStatus::Cancelled);
        active.cancelled_at = Set(Some(now));
        active.trigger_reason = Set(Some(reason.to_string()));
        active.updated_at = Set(now);
        active.update(self.db.as_ref()).await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 如果是OCO单，也取消关联的另一单
        if let Some(oco_pair_id) = order.oco_pair_id {
            let pair = TriggerOrderEntity::find_by_id(oco_pair_id)
                .one(self.db.as_ref())
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

            if let Some(pair_order) = pair {
                if pair_order.status == TriggerStatus::Pending {
                    let mut pair_active: TriggerOrderActive = pair_order.clone().into();
                    pair_active.status = Set(TriggerStatus::Cancelled);
                    pair_active.cancelled_at = Set(Some(now));
                    pair_active.trigger_reason = Set(Some(format!("OCO cancelled by pair: {}", reason)));
                    pair_active.updated_at = Set(now);
                    pair_active.update(self.db.as_ref()).await
                        .map_err(|e| AppError::Database(e.to_string()))?;
                }
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
        let mut query = TriggerOrderEntity::find()
            .filter(crate::db::trigger_order::Column::UserId.eq(user_id));

        if let Some(ref s) = status {
            match s.as_str() {
                "pending" => query = query.filter(crate::db::trigger_order::Column::Status.eq(TriggerStatus::Pending)),
                "triggered" => query = query.filter(crate::db::trigger_order::Column::Status.eq(TriggerStatus::Triggered)),
                "cancelled" => query = query.filter(crate::db::trigger_order::Column::Status.eq(TriggerStatus::Cancelled)),
                "expired" => query = query.filter(crate::db::trigger_order::Column::Status.eq(TriggerStatus::Expired)),
                _ => {}
            }
        }

        if let Some(ref sym) = symbol {
            query = query.filter(crate::db::trigger_order::Column::Symbol.eq(sym));
        }

        let orders = query
            .order_by_desc(crate::db::trigger_order::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(orders)
    }

    /// 获取单个条件单详情
    pub async fn get_trigger_order(&self, user_id: Uuid, order_id: Uuid) -> Result<TriggerOrder, AppError> {
        let order = TriggerOrderEntity::find_by_id(order_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Trigger order not found: {}", order_id)))?;

        if order.user_id != user_id {
            return Err(AppError::Forbidden("No permission to access this trigger order".to_string()));
        }

        Ok(order)
    }

    /// 处理TWAP订单切片
    pub async fn process_twap_slice(&self, order_id: Uuid, current_price: f64) -> Result<bool, AppError> {
        let order = TriggerOrderEntity::find_by_id(order_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("TWAP order not found: {}", order_id)))?;

        if order.trigger_type != TriggerType::Twap {
            return Err(AppError::BadRequest("Order is not a TWAP order".to_string()));
        }

        if order.status != TriggerStatus::Pending {
            return Ok(false); // 已完成或已取消
        }

        // 检查是否已超过结束时间
        if let Some(end_time) = order.twap_end_time {
            if Utc::now() > end_time {
                let mut active: TriggerOrderActive = order.clone().into();
                active.status = Set(TriggerStatus::Expired);
                active.updated_at = Set(Utc::now());
                active.update(self.db.as_ref()).await
                    .map_err(|e| AppError::Database(e.to_string()))?;
                return Ok(false);
            }
        }

        // 检查是否达到最大切片数
        if order.twap_executed_slices >= order.twap_max_slices {
            let mut active: TriggerOrderActive = order.clone().into();
            active.status = Set(TriggerStatus::Triggered);
            active.updated_at = Set(Utc::now());
            active.update(self.db.as_ref()).await
                .map_err(|e| AppError::Database(e.to_string()))?;
            return Ok(false);
        }

        // 计算本次成交数量
        let remaining_qty = order.quantity - order.filled_quantity;
        let slice_qty = remaining_qty.min(order.twap_slice_quantity);

        if slice_qty <= 1e-12 {
            let mut active: TriggerOrderActive = order.clone().into();
            active.status = Set(TriggerStatus::Triggered);
            active.updated_at = Set(Utc::now());
            active.update(self.db.as_ref()).await
                .map_err(|e| AppError::Database(e.to_string()))?;
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

        market_order.insert(self.db.as_ref()).await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // 更新TWAP订单状态
        let mut active: TriggerOrderActive = order.clone().into();
        active.filled_quantity = Set(order.filled_quantity + slice_qty);
        active.twap_executed_slices = Set(order.twap_executed_slices + 1);
        active.updated_at = Set(now);

        // 如果已完成，更新状态
        let new_filled = order.filled_quantity + slice_qty;
        let current_slices = order.twap_executed_slices + 1;
        if new_filled >= order.quantity - 1e-12 || current_slices >= order.twap_max_slices {
            active.status = Set(TriggerStatus::Triggered);
        }

        active.update(self.db.as_ref()).await
            .map_err(|e| AppError::Database(e.to_string()))?;

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
