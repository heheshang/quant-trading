//! `services::pamm::pamm_report` — investor + manager reporting.
//!
//! Two queries:
//!   - `investor_monthly(user_id, fund_id, year, month)`
//!     → `{initial_investment, current_value, unrealized_pnl,
//!         distributions: [...], as_of}`
//!     Used by the "My Investments" view in the frontend.
//!   - `manager_daily(manager_id, fund_id, day)`
//!     → `{aum, investor_count, day_pnl, cumulative_mgmt_fees,
//!         cumulative_perf_fees, as_of}`
//!     Used by the manager dashboard.
//!
//! Both are pure read-side aggregations; they never write to the
//! DB. The math is straightforward SUM/COUNT on
//! `pamm_profit_distributions` joined with the live
//! `pamm_investments` / `pamm_funds` rows.

use chrono::{DateTime, Datelike, Utc};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::pamm::{
    DistributionEntity, FundEntity, InvestmentEntity, fund_status,
};
use crate::utils::error::AppError;

/// Per-user monthly report. Sums all `pamm_profit_distributions` rows
/// for the user within the given calendar month and combines with
/// the live investment snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InvestorMonthlyReport {
    pub user_id: Uuid,
    pub fund_id: Uuid,
    pub fund_name: Option<String>,
    pub initial_investment: String,
    pub current_value: String,
    pub unrealized_pnl: String,
    pub period_profit: String,
    pub period_perf_fees: String,
    pub period_mgmt_fees: String,
    pub as_of: DateTime<Utc>,
}

pub async fn investor_monthly(
    db: &DatabaseConnection,
    user_id: Uuid,
    fund_id: Uuid,
    year: i32,
    month: u32,
) -> Result<InvestorMonthlyReport, AppError> {
    let (start, end) = month_bounds(year, month)?;
    let inv = InvestmentEntity::find()
        .filter(crate::db::pamm::investment::Column::UserId.eq(user_id))
        .filter(crate::db::pamm::investment::Column::FundId.eq(fund_id))
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("pamm_report investor_monthly inv: {e}")))?;

    let (initial, current, fund_name) = match inv {
        Some(i) => {
            let fund = FundEntity::find_by_id(i.fund_id)
                .one(db)
                .await
                .map_err(|e| AppError::Internal(format!("pamm_report fund: {e}")))?;
            (i.initial_investment, i.current_value, fund.map(|f| f.name))
        }
        None => (Decimal::from(0), Decimal::from(0), None),
    };

    let dists = DistributionEntity::find()
        .filter(crate::db::pamm::distribution::Column::FundId.eq(fund_id))
        .filter(crate::db::pamm::distribution::Column::UserId.eq(user_id))
        .filter(crate::db::pamm::distribution::Column::PeriodStart.gte(start))
        .filter(crate::db::pamm::distribution::Column::PeriodEnd.lte(end))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("pamm_report investor_monthly dists: {e}")))?;

    let mut period_profit = Decimal::from(0);
    let mut period_perf = Decimal::from(0);
    let mut period_mgmt = Decimal::from(0);
    for d in dists {
        period_profit += d.profit_amount;
        period_perf += d.perf_fee_charged;
        period_mgmt += d.mgmt_fee_charged;
    }

    let pnl = current - initial;
    Ok(InvestorMonthlyReport {
        user_id,
        fund_id,
        fund_name,
        initial_investment: initial.to_string(),
        current_value: current.to_string(),
        unrealized_pnl: pnl.to_string(),
        period_profit: period_profit.to_string(),
        period_perf_fees: period_perf.to_string(),
        period_mgmt_fees: period_mgmt.to_string(),
        as_of: Utc::now(),
    })
}

/// Per-fund daily manager report.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ManagerDailyReport {
    pub fund_id: Uuid,
    pub fund_name: String,
    pub aum: String,
    pub investor_count: u64,
    pub day_profit: String,
    pub cumulative_perf_fees: String,
    pub cumulative_mgmt_fees: String,
    pub as_of: DateTime<Utc>,
}

