//! SeaORM entities for the PAMM (Percent Allocation Management Module) — PRD §6 commercialization-1.
//!
//! Five tables back the full fund-management lifecycle:
//!
//! - `pamm_funds`               — one row per fund, owned by a manager.
//! - `pamm_investments`         — investor's current stake in a fund (per-manager share %).
//! - `pamm_profit_distributions`— append-only ledger of per-period profit allocations.
//! - `pamm_subscriptions`       — pending/active/refunded subscription requests.
//! - `pamm_redemptions`         — pending/completed/cancelled redemption requests.
//!
//! All five entities are colocated in this single file per the §6-1
//! task spec ("一个文件, 多个 entity"). They're declared as inline
//! `mod` blocks below; downstream code uses the public re-exports
//! (`FundEntity`, `InvestmentEntity`, `DistributionEntity`,
//! `SubscriptionEntity`, `RedemptionEntity`).
//!
//! ## Why we model subscriptions + redemptions as separate state machines
//!
//! A subscription moves Pending → Active once the manager (or a worker) marks
//! it as capital that should count toward NAV; a redemption moves Pending →
//! Completed once the cash actually leaves the master account. Keeping them
//! out of `pamm_investments` means a single "investment" row is always a
//! live, accruing stake — the request lifecycle is a separate concern and
//! can be queried for "is this user mid-flow" without scanning investments.
//!
//! ## Indexes
//!
//! All `fund_id` columns are indexed (the hot path is "show me everything
//! for fund X"). `(user_id, fund_id)` is also unique on `pamm_investments`
//! to prevent the same user from creating two competing investment rows
//! for the same fund — a redemption followed by a re-subscription would
//! race without the constraint.
//!
//! See `migrations/20260603150000_create_pamm_tables.sql` for the raw DDL.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// =========================================================================
// 1) pamm_funds — fund definition (manager + fee structure + status).
// =========================================================================
pub mod fund {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "pamm_funds")]
    pub struct Model {
        /// UUID v4 PK. Random to dodge BIGSERIAL hot-row contention on
        /// concurrent `POST /pamm/funds` (rare but worth the tiny cost).
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Manager — the user who trades the master account. A user may
        /// run multiple funds but each fund has exactly one manager.
        pub manager_id: Uuid,
        /// Display name. VARCHAR(120) is enough for "Alpha-BTC-Momentum v3".
        #[sea_orm(column_type = "String(StringLen::N(120))")]
        pub name: String,
        /// Free-text description shown in the fund list / detail view.
        #[sea_orm(column_type = "Text")]
        pub description: Option<String>,
        /// "USDT" / "BTC" / "USD". NAV is denominated in this currency.
        #[sea_orm(column_type = "String(StringLen::N(16))")]
        pub base_currency: String,
        /// Annualised management fee as a fraction (0.02 = 2%/yr). Charged
        /// pro-rata per distribution cycle.
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub management_fee_pct: Decimal,
        /// Performance fee as a fraction of *new* high-water-mark profit
        /// (0.20 = 20% of HWM-increment profit). Only triggered when NAV
        /// exceeds the previous HWM.
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub performance_fee_pct: Decimal,
        /// Whether the high-water-mark check is enabled. The manager may
        /// opt out (e.g. for a beta product) but the column is always there
        /// so the SQL schema doesn't need a migration if they change their mind.
        pub high_water_mark: bool,
        /// Current NAV. Updated by `distribute_profits` and by the strategy-
        /// link P&L rollup. Decimal(20,8) matches the rest of the platform.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub nav: Decimal,
        /// Per-share value. `current_value(share) = share_pct * share_value`.
        /// Always 1.0 at fund inception; grows with profit and shrinks with
        /// loss.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub share_value: Decimal,
        /// All-time NAV high water mark. Used to gate performance fees: a
        /// perf-fee charge is only triggered on profit that *exceeds* HWM.
        /// Stored as Decimal(20,8) — same precision as NAV.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub hwm: Decimal,
        /// Total shares outstanding across all active investments. Kept
        /// denormalised so `share_value = nav / total_shares` is O(1)
        /// instead of "sum all investments" on every read.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub total_shares: Decimal,
        /// Optional linked strategy id. When set, the strategy's realised
        /// P&L feeds into NAV. (See `services::pamm::strategy_link`.)
        pub strategy_id: Option<Uuid>,
        /// Lifecycle. `Active` trades, `Paused` blocks new subscriptions but
        /// existing investments keep accruing, `Liquidated` is terminal —
        /// no more distributions, NAV frozen.
        #[sea_orm(column_type = "String(StringLen::N(16))")]
        pub status: String,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::super::user::Entity",
            from = "Column::ManagerId",
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

    /// Well-known status strings for `pamm_funds.status`.
    pub mod status {
        pub const ACTIVE: &str = "Active";
        pub const PAUSED: &str = "Paused";
        pub const LIQUIDATED: &str = "Liquidated";
    }
}

