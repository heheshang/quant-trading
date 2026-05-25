//! ADR-015: atr_stop_loss SeaORM Entity

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "atr_stop_loss")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Uuid")]
    pub id: Uuid,
    pub position_id: Uuid,
    pub entry_price: Decimal,
    pub current_stop: Decimal,
    pub atr_value: Decimal,
    pub atr_period: i32,
    pub multiplier: Decimal,
    #[sea_orm(column_type = "String(StringLen::N(10))")]
    pub position_side: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
