//! db/dashboard.rs — Dashboard SeaORM Entities (Stats + PnL History)
//!
//! Contains: dashboard_stats, pnl_history

use sea_orm::entity::prelude::*;

// ─── Dashboard Stats Entity ─────────────────────────────────────

pub mod dashboard_stats {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "dashboard_stats")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id: i64,
        pub user_id: Uuid,
        pub total_users: i64,
        pub active_strategies: i64,
        pub total_orders_today: i64,
        pub total_pnl_today: f64,
        pub win_rate: f64,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::super::user::Entity",
            from = "Column::UserId",
            to = "super::super::user::Column::Id"
        )]
        User,
    }

    impl Related<super::super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ─── PnL History Entity ─────────────────────────────────────────

pub mod pnl_history {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "pnl_history")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = true)]
        pub id: i64,
        pub user_id: Uuid,
        pub timestamp: DateTimeUtc,
        pub pnl: f64,
        pub equity: f64,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::super::user::Entity",
            from = "Column::UserId",
            to = "super::super::user::Column::Id"
        )]
        User,
    }

    impl Related<super::super::user::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::User.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_dashboard_stats_model_fields() {
        let now = chrono::Utc::now();
        let model = dashboard_stats::Model {
            id: 1,
            user_id: Uuid::new_v4(),
            total_users: 100,
            active_strategies: 25,
            total_orders_today: 150,
            total_pnl_today: 1234.56,
            win_rate: 65.5,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(model.id, 1);
        assert_eq!(model.total_users, 100);
        assert!((model.total_pnl_today - 1234.56).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pnl_history_model_fields() {
        let now = chrono::Utc::now();
        let model = pnl_history::Model {
            id: 1,
            user_id: Uuid::new_v4(),
            timestamp: now,
            pnl: 500.25,
            equity: 100500.75,
            created_at: now,
        };
        assert_eq!(model.id, 1);
        assert!((model.pnl - 500.25).abs() < f64::EPSILON);
        assert!((model.equity - 100500.75).abs() < f64::EPSILON);
    }
}
