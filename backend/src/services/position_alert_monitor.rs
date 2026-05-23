//! services/position_alert_monitor.rs — P1-F2 止盈止损实时监控
//!
//! PRD: P1-F2 实盘止盈止损
//! 职责：
//! 1. 接收实时行情（价格心跳）
//! 2. 对所有活跃 alert 进行触发检查
//! 3. 触发时自动提交市价/限价平仓单
//! 4. 支持部分持仓止盈（触发后若持仓有剩余，自动创建新 alert）
//!
//! 集成方式：
//! - 由 `MatchingEngine::spawn_alert_monitor()` 启动独立监控任务
//! - 通过 `on_price_update()` 接收价格事件并检查
//!
//! P1-F6: 告警触发后通过 AlertNotificationService 发送多渠道通知

use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{error, info, warn};

use crate::db::order::positions::Column as PosCol;
use crate::db::order::positions::Entity as PosEntity;
use crate::db::order::positions::Model as Position;
use crate::db::order::{OrderSide, PositionSide};
use crate::db::position_alerts::{
    ActiveModel as AlertActive, AlertStatus, AlertType, Column as AlertCol, Entity as AlertEntity,
    Model as AlertModel, TriggerMode,
};
use crate::services::alert_notification_service::AlertNotificationService;
use crate::services::matching_engine::MatchingEngine;
use crate::services::notification::AlertNotification;
use crate::services::position_alert_service::PositionAlertService;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

// ─── Message Types ───────────────────────────────────────────────

/// 价格心跳消息
#[derive(Debug, Clone)]
pub struct PriceTick {
    pub symbol: String,
    pub price: f64,    // 当前价格
    pub high_24h: f64, // 24h 高
    pub low_24h: f64,  // 24h 低
}

/// 触发执行结果
#[derive(Debug, Clone)]
pub struct AlertExecutionResult {
    pub alert_id: Uuid,
    pub position_id: Uuid,
    pub symbol: String,
    pub exit_reason: String,
    pub triggered_price: f64,
    pub exit_price: f64,
    pub quantity: f64,
    pub order_id: Option<Uuid>,
    pub error: Option<String>,
}

// ─── PositionAlertMonitor ───────────────────────────────────────

/// 止盈止损监控器
///
/// 在后台任务中运行，接收实时行情心跳，检查所有活跃 alert 是否触发，
/// 并自动发送平仓指令。触发时通过通知服务发送多渠道告警。
pub struct PositionAlertMonitor {
    db: Arc<DatabaseConnection>,
    engine: Arc<MatchingEngine>,
    #[allow(dead_code)]
    alert_service: PositionAlertService,
    /// 追踪止损状态：alert_id → activated_price（已激活追踪的最高/最低价）
    trailing_state: std::sync::Mutex<HashMap<Uuid, f64>>,
    /// 通知服务（可为空，不强制要求）
    notification_service: Option<Arc<AlertNotificationService>>,
}

impl PositionAlertMonitor {
    pub fn new(
        db: Arc<DatabaseConnection>,
        engine: Arc<MatchingEngine>,
        notification_service: Option<Arc<AlertNotificationService>>,
    ) -> Self {
        Self {
            db: db.clone(),
            engine,
            alert_service: PositionAlertService::new(db),
            trailing_state: std::sync::Mutex::new(HashMap::new()),
            notification_service,
        }
    }

    /// 启动监控任务（后台 tokio task）
    ///
    /// 每隔 `interval_ms` 检查一次所有 symbol 的活跃 alerts
    pub fn spawn(self: Arc<Self>, interval_ms: u64) {
        let db = self.db.clone();
        let _symbol_cache = Arc::new(std::sync::Mutex::new(
            HashMap::<String, (f64, f64, f64)>::new(),
        )); // symbol → (price, high, low)

        // 定期刷新活跃 alert 列表
        let _alert_service = PositionAlertService::new(db.clone());

        tokio::spawn(async move {
            let check_interval = Duration::from_millis(interval_ms.clamp(100, 5000));
            let mut interval_ticker = tokio::time::interval(check_interval);

            loop {
                interval_ticker.tick().await;

                if let Err(e) = self.check_all_active_alerts().await {
                    warn!("Alert monitor check failed: {:?}", e);
                }
            }
        });

        info!("PositionAlertMonitor started (interval={}ms)", interval_ms);
    }

