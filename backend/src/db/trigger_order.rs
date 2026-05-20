//! db/trigger_order.rs — Trigger Order (条件触发单) SeaORM Entities
//!
//! P1-F3: 条件触发单 - 止损单/止盈单/OCO/TWAP
//! 支持：止损单、止盈单、OCO（One-Cancels-Other）、TWAP时间加权平均价格

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

// ─── 枚举定义 ───────────────────────────────────────────────────

/// 触发订单类型
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum TriggerType {
    /// 止损单 - 当价格触及触发价时触发
    #[sea_orm(string_value = "stop_loss")]
    StopLoss,
    /// 止盈单 - 当价格触及触发价时触发
    #[sea_orm(string_value = "take_profit")]
    TakeProfit,
    /// OCO单 (One-Cancels-Other) - 两个条件单，触发一个则取消另一个
    #[sea_orm(string_value = "oco")]
    Oco,
    /// TWAP (Time-Weighted Average Price) - 时间加权平均单
    #[sea_orm(string_value = "twap")]
    Twap,
}

impl fmt::Display for TriggerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriggerType::StopLoss => write!(f, "stop_loss"),
            TriggerType::TakeProfit => write!(f, "take_profit"),
            TriggerType::Oco => write!(f, "oco"),
            TriggerType::Twap => write!(f, "twap"),
        }
    }
}

impl Serialize for TriggerType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            TriggerType::StopLoss => "stop_loss",
            TriggerType::TakeProfit => "take_profit",
            TriggerType::Oco => "oco",
            TriggerType::Twap => "twap",
        })
    }
}

/// 触发订单状态
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum TriggerStatus {
    /// 待触发 - 等待价格条件满足
    #[sea_orm(string_value = "pending")]
    Pending,
    /// 已触发 - 条件已满足，委托已提交
    #[sea_orm(string_value = "triggered")]
    Triggered,
    /// 已取消 - 用户主动取消
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
    /// 已过期 - TWAP时间窗口结束或其他原因过期
    #[sea_orm(string_value = "expired")]
    Expired,
    /// 失败 - 触发后委托下单失败
    #[sea_orm(string_value = "failed")]
    Failed,
}

impl TriggerStatus {
    /// 是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TriggerStatus::Triggered
                | TriggerStatus::Cancelled
                | TriggerStatus::Expired
                | TriggerStatus::Failed
        )
    }
}

impl fmt::Display for TriggerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriggerStatus::Pending => write!(f, "pending"),
            TriggerStatus::Triggered => write!(f, "triggered"),
            TriggerStatus::Cancelled => write!(f, "cancelled"),
            TriggerStatus::Expired => write!(f, "expired"),
            TriggerStatus::Failed => write!(f, "failed"),
        }
    }
}

/// 触发条件方向 (用于判断价格比较逻辑)
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum TriggerDirection {
    /// 价格大于等于触发价时触发
    #[sea_orm(string_value = "up")]
    Up,
    /// 价格小于等于触发价时触发
    #[sea_orm(string_value = "down")]
    Down,
}

impl fmt::Display for TriggerDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TriggerDirection::Up => write!(f, "up"),
            TriggerDirection::Down => write!(f, "down"),
        }
    }
}

impl Serialize for TriggerDirection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            TriggerDirection::Up => "up",
            TriggerDirection::Down => "down",
        })
    }
}

/// TWAP 订单方向
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum TwapSide {
    #[sea_orm(string_value = "buy")]
    Buy,
    #[sea_orm(string_value = "sell")]
    Sell,
}

impl fmt::Display for TwapSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TwapSide::Buy => write!(f, "buy"),
            TwapSide::Sell => write!(f, "sell"),
        }
    }
}

// ─── TriggerOrders Entity ──────────────────────────────────────

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "trigger_orders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,

    /// 用户ID
    pub user_id: Uuid,

    /// 关联的持仓ID (可选，用于止损止盈)
    pub position_id: Option<Uuid>,

    /// 交易对
    pub symbol: String,

    /// 触发类型: stop_loss / take_profit / oco / twap
    pub trigger_type: TriggerType,

    /// 触发状态
    pub status: TriggerStatus,

    /// 触发方向: up / down
    pub trigger_direction: TriggerDirection,

    /// 触发价格 (止损止盈的触发价)
    pub trigger_price: f64,

    /// 触发价格上限 (OCO的止盈触发价)
    pub trigger_price_upper: Option<f64>,

    /// 触发价格下限 (OCO的止损触发价)
    pub trigger_price_lower: Option<f64>,

    /// 基础价格 (挂单价格，用于stop_loss/take_profit)
    pub base_price: Option<f64>,

    /// 订单方向: buy / sell (TWAP时需要)
    pub side: TwapSide,

    /// 委托数量 (TWAP总数量)
    pub quantity: f64,

    /// 已成交数量 (TWAP)
    pub filled_quantity: f64,

    /// 成交均价 (TWAP)
    pub avg_fill_price: Option<f64>,

    /// TWAP: 每次下单数量
    pub twap_slice_quantity: f64,

    /// TWAP: 间隔秒数
    pub twap_interval_secs: i32,

    /// TWAP: 开始时间
    pub twap_start_time: Option<DateTimeUtc>,

    /// TWAP: 结束时间
    pub twap_end_time: Option<DateTimeUtc>,

    /// TWAP: 已执行次数
    pub twap_executed_slices: i32,

    /// TWAP: 最大执行次数
    pub twap_max_slices: i32,

    /// OCO: 关联的另一个触发单ID
    pub oco_pair_id: Option<Uuid>,

    /// 关联的父订单ID (触发后创建的市价/限价单)
    pub triggered_order_id: Option<Uuid>,

    /// 触发原因
    pub trigger_reason: Option<String>,

    /// 触发时间
    pub triggered_at: Option<DateTimeUtc>,

    /// 到期时间
    pub expire_at: Option<DateTimeUtc>,

    /// 创建时间
    pub created_at: DateTimeUtc,

    /// 更新时间
    pub updated_at: DateTimeUtc,

    /// 取消时间
    pub cancelled_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigger_status_is_terminal() {
        assert!(!TriggerStatus::Pending.is_terminal());
        assert!(TriggerStatus::Triggered.is_terminal());
        assert!(TriggerStatus::Cancelled.is_terminal());
        assert!(TriggerStatus::Expired.is_terminal());
        assert!(TriggerStatus::Failed.is_terminal());
    }

    #[test]
    fn test_trigger_type_serialization() {
        let stop_loss = serde_json::to_value(&TriggerType::StopLoss).unwrap();
        assert_eq!(stop_loss, "stop_loss");

        let take_profit = serde_json::to_value(&TriggerType::TakeProfit).unwrap();
        assert_eq!(take_profit, "take_profit");

        let oco = serde_json::to_value(&TriggerType::Oco).unwrap();
        assert_eq!(oco, "oco");

        let twap = serde_json::to_value(&TriggerType::Twap).unwrap();
        assert_eq!(twap, "twap");
    }

    #[test]
    fn test_trigger_direction_serialization() {
        let up = serde_json::to_value(&TriggerDirection::Up).unwrap();
        assert_eq!(up, "up");

        let down = serde_json::to_value(&TriggerDirection::Down).unwrap();
        assert_eq!(down, "down");
    }
}
