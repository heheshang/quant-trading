use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "arbitrage_pairs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    #[sea_orm(column_type = "String(StringLen::N(32))")]
    pub pair_type: String,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub symbol_a: String,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub symbol_b: String,
    #[sea_orm(column_type = "String(StringLen::N(32))", default_value = "binance")]
    pub exchange: String,
    #[sea_orm(column_type = "String(StringLen::N(16))", default_value = "active")]
    pub status: String,
    #[sea_orm(column_type = "Decimal(None)")]
    pub spread_entry_threshold: Decimal,
    #[sea_orm(column_type = "Decimal(None)")]
    pub spread_exit_threshold: Decimal,
    #[sea_orm(column_type = "Decimal(None)")]
    pub max_position_size: Decimal,
    #[sea_orm(column_type = "String(StringLen::N(16))", default_value = "percentage")]
    pub calculation_mode: String,
    #[sea_orm(column_type = "Decimal(None)")]
    pub correlation_threshold: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(None)")]
    pub z_score_entry: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(None)")]
    pub z_score_exit: Option<Decimal>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
