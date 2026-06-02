//! db/order.rs — Order/Trade/Position/PaperAccount/SymbolConfig SeaORM Entities
//!
//! PRD: §5.1 orders 表, §5.3 SeaORM Entity 模型
//! ADR: ADR-TRADING-EXECUTION D2 (状态机), D3 (保证金), D7 (加权平均均价)

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ─── 枚举 ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(4))")]
pub enum OrderSide {
    #[sea_orm(string_value = "buy")]
    Buy,
    #[sea_orm(string_value = "sell")]
    Sell,
}

impl Serialize for OrderSide {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            OrderSide::Buy => "buy",
            OrderSide::Sell => "sell",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum OrderType {
    #[sea_orm(string_value = "limit")]
    Limit,
    #[sea_orm(string_value = "market")]
    Market,
    // P1-2: advanced order types
    #[sea_orm(string_value = "iceberg")]
    Iceberg,
    #[sea_orm(string_value = "bracket")]
    Bracket,
    #[sea_orm(string_value = "trailing_stop")]
    TrailingStop,
}

impl Serialize for OrderType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            OrderType::Limit => "limit",
            OrderType::Market => "market",
            OrderType::Iceberg => "iceberg",
            OrderType::Bracket => "bracket",
            OrderType::TrailingStop => "trailing_stop",
        })
    }
}

/// 订单状态机 (ADR D2)
///
/// pending → partial_filled → filled
/// pending → cancelled
/// pending → expired
/// partial_filled → filled
/// partial_filled → cancelled (保留已成交部分)
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum OrderStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "partial_filled")]
    PartialFilled,
    #[sea_orm(string_value = "filled")]
    Filled,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
    #[sea_orm(string_value = "expired")]
    Expired,
    #[sea_orm(string_value = "rejected")]
    Rejected,
}

impl Serialize for OrderStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            OrderStatus::Pending => "pending",
            OrderStatus::PartialFilled => "partial_filled",
            OrderStatus::Filled => "filled",
            OrderStatus::Cancelled => "cancelled",
            OrderStatus::Expired => "expired",
            OrderStatus::Rejected => "rejected",
        };
        serializer.serialize_str(s)
    }
}

impl OrderStatus {
    /// 是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            OrderStatus::Filled
                | OrderStatus::Cancelled
                | OrderStatus::Expired
                | OrderStatus::Rejected
        )
    }

    /// 允许的状态转换
    pub fn can_transition_to(&self, target: &OrderStatus) -> bool {
        match (self, target) {
            // pending → partial_filled, filled, cancelled, expired
            (OrderStatus::Pending, OrderStatus::PartialFilled) => true,
            (OrderStatus::Pending, OrderStatus::Filled) => true,
            (OrderStatus::Pending, OrderStatus::Cancelled) => true,
            (OrderStatus::Pending, OrderStatus::Expired) => true,

            // partial_filled → partial_filled (更多成交), filled, cancelled
            (OrderStatus::PartialFilled, OrderStatus::PartialFilled) => true,
            (OrderStatus::PartialFilled, OrderStatus::Filled) => true,
            (OrderStatus::PartialFilled, OrderStatus::Cancelled) => true,

            // 终态不可转换
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum TradeMode {
    #[sea_orm(string_value = "paper")]
    Paper,
    #[sea_orm(string_value = "live")]
    Live,
}

impl Serialize for TradeMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            TradeMode::Paper => "paper",
            TradeMode::Live => "live",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(10))")]
pub enum TimeInForce {
    #[sea_orm(string_value = "GTC")]
    GTC,
    #[sea_orm(string_value = "IOC")]
    IOC,
    #[sea_orm(string_value = "FOK")]
    FOK,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(5))")]
pub enum PositionSide {
    #[sea_orm(string_value = "long")]
    Long,
    #[sea_orm(string_value = "short")]
    Short,
}

impl Serialize for PositionSide {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            PositionSide::Long => "long",
            PositionSide::Short => "short",
        })
    }
}

// ─── Orders Entity ──────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "orders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub strategy_id: Option<Uuid>,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<f64>,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub avg_fill_price: Option<f64>,
    pub status: OrderStatus,
    pub mode: TradeMode,
    pub fee: f64,
    pub reject_reason: Option<String>,
    pub time_in_force: TimeInForce,
    pub expire_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub cancelled_at: Option<DateTimeUtc>,
    pub filled_at: Option<DateTimeUtc>,
    // P1-2: advanced order type parameters
    pub advanced_type: Option<String>,
    pub advanced_params: Option<Json>,
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

// ─── Trades Entity ──────────────────────────────────────────────

