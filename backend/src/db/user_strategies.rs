use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_strategies")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    #[sea_orm(column_type = "Integer")]
    pub user_id: i32,
    #[sea_orm(column_type = "Integer")]
    pub strategy_id: i32,
    #[sea_orm(column_type = "JsonBinary")]
    pub config: Json,
    pub status: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_strategies_model_serialization() {
        // Test that Model serializes correctly to JSON
        let json = serde_json::json!({
            "id": 1,
            "user_id": 100,
            "strategy_id": 10,
            "config": {
                "param1": "value1",
                "param2": 42
            },
            "status": "active",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        });

        let model: Model = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(model.id, 1);
        assert_eq!(model.user_id, 100);
        assert_eq!(model.strategy_id, 10);
        assert_eq!(model.status, "active");

        // Test serialization back
        let serialized = serde_json::to_value(&model).unwrap();
        assert_eq!(serialized["id"], 1);
        assert_eq!(serialized["status"], "active");
    }

    #[test]
    fn test_user_strategies_status_values() {
        // Valid status values: active, paused, stopped
        for status in ["active", "paused", "stopped"] {
            let json = serde_json::json!({
                "id": 1,
                "user_id": 100,
                "strategy_id": 10,
                "config": {},
                "status": status,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            });

            let model: Model = serde_json::from_value(json).unwrap();
            assert_eq!(model.status, status);
        }
    }

    #[test]
    fn test_user_strategies_config_json() {
        let json = serde_json::json!({
            "id": 1,
            "user_id": 100,
            "strategy_id": 10,
            "config": {
                "symbol": "BTCUSDT",
                "timeframe": "1h",
                "risk_percent": 0.02,
                "stop_loss_percent": 0.01
            },
            "status": "active",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        });

        let model: Model = serde_json::from_value(json).unwrap();
        let config = serde_json::to_value(&model.config).unwrap();
        assert_eq!(config["symbol"], "BTCUSDT");
        assert_eq!(config["timeframe"], "1h");
        assert_eq!(config["risk_percent"], 0.02);
    }

    #[test]
    fn test_entity_primary_key() {
        let json = serde_json::json!({
            "id": 42,
            "user_id": 1,
            "strategy_id": 1,
            "config": {},
            "status": "paused",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        });

        let model: Model = serde_json::from_value(json).unwrap();
        assert_eq!(model.id, 42);
    }
}