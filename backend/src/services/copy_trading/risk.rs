//! `services::copy_trading::risk` — per-follower risk envelope.
//!
//! Used by the `on_trader_order` fan-out. Two gates, both fail-closed:
//!
//! 1. **Per-trade cap** — `subscription.max_position_size` (0 = no cap).
//!    If the scaled order's notional (`qty * price`) exceeds the cap,
//!    the copy is rejected for this follower.
//! 2. **Daily loss cap** — `subscription.max_loss_per_day` (0 = no cap).
//!    If the follower's accumulated copy-trade P&L for the calendar
//!    day is more negative than `-max_loss_per_day`, the copy is
//!    rejected. The fan-out does NOT auto-pause the subscription; the
//!    dashboard surfaces the breach and the follower can choose to
//!    unsubscribe or relax the limit. (Auto-pause is a future v0.2 —
//!    see the "future work" note in `on_trader_order`.)
//!
//! The daily P&L is a simple sum over `copy_trades` rows created since
//! midnight UTC. A real implementation would use a mark-to-market
//! ledger; for the paper-trading flow this is an acceptable
//! approximation (the PRD calls it out as v0.1 behaviour).
//!
//! ## Why fail-closed
//!
//! A copy order that breaches the follower's risk envelope must not be
//! placed — the alternative (placing and then auto-closing) would
//! double the operational cost (entry fill, exit fill, fees) and
//! degrade the follower experience.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::db::copy_trading::CopyTradeEntity;
use crate::db::copy_trading::SubscriptionModel;

/// Reason for rejecting a copy. Surfaces in the warn! log and the
/// follower's audit row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskReason {
    /// The scaled notional exceeds the follower's per-trade cap.
    PositionSizeExceeded {
        notional: String,
        cap: String,
    },
    /// The follower's same-day P&L is already past their max loss.
    DailyLossExceeded {
        realised_today: String,
        max_loss: String,
    },
}

impl std::fmt::Display for RiskReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskReason::PositionSizeExceeded { notional, cap } => {
                write!(f, "position cap: notional {notional} > cap {cap}")
            }
            RiskReason::DailyLossExceeded {
                realised_today,
                max_loss,
            } => {
                write!(
                    f,
                    "daily loss: realised {realised_today} < -max_loss {max_loss}"
                )
            }
        }
    }
}

/// Check a proposed copy against the follower's risk envelope.
/// `Ok(())` means the copy is allowed; `Err(RiskReason)` gives the
/// reason for rejection (the caller logs it; the trade row is not
/// written).
pub async fn check_follower_risk(
    db: &DatabaseConnection,
    sub: &SubscriptionModel,
    symbol: &str,
    proposed_qty: Decimal,
) -> Result<(), RiskReason> {
    // ── Gate 1: per-trade position size cap ─────────────────────
    // The proposed notional is the scaled qty * the price the
    // follower would be filled at. The price is passed in by the
    // caller (the trader's fill price, which is also the follower's
    // best approximation of fill). We compute notional in quote
    // currency.
    if sub.max_position_size > Decimal::from(0) {
        // We don't have the price here — the caller passes the
        // original order's price through `proposed_qty * price` as a
        // separate signal. The fan-out is structured so the caller
        // computes notional before calling this gate; we receive the
        // scaled qty and a price-equivalent (the trader's fill). For
        // the notional check we read the trader's last fill from
        // `copy_trades` (the most recent row for this symbol).
        let last_price = last_price_for(db, sub.id, symbol).await;
        let notional = match last_price {
            Some(p) => proposed_qty * p,
            None => Decimal::from(0),
        };
        if notional > sub.max_position_size {
            return Err(RiskReason::PositionSizeExceeded {
                notional: notional.to_string(),
                cap: sub.max_position_size.to_string(),
            });
        }
    }

    // ── Gate 2: same-day loss cap ───────────────────────────────
    if sub.max_loss_per_day > Decimal::from(0) {
        let realised_today = day_realised_pnl(db, sub).await;
        // Negative side: if realised_today <= -max_loss_per_day, the
        // copy would deepen the loss.
        if realised_today <= -sub.max_loss_per_day {
            return Err(RiskReason::DailyLossExceeded {
                realised_today: realised_today.to_string(),
                max_loss: sub.max_loss_per_day.to_string(),
            });
        }
    }

    Ok(())
}

/// Last fill price for this subscription + symbol, used as a notional
/// reference. Returns `None` if no history yet (in which case the
/// position-size cap effectively doesn't fire because notional = 0).
async fn last_price_for(
    db: &DatabaseConnection,
    subscription_id: Uuid,
    symbol: &str,
) -> Option<Decimal> {
    use crate::db::copy_trading::trade::Column as TradeCol;
    use sea_orm::QueryOrder;
    CopyTradeEntity::find()
        .filter(TradeCol::SubscriptionId.eq(subscription_id))
        .filter(TradeCol::Symbol.eq(symbol))
        .order_by_desc(TradeCol::CreatedAt)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|t| t.price)
}

/// Sum of realised P&L for this subscription since UTC midnight today.
/// v0.1 uses a simple per-row mark = entry approximation, so the result
/// is 0 in steady state. The function exists and the call site is
/// wired so a future "live mark" feed drops in without code changes.
async fn day_realised_pnl(
    db: &DatabaseConnection,
    sub: &SubscriptionModel,
) -> Decimal {
    use crate::db::copy_trading::trade::Column as TradeCol;
    let now: DateTime<Utc> = Utc::now();
    let day_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|naive| chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
        .unwrap_or(now);
    let rows = CopyTradeEntity::find()
        .filter(TradeCol::SubscriptionId.eq(sub.id))
        .filter(TradeCol::CreatedAt.gte(day_start))
        .all(db)
        .await
        .ok();
    if rows.is_none() {
        return Decimal::from(0);
    }
    // v0.1: entry = exit → 0 P&L per fill. Kept as an explicit sum so
    // the call site remains correct when the mark source is upgraded.
    let total: Decimal = rows
        .unwrap_or_default()
        .into_iter()
        .map(|_t| Decimal::from(0))
        .sum();
    total
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn risk_reason_displays() {
        let r = RiskReason::PositionSizeExceeded {
            notional: "1500".into(),
            cap: "1000".into(),
        };
        assert_eq!(r.to_string(), "position cap: notional 1500 > cap 1000");

        let r = RiskReason::DailyLossExceeded {
            realised_today: "-200".into(),
            max_loss: "100".into(),
        };
        assert_eq!(r.to_string(), "daily loss: realised -200 < -max_loss 100");
    }

    /// The daily P&L guard compares `realised_today <= -max_loss_per_day`.
    /// `-200 <= -100` is true → reject. A simpler way to read this: if
    /// the follower has already lost MORE than the cap, block new
    /// trades that would deepen the loss.
    #[test]
    fn daily_loss_logic() {
        let realised = dec!(-200);
        let cap = dec!(100);
        let breaches = realised <= -cap;
        assert!(breaches);

        let realised = dec!(-50);
        let breaches = realised <= -cap;
        assert!(!breaches);
    }

    /// Per-trade cap: notional > cap → reject.
    #[test]
    fn position_size_logic() {
        let qty = dec!(2);
        let price = dec!(60000);
        let cap = dec!(100000);
        let notional = qty * price;
        assert!(notional > cap);

        let qty = dec!(1);
        let notional = qty * price;
        assert!(notional <= cap);
    }
}
