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

use crate::db::order::PositionSide;
use crate::db::order::positions::{Entity as PositionEntity, Model as Position};
use crate::db::position_alerts::{
    ActiveModel as AlertActive, AlertStatus, AlertType, Entity as AlertEntity, Model as AlertModel,
    TriggerMode,
};
use crate::utils::error::AppError;

// ─── Types ──────────────────────────────────────────────────────

/// 创建止盈/止损请求
/// Request payload for `create_alert`.
/// 字段语义：
/// - `position_id` 必须存在且属于当前 user（service 层二次校验）
/// - `trailing_distance` 仅对 TrailingStop 类型有效，与 `trailing_stop_params.rs` 的 (0, 0.1) 一致
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
/// PATCH-style request: all fields are optional; only present fields are written.
#[derive(Debug, Clone)]
pub struct UpdateAlertRequest {
    pub trigger_price: Option<f64>,
    pub limit_price: Option<f64>,
    pub trigger_mode: Option<TriggerMode>,
    pub trailing_distance: Option<f64>,
    pub status: Option<AlertStatus>,
}

/// 止盈止损检查结果（单个 alert）
/// Result of evaluating one alert against current market prices.
#[derive(Debug, Clone)]
pub struct AlertCheckResult {
    pub alert_id: Uuid,
    pub triggered: bool,
    pub triggered_price: Option<f64>,
    pub exit_reason: String, // "take_profit" | "stop_loss" | "trailing_stop"
}

/// 批量检查结果
/// Batch aggregation over many `AlertCheckResult`s.
#[derive(Debug, Clone)]
pub struct BatchAlertCheckResult {
    pub alerts: Vec<AlertCheckResult>,
    pub total_triggered: usize,
    pub total_partial: usize,
}

