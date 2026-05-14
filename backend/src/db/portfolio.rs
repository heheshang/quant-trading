//! db/portfolio.rs — Portfolio Equity History SeaORM Entity
//!
//! ADR: ADR-009 §2.2 portfolio_equity_history table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ─── Portfolio Equity History Entity ────────────────────────────

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "portfolio_equity_history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub user_id: Uuid,
    pub equity: f64,
    pub timestamp: DateTimeUtc,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portfolio_equity_history_model_fields() {
        // Verify the model has expected fields via construction
        let model = Model {
            id: 1,
            user_id: Uuid::new_v4(),
            equity: 100000.0,
            timestamp: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
        };
        assert_eq!(model.id, 1);
        assert!((model.equity - 100000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_portfolio_equity_history_serialization() {
        let user_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let model = Model {
            id: 42,
            user_id,
            equity: 99999.5,
            timestamp: now,
            created_at: now,
        };
        let json = serde_json::to_value(&model).unwrap();
        assert_eq!(json["id"], 42);
        assert_eq!(json["equity"], 99999.5);
        assert_eq!(json["user_id"], user_id.to_string());
    }

    #[test]
    fn test_portfolio_equity_history_relation() {
        // Verify relation to user entity
        let rel = Relation::User;
        let _def = rel.def();
    }
}