// =========================================================================
// 2) pamm_investments — one row per (user, fund) pair. A user's stake.
// =========================================================================
pub mod investment {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "pamm_investments")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub fund_id: Uuid,
        pub user_id: Uuid,
        /// Owning share as a fraction (0.40 = 40% of the fund). Recomputed
        /// by `distribute_profits` and on every subscribe/redeem.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub share_pct: Decimal,
        /// Shares owned. Multiply by `pamm_funds.share_value` to get
        /// current value in base_currency.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub shares: Decimal,
        /// Snapshot of the user-paid capital at the time the latest
        /// subscription was confirmed.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub initial_investment: Decimal,
        /// Current value (`shares * fund.share_value`). Cached so the
        /// "My Investments" view is a single-table read.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub current_value: Decimal,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// =========================================================================
// 3) pamm_profit_distributions — append-only ledger of per-period allocations.
// =========================================================================
pub mod distribution {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "pamm_profit_distributions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub fund_id: Uuid,
        pub user_id: Uuid,
        pub period_start: DateTimeUtc,
        pub period_end: DateTimeUtc,
        /// Net profit allocated to this user. Negative = loss.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub profit_amount: Decimal,
        /// HWM at the time the distribution was calculated.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub hwm: Decimal,
        /// Performance fee charged to this user for this period.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub perf_fee_charged: Decimal,
        /// Management fee charged to this user for this period.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub mgmt_fee_charged: Decimal,
        pub distributed_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// =========================================================================
// 4) pamm_subscriptions — request lifecycle for "I want to invest X".
// =========================================================================
pub mod subscription {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "pamm_subscriptions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub fund_id: Uuid,
        pub user_id: Uuid,
        /// Amount in `pamm_funds.base_currency`. The capital is held in
        /// the master account until status moves to Active.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub amount: Decimal,
        /// Pending (initial) → Active (capital moved to NAV) or
        /// Refunded (manager rejected / user cancelled before capital moved).
        #[sea_orm(column_type = "String(StringLen::N(16))")]
        pub status: String,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    pub mod status {
        pub const PENDING: &str = "Pending";
        pub const ACTIVE: &str = "Active";
        pub const REFUNDED: &str = "Refunded";
    }
}

// =========================================================================
// 5) pamm_redemptions — request lifecycle for "I want to take X out".
// =========================================================================
pub mod redemption {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "pamm_redemptions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub fund_id: Uuid,
        pub user_id: Uuid,
        /// Amount the user requested. May be larger than the user's current
        /// `investment.shares * fund.share_value` — the handler validates
        /// and clamps at submit time.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub amount_requested: Decimal,
        /// Amount actually paid out after NAV rounding. Set when status
        /// moves to Completed.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub amount_paid: Decimal,
        /// Pending → Completed (cash out) or Cancelled.
        #[sea_orm(column_type = "String(StringLen::N(16))")]
        pub status: String,
        pub requested_at: DateTimeUtc,
        pub paid_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    pub mod status {
        pub const PENDING: &str = "Pending";
        pub const COMPLETED: &str = "Completed";
        pub const CANCELLED: &str = "Cancelled";
    }
}

// =========================================================================
// Re-exports — downstream code uses these short names.
// =========================================================================
pub use fund::{Entity as FundEntity, Model as FundModel};
pub use investment::{Entity as InvestmentEntity, Model as InvestmentModel};
pub use distribution::{Entity as DistributionEntity, Model as DistributionModel};
pub use subscription::{
    Entity as SubscriptionEntity, Model as SubscriptionModel, status as subscription_status,
};
pub use redemption::{
    Entity as RedemptionEntity, Model as RedemptionModel, status as redemption_status,
};
pub use fund::status as fund_status;
