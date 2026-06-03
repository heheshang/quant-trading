//! SeaORM Entity for `audit_logs` table.
//!
//! PRD §3.16 — every privileged / mutating action the platform takes must
//! be recorded with enough fidelity that security & compliance can later
//! answer "who did what to which resource, when, from where, and what
//! changed". Concretely:
//!
//!   - `user_id`        — the actor; nullable so system-initiated writes
//!                        (cron, MQ consumer, migration scripts) can be
//!                        attributed to a sentinel like `00000000-...`
//!   - `action`         — dotted namespace, e.g.
//!                        `user.role.changed`, `api_key.created`,
//!                        `risk_rule.updated`, `auth.2fa_enabled`
//!   - `target_type`    — entity class (`user`, `api_key`, `risk_rule`)
//!   - `target_id`      — stringified primary key (kept as `String` so
//!                        we can log against any table without type
//!                        gymnastics; UUIDs stringified, decimals as well)
//!   - `diff`           — JSONB `{before: ..., after: ...}`. For create
//!                        actions `before` is null; for delete actions
//!                        `after` is null. Mutations capture both halves.
//!   - `ip_address`     — peer IP captured at the request boundary
//!                        (X-Forwarded-For aware, see request_id middleware)
//!   - `user_agent`     — optional UA string
//!   - `request_id`     — optional X-Request-Id (correlates with logs /
//!                        P0-3 request id middleware)
//!   - `created_at`     — TIMESTAMPTZ
//!
//! Indexes (see migration `m20260603120000_create_audit_logs.rs`):
//!   - `(user_id, created_at)`       — "what did user X do recently"
//!   - `(action, created_at)`        — "all `api_key.created` events"
//!   - `(target_type, target_id)`    — "history of resource Y"
//!   - `(created_at)`                — TTL / range scans / cron cleanup
//!   - `(request_id)`                — "show me everything that happened
//!                                     in HTTP request `r-abc123`"
//!
//! The table is **append-only by convention** — no service path performs
//! an UPDATE or DELETE on it. A future scheduled worker (P3-? retention)
//! will age out rows older than the configured retention window.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    /// Primary key, UUID v4. Not sequential — a `Uuid::new_v4()` lets us
    /// avoid the hot-row contention a `BIGSERIAL` would create when
    /// bursts of audits land at the same instant.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// The actor. Nullable for system / background writes — callers that
    /// produce an audit from a non-user context pass `None` and we record
    /// the action under the special "system" identity downstream.
    pub user_id: Option<Uuid>,
    /// Dotted action name, e.g. `user.role.changed`. We deliberately do
    /// NOT constrain this with an enum at the DB layer — new actions land
    /// frequently (every feature adds at least one) and a migration per
    /// action would be operationally painful.
    #[sea_orm(column_type = "String(StringLen::N(120))")]
    pub action: String,
    /// The class of the affected resource: `user`, `api_key`,
    /// `risk_rule`, `auth`, etc. Indexed together with `target_id` so
    /// "all audits touching resource X" is a cheap lookup.
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub target_type: String,
    /// Stringified primary key of the resource. Kept as `String` (not
    /// `Uuid`) so a future table with a non-UUID PK can be audited
    /// without changing the column type.
    #[sea_orm(column_type = "String(StringLen::N(128))")]
    pub target_id: String,
    /// `{before: ..., after: ...}`. JSONB so we can query into the
    /// payload (e.g. "find all role changes to admin") and so callers
    /// pick their own before/after shape per resource.
    #[sea_orm(column_type = "JsonBinary")]
    pub diff: Json,
    /// Peer IP at request time, captured before any post-middleware
    /// X-Forwarded-For rewriting. The `request_id` middleware
    /// (P0-3) populates the connection-level IP in the request
    /// extensions; the audit hook reads it from there.
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub ip_address: String,
    /// Optional UA string. Stored on the row directly (rather than
    /// pushed to a separate table) because most consumers want
    /// `agent`-filtered lists in the same query as the rest of the
    /// payload.
    pub user_agent: Option<String>,
    /// Optional `X-Request-Id`. Pairs with the P0-3 middleware so an
    /// operator can pull every audit row that fired inside a single
    /// HTTP request — invaluable when the request itself spans several
    /// services (e.g. risk check + audit + notifier).
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub request_id: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Well-known sentinel UUID representing the "system" actor — used when
/// a write did not originate from an authenticated user (cron, MQ
/// consumer, migration). Recording it explicitly (rather than `NULL`)
/// keeps the user_id column indexed-friendly: every row is a valid
/// member of the user_id space.
pub const SYSTEM_ACTOR_ID: Uuid = Uuid::nil();

/// Well-known action constants. Service / handler code is encouraged
/// to use these rather than string literals so a future refactor that
/// tightens the namespace catches every call-site at compile time.
pub mod actions {
    // ─── User & role changes (admin scope) ───────────────────────
    pub const USER_ROLE_CHANGED: &str = "user.role.changed";
    pub const USER_UPDATED: &str = "user.updated";
    pub const USER_DELETED: &str = "user.deleted";
    pub const USER_DEACTIVATED: &str = "user.deactivated";
    // api key
    pub const API_KEY_CREATED: &str = "api_key.created";
    pub const API_KEY_UPDATED: &str = "api_key.updated";
    pub const API_KEY_DELETED: &str = "api_key.deleted";

    // ─── Risk rules ──────────────────────────────────────────────
    pub const RISK_RULE_UPDATED: &str = "risk_rule.updated";

    // ─── Auth (self-service) ─────────────────────────────────────
    pub const AUTH_PASSWORD_CHANGED: &str = "auth.password.changed";
    pub const AUTH_2FA_ENABLED: &str = "auth.2fa_enabled";
    pub const AUTH_2FA_DISABLED: &str = "auth.2fa_disabled";
    pub const AUTH_LOGIN_SUCCESS: &str = "auth.login.success";
    pub const AUTH_LOGIN_FAILED: &str = "auth.login.failed";

    // ─── Notification channel binding ────────────────────────────
    pub const TELEGRAM_BOUND: &str = "telegram.bound";
    pub const TELEGRAM_UNBOUND: &str = "telegram.unbound";
}

/// Well-known target types. Mirror the table names whose mutations we
/// audit, plus a few virtual ones (`auth`, `session`) that don't have
/// a 1:1 backing table.
pub mod target_types {
    pub const USER: &str = "user";
    pub const API_KEY: &str = "api_key";
    pub const RISK_RULE: &str = "risk_rule";
    pub const AUTH: &str = "auth";
    pub const TELEGRAM: &str = "telegram";
}
