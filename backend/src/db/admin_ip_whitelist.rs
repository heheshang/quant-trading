//! SeaORM Entity for `admin_ip_whitelist` table.
//!
//! PRD §5: IP 白名单（admin 端）
//! Per-user allow-list of CIDR ranges. Used by the
//! `middleware::admin_ip_check` layer to gate every admin route: requests
//! whose peer IP is not contained in the user's allow-list get 403.
//!
//! Schema:
//!   id          UUID        PK
//!   user_id     UUID        FK → users(id) — the admin this entry belongs to
//!   ip_cidr     VARCHAR(64) e.g. "192.168.1.0/24" or "203.0.113.5"
//!   label       VARCHAR(120) free-form description ("home office", "VPN", ...)
//!   created_at  TIMESTAMPTZ
//!
//! Note: we store one row per (user, CIDR) pair — no de-dup at the DB layer
//! beyond the application-side UNIQUE INDEX (created in the migration).
//! Allowing duplicates at the row level is fine: `admin_ip_check` only cares
//! whether ANY entry matches, so two identical rows behave identically to one.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "admin_ip_whitelist")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    /// IPv4 or IPv6 CIDR. Examples: "10.0.0.0/8", "192.168.1.0/24",
    /// "203.0.113.5" (treated as a single host — `/32` for v4, `/128` for v6).
    pub ip_cidr: String,
    /// Free-form label the admin can set (e.g. "Office VPN", "Home"). Optional
    /// for backwards compatibility; defaulting to empty string in DB.
    pub label: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
