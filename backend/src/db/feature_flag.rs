//! SeaORM Entity for `feature_flags` table.
//!
//! PRD §3.17: lightweight in-house feature flag system (avoiding the heavy
//! `unleash` dependency). Each row is a flag with three independent switches:
//!
//!   * `enabled`              — global on/off. Short-circuit: when `false`,
//!                              no other column matters.
//!   * `user_whitelist`       — JSON array of user UUIDs; a user in this list
//!                              bypasses both the global switch and the
//!                              percentage rollout. (Whitelisted users see the
//!                              feature even if globally disabled — useful for
//!                              staging/QA where a small set of admins needs
//!                              the path live while it's hidden from
//!                              production traffic.)
//!   * `percentage_rollout`   — 0..=100. When the global switch is on, this
//!                              controls the share of users that see the
//!                              feature based on a stable hash of their
//!                              `user_id`. Idempotent per user (same user →
//!                              same decision across requests and restarts).
//!   * `metadata`             — free-form JSON, e.g. `{experiment: "v2",
//!                              control_group: "a"}`.
//!
//! Evaluation order (matches `services::feature_flag::is_enabled`):
//!   1. If `user_id` is in `user_whitelist`            → `true`
//!   2. Else if `enabled` is `false`                   → `false`
//!   3. Else                                            → `percentage_rollout`
//!      hash (fallback to `false` if no `user_id` is
//!      supplied and pct < 100).
//!
//! Schema:
//!   key                  VARCHAR(64) PK        e.g. "iceberg_order"
//!   description          VARCHAR(500)          free text shown in admin UI
//!   enabled              BOOLEAN               global switch
//!   user_whitelist       JSONB                 ["uuid", ...]
//!   percentage_rollout   INT  (0..=100)        hash-bucket share
//!   metadata             JSONB                 arbitrary
//!   created_at           TIMESTAMPTZ
//!   updated_at           TIMESTAMPTZ
//!   updated_by           UUID NULL             admin who last edited it

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "feature_flags")]
pub struct Model {
    /// Primary key, e.g. "iceberg_order", "ai_v2_predictions". VARCHAR(64)
    /// to leave room for namespaced flags like "exp.bracket_v3".
    #[sea_orm(primary_key, auto_increment = false)]
    pub key: String,
    pub description: String,
    pub enabled: bool,
    /// JSONB array of user UUIDs that always see the feature as on. Stored as
    /// `JsonBinary` so SeaORM uses the Postgres binary JSON codec.
    #[sea_orm(column_type = "JsonBinary")]
    pub user_whitelist: Json,
    /// 0..=100. Negative values are clamped to 0 on read; values > 100 are
    /// clamped to 100. Persisted as a signed INT for ergonomics in queries
    /// (no need to special-case `0`/`100`).
    pub percentage_rollout: i32,
    /// Free-form metadata, defaulting to an empty JSON object (`{}`) so the
    /// admin UI can read it unconditionally.
    #[sea_orm(column_type = "JsonBinary")]
    pub metadata: Json,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    /// The admin user who last changed the flag. Nullable so we can seed
    /// system flags before any user has touched them.
    pub updated_by: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Default well-known flag keys. The seed routine in `db::mod::run_migrations`
/// inserts these with sensible starting values if no rows exist yet.
pub mod defaults {
    /// New market data API (alternative to the WS-only path).
    pub const NEW_MARKET_DATA_API: &str = "new_market_data_api";
    /// V2 AI predictions (model trained on the larger feature set).
    pub const AI_V2_PREDICTIONS: &str = "ai_v2_predictions";
    /// Grid trading strategy.
    pub const GRID_STRATEGY: &str = "grid_strategy";
    /// Kafka-based event bus experiment.
    pub const KAFKA_EXPERIMENT: &str = "kafka_experiment";
    /// Iceberg order type (advanced parent/child).
    pub const ICEBERG_ORDER: &str = "iceberg_order";

    /// All five — `seed_default_flags` iterates over this list.
    pub const ALL: &[&str] = &[
        NEW_MARKET_DATA_API,
        AI_V2_PREDICTIONS,
        GRID_STRATEGY,
        KAFKA_EXPERIMENT,
        ICEBERG_ORDER,
    ];
}
