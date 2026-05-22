use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "arbitrage_signals")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    pub pair_id: i32,
    #[sea_orm(column_type = "String(StringLen::N(16))")]
    pub signal_type: String,
    #[sea_orm(column_type = "Decimal(None)")]
    pub spread: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(None)")]
    pub z_score: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(None)")]
    pub confidence: Option<Decimal>,
    #[sea_orm(default_value = "false")]
    pub executed: bool,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
