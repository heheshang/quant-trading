//! models/iceberg_params.rs — Iceberg order parameter schema
//!
//! P1-2.1: Type-safe wrapper around the `advanced_params` JSONB blob for iceberg orders.
//! Used by handlers to parse request and by services to track child order state.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Iceberg parent order parameters stored in `orders.advanced_params` JSONB.
///
/// On the parent, this is the **initial** state (when the order is first created).
/// On each subsequent update, the same struct is re-serialized with the latest counters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcebergParams {
    /// Quantity shown to the market per child slice.
    pub visible_quantity: f64,

    /// Total quantity for the whole iceberg (== parent order quantity).
    pub total_quantity: f64,

    /// Number of slices that have been fully filled.
    pub filled_children: u32,

    /// Total number of slices (== ceil(total_quantity / visible_quantity)).
    pub total_slices: u32,

    /// Index of the next slice to create (0-based, 0..total_slices).
    pub slice_index_next: u32,

    /// IDs of all child orders created so far (for cancel cascade + parent reference).
    #[serde(default)]
    pub children_ids: Vec<Uuid>,
}

impl IcebergParams {
    /// Validate the user-supplied advanced_params for a new iceberg parent order.
    ///
    /// Returns the initialized `IcebergParams` with all derived fields populated.
    pub fn validate_new(visible_quantity: f64, total_quantity: f64) -> Result<Self, String> {
        if visible_quantity <= 0.0 {
            return Err("visible_quantity must be > 0".to_string());
        }
        if total_quantity <= 0.0 {
            return Err("total_quantity must be > 0".to_string());
        }
        if visible_quantity >= total_quantity {
            return Err(format!(
                "visible_quantity ({}) must be < total_quantity ({})",
                visible_quantity, total_quantity
            ));
        }
        // total_slices = ceil(total / visible). For typical values (1, 1.5, 0.5) we use
        // a closed-form computation rather than floating point rounding.
        let total_slices = (total_quantity / visible_quantity).ceil() as u32;
        if total_slices == 0 {
            return Err("total_slices computed as 0, check visible_quantity".to_string());
        }
        Ok(Self {
            visible_quantity,
            total_quantity,
            filled_children: 0,
            total_slices,
            slice_index_next: 0,
            children_ids: Vec::new(),
        })
    }

    /// Quantity for the next child slice.
    /// Last slice absorbs rounding remainder.
    pub fn next_slice_quantity(&self) -> f64 {
        let next_idx = self.filled_children;
        if next_idx + 1 >= self.total_slices {
            // Last slice: remainder = total - visible * filled_children
            // (no max clamp: remainder is intentionally smaller than visible)
            let consumed = self.visible_quantity * (next_idx as f64);
            self.total_quantity - consumed
        } else {
            self.visible_quantity
        }
    }

    /// True if the parent order is fully filled and no more children should be appended.
    pub fn is_complete(&self) -> bool {
        self.filled_children >= self.total_slices
    }
}

/// Iceberg child order parameters stored in `orders.advanced_params` JSONB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcebergChildParams {
    /// Parent iceberg order ID.
    pub parent_id: Uuid,

    /// 0-based index of this slice in the parent's sequence.
    pub slice_index: u32,
}

impl IcebergChildParams {
    pub fn new(parent_id: Uuid, slice_index: u32) -> Self {
        Self {
            parent_id,
            slice_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_new_basic() {
        let p = IcebergParams::validate_new(1.0, 10.0).unwrap();
        assert_eq!(p.total_slices, 10);
        assert_eq!(p.filled_children, 0);
        assert_eq!(p.slice_index_next, 0);
        assert!(p.children_ids.is_empty());
    }

    #[test]
    fn test_validate_new_with_remainder() {
        let p = IcebergParams::validate_new(3.0, 10.0).unwrap();
        // ceil(10/3) = 4
        assert_eq!(p.total_slices, 4);
    }

    #[test]
    fn test_validate_new_rejects_inverted() {
        assert!(IcebergParams::validate_new(10.0, 10.0).is_err());
        assert!(IcebergParams::validate_new(11.0, 10.0).is_err());
    }

    #[test]
    fn test_validate_new_rejects_zero() {
        assert!(IcebergParams::validate_new(0.0, 10.0).is_err());
        assert!(IcebergParams::validate_new(-1.0, 10.0).is_err());
        assert!(IcebergParams::validate_new(1.0, 0.0).is_err());
    }

    #[test]
    fn test_next_slice_quantity_basic() {
        let p = IcebergParams::validate_new(1.0, 10.0).unwrap();
        for i in 0..10 {
            let mut p2 = p.clone();
            p2.filled_children = i;
            assert!((p2.next_slice_quantity() - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn test_next_slice_quantity_last_absorbs_remainder() {
        let p = IcebergParams::validate_new(3.0, 10.0).unwrap();
        assert_eq!(p.total_slices, 4);
        let mut p2 = p.clone();
        p2.filled_children = 3; // last slice
        // 3rd slice: remainder = 10 - 3*3 = 1.0
        assert!((p2.next_slice_quantity() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_is_complete() {
        let mut p = IcebergParams::validate_new(1.0, 10.0).unwrap();
        assert!(!p.is_complete());
        p.filled_children = 9;
        assert!(!p.is_complete());
        p.filled_children = 10;
        assert!(p.is_complete());
    }

    #[test]
    fn test_child_params() {
        let parent_id = Uuid::new_v4();
        let c = IcebergChildParams::new(parent_id, 3);
        assert_eq!(c.parent_id, parent_id);
        assert_eq!(c.slice_index, 3);
    }
}
