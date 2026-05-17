//! Kline Entity - SeaORM model for klines table (Historical Kline Persistence)
//!
//! Phase 4 T4: Historical Kline Persistence
//! Partitioned table by open_time month

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "klines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub symbol: String,
    pub interval: String,
    pub open_time: DateTimeUtc,
    pub close_time: DateTimeUtc,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub quote_volume: Decimal,
    pub trades: i32,
    pub source: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}