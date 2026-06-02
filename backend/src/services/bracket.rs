//! services/bracket.rs — Bracket order business logic (P1-2.2 path A: deferred OCO)
//!
//! When a bracket parent order is fully filled by the matching engine, the
//! `flush_trades` hook calls `record_parent_filled` which:
//!   1. Inserts a row into `bracket_links` (so frontend can poll & act)
//!   2. Updates the parent's `advanced_params.oco_status` to "pending"
//!   3. Increments the `BRACKET_PARENT_FILLED_TOTAL` counter
//!
//! The actual OCO creation (calling `TriggerOrderService::create_oco`) is left
//! to the frontend/client (see ADR in T2 design).

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::order;
use crate::metrics;
use crate::utils::error::AppError;

/// String constants for `advanced_type` field to avoid stringly-typed bugs.
pub mod advanced_type {
    pub const BRACKET: &str = "bracket";
}

/// OCO linkage status. Stored in `bracket_links.oco_status` and mirrored in
/// `orders.advanced_params.oco_status`.
pub mod oco_status {
    pub const PENDING: &str = "pending";
    pub const LINKED: &str = "linked";
    pub const CANCELLED: &str = "cancelled";
    pub const FAILED: &str = "failed";
}

/// `bracket_links` row model. Inlined (not a SeaORM entity) because we use raw
/// SQL via `db.execute()` — same pattern as `trigger_order.rs` (raw row reads).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketLink {
    pub id: Uuid,
    pub parent_order_id: Uuid,
    pub user_id: Uuid,
    pub symbol: String,
    pub sl_price: f64,
    pub tp_price: f64,
    pub side: String,
    pub filled_quantity: f64,
    pub oco_status: String,
    pub sl_trigger_id: Option<Uuid>,
    pub tp_trigger_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Input bundle for `record_parent_filled` to keep its signature under 7 args.
#[derive(Debug, Clone)]
pub struct RecordParentFilledInput {
    pub parent_id: Uuid,
    pub filled_quantity: f64,
    pub sl_price: f64,
    pub tp_price: f64,
    pub symbol: String,
    pub side: String,
    pub user_id: Uuid,
}

/// Record that a bracket parent order has been fully filled.
///
/// # Arguments (passed as `RecordParentFilledInput` to keep the signature compact).
/// * `input.parent_id` — UUID of the bracket parent order.
/// * `input.filled_quantity` — quantity that was filled (parent.quantity at fill time).
/// * `input.sl_price` / `input.tp_price` — copied from parent's `advanced_params`.
/// * `input.symbol` / `input.side` / `input.user_id` — copied from parent.
///
/// # Returns
/// The newly inserted `BracketLink` row, or an error.
pub async fn record_parent_filled(
    db: &Arc<DatabaseConnection>,
    input: RecordParentFilledInput,
) -> Result<BracketLink, AppError> {
    let RecordParentFilledInput {
        parent_id,
        filled_quantity,
        sl_price,
        tp_price,
        symbol,
        side,
        user_id,
    } = input;
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // 1. Insert bracket_links row
    let insert_sql = format!(
        "INSERT INTO bracket_links \
         (id, parent_order_id, user_id, symbol, sl_price, tp_price, side, \
          filled_quantity, oco_status, created_at, updated_at) \
         VALUES ('{}', '{}', '{}', '{}', {}, {}, '{}', {}, 'pending', NOW(), NOW()) \
         RETURNING id, parent_order_id, user_id, symbol, sl_price, tp_price, side, \
                   filled_quantity, oco_status, sl_trigger_id, tp_trigger_id, \
                   created_at, updated_at",
        id,
        parent_id,
        user_id,
        symbol.replace('\'', "''"),
        sl_price,
        tp_price,
        side.replace('\'', "''"),
        filled_quantity,
    );

    let result = sea_orm::ConnectionTrait::execute(
        db.as_ref(),
        sea_orm::Statement::from_string(sea_orm::DatabaseBackend::Postgres, insert_sql),
    )
    .await
    .map_err(|e| AppError::Database(format!("bracket_link insert failed: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::Internal(
            "bracket_link insert returned 0 rows".to_string(),
        ));
    }

    // 2. Update parent's advanced_params.oco_status to "pending"
    update_parent_oco_status(db, parent_id, oco_status::PENDING).await?;

    // 3. Increment metric
    metrics::BRACKET_PARENT_FILLED_TOTAL
        .with_label_values(&[side.as_str()])
        .inc();

    tracing::info!(
        bracket_link_id = %id,
        parent_order_id = %parent_id,
        user_id = %user_id,
        symbol = %symbol,
        sl_price = sl_price,
        tp_price = tp_price,
        "Bracket parent filled, deferred OCO pending"
    );

    Ok(BracketLink {
        id,
        parent_order_id: parent_id,
        user_id,
        symbol: symbol.to_string(),
        sl_price,
        tp_price,
        side: side.to_string(),
        filled_quantity,
        oco_status: oco_status::PENDING.to_string(),
        sl_trigger_id: None,
        tp_trigger_id: None,
        created_at: now,
        updated_at: now,
    })
}