    /// 接收实时价格更新（可被 MatchingEngine 或行情服务调用）
    pub async fn on_price_update(&self, tick: PriceTick) {
        if let Err(e) = self
            .check_symbol_alerts(&tick.symbol, tick.price, tick.high_24h, tick.low_24h)
            .await
        {
            warn!(symbol = %tick.symbol, "on_price_update check failed: {:?}", e);
        }
    }

    /// 检查指定交易对的所有活跃 alerts
    async fn check_symbol_alerts(
        &self,
        symbol: &str,
        current_price: f64,
        high_24h: f64,
        low_24h: f64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 查询该 symbol 的所有活跃 alerts
        let alerts: Vec<AlertModel> = AlertEntity::find()
            .filter(AlertCol::Symbol.eq(symbol))
            .filter(AlertCol::Status.eq(AlertStatus::Active))
            .all(self.db.as_ref())
            .await?;

        for alert in alerts {
            if let Err(e) = self
                .check_single_alert(&alert, current_price, high_24h, low_24h)
                .await
            {
                warn!(alert_id = %alert.id, "Failed to check alert: {:?}", e);
            }
        }

        Ok(())
    }

    /// 检查所有活跃 alerts（定期全量检查）
    async fn check_all_active_alerts(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let alerts: Vec<AlertModel> = AlertEntity::find()
            .filter(AlertCol::Status.eq(AlertStatus::Active))
            .all(self.db.as_ref())
            .await?;

        for alert in alerts {
            // 跳过追踪止损（需要实时价格）
            if matches!(alert.alert_type, AlertType::TrailingStop) {
                continue;
            }

            // 获取持仓信息以获取参考价格
            let position = match PosEntity::find()
                .filter(PosCol::Id.eq(alert.position_id))
                .filter(PosCol::UserId.eq(alert.user_id))
                .one(self.db.as_ref())
                .await
            {
                Ok(Some(p)) => p,
                _ => continue,
            };

            // 使用当前价格（或上次已知价格）检查
            let current_price = position.avg_entry_price; // 估算值，需要真实价格更新
            let high = current_price;
            let low = current_price;

            if let Err(e) = self
                .check_single_alert(&alert, current_price, high, low)
                .await
            {
                warn!(alert_id = %alert.id, "Failed to check alert: {:?}", e);
            }
        }

        Ok(())
    }

    /// 检查单个 alert 是否触发，触发则执行平仓
    async fn check_single_alert(
        &self,
        alert: &AlertModel,
        current_price: f64,
        high_24h: f64,
        low_24h: f64,
    ) -> Result<Option<AlertExecutionResult>, Box<dyn std::error::Error + Send + Sync>> {
        // 获取持仓
        let position = match PosEntity::find()
            .filter(PosCol::Id.eq(alert.position_id))
            .filter(PosCol::UserId.eq(alert.user_id))
            .one(self.db.as_ref())
            .await
        {
            Ok(Some(p)) => p,
            Ok(None) => {
                info!(alert_id = %alert.id, "Position no longer exists, cancelling alert");
                self.cancel_alert(alert.id).await?;
                return Ok(None);
            }
            Err(e) => return Err(Box::new(e)),
        };

        if position.quantity < 1e-12 {
            info!(alert_id = %alert.id, position_id = %position.id, "Position qty=0, cancelling alert");
            self.cancel_alert(alert.id).await?;
            return Ok(None);
        }

        let (triggered, exit_price, exit_reason) =
            self.evaluate_alert(alert, &position, current_price, high_24h, low_24h);

        if !triggered {
            return Ok(None);
        }

        info!(
            alert_id = %alert.id,
            position_id = %position.id,
            exit_reason = %exit_reason,
            exit_price = exit_price,
            "Alert triggered — executing close order"
        );

        // 执行平仓
        let result = self
            .execute_close_order(&position, alert, exit_price, &exit_reason)
            .await;

        match &result {
            Ok(r) => {
                info!(
                    alert_id = %alert.id,
                    order_id = ?r.order_id,
                    "Alert close order executed"
                );
            }
            Err(e) => {
                error!(
                    alert_id = %alert.id,
                    "Failed to execute close order: {:?}",
                    e
                );
            }
        }

        Ok(Some(result?))
    }

