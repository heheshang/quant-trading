//! ADR-013 D2: risk_rules SeaORM Entity

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "risk_rules")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Uuid")]
    pub user_id: Uuid,
    pub daily_loss_limit: Decimal,
    pub daily_loss_auto_close: bool,
    pub single_trade_loss_ratio: Decimal,
    pub max_drawdown_ratio: Decimal,
    pub drawdown_auto_close: bool,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub stop_loss_type: String,
    pub atr_period: Option<i32>,
    pub atr_multiplier: Option<Decimal>,
    pub is_active: bool,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
