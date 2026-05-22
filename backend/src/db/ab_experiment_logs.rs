use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ab_experiment_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub user_id: Uuid,
    pub experiment_id: Uuid,    // groups logs by experiment
    pub model_version: String,  // which model version made this decision
    pub order_id: Option<Uuid>, // associated order (if any)
    pub symbol: String,
    pub signal_direction: String, // "long" | "short" | "neutral"
    pub signal_strength: f64,     // 0.0 ~ 1.0
    pub traffic_ratio: i32,       // % traffic assigned at time of decision
    pub decision_at: DateTimeUtc,
    pub order_result: Option<String>, // "filled" | "rejected" | "pending" | null
    pub pnl: Option<f64>,            // PnL if order completed
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
