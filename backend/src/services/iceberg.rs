//! services/iceberg.rs — Iceberg order business logic
//!
//! P1-2.1: Splits iceberg parents into child limit slices, replenishes the
//! order book on each child fill, and cascades parent cancel to all children.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::db::order::{self, OrderStatus, OrderType};
use crate::metrics;
use crate::models::iceberg_params::{IcebergChildParams, IcebergParams};
use crate::services::matching_engine::MatchingEngine;
use crate::utils::error::AppError;

/// String constants for `advanced_type` field to avoid stringly-typed bugs.
pub mod advanced_type {
    pub const ICEBERG: &str = "iceberg";
    pub const ICEBERG_CHILD: &str = "iceberg_child";
}

/// P1-2.1: Split an iceberg parent into N child limit orders and insert the
/// first child into the matching engine order book.
///
/// Pre-conditions (caller must ensure):
/// - `parent` has `order_type = Iceberg` and `advanced_type = advanced_type::ICEBERG`
/// - `parent.advanced_params` deserializes to a valid `IcebergParams`
/// - `parent.quantity == params.total_quantity`
pub async fn split_into_children(
    db: &Arc<DatabaseConnection>,
    engine: &Arc<MatchingEngine>,
    parent: &order::Model,
) -> Result<IcebergParams, AppError> {
    // Re-parse params from DB Model for safety
    let mut params: IcebergParams = parent
        .advanced_params
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| {
            AppError::Internal(format!("Parent {} missing advanced_params", parent.id))
        })?;

    if (params.children_ids.len() as u32) >= params.total_slices {
        // Already split (idempotent: caller might retry)
        return Ok(params);
    }

    // Create the first child
    let first_child = create_child_order(db, parent, &params, 0).await?;

    // Update params
    params.children_ids.push(first_child.id);
    params.slice_index_next = 1;

    // Persist updated params on the parent
    update_parent_params(db, parent.id, &params).await?;

    // Insert the first child into the matching engine book
    engine.insert_limit_order(crate::services::matching_engine::OrderEntry {
        order_id: first_child.id,
        user_id: first_child.user_id,
        symbol: first_child.symbol.clone(),
        side: first_child.side.clone(),
        price: first_child.price,
        remaining_quantity: first_child.quantity - first_child.filled_quantity,
        created_at: first_child.created_at,
    });

    metrics::ICEBERG_CHILD_ORDERS_TOTAL
        .with_label_values(&["created"])
        .inc();

    Ok(params)
}

/// P1-2.1: Append the next child slice into the order book after a fill.
///
/// Called by `matching_engine::flush_trades` after a child fills.
/// No-op when the parent is already fully filled.
pub async fn append_next_child(
    db: &Arc<DatabaseConnection>,
    engine: &Arc<MatchingEngine>,
    child_id: Uuid,
    child_filled_qty: f64,
) -> Result<(), AppError> {
    // 1. Load child + parent
    let child = order::Entity::find_by_id(child_id)
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Child order {} not found", child_id)))?;

    let child_params: IcebergChildParams = child
        .advanced_params
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| {
            AppError::Internal(format!("Child {} missing advanced_params", child_id))
        })?;

    let parent = order::Entity::find_by_id(child_params.parent_id)
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Parent {} not found", child_params.parent_id))
        })?;

    // 2. Update child record: filled_quantity, status=Filled
    let mut child_active: order::ActiveModel = child.clone().into();
    child_active.filled_quantity = Set(child_filled_qty);
    child_active.avg_fill_price = Set(child.price);
    child_active.status = Set(OrderStatus::Filled);
    child_active.filled_at = Set(Some(chrono::Utc::now()));
    child_active.updated_at = Set(chrono::Utc::now());
    child_active
        .update(db.as_ref())
        .await
        .map_err(|e| AppError::Database(format!("update child: {}", e)))?;

    metrics::ICEBERG_CHILD_ORDERS_TOTAL
        .with_label_values(&["filled"])
        .inc();

    // 3. Update parent params
    let mut parent_params: IcebergParams = parent
        .advanced_params
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| {
            AppError::Internal(format!("Parent {} missing advanced_params", parent.id))
        })?;

    parent_params.filled_children += 1;

    // 4. Compute parent new filled_quantity and is_complete
    let parent_new_filled = parent.filled_quantity + child_filled_qty;
    let is_complete = parent_params.is_complete();

    // 5. If not complete, create + insert next child
    if !is_complete {
        let next_idx = parent_params.filled_children;
        let next_child = create_child_order(db, &parent, &parent_params, next_idx).await?;
        parent_params.children_ids.push(next_child.id);
        parent_params.slice_index_next = next_idx + 1;

        engine.insert_limit_order(crate::services::matching_engine::OrderEntry {
            order_id: next_child.id,
            user_id: next_child.user_id,
            symbol: next_child.symbol.clone(),
            side: next_child.side.clone(),
            price: next_child.price,
            remaining_quantity: next_child.quantity - next_child.filled_quantity,
            created_at: next_child.created_at,
        });

        metrics::ICEBERG_CHILD_ORDERS_TOTAL
            .with_label_values(&["created"])
            .inc();
    }

    // 6. Persist parent updates
    let parent_advanced = serde_json::to_value(&parent_params)
        .map_err(|e| AppError::Internal(format!("serialize parent params: {}", e)))?;
    let mut parent_active: order::ActiveModel = parent.clone().into();
    parent_active.filled_quantity = Set(parent_new_filled);
    parent_active.advanced_params = Set(Some(parent_advanced));
    if is_complete {
        parent_active.status = Set(OrderStatus::Filled);
        parent_active.filled_at = Set(Some(chrono::Utc::now()));
    } else if parent_new_filled > 0.0 {
        parent_active.status = Set(OrderStatus::PartialFilled);
    }
    parent_active.updated_at = Set(chrono::Utc::now());
    parent_active
        .update(db.as_ref())
        .await
        .map_err(|e| AppError::Database(format!("update parent: {}", e)))?;

    Ok(())
}

