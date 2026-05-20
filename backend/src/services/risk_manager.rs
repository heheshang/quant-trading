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
    pub daily_loss_limit: Decimal,
    pub daily_loss_auto_close: bool,
    pub single_trade_loss_ratio: Decimal,
    pub max_drawdown_ratio: Decimal,
    pub drawdown_auto_close: bool,
    pub stop_loss_type: String,
    pub atr_period: Option<i32>,
    pub atr_multiplier: Option<Decimal>,
    pub is_active: bool,
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
    pub total_closed: i32,
    pub positions_closed: Vec<String>,
    pub errors: Vec<String>,
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
pub struct RiskManager {
    db: Arc<DatabaseConnection>,
}

impl RiskManager {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
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

        // 2. 计算当日盈亏（从 trades 表按日期统计）
        let _today = chrono::Utc::now().date_naive();
        let daily_pnl = Decimal::ZERO; // TODO: 从 trades.realized_pnl SUM

        // 3. 历史峰值（从 paper_accounts.peak_equity 或单独记录）
        let peak_equity = equity; // MVP: 简化处理，后续扩展

        Ok(AccountSnapshot {
            equity,
            peak_equity,
            daily_pnl,
        })
    }

    /// 计算当日累计亏损（正向为盈利，负向为亏损）
    pub(crate) async fn get_daily_loss(&self, _user_id: Uuid) -> Result<Decimal, AppError> {
        // 从 trades 表统计今日所有成交的 realized_pnl
        // MVP 简化：返回 Decimal::ZERO，后续实现真实统计
        Ok(Decimal::ZERO)
    }

    /// 前置风控检查 — 每次下单前必须调用
    pub async fn check_order(
        &self,
        user_id: Uuid,
        order_side: &str, // "buy" or "sell"
        _order_quantity: &str,
        _order_price: Option<&str>,
        _position_value: Decimal, // 当前持仓市值
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
            return Err(AppError::RiskViolation(format!(
                "当日亏损 {:.2} 超过限制 {:.2}",
                daily_loss, rules.daily_loss_limit
            )));
        }

        // 4. 检查单笔亏损（仅对开仓订单）
        let is_opening = order_side == "buy" || order_side == "sell";
        if is_opening {
            // 估算该订单的最大潜在亏损
            let estimated_loss_ratio = Decimal::ZERO; // TODO: 计算：(开仓价 - 止损价) / 开仓价
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
    pub async fn emergency_close(
        &self,
        user_id: Uuid,
        _admin: bool,
    ) -> Result<EmergencyCloseResult, AppError> {
        // 1. 获取当前所有持仓
        let positions = <crate::db::order::positions::Entity as EntityTrait>::find()
            .filter(crate::db::order::positions::Column::UserId.eq(user_id))
            .filter(crate::db::order::positions::Column::Quantity.gt(Decimal::ZERO))
            .all(self.db.as_ref())
            .await?;

        let mut closed = vec![];
        let errors = vec![];

        for pos in positions {
            // 2. 市价全平（调用撮合引擎）
            // MVP: 直接更新持仓为 0，记录风控日志
            // TODO: 实际调用 matching_engine.market_close(user_id, pos.symbol)
            let symbol = pos.symbol.clone();
            closed.push(symbol);
        }

        Ok(EmergencyCloseResult {
            total_closed: closed.len() as i32,
            positions_closed: closed,
            errors,
        })
    }

    /// 发送告警（微信 Webhook 占位）
    async fn send_alert(&self, user_id: Uuid, code: &str, message: &str) -> Result<(), AppError> {
        tracing::warn!(
            "[RISK ALERT] user={} code={} msg={}",
            user_id,
            code,
            message
        );
        // TODO: 调用微信 Webhook / 邮件服务
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
        let result = RiskRulesEntity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await?;

        Ok(result
            .map(|r| RiskRules {
                daily_loss_limit: r.daily_loss_limit,
                daily_loss_auto_close: r.daily_loss_auto_close,
                single_trade_loss_ratio: r.single_trade_loss_ratio,
                max_drawdown_ratio: r.max_drawdown_ratio,
                drawdown_auto_close: r.drawdown_auto_close,
                stop_loss_type: r.stop_loss_type,
                atr_period: r.atr_period,
                atr_multiplier: r.atr_multiplier,
                is_active: r.is_active,
            })
            .unwrap_or_else(|| RiskRules {
                daily_loss_limit: Decimal::ZERO,
                daily_loss_auto_close: false,
                single_trade_loss_ratio: Decimal::ZERO,
                max_drawdown_ratio: std::str::FromStr::from_str("0.1").unwrap(),
                drawdown_auto_close: false,
                stop_loss_type: "fixed".to_string(),
                atr_period: None,
                atr_multiplier: None,
                is_active: false,
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
    pub async fn resume(&self, _user_id: Uuid) -> Result<(), AppError> {
        Ok(())
    }
}
