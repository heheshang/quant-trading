//! Kline Entity - SeaORM model for klines table (Historical Kline Persistence)
//!
//! Phase 4 T4: Historical Kline Persistence
//! Partitioned table by open_time month

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "klines_phase4")]
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
    /// Soft delete - if set, this kline has been logically deleted
    pub deleted_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Implement to_active_model for KlineWriter compatibility
impl Model {
    /// Convert to ActiveModel with deleted_at handling
    pub fn to_active_model(self) -> crate::models::kline_entity::ActiveModel {
        use sea_orm::Set;
        crate::models::kline_entity::ActiveModel {
            id: Set(self.id),
            symbol: Set(self.symbol),
            interval: Set(self.interval),
            open_time: Set(self.open_time),
            close_time: Set(self.close_time),
            open: Set(self.open),
            high: Set(self.high),
            low: Set(self.low),
            close: Set(self.close),
            volume: Set(self.volume),
            quote_volume: Set(self.quote_volume),
            trades: Set(self.trades),
            source: Set(self.source),
            created_at: Set(self.created_at),
            deleted_at: Set(self.deleted_at),
        }
    }
}