/// P1-2.1: Cancel all pending children of an iceberg parent.
///
/// Returns the number of children that were cancelled.
pub async fn cancel_iceberg_children(
    db: &Arc<DatabaseConnection>,
    engine: &Arc<MatchingEngine>,
    parent_id: Uuid,
) -> Result<u32, AppError> {
    let all_children = order::Entity::find()
        .filter(order::Column::AdvancedType.eq(advanced_type::ICEBERG_CHILD))
        .filter(order::Column::Status.is_in(vec![
            OrderStatus::Pending,
            OrderStatus::PartialFilled,
        ]))
        .all(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut cancelled_count = 0u32;
    for child in all_children {
        let child_params: IcebergChildParams = match child
            .advanced_params
            .as_ref()
            .and_then(|v| serde_json::from_value::<IcebergChildParams>(v.clone()).ok())
        {
            Some(p) => p,
            None => continue,
        };

        if child_params.parent_id != parent_id {
            continue;
        }

        // Remove from book
        engine.remove_from_book(child.id);

        // Update DB
        let mut active: order::ActiveModel = child.into();
        active.status = Set(OrderStatus::Cancelled);
        active.cancelled_at = Set(Some(chrono::Utc::now()));
        active.updated_at = Set(chrono::Utc::now());
        active
            .update(db.as_ref())
            .await
            .map_err(|e| AppError::Database(format!("cancel child: {}", e)))?;

        cancelled_count += 1;
    }

    if cancelled_count > 0 {
        metrics::ICEBERG_CHILD_ORDERS_TOTAL
            .with_label_values(&["cancelled"])
            .inc_by(cancelled_count as u64);
    }

    Ok(cancelled_count)
}

// ─── Helpers ─────────────────────────────────────────────────────

async fn create_child_order(
    db: &DatabaseConnection,
    parent: &order::Model,
    params: &IcebergParams,
    slice_index: u32,
) -> Result<order::Model, AppError> {
    let child_id = Uuid::new_v4();
    let child_qty = if slice_index + 1 >= params.total_slices {
        // Last slice: remainder
        let consumed = params.visible_quantity * (slice_index as f64);
        params.total_quantity - consumed
    } else {
        params.visible_quantity
    };

    let child_params = IcebergChildParams::new(parent.id, slice_index);
    let child_advanced = serde_json::to_value(&child_params)
        .map_err(|e| AppError::Internal(format!("serialize child params: {}", e)))?;

    let now = chrono::Utc::now();
    let active = order::ActiveModel {
        id: Set(child_id),
        user_id: Set(parent.user_id),
        strategy_id: Set(parent.strategy_id),
        symbol: Set(parent.symbol.clone()),
        side: Set(parent.side.clone()),
        order_type: Set(OrderType::Limit),
        price: Set(parent.price),
        quantity: Set(child_qty),
        filled_quantity: Set(0.0),
        avg_fill_price: Set(None),
        status: Set(OrderStatus::Pending),
        mode: Set(parent.mode.clone()),
        fee: Set(0.0),
        reject_reason: Set(None),
        time_in_force: Set(parent.time_in_force.clone()),
        expire_at: Set(parent.expire_at),
        created_at: Set(now),
        updated_at: Set(now),
        cancelled_at: Set(None),
        filled_at: Set(None),
        advanced_type: Set(Some(advanced_type::ICEBERG_CHILD.to_string())),
        advanced_params: Set(Some(child_advanced)),
    };

    active
        .insert(db)
        .await
        .map_err(|e| AppError::Database(format!("insert child order: {}", e)))
}

async fn update_parent_params(
    db: &DatabaseConnection,
    parent_id: Uuid,
    params: &IcebergParams,
) -> Result<(), AppError> {
    let value = serde_json::to_value(params)
        .map_err(|e| AppError::Internal(format!("serialize: {}", e)))?;
    let parent = order::Entity::find_by_id(parent_id)
        .one(db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Parent {} not found", parent_id)))?;
    let mut active: order::ActiveModel = parent.into();
    active.advanced_params = Set(Some(value));
    active.updated_at = Set(chrono::Utc::now());
    active
        .update(db)
        .await
        .map_err(|e| AppError::Database(format!("update parent params: {}", e)))?;
    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::iceberg_params::IcebergParams;

    #[test]
    fn test_advanced_type_constants() {
        assert_eq!(advanced_type::ICEBERG, "iceberg");
        assert_eq!(advanced_type::ICEBERG_CHILD, "iceberg_child");
    }

    #[test]
    fn test_iceberg_params_serialize_roundtrip() {
        let p = IcebergParams {
            visible_quantity: 1.0,
            total_quantity: 10.0,
            filled_children: 3,
            total_slices: 10,
            slice_index_next: 4,
            children_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };
        let json = serde_json::to_value(&p).unwrap();
        let p2: IcebergParams = serde_json::from_value(json).unwrap();
        assert_eq!(p, p2);
    }

    #[test]
    fn test_iceberg_child_params_serialize() {
        let c = IcebergChildParams::new(Uuid::new_v4(), 5);
        let json = serde_json::to_value(&c).unwrap();
        let c2: IcebergChildParams = serde_json::from_value(json).unwrap();
        assert_eq!(c, c2);
    }

    #[test]
    fn test_iceberg_params_validate_inverted() {
        // D5: validate_new enforces visible < total
        let r = IcebergParams::validate_new(10.0, 5.0);
        assert!(r.is_err());
    }

    #[test]
    fn test_iceberg_params_validate_zero_visible() {
        let r = IcebergParams::validate_new(0.0, 10.0);
        assert!(r.is_err());
    }

    #[test]
    fn test_iceberg_params_last_slice_remainder() {
        let p = IcebergParams::validate_new(3.0, 10.0).unwrap();
        assert_eq!(p.total_slices, 4);
        // filled=3 → next slice is last → remainder
        let mut p2 = p.clone();
        p2.filled_children = 3;
        let q = p2.next_slice_quantity();
        assert!((q - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_iceberg_params_is_complete_progression() {
        let mut p = IcebergParams::validate_new(1.0, 3.0).unwrap();
        assert_eq!(p.total_slices, 3);
        assert!(!p.is_complete());
        p.filled_children = 1;
        assert!(!p.is_complete());
        p.filled_children = 2;
        assert!(!p.is_complete());
        p.filled_children = 3;
        assert!(p.is_complete());
    }

    #[test]
    fn test_validate_new_exact_multiple() {
        // total=10, visible=2 → 5 slices, all equal
        let p = IcebergParams::validate_new(2.0, 10.0).unwrap();
        assert_eq!(p.total_slices, 5);
        for i in 0..4 {
            let mut p2 = p.clone();
            p2.filled_children = i;
            assert!((p2.next_slice_quantity() - 2.0).abs() < 1e-9);
        }
        // last slice (i=4) — same as others when exact multiple
        let mut p3 = p.clone();
        p3.filled_children = 4;
        assert!((p3.next_slice_quantity() - 2.0).abs() < 1e-9);
    }
}