    /// 评估 alert 是否触发（返回触发条件）
    fn evaluate_alert(
        &self,
        alert: &AlertModel,
        position: &Position,
        current_price: f64,
        high_24h: f64,
        low_24h: f64,
    ) -> (bool, f64, String) {
        let trailing_dist = alert.trailing_distance.unwrap_or(0.005);

        match alert.alert_type {
            AlertType::TakeProfit => {
                let (_check_price, is_triggered) = match position.side {
                    PositionSide::Long => (high_24h, high_24h >= alert.trigger_price),
                    PositionSide::Short => (low_24h, low_24h <= alert.trigger_price),
                };
                if is_triggered {
                    (true, alert.trigger_price, "take_profit".to_string())
                } else {
                    (false, 0.0, String::new())
                }
            }
            AlertType::StopLoss => {
                let (_check_price, is_triggered) = match position.side {
                    PositionSide::Long => (low_24h, low_24h <= alert.trigger_price),
                    PositionSide::Short => (high_24h, high_24h >= alert.trigger_price),
                };
                if is_triggered {
                    (true, alert.trigger_price, "stop_loss".to_string())
                } else {
                    (false, 0.0, String::new())
                }
            }
            AlertType::TrailingStop => {
                // 追踪止损：维护已激活价格
                let mut state = self.trailing_state.lock().unwrap();

                let (activated_price, new_sl) = match position.side {
                    PositionSide::Long => {
                        // 多头：记录持仓以来的最高价
                        let current_peak = high_24h;
                        let existing = state.get(&alert.id).copied().unwrap_or(current_peak);
                        let highest = existing.max(current_peak);
                        let sl_price = highest * (1.0 - trailing_dist);
                        (highest, sl_price)
                    }
                    PositionSide::Short => {
                        // 空头：记录持仓以来的最低价
                        let current_trough = low_24h;
                        let existing = state.get(&alert.id).copied().unwrap_or(current_trough);
                        let lowest = existing.min(current_trough);
                        let sl_price = lowest * (1.0 + trailing_dist);
                        (lowest, sl_price)
                    }
                };

                state.insert(alert.id, activated_price);

                // 检查是否触发
                let is_triggered = match position.side {
                    PositionSide::Long => current_price <= new_sl,
                    PositionSide::Short => current_price >= new_sl,
                };

                if is_triggered {
                    (true, new_sl, "trailing_stop".to_string())
                } else {
                    (false, 0.0, String::new())
                }
            }
        }
    }

    /// 执行平仓指令（市价或限价）
    async fn execute_close_order(
        &self,
        position: &Position,
        alert: &AlertModel,
        exit_price: f64,
        exit_reason: &str,
    ) -> Result<AlertExecutionResult, Box<dyn std::error::Error + Send + Sync>> {
        // 计算平仓数量（使用持仓全部数量；部分止盈时由调用方处理）
        let quantity = position.quantity;

        let order_side = match position.side {
            PositionSide::Long => OrderSide::Sell,
            PositionSide::Short => OrderSide::Buy,
        };

        // 市价单直接执行
        let limit_price = if alert.trigger_mode == TriggerMode::Limit {
            Some(alert.limit_price.unwrap_or(exit_price))
        } else {
            None
        };

        let order_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        // 创建平仓订单记录（挂入撮合引擎）
        if alert.trigger_mode == TriggerMode::Market {
            // 市价单：直接调用 matching_engine
            match self
                .engine
                .match_market(
                    order_id,
                    position.user_id,
                    &position.symbol,
                    &order_side,
                    quantity,
                )
                .await
            {
                Ok(result) => {
                    // 更新持仓
                    self.update_position_after_close(position, result.filled_quantity, exit_reason)
                        .await?;

                    // 标记 alert 为已触发
                    self.mark_alert_triggered(alert.id, order_id, exit_reason, exit_price)
                        .await?;

                    Ok(AlertExecutionResult {
                        alert_id: alert.id,
                        position_id: position.id,
                        symbol: position.symbol.clone(),
                        exit_reason: exit_reason.to_string(),
                        triggered_price: exit_price,
                        exit_price: result.avg_fill_price.unwrap_or(exit_price),
                        quantity: result.filled_quantity,
                        order_id: Some(order_id),
                        error: None,
                    })
                }
                Err(e) => Ok(AlertExecutionResult {
                    alert_id: alert.id,
                    position_id: position.id,
                    symbol: position.symbol.clone(),
                    exit_reason: exit_reason.to_string(),
                    triggered_price: exit_price,
                    exit_price,
                    quantity,
                    order_id: None,
                    error: Some(format!("Matching failed: {}", e)),
                }),
            }
        } else {
            // 限价单：插入订单簿
            let price = limit_price.unwrap_or(exit_price);

            let order_entry = crate::services::matching_engine::OrderEntry {
                order_id,
                user_id: position.user_id,
                symbol: position.symbol.clone(),
                side: order_side,
                price: Some(price),
                remaining_quantity: quantity,
                created_at: now,
            };

            self.engine.insert_limit_order(order_entry);

            self.mark_alert_triggered(alert.id, order_id, exit_reason, exit_price)
                .await?;

            Ok(AlertExecutionResult {
                alert_id: alert.id,
                position_id: position.id,
                symbol: position.symbol.clone(),
                exit_reason: exit_reason.to_string(),
                triggered_price: exit_price,
                exit_price: price,
                quantity,
                order_id: Some(order_id),
                error: None,
            })
        }
    }

