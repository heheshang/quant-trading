//! models/bracket_params.rs — Bracket order parameter schema
//!
//! P1-2.2: Type-safe wrapper around the `advanced_params` JSONB blob for bracket orders.
//!
//! Bracket order = entry order + 2 conditional sub-orders (stop-loss + take-profit).
//! In P1-2.2 (path A: deferred OCO), the entry order is created normally; once the
//! entry fills, the matching engine records a row in `bracket_links` and emits a
//! `bracket_filled` event. The actual OCO creation is performed by the client
//! (typically a frontend polling `/api/v1/bracket-links/pending` and then calling
//! `/api/v1/trigger-orders/oco`).

use serde::{Deserialize, Serialize};

/// Bracket parent order parameters stored in `orders.advanced_params` JSONB.
///
/// Validation enforces that the SL/TP prices are on opposite sides of the entry price,
/// which matches standard exchange bracket semantics:
///   - buy bracket: stop_loss < entry < take_profit
///   - sell bracket: take_profit < entry < stop_loss
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BracketParams {
    /// Stop-loss trigger price.
    pub stop_loss_price: f64,

    /// Take-profit trigger price.
    pub take_profit_price: f64,

    /// Status of the OCO linkage (mirrored from `bracket_links.oco_status`).
    /// Possible values: "pending" | "linked" | "cancelled" | "failed".
    #[serde(default = "default_oco_status")]
    pub oco_status: String,
}

fn default_oco_status() -> String {
    "pending".to_string()
}

impl BracketParams {
    /// Validate the user-supplied advanced_params for a new bracket parent order.
    ///
    /// # Arguments
    /// * `side` — "buy" or "sell" (the entry side of the bracket).
    /// * `entry_price` — the limit price of the entry order.
    /// * `stop_loss_price` — proposed SL trigger.
    /// * `take_profit_price` — proposed TP trigger.
    /// * `quantity` — entry quantity (must be > 0).
    pub fn validate_new(
        side: &str,
        entry_price: f64,
        stop_loss_price: f64,
        take_profit_price: f64,
        quantity: f64,
    ) -> Result<Self, String> {
        if quantity <= 0.0 {
            return Err("quantity must be > 0".to_string());
        }
        if entry_price <= 0.0 {
            return Err("entry_price must be > 0".to_string());
        }
        if stop_loss_price <= 0.0 {
            return Err("stop_loss_price must be > 0".to_string());
        }
        if take_profit_price <= 0.0 {
            return Err("take_profit_price must be > 0".to_string());
        }
        match side {
            "buy" => {
                if stop_loss_price >= entry_price {
                    return Err(format!(
                        "stop_loss_price ({}) must be < entry_price ({}) for buy bracket",
                        stop_loss_price, entry_price
                    ));
                }
                if take_profit_price <= entry_price {
                    return Err(format!(
                        "take_profit_price ({}) must be > entry_price ({}) for buy bracket",
                        take_profit_price, entry_price
                    ));
                }
            }
            "sell" => {
                if stop_loss_price <= entry_price {
                    return Err(format!(
                        "stop_loss_price ({}) must be > entry_price ({}) for sell bracket",
                        stop_loss_price, entry_price
                    ));
                }
                if take_profit_price >= entry_price {
                    return Err(format!(
                        "take_profit_price ({}) must be < entry_price ({}) for sell bracket",
                        take_profit_price, entry_price
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "Invalid side: {}, expected 'buy' or 'sell'",
                    side
                ));
            }
        }
        if (stop_loss_price - take_profit_price).abs() < f64::EPSILON {
            return Err("stop_loss_price and take_profit_price must differ".to_string());
        }
        Ok(Self {
            stop_loss_price,
            take_profit_price,
            oco_status: default_oco_status(),
        })
    }

    /// True if the OCO has not yet been created (`pending` status).
    pub fn is_oco_pending(&self) -> bool {
        self.oco_status == "pending"
    }

    /// True if the OCO is fully created (`linked` status).
    pub fn is_oco_linked(&self) -> bool {
        self.oco_status == "linked"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_new_buy_valid() {
        let p = BracketParams::validate_new("buy", 100.0, 95.0, 110.0, 1.0).unwrap();
        assert_eq!(p.stop_loss_price, 95.0);
        assert_eq!(p.take_profit_price, 110.0);
        assert!(p.is_oco_pending());
    }

    #[test]
    fn test_validate_new_sell_valid() {
        // Sell bracket: SL above entry, TP below entry
        let p = BracketParams::validate_new("sell", 100.0, 110.0, 90.0, 1.0).unwrap();
        assert_eq!(p.stop_loss_price, 110.0);
        assert_eq!(p.take_profit_price, 90.0);
    }

    #[test]
    fn test_validate_new_buy_sl_above_entry_rejected() {
        assert!(BracketParams::validate_new("buy", 100.0, 105.0, 110.0, 1.0).is_err());
    }

    #[test]
    fn test_validate_new_buy_tp_below_entry_rejected() {
        assert!(BracketParams::validate_new("buy", 100.0, 95.0, 99.0, 1.0).is_err());
    }

    #[test]
    fn test_validate_new_sell_sl_below_entry_rejected() {
        assert!(BracketParams::validate_new("sell", 100.0, 90.0, 80.0, 1.0).is_err());
    }

    #[test]
    fn test_validate_new_rejects_zero_quantity() {
        assert!(BracketParams::validate_new("buy", 100.0, 95.0, 110.0, 0.0).is_err());
    }

    #[test]
    fn test_validate_new_rejects_invalid_side() {
        assert!(BracketParams::validate_new("hold", 100.0, 95.0, 110.0, 1.0).is_err());
    }

    #[test]
    fn test_is_oco_pending_and_linked() {
        let mut p = BracketParams::validate_new("buy", 100.0, 95.0, 110.0, 1.0).unwrap();
        assert!(p.is_oco_pending());
        assert!(!p.is_oco_linked());
        p.oco_status = "linked".to_string();
        assert!(!p.is_oco_pending());
        assert!(p.is_oco_linked());
    }
}
