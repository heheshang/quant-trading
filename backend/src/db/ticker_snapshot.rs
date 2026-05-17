//! TickerSnapshot Entity - SeaORM model for ticker_snapshots table
//!
//! Phase 4 P1: Ticker Historical Snapshots
//!
//! Stores periodic snapshots of ticker data for historical analysis.
//! Partitioned by timestamp month.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ticker_snapshots")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub symbol: String,
    #[sea_orm(column_name = "price")]
    pub price: Decimal,
    #[sea_orm(column_name = "change")]
    pub change: Decimal,
    #[sea_orm(column_name = "change_percent")]
    pub change_percent: Decimal,
    pub volume: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub bid: Decimal,
    pub ask: Decimal,
    /// Ticker snapshot timestamp (when the data was captured)
    pub timestamp: DateTimeUtc,
    /// When this record was inserted
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
