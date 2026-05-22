use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "arbitrage_positions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub pair_id: i32,
    #[sea_orm(column_type = "String(StringLen::N(16))")]
    pub direction: String,
    #[sea_orm(column_type = "Decimal(None)")]
    pub size_a: Decimal,
    #[sea_orm(column_type = "Decimal(None)")]
    pub size_b: Decimal,
    #[sea_orm(column_type = "Decimal(None)")]
    pub entry_spread: Decimal,
    #[sea_orm(column_type = "Decimal(None)")]
    pub current_spread: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(None)")]
    pub unrealized_pnl: Option<Decimal>,
    #[sea_orm(column_type = "String(StringLen::N(16))", default_value = "open")]
    pub status: String,
    pub opened_at: DateTimeUtc,
    pub closed_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
