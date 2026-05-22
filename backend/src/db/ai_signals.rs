use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_signals")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub user_id: Uuid,
    pub symbol: String,                 // e.g. "BTCUSDT"
    pub interval: String,               // e.g. "1h"
    pub direction: String,              // "long" | "short" | "neutral"
    pub probability: f64,               // 0.0 ~ 1.0
    pub confidence: f64,              // 0.0 ~ 1.0
    pub model_version: String,         // e.g. "v1.0"
    pub price_target: Option<f64>,
    #[sea_orm(column_type = "JsonBinary")]
    pub feature_snapshot: Json,         // snapshot of features used for this prediction
    pub is_fused: bool,                // whether this signal went through fusion
    pub fused_with_rule: bool,         // whether rule-based signal was available
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