/// Alert 响应（带持仓信息）
/// HTTP response shape for an alert (string-formatted numbers to avoid float JSON quirks).
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
///
/// Manages TP/SL/TrailingStop rules for open positions.
/// Provides CRUD over alerts plus a pure trigger-check function.
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
    /// Create a TP/SL/TrailingStop alert for an existing position.
    ///
    /// 中文校验：
    ///   1. 持仓存在且属于当前 user（防越权）
    ///   2. 同 position_id + alert_type + Active 状态的 alert 已存在 → 拒绝（避免重复触发）
    ///   3. TakeProfit 校验：short 持仓不支持（语义反转：short 的 TP 应为价格下跌）
    ///   4. StopLoss 对 long/short 都支持
    ///   5. 默认 TrailingStop 未激活（trailing_activated=false），价格首次创 high/low 后由 monitor 激活
    ///
    /// English validation:
    ///   1. Position exists and is owned by `user_id`
    ///   2. Duplicate detection: same position_id + alert_type + Active already → reject
    ///   3. TakeProfit on Short positions is rejected (semantic mismatch)
    ///   4. StopLoss is valid for both Long and Short
    ///   5. TrailingStop starts deactivated; the monitor activates it on the first qualifying tick
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
            .ok_or_else(|| {
                AppError::NotFound(format!("Position not found: {}", req.position_id))
            })?;

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
    /// Patch an active alert. Only `Active` alerts can be modified.
    ///
    /// 中文：
    ///   - 终态（triggered / cancelled）的 alert 拒绝修改
    ///   - PATCH 语义：仅 req 中 Some 的字段被写入
    ///   - status 字段可显式置为非 Active（手动 disable，不算 cancel）
    ///
    /// English:
    ///   - Terminal-status alerts (triggered / cancelled) refuse updates
    ///   - PATCH semantics: only fields present in `req` are written
    ///   - `status` may be set explicitly to non-Active (manual disable, not cancel)
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
    /// Cancel an alert. Differs from `update_alert(status=cancelled)` by also
    /// stamping `cancelled_at` for audit. Idempotent on already-cancelled alerts.
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
    /// List the user's `Active` alerts. Optional `symbol` narrows to one trading pair.
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
    /// List ALL alerts (any status) for one position. Used for history views.
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
    /// Mark an alert as triggered, recording the resulting close-order id.
    ///
    /// 中文：
    ///   - 由 `position_alert_monitor` 在 close order 创建成功后调用
    ///   - 不再做 user_id 二次过滤（内部调用，可信）
    ///   - **保留行**（不删除），用于审计回溯
    pub async fn mark_triggered(&self, alert_id: Uuid, order_id: Uuid) -> Result<(), AppError> {
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
    /// Pure trigger evaluation: returns `(triggered, trigger_price, exit_reason)`.
    ///
    /// 中文：
    ///   - **纯函数**：无副作用，不修改 alert 状态（写操作在 `mark_triggered`）
    ///   - 三个分支：TakeProfit / StopLoss / TrailingStop
    ///   - TrailingStop 必须先 `trailing_activated=true` 才参与触发判定
    ///     （`position_alert_monitor` 在价格创 high/low 后激活）
    ///   - long/short 对称：long 用 high_since_open，short 用 low_since_open
    ///
    /// English:
    ///   - **Pure function**: no side effects; persistence happens in `mark_triggered`
    ///   - Three branches: TakeProfit / StopLoss / TrailingStop
    ///   - TrailingStop only triggers once `trailing_activated=true` (set by the monitor on first qualifying tick)
    ///   - Long uses `high_since_open`; short uses `low_since_open` (symmetric)
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
                            alert
                                .activated_price
                                .unwrap_or(high_since_open)
                                .max(high_since_open)
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
                            (false, None, String::new())
                        }
                    }
                    PositionSide::Short => {
                        // 空头追踪止损：记录最低价，止损 = 最低价 * (1 + trailing_dist%)
                        let lowest = if alert.trailing_activated {
                            alert
                                .activated_price
                                .unwrap_or(low_since_open)
                                .min(low_since_open)
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
    /// Look up the position side for an alert (denormalized lookup).
    ///
    /// 中文：
    ///   - 同步阻塞查询（`futures::executor::block_on`），仅供纯函数 `check_alert_trigger` 调用
    ///   - 默认 Long 是兜底（持仓已不存在时不应走到这里，但需防止 panic）
    ///   - 未来可加 LRU 缓存避免每 tick 一次 DB hit
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
    /// Batch trigger check for all active alerts of `(user_id, symbol)`.
    ///
    /// 中文：
    ///   - 由 `position_alert_monitor` 的 push 模型调用
    ///   - 失败的 listing 仅 warn（不中断其他用户/symbol 的检查）
    ///   - 持仓已不存在的 alert 静默跳过
    ///   - 返回平铺的 `AlertCheckResult` 列表
    pub async fn check_alerts_batch(
        &self,
        user_id: Uuid,
        symbol: &str,
        current_price: f64,
        _high_24h: f64,
        _low_24h: f64,
    ) -> Vec<AlertCheckResult> {
        let alerts = match self
            .list_active_alerts(user_id, Some(symbol.to_string()))
            .await
        {
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
    /// Split a fired alert's quantity into (exited, remaining) for partial close.
    ///
    /// 中文：
    ///   - **纯函数**：仅做数学，不写 DB
    ///   - 剩余仓位 < 1e-12 视为全平（防浮点尾数）
    ///   - 返回 (已平, 剩余) — caller 决定是否给剩余仓位建新 alert
    ///
    /// English:
    ///   - **Pure function**: no DB writes
    ///   - Residual qty < 1e-12 is treated as full close (epsilon guard)
    ///   - Returns `(exited, remaining)` — caller decides whether to create a new alert
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
    /// Build a JSON-safe response DTO from a SeaORM model.
    ///
    /// 中文：
    ///   - 用 `format!("{:.8}", p)` 字符串化 f64 避免 JSON 浮点尾数
    ///   - enum 字段用 serde 转 string 防止外部依赖 enum 字面量
    ///   - 默认空串兜底（不返回 null）
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