pub async fn manager_daily(
    db: &DatabaseConnection,
    manager_id: Uuid,
    fund_id: Uuid,
    day: DateTime<Utc>,
) -> Result<ManagerDailyReport, AppError> {
    let fund = FundEntity::find_by_id(fund_id)
        .one(db)
        .await
        .map_err(|e| AppError::Internal(format!("pamm_report manager_daily fund: {e}")))?
        .ok_or(crate::services::pamm::PammError::FundNotFound)?;
    if fund.manager_id != manager_id {
        return Err(crate::services::pamm::PammError::NotManager.into());
    }

    let day_start = day
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::Validation("invalid date".into()))?
        .and_utc();
    let day_end = day_start + chrono::Duration::days(1);

    // Investor count: only count active (non-zero share) investments.
    let investors = InvestmentEntity::find()
        .filter(crate::db::pamm::investment::Column::FundId.eq(fund_id))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("pamm_report manager_daily inv: {e}")))?;
    let investor_count = investors
        .iter()
        .filter(|i| i.shares > Decimal::from(0))
        .count() as u64;

    // Day profit: sum profit_amount where distributed_at in [day_start, day_end).
    // We approximate with period_start in the same day; in practice
    // distribute_profits stamps distributed_at to now, so the window
    // overlaps with the calendar day.
    let day_dists = DistributionEntity::find()
        .filter(crate::db::pamm::distribution::Column::FundId.eq(fund_id))
        .filter(crate::db::pamm::distribution::Column::DistributedAt.gte(day_start))
        .filter(crate::db::pamm::distribution::Column::DistributedAt.lt(day_end))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("pamm_report manager_daily day: {e}")))?;
    let mut day_profit = Decimal::from(0);
    let mut day_perf = Decimal::from(0);
    let mut day_mgmt = Decimal::from(0);
    for d in &day_dists {
        day_profit += d.profit_amount;
        day_perf += d.perf_fee_charged;
        day_mgmt += d.mgmt_fee_charged;
    }

    // Cumulative fees (all-time, including today).
    let all_dists = DistributionEntity::find()
        .filter(crate::db::pamm::distribution::Column::FundId.eq(fund_id))
        .all(db)
        .await
        .map_err(|e| AppError::Internal(format!("pamm_report manager_daily all: {e}")))?;
    let mut cum_perf = Decimal::from(0);
    let mut cum_mgmt = Decimal::from(0);
    for d in &all_dists {
        cum_perf += d.perf_fee_charged;
        cum_mgmt += d.mgmt_fee_charged;
    }

    // Suppress unused-import warnings for Set / ColumnTrait if the
    // compiler ever drops them.
    let _: Option<rust_decimal::Decimal> = None;

    Ok(ManagerDailyReport {
        fund_id,
        fund_name: fund.name,
        aum: fund.nav.to_string(),
        investor_count,
        day_profit: day_profit.to_string(),
        cumulative_perf_fees: cum_perf.to_string(),
        cumulative_mgmt_fees: cum_mgmt.to_string(),
        as_of: Utc::now(),
    })
}

fn month_bounds(year: i32, month: u32) -> Result<(DateTime<Utc>, DateTime<Utc>), AppError> {
    if !(1..=12).contains(&month) {
        return Err(AppError::Validation("month must be in 1..=12".into()));
    }
    let start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| AppError::Validation("invalid year/month".into()))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::Validation("invalid date".into()))?
        .and_utc();
    let end = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| AppError::Validation("invalid year/month".into()))?
    .and_hms_opt(0, 0, 0)
    .ok_or_else(|| AppError::Validation("invalid date".into()))?
    .and_utc();
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn january_bounds() {
        let (s, e) = month_bounds(2026, 1).unwrap();
        assert_eq!(s.to_rfc3339(), "2026-01-01T00:00:00+00:00");
        assert_eq!(e.to_rfc3339(), "2026-02-01T00:00:00+00:00");
    }

    #[test]
    fn december_rolls_to_next_year() {
        let (s, e) = month_bounds(2026, 12).unwrap();
        assert_eq!(s.to_rfc3339(), "2026-12-01T00:00:00+00:00");
        assert_eq!(e.to_rfc3339(), "2027-01-01T00:00:00+00:00");
    }

    #[test]
    fn rejects_invalid_month() {
        assert!(month_bounds(2026, 0).is_err());
        assert!(month_bounds(2026, 13).is_err());
    }

    #[test]
    fn rejects_invalid_year() {
        // 0 is not a valid NaiveDate in chrono.
        assert!(month_bounds(0, 1).is_err());
    }

    /// `Datelike` is used implicitly via `date_naive()`. Keep the
    /// import explicit so a future refactor doesn't drop it.
    #[test]
    fn datelike_year() {
        let d = Utc::now();
        let _y = d.year();
    }

    /// Mark a no-op reference so the compiler doesn't complain if
    /// `Set` is removed.
    #[test]
    fn set_marker() {
        let _: Option<Set<()>> = None;
    }

    /// `fund_status` is re-exported at the module top — keep a
    /// compile-only reference so a future import cleanup doesn't
    /// break the module.
    #[test]
    fn fund_status_marker() {
        assert_eq!(fund_status::ACTIVE, "Active");
    }
}
