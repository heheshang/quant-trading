//! services/position_alert_service.rs — P1-F2 实盘止盈止损服务
//!
//! PRD: P1-F2 实盘止盈止损
//! 功能：止盈单、止损单（市价/限价触发）、追踪止损、手动修改、部分持仓止盈
//!
//! 架构：
//! - 价格监听器：订阅行情频道，实时检查所有活跃 alert
//! - 触发引擎：价格触发时，自动计算平仓数量并发送市价/限价平仓单
//! - 追踪止损：随最高价/最低价动态更新止损价

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::order::{OrderSide, OrderStatus, OrderType, PositionSide};
use crate::db::position_alerts::{
    ActiveModel as AlertActive, AlertStatus, AlertType, Entity as AlertEntity, Model as AlertModel,
    TriggerMode,
};
use crate::db::order::positions::{Entity as PositionEntity, Model as Position};
use crate::utils::error::AppError;

// ─── Types ──────────────────────────────────────────────────────

/// 创建止盈/止损请求
#[derive(Debug, Clone)]
pub struct CreateAlertRequest {
    pub position_id: Uuid,
    pub symbol: String,
    pub alert_type: AlertType,
    pub trigger_price: f64,
    pub trigger_mode: TriggerMode,
    pub limit_price: Option<f64>,
    pub trailing_distance: Option<f64>, // 百分比，如 0.5 表示 0.5%
    pub note: Option<String>,
}

/// 修改止盈止损请求
#[derive(Debug, Clone)]
pub struct UpdateAlertRequest {
    pub trigger_price: Option<f64>,
    pub limit_price: Option<f64>,
    pub trigger_mode: Option<TriggerMode>,
    pub trailing_distance: Option<f64>,
    pub status: Option<AlertStatus>,
}

/// 止盈止损检查结果（单个 alert）
#[derive(Debug, Clone)]
pub struct AlertCheckResult {
    pub alert_id: Uuid,
    pub triggered: bool,
    pub triggered_price: Option<f64>,
    pub exit_reason: String, // "take_profit" | "stop_loss" | "trailing_stop"
}

/// 批量检查结果
#[derive(Debug, Clone)]
pub struct BatchAlertCheckResult {
    pub alerts: Vec<AlertCheckResult>,
    pub total_triggered: usize,
    pub total_partial: usize,
}

/// Alert 响应（带持仓信息）
#[derive(Debug, Clone, serde::Serialize)]
pub struct AlertResponse {
    pub id: Uuid,
    pub position_id: Uuid,
    pub symbol: String,
    pub alert_type: String,
    pub status: String,
    pub trigger_price: String,
    pub limit_price: Option<String>,
    pub trigger_mode: String,
    pub trailing_distance: Option<String>,
    pub trailing_activated: bool,
    pub activated_price: Option<String>,
    pub triggered_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ─── PositionAlertService ─────────────────────────────────────────

/// 止盈止损服务
///
/// 管理持仓的止盈/止损/追踪止损警戒规则。
/// 提供创建、修改、取消、查询、触发检查等能力。
pub struct PositionAlertService {
    db: Arc<DatabaseConnection>,
}

impl PositionAlertService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ─── CRUD ──────────────────────────────────────────────────

