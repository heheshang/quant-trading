use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "backtest_results")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub strategy_id: Uuid,
    #[sea_orm(column_type = "JsonBinary")]
    pub config: Json,
    pub status: String,
    pub progress: i32,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub metrics: Option<Json>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub trades: Option<Json>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub equity_curve: Option<Json>,
    pub start_time: Option<DateTimeUtc>,
    pub end_time: Option<DateTimeUtc>,
    pub duration_ms: Option<i64>,
    pub error: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
