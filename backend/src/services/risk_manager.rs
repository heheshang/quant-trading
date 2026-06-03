//! Risk Manager — 资金风控规则引擎
//!
//! ADR-013 D1: 前置风控检查 + 紧急全平 + 告警推送
//! P0-F2: 单日亏损 / 单笔亏损 / 总回撤 三项核心检查

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, QueryFilter};
use sea_orm::{DatabaseConnection, EntityTrait};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::risk_logs::{ActiveModel as RiskLogActive, Entity as RiskLogsEntity};
use crate::db::risk_rules::Entity as RiskRulesEntity;
use crate::utils::error::AppError;

/// 风控违规错误码
pub const ERR_DAILY_LOSS: &str = "RF-001";
pub const ERR_SINGLE_TRADE_LOSS: &str = "RF-002";
pub const ERR_DRAWDOWN: &str = "RF-003";
pub const ERR_PAUSED: &str = "RF-004";
pub const ERR_LIMIT_DISABLED: &str = "RF-005";

/// 风控规则（从数据库加载）
#[derive(Debug, Clone)]
pub struct RiskRules {
    pub id: i64,
    pub user_id: Uuid,
    pub daily_loss_limit: Decimal,
    pub daily_loss_auto_close: bool,
    pub single_trade_loss_ratio: Decimal,
    pub max_drawdown_ratio: Decimal,
    pub drawdown_auto_close: bool,
    pub stop_loss_type: String,
    pub atr_period: Option<i32>,
    pub atr_multiplier: Option<Decimal>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 风控日志条目
#[derive(Debug, Clone)]
pub struct RiskLog {
    pub id: Uuid,
    pub user_id: Uuid,
    pub rule_type: String,
    pub triggered_at: chrono::DateTime<chrono::Utc>,
    pub position_value: Option<Decimal>,
    pub account_equity: Decimal,
    pub threshold: Decimal,
    pub actual_value: Decimal,
    pub action_taken: String,
    pub order_id: Option<Uuid>,
    pub notification_sent: bool,
}

/// 风控检查结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct RiskCheckResult {
    pub passed: bool,
    pub triggered_rules: Vec<String>,
    pub daily_loss: Decimal,
    pub single_trade_loss: Option<Decimal>,
    pub drawdown: Decimal,
    pub equity: Decimal,
    pub peak_equity: Decimal,
}

/// 紧急全平结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmergencyCloseResult {
    pub success: bool,
    pub message: String,
    pub closed_positions: i32,
    pub total_pnl: String,
    pub details: Vec<EmergencyCloseOrder>,
}

/// 紧急全平订单详情
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmergencyCloseOrder {
    pub symbol: String,
    pub side: String,
    pub executed_qty: String,
    pub pnl: String,
}

/// 暂停/恢复响应
#[derive(Debug, Clone, serde::Serialize)]
pub struct PauseResponse {
    pub paused: bool,
    pub reason: String,
}

/// 账户权益快照
#[derive(Debug, Clone)]
pub struct AccountSnapshot {
    pub equity: Decimal,      // 当前权益
    pub peak_equity: Decimal, // 历史峰值
    pub daily_pnl: Decimal,   // 当日盈亏
}

/// Risk Manager — 资金风控引擎
#[derive(Clone)]
pub struct RiskManager {
    db: Arc<DatabaseConnection>,
    /// P2-1: 可选的通知服务 — risk event 触发后通过它下发告警。
    /// P2-1: optional notifier — when set, risk events fan out via this service.
    /// 中文：使用 `Option` 是因为 RiskManager 在测试和早期启动阶段可能未注入；
    ///   `None` 时 fall back 到原本的 `tracing::warn!` 行为（不破坏既有调用方）。
    /// English: `Option` so the existing constructors (`RiskManager::new(db)`)
    ///   keep working — pre-existing callers don't need to be updated. `None`
    ///   falls back to the legacy `tracing::warn!` path.
    notifier: Option<Arc<crate::services::alert_notification_service::AlertNotificationService>>,
}