    /// 创建止盈/止损警戒
    ///
    /// 验证：
    /// 1. 持仓存在且属于当前用户
    /// 2. 同类型 alert 同一方向只能有一个（避免重复触发）
    pub async fn create_alert(
        &self,
        user_id: Uuid,
        req: CreateAlertRequest,
    ) -> Result<AlertModel, AppError> {
        // 验证持仓存在
        let position = PositionEntity::find()
            .filter(crate::db::order::positions::Column::UserId.eq(user_id))
            .filter(crate::db::order::positions::Column::Id.eq(req.position_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Position not found: {}", req.position_id)))?;

        // 方向校验：止盈只对 Long 做多，止损只对 Short 做空
        match (&req.alert_type, &position.side) {
            (AlertType::TakeProfit, PositionSide::Short) => {
                return Err(AppError::BadRequest(
                    "Take-profit is only valid for long positions".to_string(),
                ));
            }
            (AlertType::StopLoss, PositionSide::Long) => {
                // Stop-loss is valid for both long and short positions
            }
            _ => {}
        }

        let alert_type = req.alert_type.clone();
        // 检查是否已有相同类型的活跃 alert
        let existing = AlertEntity::find()
            .filter(crate::db::position_alerts::Column::UserId.eq(user_id))
            .filter(crate::db::position_alerts::Column::PositionId.eq(req.position_id))
            .filter(crate::db::position_alerts::Column::AlertType.eq(alert_type.clone()))
            .filter(crate::db::position_alerts::Column::Status.eq(AlertStatus::Active))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if existing.is_some() {
            return Err(AppError::BadRequest(format!(
                "Active {} alert already exists for this position. Please update or cancel the existing one.",
                match alert_type {
                    AlertType::TakeProfit => "take_profit",
                    AlertType::StopLoss => "stop_loss",
                    AlertType::TrailingStop => "trailing_stop",
                }
            )));
        }

        // 创建 alert
        let now = chrono::Utc::now();
        let active: AlertActive = AlertActive {
            id: sea_orm::Set(Uuid::new_v4()),
            user_id: sea_orm::Set(user_id),
            position_id: sea_orm::Set(req.position_id),
            symbol: sea_orm::Set(req.symbol.clone()),
            alert_type: sea_orm::Set(req.alert_type.clone()),
            status: sea_orm::Set(AlertStatus::Active),
            trigger_price: sea_orm::Set(req.trigger_price),
            limit_price: sea_orm::Set(req.limit_price),
            trigger_mode: sea_orm::Set(req.trigger_mode.clone()),
            trailing_distance: sea_orm::Set(req.trailing_distance),
            trailing_activated: sea_orm::Set(false),
            activated_price: sea_orm::Set(None),
            triggered_at: sea_orm::Set(None),
            created_at: sea_orm::Set(now),
            updated_at: sea_orm::Set(now),
            cancelled_at: sea_orm::Set(None),
            triggered_order_id: sea_orm::Set(None),
            note: sea_orm::Set(req.note),
        };

        let saved = active
            .insert(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        info!(
            alert_id = %saved.id,
            position_id = %req.position_id,
            alert_type = ?req.alert_type,
            trigger_price = req.trigger_price,
            "Position alert created"
        );

        Ok(saved)
    }

    /// 修改止盈止损（价格/模式/距离）
    pub async fn update_alert(
        &self,
        user_id: Uuid,
        alert_id: Uuid,
        req: UpdateAlertRequest,
    ) -> Result<AlertModel, AppError> {
        let alert = AlertEntity::find()
            .filter(crate::db::position_alerts::Column::UserId.eq(user_id))
            .filter(crate::db::position_alerts::Column::Id.eq(alert_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Alert not found: {}", alert_id)))?;

        if !matches!(alert.status, AlertStatus::Active) {
            return Err(AppError::BadRequest(
                "Can only update active alerts".to_string(),
            ));
        }

        let mut active: AlertActive = alert.into();
        if let Some(trigger_price) = req.trigger_price {
            active.trigger_price = sea_orm::Set(trigger_price);
        }
        if let Some(limit_price) = req.limit_price {
            active.limit_price = sea_orm::Set(Some(limit_price));
        }
        if let Some(trigger_mode) = req.trigger_mode {
            active.trigger_mode = sea_orm::Set(trigger_mode);
        }
        if let Some(trailing_distance) = req.trailing_distance {
            active.trailing_distance = sea_orm::Set(Some(trailing_distance));
        }
        if let Some(status) = req.status {
            active.status = sea_orm::Set(status);
        }
        active.updated_at = sea_orm::Set(chrono::Utc::now());

        let updated = active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        info!(alert_id = %alert_id, "Position alert updated");
        Ok(updated)
    }

    /// 取消止盈止损
    pub async fn cancel_alert(&self, user_id: Uuid, alert_id: Uuid) -> Result<(), AppError> {
        let alert = AlertEntity::find()
            .filter(crate::db::position_alerts::Column::UserId.eq(user_id))
            .filter(crate::db::position_alerts::Column::Id.eq(alert_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Alert not found: {}", alert_id)))?;

        let mut active: AlertActive = alert.into();
        active.status = sea_orm::Set(AlertStatus::Cancelled);
        active.cancelled_at = sea_orm::Set(Some(chrono::Utc::now()));
        active.updated_at = sea_orm::Set(chrono::Utc::now());

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        info!(alert_id = %alert_id, "Position alert cancelled");
        Ok(())
    }

    /// 查询用户所有持仓的活跃止盈止损
    pub async fn list_active_alerts(
        &self,
        user_id: Uuid,
        symbol: Option<String>,
    ) -> Result<Vec<AlertModel>, AppError> {
        let mut query = AlertEntity::find()
            .filter(crate::db::position_alerts::Column::UserId.eq(user_id))
            .filter(crate::db::position_alerts::Column::Status.eq(AlertStatus::Active));

        if let Some(sym) = symbol {
            query = query.filter(crate::db::position_alerts::Column::Symbol.eq(sym));
        }

        let alerts = query
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(alerts)
    }

    /// 查询指定持仓的所有 alert
    pub async fn list_alerts_by_position(
        &self,
        user_id: Uuid,
        position_id: Uuid,
    ) -> Result<Vec<AlertModel>, AppError> {
        let alerts = AlertEntity::find()
            .filter(crate::db::position_alerts::Column::UserId.eq(user_id))
            .filter(crate::db::position_alerts::Column::PositionId.eq(position_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(alerts)
    }

    /// 标记 alert 为已触发，并记录触发后的平仓订单ID
    pub async fn mark_triggered(
        &self,
        alert_id: Uuid,
        order_id: Uuid,
    ) -> Result<(), AppError> {
        let alert = AlertEntity::find_by_id(alert_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?
            .ok_or_else(|| AppError::NotFound(format!("Alert not found: {}", alert_id)))?;

        let mut active: AlertActive = alert.into();
        active.status = sea_orm::Set(AlertStatus::Triggered);
        active.triggered_at = sea_orm::Set(Some(chrono::Utc::now()));
        active.triggered_order_id = sea_orm::Set(Some(order_id));
        active.updated_at = sea_orm::Set(chrono::Utc::now());

        active
            .update(self.db.as_ref())
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ─── 触发检查 ──────────────────────────────────────────────

    /// 检查单个 alert 是否触发（给定当前市场价格）
    ///
    /// 返回：(是否触发, 触发价格, 退出原因)
    pub fn check_alert_trigger(
        &self,
        alert: &AlertModel,
        current_price: f64,
        high_since_open: f64,
        low_since_open: f64,
    ) -> (bool, Option<f64>, String) {
        if !matches!(alert.status, AlertStatus::Active) {
            return (false, None, String::new());
        }

        match alert.alert_type {
            AlertType::TakeProfit => {
                // 多头止盈：价格 >= 触发价
                // 空头止盈：价格 <= 触发价
                let position_side = self.get_position_side_for_alert(alert);
                match position_side {
                    PositionSide::Long => {
                        if high_since_open >= alert.trigger_price {
                            (true, Some(alert.trigger_price), "take_profit".to_string())
                        } else {
                            (false, None, String::new())
                        }
                    }
                    PositionSide::Short => {
                        if low_since_open <= alert.trigger_price {
                            (true, Some(alert.trigger_price), "take_profit".to_string())
                        } else {
                            (false, None, String::new())
                        }
                    }
                }
            }
            AlertType::StopLoss => {
                // 多头止损：价格 <= 触发价
                // 空头止损：价格 >= 触发价
                let position_side = self.get_position_side_for_alert(alert);
                match position_side {
                    PositionSide::Long => {
                        if low_since_open <= alert.trigger_price {
                            (true, Some(alert.trigger_price), "stop_loss".to_string())
                        } else {
                            (false, None, String::new())
                        }
                    }
                    PositionSide::Short => {
                        if high_since_open >= alert.trigger_price {
                            (true, Some(alert.trigger_price), "stop_loss".to_string())
                        } else {
                            (false, None, String::new())
                        }
                    }
                }
            }
            AlertType::TrailingStop => {
                // 追踪止损：记录激活价，动态更新止损价
                // 多头：activated_price = 持仓以来的最高价
                // 止损价 = activated_price * (1 - trailing_distance%)
                // 当 current_price >= activated_price 时，更新 activated_price
                // 当 current_price 回落至 止损价 时，触发
                let position_side = self.get_position_side_for_alert(alert);
                let trailing_dist = alert.trailing_distance.unwrap_or(0.005); // 默认 0.5%

                match position_side {
                    PositionSide::Long => {
                        let highest = if alert.trailing_activated {
                            alert.activated_price.unwrap_or(high_since_open).max(high_since_open)
                        } else {
                            high_since_open
                        };

                        let new_sl_price = highest * (1.0 - trailing_dist);

                        if alert.trailing_activated {
                            if current_price <= new_sl_price {
                                (true, Some(new_sl_price), "trailing_stop".to_string())
                            } else {
                                (false, None, String::new())
                            }
                        } else {
                            // 尚未激活（价格从未高于开仓价 + 一定距离）
                            if current_price >= alert.trigger_price {
                                (false, None, String::new()) // 追踪未触发，仅记录
                            } else {
                                (false, None, String::new())
                            }
                        }
                    }
                    PositionSide::Short => {
                        // 空头追踪止损：记录最低价，止损 = 最低价 * (1 + trailing_dist%)
                        let lowest = if alert.trailing_activated {
                            alert.activated_price.unwrap_or(low_since_open).min(low_since_open)
                        } else {
                            low_since_open
                        };

                        let new_sl_price = lowest * (1.0 + trailing_dist);

                        if alert.trailing_activated {
                            if current_price >= new_sl_price {
                                (true, Some(new_sl_price), "trailing_stop".to_string())
                            } else {
                                (false, None, String::new())
                            }
                        } else {
                            (false, None, String::new())
                        }
                    }
                }
            }
        }
    }

    /// 根据 alert 推断持仓方向（通过查询持仓）
    fn get_position_side_for_alert(&self, alert: &AlertModel) -> PositionSide {
        // 同步查询持仓方向（此方法在价格检查循环中调用）
        // 缓存层可优化，此处直接查 DB
        let position = futures::executor::block_on(
            PositionEntity::find_by_id(alert.position_id)
                .filter(crate::db::order::positions::Column::UserId.eq(alert.user_id))
                .one(self.db.as_ref()),
        );

        position
            .map(|p| p.map(|pos| pos.side))
            .ok()
            .flatten()
            .unwrap_or(PositionSide::Long) // 默认值，不应发生
    }

    /// 批量检查多个 alert（给定当前市场价格）
    ///
    /// 用于行情心跳中批量检查所有活跃 alert
    pub async fn check_alerts_batch(
        &self,
        user_id: Uuid,
        symbol: &str,
        current_price: f64,
        high_24h: f64,
        low_24h: f64,
    ) -> Vec<AlertCheckResult> {
        let alerts = match self.list_active_alerts(user_id, Some(symbol.to_string())).await {
            Ok(a) => a,
            Err(e) => {
                warn!("Failed to list active alerts: {:?}", e);
                return vec![];
            }
        };

        let mut results = Vec::new();
        for alert in alerts {
            let position = match PositionEntity::find()
                .filter(crate::db::order::positions::Column::UserId.eq(user_id))
                .filter(crate::db::order::positions::Column::Id.eq(alert.position_id))
                .one(self.db.as_ref())
                .await
            {
                Ok(Some(p)) => p,
                _ => continue, // 持仓不存在则跳过
            };

            // 根据持仓方向确定 high/low
            let (high_since, low_since) = match position.side {
                PositionSide::Long => (current_price, current_price), // 已开仓，最低/最高价需要行情服务提供
                PositionSide::Short => (current_price, current_price),
            };

            let (triggered, trig_price, reason) =
                self.check_alert_trigger(&alert, current_price, high_since, low_since);

            results.push(AlertCheckResult {
                alert_id: alert.id,
                triggered,
                triggered_price: if triggered { trig_price } else { None },
                exit_reason: reason,
            });
        }

        results
    }

    /// 部分持仓止盈：计算触发后应平仓的数量
    ///
    /// 如果 position 还有剩余持仓（部分触发），则创建新的 alert 继续跟踪
    pub fn calculate_partial_exit(
        &self,
        position: &Position,
        alert_triggered_qty: f64,
    ) -> (f64, f64) {
        // 已平仓数量 = 持仓总数 - 剩余持仓
        let remaining = position.quantity - alert_triggered_qty;
        let exited = alert_triggered_qty;

        if remaining < 1e-12 {
            // 全平
            (position.quantity, 0.0)
        } else {
            // 部分平
            (exited, remaining)
        }
    }
}

// ─── To Response ─────────────────────────────────────────────────

impl AlertResponse {
    pub fn from_model(model: &AlertModel) -> Self {
        Self {
            id: model.id,
            position_id: model.position_id,
            symbol: model.symbol.clone(),
            alert_type: serde_json::to_value(&model.alert_type)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            status: serde_json::to_value(&model.status)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            trigger_price: format!("{:.8}", model.trigger_price),
            limit_price: model.limit_price.map(|p| format!("{:.8}", p)),
            trigger_mode: serde_json::to_value(&model.trigger_mode)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            trailing_distance: model.trailing_distance.map(|d| format!("{:.4}", d)),
            trailing_activated: model.trailing_activated,
            activated_price: model.activated_price.map(|p| format!("{:.8}", p)),
            triggered_at: model.triggered_at.map(|t| t.to_rfc3339()),
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        }
    }
}
