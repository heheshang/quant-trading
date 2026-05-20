//! ADR-013 D2: risk_logs SeaORM Entity

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "risk_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Uuid")]
    pub id: Uuid,
    pub user_id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub rule_type: String,
    pub triggered_at: DateTimeUtc,
    pub position_value: Option<Decimal>,
    pub account_equity: Decimal,
    pub threshold: Decimal,
    pub actual_value: Decimal,
    #[sea_orm(column_type = "String(StringLen::N(20))")]
    pub action_taken: String,
    pub order_id: Option<Uuid>,
    pub notification_sent: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
