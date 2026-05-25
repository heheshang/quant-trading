use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "model_versions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub status: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub metrics_json: Json,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
