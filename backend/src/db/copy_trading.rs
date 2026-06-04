//! SeaORM entities for the Copy Trading module — PRD §6 commercialization-2.
//!
//! Four tables back the full copy-trading lifecycle:
//!
//! - `copy_traders`        — the trader profile (display name, bio, aggregated P&L).
//! - `copy_subscriptions`  — one row per (follower, trader) with risk limits.
//! - `copy_trades`         — append-only log of every copied order (audit).
//! - `copy_profit_shares`  — period profit allocation between trader + follower.
//!
//! All four entities are colocated in this single file per the §6-2
//! task spec ("一个文件, 多个 entity"). They're declared as inline
//! `mod` blocks below; downstream code uses the public re-exports
//! (`TraderEntity`, `SubscriptionEntity`, `CopyTradeEntity`,
//! `ProfitShareEntity`).
//!
//! ## Why we model subscriptions + trades as separate state machines
//!
//! A subscription moves Active → Paused (user explicitly pauses copying
//! while keeping the relationship) or → Cancelled (terminal). A trade is
//! append-only — a failed copy never writes a row, and a successful one
//! is the join key for the follower's audit history.
//!
//! ## Indexes
//!
//! All `trader_id` columns are indexed (the hot path is "fan-out from
//! trader order → all active subscribers"). `monthly_pnl` is the trader
//! list ordering column. `(follower_id, trader_id)` is unique on
//! `copy_subscriptions` to prevent the same user from creating two
//! competing active rows for the same trader — a resubscribe after
//! unsubscribe creates a new row (preserving audit), but a fresh active
//! row is unique on (follower, trader) by index.
//!
//! See `migrations/20260604130000_create_copy_trading_tables.sql` for
//! the raw DDL.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// =========================================================================
// 1) copy_traders — the trader profile (display + aggregated stats).
// =========================================================================
pub mod trader {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "copy_traders")]
    pub struct Model {
        /// UUID v4 PK.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning user. UNIQUE: a user can register as a trader at most once.
        pub user_id: Uuid,
        /// Trader-facing display name. VARCHAR(120) like PAMM funds.
        #[sea_orm(column_type = "String(StringLen::N(120))")]
        pub display_name: String,
        /// Free-text bio. Shown in the detail view.
        #[sea_orm(column_type = "Text")]
        pub bio: Option<String>,
        /// All-time realised P&L. Updated by `update_trader_stats` on a
        /// periodic cron (PRD §6-2 Part 2).
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub total_pnl: Decimal,
        /// Trailing 30-day realised P&L. The list-view sort key.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub monthly_pnl: Decimal,
        /// Win rate over recent closed trades (0.55 = 55%). Bounded [0, 1].
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub win_rate: Decimal,
        /// Cached follower count. Maintained on subscribe / unsubscribe
        /// inside the service layer (cheap denormalisation to keep the
        /// trader list view O(1)).
        pub follower_count: i64,
        /// `Active` accepts new followers, `Paused` blocks new subscribes
        /// but keeps existing ones live, `Banned` is admin-imposed and
        /// halts all fan-out (the order path no longer routes here).
        #[sea_orm(column_type = "String(StringLen::N(16))")]
        pub status: String,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::super::user::Entity",
            from = "Column::UserId",
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

    /// Well-known status strings for `copy_traders.status`.
    pub mod status {
        pub const ACTIVE: &str = "Active";
        pub const PAUSED: &str = "Paused";
        pub const BANNED: &str = "Banned";
    }
}

// =========================================================================
// 2) copy_subscriptions — follower ↔ trader with risk limits.
// =========================================================================
pub mod subscription {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "copy_subscriptions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub trader_id: Uuid,
        pub follower_id: Uuid,
        /// Copy ratio: 0.10 = follower copies 10% of the trader's notional.
        /// Bounded [0, 1] at the DB level (see CHECK constraint).
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub ratio: Decimal,
        /// Per-trade cap: follower won't be copied into a position larger
        /// than this. 0 = no cap.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub max_position_size: Decimal,
        /// Daily P&L loss limit. When the follower's copy positions' mark
        /// P&L drops below -max_loss_per_day in a calendar day, the
        /// subscription auto-pauses and the user is notified.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub max_loss_per_day: Decimal,
        /// `Active` receives copy orders, `Paused` skipped by the fan-out
        /// (the relationship is preserved), `Cancelled` is terminal and
        /// the fan-out never looks at the row.
        #[sea_orm(column_type = "String(StringLen::N(16))")]
        pub status: String,
        pub started_at: DateTimeUtc,
        /// Set when the user unsubscribes; null while the relationship is
        /// still live (Active or Paused).
        pub ended_at: Option<DateTimeUtc>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    pub mod status {
        pub const ACTIVE: &str = "Active";
        pub const PAUSED: &str = "Paused";
        pub const CANCELLED: &str = "Cancelled";
    }
}

// =========================================================================
// 3) copy_trades — append-only log of copied orders (audit trail).
// =========================================================================
pub mod trade {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "copy_trades")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub subscription_id: Uuid,
        /// Nullable for the strategy-link path where the trader's signal
        /// doesn't always materialise as an `orders` row.
        pub original_order_id: Option<Uuid>,
        /// NEVER NULL — the follower's order. If the copy was rejected
        /// (risk check), no row is written here.
        pub copied_order_id: Uuid,
        pub symbol: String,
        /// `buy` | `sell` (string for DB portability, same shape as
        /// `orders.side` after a `to_string()` round-trip).
        #[sea_orm(column_type = "String(StringLen::N(8))")]
        pub side: String,
        /// Follower's qty, post-ratio scaling.
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub qty: Decimal,
        /// Price the follower was filled at (typically equal to the
        /// trader's fill; we don't simulate slippage yet).
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub price: Decimal,
        /// `Copied` (order placed) | `Filled` (matched). Updated when the
        /// copied order transitions.
        #[sea_orm(column_type = "String(StringLen::N(16))")]
        pub status: String,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}

    pub mod status {
        pub const COPIED: &str = "Copied";
        pub const FILLED: &str = "Filled";
        pub const REJECTED: &str = "Rejected";
    }
}

// =========================================================================
// 4) copy_profit_shares — period profit allocation (trader + follower).
// =========================================================================
pub mod profit_share {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "copy_profit_shares")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub subscription_id: Uuid,
        pub period_start: DateTimeUtc,
        pub period_end: DateTimeUtc,
        /// Net profit for the follower across the period. Negative
        /// allowed (a losing month for the follower).
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub follower_profit: Decimal,
        /// Trader's cut. Always non-negative: a loss month for the
        /// follower means trader_profit = 0 (we don't share the
        /// downside).
        #[sea_orm(column_type = "Decimal(Some((20, 8)))")]
        pub trader_profit: Decimal,
        /// Performance fee applied (e.g. 0.30 = 30%). Captured for audit.
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub trader_fee_pct: Decimal,
        pub distributed_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// =========================================================================
// Re-exports — downstream code uses these short names.
// =========================================================================
pub use trader::{
    Entity as TraderEntity, Model as TraderModel, status as trader_status,
};
pub use subscription::{
    Entity as SubscriptionEntity, Model as SubscriptionModel, status as subscription_status,
};
pub use trade::{Entity as CopyTradeEntity, Model as CopyTradeModel, status as copy_trade_status};
pub use profit_share::{
    Entity as ProfitShareEntity, Model as ProfitShareModel,
};