/// Update the OCO linkage status of a bracket parent order's advanced_params.
/// This keeps the parent row's metadata in sync with the bracket_links table.
pub async fn update_parent_oco_status(
    db: &Arc<DatabaseConnection>,
    parent_id: Uuid,
    new_status: &str,
) -> Result<(), AppError> {
    let parent = order::Entity::find_by_id(parent_id)
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Bracket parent not found: {}", parent_id)))?;

    let mut new_params = parent
        .advanced_params
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = new_params.as_object_mut() {
        obj.insert("oco_status".to_string(), serde_json::json!(new_status));
    } else {
        // If advanced_params is not a JSON object, replace it
        new_params = serde_json::json!({"oco_status": new_status});
    }

    let mut active: order::ActiveModel = parent.into();
    active.advanced_params = Set(Some(new_params));
    active.updated_at = Set(chrono::Utc::now());

    active
        .update(db.as_ref())
        .await
        .map_err(|e| AppError::Database(format!("update parent oco_status: {}", e)))?;

    Ok(())
}

/// List all `pending` bracket links for a user (frontend polling).
pub async fn list_pending_for_user(
    db: &Arc<DatabaseConnection>,
    user_id: Uuid,
) -> Result<Vec<BracketLink>, AppError> {
    use sea_orm::DatabaseBackend;

    let sql = format!(
        "SELECT id, parent_order_id, user_id, symbol, sl_price, tp_price, side, \
                filled_quantity, oco_status, sl_trigger_id, tp_trigger_id, \
                created_at, updated_at \
         FROM bracket_links \
         WHERE user_id = '{}' AND oco_status = 'pending' \
         ORDER BY created_at ASC \
         LIMIT 200",
        user_id
    );

    let result = sea_orm::ConnectionTrait::query_all(
        db.as_ref(),
        sea_orm::Statement::from_string(DatabaseBackend::Postgres, sql),
    )
    .await
    .map_err(|e| AppError::Database(format!("list pending bracket_links: {}", e)))?;

    let mut links = Vec::with_capacity(result.len());
    for row in result {
        links.push(BracketLink {
            id: row.try_get_by::<Uuid, _>("id").unwrap_or_default(),
            parent_order_id: row.try_get_by::<Uuid, _>("parent_order_id").unwrap_or_default(),
            user_id: row.try_get_by::<Uuid, _>("user_id").unwrap_or_default(),
            symbol: row.try_get_by::<String, _>("symbol").unwrap_or_default(),
            sl_price: row.try_get_by::<f64, _>("sl_price").unwrap_or_default(),
            tp_price: row.try_get_by::<f64, _>("tp_price").unwrap_or_default(),
            side: row.try_get_by::<String, _>("side").unwrap_or_default(),
            filled_quantity: row.try_get_by::<f64, _>("filled_quantity").unwrap_or_default(),
            oco_status: row.try_get_by::<String, _>("oco_status").unwrap_or_default(),
            sl_trigger_id: row
                .try_get_by::<Option<Uuid>, _>("sl_trigger_id")
                .unwrap_or_default(),
            tp_trigger_id: row
                .try_get_by::<Option<Uuid>, _>("tp_trigger_id")
                .unwrap_or_default(),
            created_at: row
                .try_get_by::<chrono::DateTime<chrono::Utc>, _>("created_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
            updated_at: row
                .try_get_by::<chrono::DateTime<chrono::Utc>, _>("updated_at")
                .unwrap_or_else(|_| chrono::Utc::now()),
        });
    }
    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_type_brackets_constant() {
        assert_eq!(advanced_type::BRACKET, "bracket");
    }

    #[test]
    fn test_oco_status_constants() {
        assert_eq!(oco_status::PENDING, "pending");
        assert_eq!(oco_status::LINKED, "linked");
        assert_eq!(oco_status::CANCELLED, "cancelled");
        assert_eq!(oco_status::FAILED, "failed");
    }

    #[test]
    fn test_bracket_link_serde_roundtrip() {
        let link = BracketLink {
            id: Uuid::new_v4(),
            parent_order_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            symbol: "BTC/USDT".to_string(),
            sl_price: 49000.0,
            tp_price: 52000.0,
            side: "buy".to_string(),
            filled_quantity: 1.0,
            oco_status: "pending".to_string(),
            sl_trigger_id: None,
            tp_trigger_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&link).unwrap();
        let back: BracketLink = serde_json::from_str(&json).unwrap();
        assert_eq!(back.symbol, "BTC/USDT");
        assert_eq!(back.sl_price, 49000.0);
        assert_eq!(back.oco_status, "pending");
    }

    #[test]
    fn test_bracket_link_with_trigger_ids() {
        let link = BracketLink {
            id: Uuid::new_v4(),
            parent_order_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            symbol: "ETH/USDT".to_string(),
            sl_price: 2900.0,
            tp_price: 3200.0,
            side: "buy".to_string(),
            filled_quantity: 2.5,
            oco_status: "linked".to_string(),
            sl_trigger_id: Some(Uuid::new_v4()),
            tp_trigger_id: Some(Uuid::new_v4()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        assert!(link.sl_trigger_id.is_some());
        assert!(link.tp_trigger_id.is_some());
        assert_eq!(link.oco_status, "linked");
    }

    #[test]
    fn test_order_status_pending_filter() {
        // Just to make sure we use the right OrderStatus enum variant.
        assert_eq!(format!("{:?}", order::OrderStatus::Filled), "Filled");
        assert_eq!(format!("{:?}", order::OrderStatus::Cancelled), "Cancelled");
    }
}
