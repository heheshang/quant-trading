use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ab_experiment_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub experiment_id: String,
    pub model_version: String,
    pub order_id: Option<i64>,
    pub signal_strength: Option<f64>,
    pub ai_probability: Option<f64>,
    pub sentiment_score: Option<f64>,
    pub final_decision: Option<String>,
    pub decision_at: DateTimeUtc,
    #[sea_orm(column_type = "JsonBinary")]
    pub metadata: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
