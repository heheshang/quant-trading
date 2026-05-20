//! db/position_alerts.rs — PositionAlert SeaORM Entity
//!
//! PRD: P1-F2 实盘止盈止损
//! 存储用户的止盈/止损/追踪止损警戒触发规则

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ─── 枚举 ───────────────────────────────────────────────────────

/// 警戒类型：止盈 / 止损 / 追踪止损
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum AlertType {
    #[sea_orm(string_value = "take_profit")]
    TakeProfit,
    #[sea_orm(string_value = "stop_loss")]
    StopLoss,
    #[sea_orm(string_value = "trailing_stop")]
    TrailingStop,
}

/// 警戒状态：活跃 / 已触发 / 已取消 / 已暂停
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum AlertStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "triggered")]
    Triggered,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
    #[sea_orm(string_value = "paused")]
    Paused,
}

/// 触发方式：市价触发 / 限价触发
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum TriggerMode {
    #[sea_orm(string_value = "market")]
    Market,
    #[sea_orm(string_value = "limit")]
    Limit,
}

// ─── Entity Model ───────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "position_alerts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub position_id: Uuid,
    pub symbol: String,
    pub alert_type: AlertType,
    pub status: AlertStatus,
    pub trigger_price: f64,
    pub limit_price: Option<f64>,
    pub trigger_mode: TriggerMode,
    pub trailing_distance: Option<f64>,
    pub trailing_activated: bool,
    pub activated_price: Option<f64>,
    pub triggered_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub cancelled_at: Option<DateTimeUtc>,
    pub triggered_order_id: Option<Uuid>,
    pub note: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
