//! SeaORM Entity for exchange_api_keys table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

pub mod exchange_api_keys {
    #![allow(clippy::module_inception)]

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "exchange_api_keys")]
    pub struct Model {
        #[sea_orm(primary_key, column_name = "id")]
        pub id: Uuid,

        #[sea_orm(column_name = "user_id")]
        pub user_id: Uuid,

        #[sea_orm(column_name = "exchange")]
        pub exchange: String,

        #[sea_orm(column_name = "api_key")]
        pub api_key: String,

        #[sea_orm(column_name = "secret_encrypted")]
        pub secret_encrypted: String,

        #[sea_orm(column_name = "nonce")]
        pub nonce: String,

        #[sea_orm(column_name = "permissions")]
        pub permissions: String,

        #[sea_orm(column_name = "is_active")]
        pub is_active: bool,

        #[sea_orm(column_name = "last_used_at")]
        pub last_used_at: Option<DateTimeUtc>,

        #[sea_orm(column_name = "created_at")]
        pub created_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}
