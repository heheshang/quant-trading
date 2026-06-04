//! `services::pamm::strategy_link` — bridge between `pamm_funds` and
//! the existing `strategies` table.
//!
//! When a fund has `strategy_id` set, the strategy's realised P&L
//! flows into the fund's NAV. The actual P&L rollup runs in the
//! backtest / live-trading loop; this module just exposes the
//! helper that the rollup calls.
//!
//! PRD §6-1 Part 4 calls out:
//!   - pamm_fund can link one strategy
//!   - strategy live trades' P&L is attributed to the fund NAV
//!
//! v0.1: we expose a single `attribute_pnl(fund_id, pnl_delta)` helper
//! that the strategy runner invokes when it closes a position. The
//! fund row's `nav` is incremented by `pnl_delta` (positive for
//! profit, negative for loss) and `total_shares` / `share_value` are
//! recomputed. `distribute_profits` then re-derives the
//! per-investor allocation.
//!
//! A future "live P&L tick" worker can call this on every
//! strategy fill, or batch once a minute — the math is identical.

use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use tracing::info;
use uuid::Uuid;

use crate::db::pamm::FundEntity;
use crate::utils::error::AppError;

/// Apply a realised P&L delta to a fund's NAV. `pnl_delta` is signed:
/// positive for profit, negative for loss. The fund's
/// `total_shares` is unchanged (existing investors' shares don't
/// change just because the fund made money — their *value* per
/// share does, which is captured by `share_value`).
pub async fn attribute_pnl(
    db: &DatabaseConnection,
    fund_id: Uuid,
    pnl_delta: Decimal,
) -> Result<Decimal, AppError> {
    let fund = FundEntity::find_by_id(fund_id)
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("pamm strategy_link find: {e}")))?
        .ok_or(crate::services::pamm::PammError::FundNotFound)?;

    let new_nav = (fund.nav + pnl_delta).max(Decimal::from(0));
    let new_share_value = if fund.total_shares > Decimal::from(0) {
        new_nav / fund.total_shares
    } else {
        Decimal::from(1)
    };

    let mut am: crate::db::pamm::fund::ActiveModel = fund.into();
    am.nav = Set(new_nav);
    am.share_value = Set(new_share_value);
    am.updated_at = Set(chrono::Utc::now());
    am.update(db)
        .await
        .map_err(|e| AppError::Internal(format!("pamm strategy_link update: {e}")))?;

    info!(
        fund_id = %fund_id,
        pnl_delta = %pnl_delta,
        new_nav = %new_nav,
        new_share_value = %new_share_value,
        "pamm strategy_link pnl attributed"
    );
    Ok(new_share_value)
}

/// Return the strategy id attached to a fund, if any. Used by the
/// strategy runner to short-circuit when a strategy is being traded
/// under a fund (so it can skip the user-account paper-trading path
/// and write P&L to the fund NAV instead).
pub async fn fund_strategy_id(
    db: &DatabaseConnection,
    fund_id: Uuid,
) -> Result<Option<Uuid>, AppError> {
    let fund = FundEntity::find_by_id(fund_id)
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("pamm strategy_link fund_strategy_id: {e}")))?
        .ok_or(crate::services::pamm::PammError::FundNotFound)?;
    Ok(fund.strategy_id)
}