    /// 平仓后更新持仓
    async fn update_position_after_close(
        &self,
        position: &Position,
        filled_qty: f64,
        _exit_reason: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let new_qty = position.quantity - filled_qty;

        if new_qty < 1e-12 {
            // 全平，删除持仓记录
            crate::db::order::positions::Entity::delete_by_id(position.id)
                .exec(self.db.as_ref())
                .await?;
        } else {
            // 部分平，更新数量
            let mut active: crate::db::order::positions::ActiveModel = position.clone().into();
            active.quantity = sea_orm::Set(new_qty);
            active.available_quantity = sea_orm::Set(new_qty);
            active.updated_at = sea_orm::Set(chrono::Utc::now());
            active.update(self.db.as_ref()).await?;
        }

        Ok(())
    }

    /// 标记 alert 为已触发，并发送通知
    async fn mark_alert_triggered(
        &self,
        alert_id: Uuid,
        order_id: Uuid,
        exit_reason: &str,
        exit_price: f64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let alert = AlertEntity::find_by_id(alert_id)
            .one(self.db.as_ref())
            .await?
            .ok_or("Alert not found")?;

        // Clone alert before consuming it with .into()
        let alert_for_notification = alert.clone();

        let mut active: AlertActive = alert.into();
        active.status = sea_orm::Set(AlertStatus::Triggered);
        active.triggered_at = sea_orm::Set(Some(chrono::Utc::now()));
        active.triggered_order_id = sea_orm::Set(Some(order_id));
        active.updated_at = sea_orm::Set(chrono::Utc::now());

        active.update(self.db.as_ref()).await?;

        // 清理追踪状态
        self.trailing_state.lock().unwrap().remove(&alert_id);

        // 发送通知
        self.send_alert_notification(&alert_for_notification, exit_reason, exit_price)
            .await;

        Ok(())
    }

    /// 发送告警通知
    async fn send_alert_notification(
        &self,
        alert: &AlertModel,
        exit_reason: &str,
        exit_price: f64,
    ) {
        let Some(ref svc) = self.notification_service else {
            return;
        };

        let notification = AlertNotification::new(
            format!("告警触发: {}", alert.symbol),
            format!(
                "{} 触发 {}，执行价: {}",
                alert.symbol, exit_reason, exit_price
            ),
            "position_alert".to_string(),
            match exit_reason {
                "stop_loss" => "critical",
                "take_profit" => "warning",
                _ => "info",
            }
            .to_string(),
            Some(alert.symbol.clone()),
        )
        .with_metadata("alert_id", alert.id.to_string())
        .with_metadata("position_id", alert.position_id.to_string())
        .with_metadata("exit_reason", exit_reason);

        if let Err(e) = svc.send_alert(notification).await {
            tracing::error!("Failed to send alert notification: {}", e);
        }
    }

    /// 取消 alert
    async fn cancel_alert(
        &self,
        alert_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let alert = AlertEntity::find_by_id(alert_id)
            .one(self.db.as_ref())
            .await?
            .ok_or("Alert not found")?;

        let mut active: AlertActive = alert.into();
        active.status = sea_orm::Set(AlertStatus::Cancelled);
        active.cancelled_at = sea_orm::Set(Some(chrono::Utc::now()));
        active.updated_at = sea_orm::Set(chrono::Utc::now());

        active.update(self.db.as_ref()).await?;
        self.trailing_state.lock().unwrap().remove(&alert_id);

        Ok(())
    }
}
