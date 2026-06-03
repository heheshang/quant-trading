//! handlers/admin_ip_whitelist.rs — P3-A admin IP whitelist CRUD
//!
//! Three endpoints, all on `/api/v1/admin/ip-whitelist`:
//!   GET    /api/v1/admin/ip-whitelist          — list entries for the current admin
//!   POST   /api/v1/admin/ip-whitelist          — add a CIDR to the current admin
//!   DELETE /api/v1/admin/ip-whitelist/{id}     — remove a CIDR by id (owned by current admin)
//!
//! Access control:
//!   - Caller must be authenticated (auth_middleware) AND have role="admin"
//!     (checked in `require_admin`).
//!   - The IP gate (`admin_ip_check` middleware) must also pass. The router
//!     is responsible for layering both — see `main.rs`.
//!
//! Per-user scoping:
//!   - An admin can only see/manage their OWN allow-list. There is no
//!     cross-admin view; the API would need a separate audit-only route
//!     for that, which is intentionally out of scope for P3-A.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::DbPool;
use crate::db::admin_ip_whitelist::{
    ActiveModel as WlActive, Column as WlCol, Entity as WlEntity, Model as WlModel,
};
use crate::middleware::auth::AuthenticatedUser;
use crate::services::ip_cidr::{canonicalise, parse_cidr};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ─── DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddIpRequest {
    /// IPv4 or IPv6 CIDR, e.g. "192.168.1.0/24" or "10.0.0.5".
    /// Single hosts are accepted and normalised to /32 or /128.
    pub ip_cidr: String,
    /// Optional human label ("Office VPN", "Home", etc.). Defaults to "".
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct IpEntryResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub ip_cidr: String,
    pub label: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<WlModel> for IpEntryResponse {
    fn from(m: WlModel) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            ip_cidr: m.ip_cidr,
            label: m.label,
            created_at: m.created_at,
        }
    }
}

// ─── Handlers ──────────────────────────────────────────────────────────

fn require_admin(user: &AuthenticatedUser) -> Result<(), AppError> {
    if user.role != "admin" {
        return Err(AppError::Forbidden("Admin privileges required".into()));
    }
    Ok(())
}

/// GET /api/v1/admin/ip-whitelist
///
/// Returns the calling admin's CIDR allow-list, ordered by created_at
/// (oldest first — admins typically want to see "what was set up first" at
/// the top).
pub async fn list_ip_whitelist(
    user: AuthenticatedUser,
    State(db): State<DbPool>,
) -> Result<Json<ApiResponse<Vec<IpEntryResponse>>>, AppError> {
    require_admin(&user)?;
    let rows = WlEntity::find()
        .filter(WlCol::UserId.eq(user.user_id))
        .all(db.as_ref())
        .await?;
    let resp: Vec<IpEntryResponse> = rows.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// POST /api/v1/admin/ip-whitelist
///
/// Validates the CIDR (rejects malformed, trims, normalises single hosts to
/// /32 or /128), and inserts a new row. Duplicates (same user_id+cidr) are
/// caught by the UNIQUE INDEX and surfaced as 409.
pub async fn add_ip_whitelist(
    user: AuthenticatedUser,
    State(db): State<DbPool>,
    Json(body): Json<AddIpRequest>,
) -> Result<(StatusCode, Json<ApiResponse<IpEntryResponse>>), AppError> {
    require_admin(&user)?;

    // Validate + normalise. parse_cidr maps to AppError::Validation (→ 400).
    let net = parse_cidr(&body.ip_cidr)?;
    let cidr = canonicalise(net);

    let label = body.label.trim().to_string();
    // Cap label length to match the column.
    if label.len() > 120 {
        return Err(AppError::Validation(
            "label must be 120 characters or fewer".into(),
        ));
    }

    let now = chrono::Utc::now();
    let new_entry = WlActive {
        id: Set(Uuid::new_v4()),
        user_id: Set(user.user_id),
        ip_cidr: Set(cidr.clone()),
        label: Set(label),
        created_at: Set(now),
    };

    let saved = match new_entry.insert(db.as_ref()).await {
        Ok(m) => m,
        Err(e) => {
            // SeaORM wraps the unique-constraint violation into DbErr; we
            // pattern-match on the message since it carries the constraint
            // name. Be liberal: anything looking like a uniqueness conflict
            // → 409.
            let msg = e.to_string();
            if msg.contains("uq_admin_ip_whitelist_user_cidr")
                || msg.contains("duplicate key")
                || msg.contains("unique constraint")
            {
                return Err(AppError::Conflict(format!(
                    "CIDR {} is already in this admin's allow-list",
                    cidr
                )));
            }
            return Err(AppError::Internal(format!(
                "failed to insert IP whitelist entry: {}",
                e
            )));
        }
    };

    tracing::info!(
        admin_id = %user.user_id,
        cidr = %saved.ip_cidr,
        "admin IP whitelist entry added"
    );

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(IpEntryResponse::from(saved))),
    ))
}

/// DELETE /api/v1/admin/ip-whitelist/{id}
///
/// Deletes an entry by id, scoped to the calling admin (a non-owner admin
/// gets 404, not 403, to avoid leaking the existence of other admins'
/// entries).
pub async fn delete_ip_whitelist(
    user: AuthenticatedUser,
    State(db): State<DbPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_admin(&user)?;

    let entry = WlEntity::find_by_id(id)
        .filter(WlCol::UserId.eq(user.user_id))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("IP whitelist entry not found".into()))?;

    let am: WlActive = entry.into();
    am.delete(db.as_ref()).await?;

    tracing::info!(
        admin_id = %user.user_id,
        entry_id = %id,
        "admin IP whitelist entry removed"
    );

    Ok(Json(ApiResponse::success(())))
}