pub mod trades {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "trades")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub order_id: Uuid,
        pub user_id: Uuid,
        pub symbol: String,
        pub side: OrderSide,
        pub price: f64,
        pub quantity: f64,
        pub fee: f64,
        pub is_maker: bool,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::super::user::Entity",
            from = "Column::UserId",
            to = "super::super::user::Column::Id"
        )]
        User,
        #[sea_orm(
            belongs_to = "super::Entity",
            from = "Column::OrderId",
            to = "super::Column::Id"
        )]
        Order,
    }

    impl Related<super::super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl Related<super::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Order.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ─── Positions Entity ───────────────────────────────────────────

pub mod positions {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "positions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub user_id: Uuid,
        pub symbol: String,
        pub side: PositionSide,
        pub quantity: f64,
        pub available_quantity: f64,
        pub avg_entry_price: f64,
        pub unrealized_pnl: f64,
        pub realized_pnl: f64,
        pub mode: TradeMode,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::super::user::Entity",
            from = "Column::UserId",
            to = "super::super::user::Column::Id"
        )]
        User,
    }

    impl Related<super::super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ─── PaperAccounts Entity ───────────────────────────────────────

pub mod paper_accounts {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "paper_accounts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub user_id: Uuid,
        pub balance: f64,
        pub frozen_balance: f64,
        pub initial_balance: f64,
        pub total_pnl: f64,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::super::user::Entity",
            from = "Column::UserId",
            to = "super::super::user::Column::Id"
        )]
        User,
    }

    impl Related<super::super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ─── SymbolConfigs Entity ───────────────────────────────────────

pub mod symbol_configs {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "symbol_configs")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub symbol: String,
        pub base_currency: String,
        pub quote_currency: String,
        pub price_precision: i16,
        pub quantity_precision: i16,
        pub min_quantity: f64,
        pub max_quantity: f64,
        pub min_notional: f64,
        pub fee_rate: f64,
        pub enabled: bool,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_status_is_terminal() {
        assert!(!OrderStatus::Pending.is_terminal());
        assert!(!OrderStatus::PartialFilled.is_terminal());
        assert!(OrderStatus::Filled.is_terminal());
        assert!(OrderStatus::Cancelled.is_terminal());
        assert!(OrderStatus::Expired.is_terminal());
        assert!(OrderStatus::Rejected.is_terminal());
    }

    #[test]
    fn test_order_status_can_transition() {
        // pending can transition to partial_filled, filled, cancelled, expired
        assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::PartialFilled));
        assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Filled));
        assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Cancelled));
        assert!(OrderStatus::Pending.can_transition_to(&OrderStatus::Expired));
        assert!(!OrderStatus::Pending.can_transition_to(&OrderStatus::Rejected));

        // partial_filled can transition to partial_filled, filled, cancelled
        assert!(OrderStatus::PartialFilled.can_transition_to(&OrderStatus::PartialFilled));
        assert!(OrderStatus::PartialFilled.can_transition_to(&OrderStatus::Filled));
        assert!(OrderStatus::PartialFilled.can_transition_to(&OrderStatus::Cancelled));
        assert!(!OrderStatus::PartialFilled.can_transition_to(&OrderStatus::Expired));

        // terminal states cannot transition
        assert!(!OrderStatus::Filled.can_transition_to(&OrderStatus::Pending));
        assert!(!OrderStatus::Cancelled.can_transition_to(&OrderStatus::Pending));
        assert!(!OrderStatus::Expired.can_transition_to(&OrderStatus::Filled));
        assert!(!OrderStatus::Rejected.can_transition_to(&OrderStatus::Pending));
    }

    #[test]
    fn test_order_side_serialization() {
        let buy = serde_json::to_value(&OrderSide::Buy).unwrap();
        assert_eq!(buy, "buy");
        let sell = serde_json::to_value(&OrderSide::Sell).unwrap();
        assert_eq!(sell, "sell");
    }

    #[test]
    fn test_order_type_serialization() {
        let limit = serde_json::to_value(&OrderType::Limit).unwrap();
        assert_eq!(limit, "limit");
        let market = serde_json::to_value(&OrderType::Market).unwrap();
        assert_eq!(market, "market");
    }

    #[test]
    fn test_trade_mode_serialization() {
        let paper = serde_json::to_value(&TradeMode::Paper).unwrap();
        assert_eq!(paper, "paper");
        let live = serde_json::to_value(&TradeMode::Live).unwrap();
        assert_eq!(live, "live");
    }

    #[test]
    fn test_time_in_force_serialization() {
        let gtc = serde_json::to_value(&TimeInForce::GTC).unwrap();
        assert_eq!(gtc, "GTC");
        let ioc = serde_json::to_value(&TimeInForce::IOC).unwrap();
        assert_eq!(ioc, "IOC");
        let fok = serde_json::to_value(&TimeInForce::FOK).unwrap();
        assert_eq!(fok, "FOK");
    }

    #[test]
    fn test_position_side_serialization() {
        let long = serde_json::to_value(&PositionSide::Long).unwrap();
        assert_eq!(long, "long");
        let short = serde_json::to_value(&PositionSide::Short).unwrap();
        assert_eq!(short, "short");
    }
}
