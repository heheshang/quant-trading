use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "model_versions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,    // e.g. "BTC Trend Predictor"
    pub version: String, // e.g. "v1.0"
    pub description: String,
    pub status: String,     // "staged" | "active" | "deprecated" | "deactivated"
    pub model_type: String, // "lstm" | "transformer" | "sentiment"
    #[sea_orm(column_type = "JsonBinary")]
    pub metrics_json: Json, // { "accuracy": 0.72, "sharpe": 1.5, ... }
    #[sea_orm(column_type = "JsonBinary")]
    pub config_json: Json, // { "lookback": 100, "features": [...], ... }
    pub traffic_ratio: i32, // 0-100, percentage of traffic this version receives
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
