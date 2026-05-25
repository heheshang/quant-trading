use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_signals")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub timestamp: DateTimeUtc,
    pub symbol: String,
    #[sea_orm(column_name = "timeframe")]
    pub timeframe: String,
    pub direction: Option<String>,
    pub probability: Option<f64>,
    pub sentiment_score: Option<f64>,
    pub final_score: Option<f64>,
    pub model_version: Option<String>,
    #[sea_orm(column_type = "JsonBinary")]
    pub feature_snapshot: Json,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