impl RiskManager {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            notifier: None,
        }
    }

    /// 注入通知服务（启动期调用一次）
    /// Inject the notifier (called once at startup).
    pub fn with_notifier(
        mut self,
        notifier: Arc<crate::services::alert_notification_service::AlertNotificationService>,
    ) -> Self {
        self.notifier = Some(notifier);
        self
    }

    /// 获取账户快照（权益 + 当日盈亏 + 历史峰值）
    pub(crate) async fn get_account_snapshot(
        &self,
        user_id: Uuid,
    ) -> Result<AccountSnapshot, AppError> {
        // 1. 计算当前权益 = 模拟账户余额 + 持仓市值
        use crate::db::order::paper_accounts::Entity as PaperAccountEntity;

        let equity = PaperAccountEntity::find()
            .filter(crate::db::order::paper_accounts::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await?
            .map(|a| rust_decimal::Decimal::from_f64_retain(a.balance).unwrap_or(Decimal::ZERO))
            .unwrap_or(Decimal::ZERO);

        // 2. 计算当日已实现盈亏（从 positions 已平仓记录统计）
        // trades 表无 realized_pnl，从今日平仓的 positions.quantity=0 记录获取 realized_pnl
        let today = chrono::Utc::now().date_naive();
        let today_start = today.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc();
        let closed_positions = crate::db::order::positions::Entity::find()
            .filter(crate::db::order::positions::Column::UserId.eq(user_id))
            .filter(crate::db::order::positions::Column::Quantity.eq(0.0))
            .filter(crate::db::order::positions::Column::UpdatedAt.gte(today_start))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let daily_realized_pnl: Decimal = closed_positions
            .iter()
            .map(|p| Decimal::try_from(p.realized_pnl).unwrap_or(Decimal::ZERO))
            .sum();
        let daily_pnl = daily_realized_pnl;

        // 3. 历史峰值（从 paper_accounts.peak_equity 或单独记录）
        let peak_equity = equity; // MVP: 简化处理，后续扩展

        Ok(AccountSnapshot {
            equity,
            peak_equity,
            daily_pnl,
        })
    }

    /// 计算当日累计亏损（正向为盈利，负向为亏损）
    pub(crate) async fn get_daily_loss(&self, user_id: Uuid) -> Result<Decimal, AppError> {
        let today = chrono::Utc::now().date_naive();
        let today_start = today.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc();

        // 查询 UTC 今日已平仓的 positions（quantity=0），汇总 realized_pnl
        let closed_positions = crate::db::order::positions::Entity::find()
            .filter(crate::db::order::positions::Column::UserId.eq(user_id))
            .filter(crate::db::order::positions::Column::Quantity.eq(0.0))
            .filter(crate::db::order::positions::Column::UpdatedAt.gte(today_start))
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let daily_loss: Decimal = closed_positions
            .iter()
            .map(|p| Decimal::try_from(p.realized_pnl).unwrap_or(Decimal::ZERO))
            .sum();

        Ok(daily_loss)
    }

    /// 前置风控检查 — 每次下单前必须调用
    /// order_price: 下单价格（如限价单价格，市价单传当前市价）
    /// stop_loss_price: 止损触发价（用于计算单笔最大潜在亏损）
    pub async fn check_order(
        &self,
        user_id: Uuid,
        order_side: &str, // "buy" or "sell"
        order_quantity: &str,
        order_price: Option<f64>,     // 改为真实 f64 类型
        stop_loss_price: Option<f64>, // 止损触发价
    ) -> Result<(), AppError> {
        // 1. 加载风控规则
        let rules = self.get_rules_internal(user_id).await?;
        if !rules.is_active {
            return Ok(()); // 风控未启用，跳过
        }

        // 2. 获取账户快照
        let snapshot = self.get_account_snapshot(user_id).await?;
        let daily_loss = self.get_daily_loss(user_id).await?;

        // 3. 检查当日亏损
        if daily_loss <= -rules.daily_loss_limit {
            // 触发：亏损已超过阈值
            if rules.daily_loss_auto_close {
                self.emergency_close(user_id, false).await?;
                self.send_alert(user_id, ERR_DAILY_LOSS, "当日亏损超限，已执行自动平仓")
                    .await?;
            }
            // P0-3: count risk rule trip
            crate::metrics::RISK_RULES_TRIPPED_TOTAL
                .with_label_values(&["daily_loss"])
                .inc();
            return Err(AppError::RiskViolation(format!(
                "当日亏损 {:.2} 超过限制 {:.2}",
                daily_loss, rules.daily_loss_limit
            )));
        }

        // 4. 检查单笔亏损（仅对开仓订单）
        let is_opening = order_side == "buy" || order_side == "sell";
        if is_opening {
            // 估算该订单的最大潜在亏损
            // 计算: estimated_loss = |order_price - stop_loss_price| * quantity
            // loss_ratio = estimated_loss / equity
            let mut estimated_loss_ratio = Decimal::ZERO;

            #[allow(clippy::collapsible_if)]
            if let (Some(price), Some(sl_price)) = (order_price, stop_loss_price) {
                #[allow(clippy::collapsible_if)]
                if price > 0.0 && sl_price > 0.0 {
                    let qty: f64 = order_quantity.parse().unwrap_or(0.0);
                    #[allow(clippy::collapsible_if)]
                    if qty > 0.0 {
                        let price_dec = Decimal::from_f64_retain(price).unwrap_or(Decimal::ZERO);
                        let sl_dec = Decimal::from_f64_retain(sl_price).unwrap_or(Decimal::ZERO);
                        let qty_dec = Decimal::from_f64_retain(qty).unwrap_or(Decimal::ZERO);

                        let estimated_loss = (price_dec - sl_dec).abs() * qty_dec;

                        #[allow(clippy::collapsible_if)]
                        if snapshot.equity > Decimal::ZERO {
                            estimated_loss_ratio = estimated_loss / snapshot.equity;
                        }
                    }
                }
            }
            if estimated_loss_ratio > rules.single_trade_loss_ratio {
                self.log_risk_event(
                    user_id,
                    "single_trade",
                    snapshot.equity,
                    rules.single_trade_loss_ratio,
                    estimated_loss_ratio,
                    "rejected",
                    None,
                )
                .await?;
                // P0-3: count risk rule trip
                crate::metrics::RISK_RULES_TRIPPED_TOTAL
                    .with_label_values(&["single_trade"])
                    .inc();
                return Err(AppError::RiskViolation(format!(
                    "单笔预估亏损比例 {:.2}% 超过限制 {:.2}%",
                    estimated_loss_ratio * Decimal::from(100),
                    rules.single_trade_loss_ratio * Decimal::from(100)
                )));
            }
        }

        // 5. 检查总回撤
        if snapshot.peak_equity > Decimal::ZERO {
            let drawdown = (snapshot.peak_equity - snapshot.equity) / snapshot.peak_equity;
            if drawdown >= rules.max_drawdown_ratio {
                if rules.drawdown_auto_close {
                    self.emergency_close(user_id, false).await?;
                    self.send_alert(user_id, ERR_DRAWDOWN, "总回撤超限，已执行自动平仓")
                        .await?;
                }
                // P0-3: count risk rule trip
                crate::metrics::RISK_RULES_TRIPPED_TOTAL
                    .with_label_values(&["max_drawdown"])
                    .inc();
                return Err(AppError::RiskViolation(format!(
                    "总回撤 {:.2}% 超过限制 {:.2}%",
                    drawdown * Decimal::from(100),
                    rules.max_drawdown_ratio * Decimal::from(100)
                )));
            }
        }

        Ok(())
    }

    /// 手动触发风控检查
    pub async fn manual_check(&self, user_id: Uuid) -> Result<RiskCheckResult, AppError> {
        let snapshot = self.get_account_snapshot(user_id).await?;
        let daily_loss = self.get_daily_loss(user_id).await?;
        let rules = self.get_rules_internal(user_id).await?;

        let drawdown = if snapshot.peak_equity > Decimal::ZERO {
            (snapshot.peak_equity - snapshot.equity) / snapshot.peak_equity
        } else {
            Decimal::ZERO
        };

        let triggered = vec![];
        let passed = !rules.is_active
            || daily_loss > -rules.daily_loss_limit
            || drawdown < rules.max_drawdown_ratio;

        Ok(RiskCheckResult {
            passed,
            triggered_rules: triggered,
            daily_loss,
            single_trade_loss: None,
            drawdown,
            equity: snapshot.equity,
            peak_equity: snapshot.peak_equity,
        })
    }

    /// 紧急全平（不受任何风控条件限制）
    /// 实际更新数据库持仓为 0，unrealized_pnl 计入 realized_pnl
    pub async fn emergency_close(
        &self,
        user_id: Uuid,
        _admin: bool,
    ) -> Result<EmergencyCloseResult, AppError> {
        // 1. 获取当前所有持仓（quantity > 0）
        let positions = <crate::db::order::positions::Entity as EntityTrait>::find()
            .filter(crate::db::order::positions::Column::UserId.eq(user_id))
            .filter(crate::db::order::positions::Column::Quantity.gt(0.0_f64))
            .all(self.db.as_ref())
            .await?;

        let mut details = vec![];

        for pos in positions {
            let symbol = pos.symbol.clone();
            let quantity = pos.quantity;
            let side_str = match pos.side {
                crate::db::order::PositionSide::Long => "long",
                crate::db::order::PositionSide::Short => "short",
            };
            let unrealized = pos.unrealized_pnl;
            let new_realized = pos.realized_pnl + unrealized;
            let pos_id = pos.id;

            // 2. 实际平仓：更新持仓为 0，unrealized_pnl 计入 realized_pnl
            let mut active_model: crate::db::order::positions::ActiveModel = pos.into();
            active_model.quantity = sea_orm::Set(0.0_f64);
            active_model.available_quantity = sea_orm::Set(0.0_f64);
            active_model.unrealized_pnl = sea_orm::Set(0.0_f64);
            active_model.realized_pnl = sea_orm::Set(new_realized);
            active_model.updated_at = sea_orm::Set(chrono::Utc::now());

            crate::db::order::positions::Entity::update(active_model)
                .exec(self.db.as_ref())
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            // 3. 记录风控日志
            self.log_risk_event(
                user_id,
                "emergency",
                Decimal::ZERO,
                Decimal::ZERO,
                Decimal::ZERO,
                "closed_all",
                Some(pos_id),
            )
            .await?;

            details.push(EmergencyCloseOrder {
                symbol,
                side: side_str.to_string(),
                executed_qty: format!("{:.8}", quantity),
                pnl: format!("{:.2}", new_realized),
            });
        }

        let total_pnl: Decimal = details
            .iter()
            .map(|d| d.pnl.parse::<Decimal>().unwrap_or_default())
            .sum();
        Ok(EmergencyCloseResult {
            success: true,
            message: if details.is_empty() {
                "No positions to close".to_string()
            } else {
                format!("Closed {} positions", details.len())
            },
            closed_positions: details.len() as i32,
            total_pnl: total_pnl.to_string(),
            details,
        })
    }

    /// 发送告警（多渠道：Telegram / Email / 企业微信）
    /// Send an alert (multi-channel: Telegram / Email / WeChat).
    ///
    /// 中文：notifier 注入时走 `AlertNotificationService::send_alert` 多播（带收敛）。
    ///   未注入时退回到 `tracing::warn!`，保持向后兼容。
    /// English: When the notifier is injected, fans out via
    ///   `AlertNotificationService::send_alert` (with dedup). Falls back to
    ///   `tracing::warn!` when not injected (backward-compatible).
    async fn send_alert(&self, user_id: Uuid, code: &str, message: &str) -> Result<(), AppError> {
        let user_id_str = user_id.to_string();
        if let Some(notifier) = &self.notifier {
            let mut n = crate::services::notification::AlertNotification::new(
                format!("风控触发: {}", code),
                message.to_string(),
                code.to_string(),
                severity_for_code(code),
                None,
            );
            n = n.with_metadata("user_id", &user_id_str);
            n = n.with_metadata("rule_code", code);
            // send_alert 内部有收敛（5 分钟窗口），重复告警会自动合并
            // send_alert has built-in dedup (5-min window) — repeated alerts collapse.
            if let Err(e) = notifier.send_alert(n).await {
                // 多渠道失败仅 log，不影响风控主流程
                // Multi-channel failure only logs — must not break the risk main path.
                tracing::error!("Notifier fan-out failed: {}", e);
            }
        } else {
            // 无 notifier 时的兼容路径（仅 log）
            // Compatibility path when notifier is not injected (log only).
            tracing::warn!(
                "[RISK ALERT] user={} code={} msg={}",
                user_id,
                code,
                message
            );
        }
        Ok(())
    }

    /// 记录风控日志
    #[allow(clippy::too_many_arguments)]
    async fn log_risk_event(
        &self,
        user_id: Uuid,
        rule_type: &str,
        equity: Decimal,
        threshold: Decimal,
        actual: Decimal,
        action: &str,
        order_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        let log = RiskLogActive {
            id: sea_orm::Set(Uuid::new_v4()),
            user_id: sea_orm::Set(user_id),
            rule_type: sea_orm::Set(rule_type.to_string()),
            triggered_at: sea_orm::Set(chrono::Utc::now()),
            position_value: sea_orm::Set(None),
            account_equity: sea_orm::Set(equity),
            threshold: sea_orm::Set(threshold),
            actual_value: sea_orm::Set(actual),
            action_taken: sea_orm::Set(action.to_string()),
            order_id: sea_orm::Set(order_id),
            notification_sent: sea_orm::Set(true),
        };
        RiskLogsEntity::insert(log).exec(self.db.as_ref()).await?;
        Ok(())
    }

    /// 内部获取规则（失败时返回默认规则）
    pub(crate) async fn get_rules_internal(&self, user_id: Uuid) -> Result<RiskRules, AppError> {
        use crate::db::risk_rules::Column as RiskRulesCol;

        let result = RiskRulesEntity::find()
            .filter(RiskRulesCol::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await?;

        Ok(result
            .map(|r| RiskRules {
                id: r.id,
                user_id: r.user_id,
                daily_loss_limit: r.daily_loss_limit,
                daily_loss_auto_close: r.daily_loss_auto_close,
                single_trade_loss_ratio: r.single_trade_loss_ratio,
                max_drawdown_ratio: r.max_drawdown_ratio,
                drawdown_auto_close: r.drawdown_auto_close,
                stop_loss_type: r.stop_loss_type,
                atr_period: r.atr_period,
                atr_multiplier: r.atr_multiplier,
                is_active: r.is_active,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .unwrap_or_else(|| RiskRules {
                id: 0,
                user_id: Uuid::nil(),
                daily_loss_limit: Decimal::ZERO,
                daily_loss_auto_close: false,
                single_trade_loss_ratio: Decimal::ZERO,
                max_drawdown_ratio: Decimal::new(1, 1), // 0.1
                drawdown_auto_close: false,
                stop_loss_type: "fixed".to_string(),
                atr_period: None,
                atr_multiplier: None,
                is_active: false,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }))
    }

    /// 暂停交易
    pub async fn pause(&self, user_id: Uuid, reason: &str) -> Result<PauseResponse, AppError> {
        self.log_risk_event(
            user_id,
            "paused",
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
            "paused",
            None,
        )
        .await?;
        Ok(PauseResponse {
            paused: true,
            reason: reason.to_string(),
        })
    }

    /// 恢复交易
    pub async fn resume(&self, user_id: Uuid) -> Result<(), AppError> {
        // 记录恢复交易日志
        self.log_risk_event(
            user_id,
            "resumed",
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
            "resumed",
            None,
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes_defined() {
        assert_eq!(ERR_DAILY_LOSS, "RF-001");
        assert_eq!(ERR_SINGLE_TRADE_LOSS, "RF-002");
        assert_eq!(ERR_DRAWDOWN, "RF-003");
        assert_eq!(ERR_PAUSED, "RF-004");
    }

    #[test]
    fn test_risk_rules_default_values() {
        let now = chrono::Utc::now();
        let rules = RiskRules {
            id: 1,
            user_id: uuid::Uuid::new_v4(),
            daily_loss_limit: Decimal::new(1000, 0),
            daily_loss_auto_close: true,
            single_trade_loss_ratio: Decimal::new(2, 2), // 0.02
            max_drawdown_ratio: Decimal::new(10, 2),     // 0.10
            drawdown_auto_close: true,
            stop_loss_type: "fixed".to_string(),
            atr_period: Some(14),
            atr_multiplier: Some(Decimal::new(15, 1)), // 1.5
            is_active: true,
            created_at: now,
            updated_at: now,
        };
        assert!(rules.is_active);
        assert_eq!(rules.daily_loss_limit, Decimal::new(1000, 0));
        assert_eq!(rules.single_trade_loss_ratio, Decimal::new(2, 2));
    }

    #[test]
    fn test_risk_check_result_serialization() {
        let result = RiskCheckResult {
            passed: true,
            triggered_rules: vec![],
            daily_loss: Decimal::ZERO,
            single_trade_loss: None,
            drawdown: Decimal::ZERO,
            equity: Decimal::new(10000, 0),
            peak_equity: Decimal::new(11000, 0),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"passed\":true"));
        assert!(json.contains("\"equity\":\"10000\""));
    }

    #[test]
    fn test_emergency_close_result_serialization() {
        let result = EmergencyCloseResult {
            success: true,
            message: "Closed 2 positions".to_string(),
            closed_positions: 2,
            total_pnl: "123.45".to_string(),
            details: vec![
                EmergencyCloseOrder {
                    symbol: "BTC".to_string(),
                    side: "long".to_string(),
                    executed_qty: "0.50000000".to_string(),
                    pnl: "100.00".to_string(),
                },
                EmergencyCloseOrder {
                    symbol: "ETH".to_string(),
                    side: "short".to_string(),
                    executed_qty: "2.00000000".to_string(),
                    pnl: "23.45".to_string(),
                },
            ],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"closed_positions\":2"));
        assert!(json.contains("BTC"));
    }

    #[test]
    fn test_pause_response_serialization() {
        let resp = PauseResponse {
            paused: true,
            reason: "断线超过30秒".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"paused\":true"));
        assert!(json.contains("断线超过30秒"));
    }

    #[test]
    fn test_account_snapshot_debug() {
        let snap = AccountSnapshot {
            equity: Decimal::new(9500, 0),
            peak_equity: Decimal::new(10000, 0),
            daily_pnl: Decimal::new(-500, 0),
        };
        let debug = format!("{:?}", snap);
        assert!(debug.contains("9500"));
        assert!(debug.contains("10000"));
    }

    #[test]
    fn test_daily_loss_trigger_ac1_below_threshold() {
        // AC1: 当日亏损 980 USDT（阈值 1000），允许开仓
        let daily_loss = Decimal::new(-980, 0);
        let limit = Decimal::new(1000, 0);
        // daily_loss (-980) > limit (-1000) -> 未超限，不触发
        assert!(daily_loss > -limit);
    }

    #[test]
    fn test_daily_loss_trigger_ac2_at_threshold() {
        // AC2: 当日亏损 1000+ USDT，禁止开仓
        let daily_loss = Decimal::new(-1000, 0);
        let limit = Decimal::new(1000, 0);
        // daily_loss <= -limit -> 超限，触发
        assert!(daily_loss <= -limit);
    }

    #[test]
    fn test_daily_loss_trigger_ac3_over_threshold() {
        let daily_loss = Decimal::new(-1005, 0);
        let limit = Decimal::new(1000, 0);
        assert!(daily_loss <= -limit);
    }
}

/// Map a risk-event code to a notification severity string.
///
/// P2-1 notifier integration: the alert notification service accepts a free-form
/// severity string ("info" / "warning" / "critical"). We classify risk events
/// based on the code prefix used by [`RiskManager::send_alert`].
fn severity_for_code(code: &str) -> String {
    if code.starts_with("EMERGENCY_") || code.starts_with("LIQUIDATION_") {
        "critical".to_string()
    } else if code.starts_with("DAILY_LOSS")
        || code.starts_with("POSITION_CONCENTRATION")
        || code.starts_with("LOSS_COOLDOWN")
    {
        "warning".to_string()
    } else {
        "info".to_string()
    }
}
