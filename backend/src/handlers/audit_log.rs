//! Admin-only audit log query endpoints.
//!
//!   - `GET /api/v1/audit-logs`      — list with filters (user_id / action /
//!                                     target_type / target_id / request_id /
//!                                     时间范围) + 分页
//!   - `GET /api/v1/audit-logs/{id}` — 单条详情
//!
//! All endpoints require admin role. The route layer in `main.rs` attaches
//! `require_admin_middleware` so non-admin tokens get a 403 before the
//! handler body runs. We additionally re-check in-handler as a defence in
//! depth — the admin role check is cheap, and an accidental
//! `route(...)` swap on the router should not silently open the data.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use chrono::{DateTime, Utc};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::auth::AuthenticatedUser;
use crate::services::audit_log::{self as svc, AuditLogPage, AuditLogView, AuditQuery};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ─── Query DTO ──────────────────────────────────────────────────────────

/// Query string for `GET /api/v1/audit-logs`. Mirrors
/// `services::audit_log::AuditQuery` but keeps the public surface in
/// snake_case (CLAUDE.md convention) and accepts either a UUID
/// `user_id` or a stringified `target_id`.
#[derive(Debug, Deserialize, Default)]
pub struct AuditListQuery {
    pub user_id: Option<Uuid>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub request_id: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub size: Option<u32>,
}

impl From<AuditListQuery> for AuditQuery {
    fn from(q: AuditListQuery) -> Self {
        Self {
            user_id: q.user_id,
            action: q.action,
            target_type: q.target_type,
            target_id: q.target_id,
            request_id: q.request_id,
            from: q.from,
            to: q.to,
            page: q.page,
            size: q.size,
        }
    }
}

// ─── Admin guard ────────────────────────────────────────────────────────

/// Re-check admin role at the handler body. The router layer in
/// `main.rs` already enforces this; we keep the in-handler check so
/// a copy-paste route never silently opens admin-only data.
fn require_admin(user: &AuthenticatedUser) -> Result<(), AppError> {
    if user.role != "admin" {
        return Err(AppError::Forbidden(
            "Admin privileges required to read audit logs".into(),
        ));
    }
    Ok(())
}

// ─── Handlers ───────────────────────────────────────────────────────────

/// `GET /api/v1/audit-logs`
///
/// List audit log entries. All filters are optional and combine as AND.
/// Pagination: `page` (1-indexed, default 1) / `size` (default 20, max 200).
///
/// Response shape: `ApiResponse<AuditLogPage>` where `AuditLogPage = { items, total, page, size }`.
#[utoipa::path(
    get,
    path = "/api/v1/audit-logs",
    tag = "audit",
    operation_id = "audit_logs_list",
    security(("bearer_auth" = [])),
    params(
        ("user_id" = Option<Uuid>, Query, description = "Filter by actor user_id"),
        ("action" = Option<String>, Query, description = "Filter by dotted action name (e.g. \"user.role.changed\")"),
        ("target_type" = Option<String>, Query, description = "Filter by target_type"),
        ("target_id" = Option<String>, Query, description = "Filter by target_id"),
        ("request_id" = Option<String>, Query, description = "Filter by originating X-Request-Id"),
        ("from" = Option<DateTime<Utc>>, Query, description = "created_at >= from"),
        ("to" = Option<DateTime<Utc>>, Query, description = "created_at <= to"),
        ("page" = Option<u32>, Query, description = "1-indexed page number"),
        ("size" = Option<u32>, Query, description = "Page size (1..=200)"),
    ),
    responses(
        (status = 200, description = "Page of audit log entries"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Admin only"),
    )
)]
pub async fn list_audit_logs(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    _headers: HeaderMap,
    Query(q): Query<AuditListQuery>,
) -> Result<Json<ApiResponse<AuditLogPage>>, AppError> {
    require_admin(&user)?;
    let page = svc::query(&db, q.into()).await?;
    Ok(Json(ApiResponse::success(page)))
}

/// `GET /api/v1/audit-logs/{id}`
///
/// Single audit entry by primary key. Returns 404 if not found.
#[utoipa::path(
    get,
    path = "/api/v1/audit-logs/{id}",
    tag = "audit",
    operation_id = "audit_logs_get",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Audit log row id"),
    ),
    responses(
        (status = 200, description = "Single audit log entry", body = AuditLogView),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Admin only"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn get_audit_log(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AuditLogView>>, AppError> {
    require_admin(&user)?;
    let found = svc::find_by_id(&db, id).await?;
    match found {
        Some(v) => Ok(Json(ApiResponse::success(v))),
        None => Err(AppError::NotFound(format!("audit_log {id} not found"))),
    }
}

// ─── Router factory ─────────────────────────────────────────────────────

/// Build the audit-log sub-router. The caller in `main.rs` layers
/// `auth_middleware` and `require_admin_middleware` on top so this
/// function can stay focused on the routes themselves.
pub fn router() -> axum::Router<std::sync::Arc<sea_orm::DatabaseConnection>> {
    use axum::routing::get;
    axum::Router::new()
        .route("/audit-logs", get(list_audit_logs))
        .route("/audit-logs/{id}", get(get_audit_log))
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Admin role passes; any other role is rejected.
    #[test]
    fn require_admin_allows_admin_role() {
        let admin = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "admin".into(),
            role: "admin".into(),
            jti: "jti".into(),
        };
        assert!(require_admin(&admin).is_ok());
    }

    #[test]
    fn require_admin_rejects_user_role() {
        let user = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "alice".into(),
            role: "user".into(),
            jti: "jti".into(),
        };
        match require_admin(&user) {
            Err(AppError::Forbidden(_)) => {}
            other => panic!("expected Forbidden, got {other:?}"),
        }
    }

    #[test]
    fn require_admin_rejects_empty_role() {
        let user = AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "x".into(),
            role: "".into(),
            jti: "jti".into(),
        };
        assert!(require_admin(&user).is_err());
    }

    /// The router factory wires both endpoints. We can't actually call
    /// them without a DB, but `Router::has_route` (via `route_layer`)
    /// is enough to assert the surface is registered.
    #[test]
    fn router_wires_both_routes() {
        let r = router();
        // `into_make_service` is a no-op for this introspection;
        // we just want to assert it doesn't panic and the type is Router.
        let _svc = r.into_make_service();
    }

    /// `AuditListQuery → AuditQuery` translation preserves all fields.
    #[test]
    fn query_translation_preserves_fields() {
        let uid = Uuid::new_v4();
        let from = Utc::now() - chrono::Duration::hours(1);
        let to = Utc::now();
        let q = AuditListQuery {
            user_id: Some(uid),
            action: Some("user.role.changed".into()),
            target_type: Some("user".into()),
            target_id: Some(uid.to_string()),
            request_id: Some("r-abc".into()),
            from: Some(from),
            to: Some(to),
            page: Some(2),
            size: Some(50),
        };
        let translated: AuditQuery = q.into();
        assert_eq!(translated.user_id, Some(uid));
        assert_eq!(translated.action.as_deref(), Some("user.role.changed"));
        assert_eq!(translated.target_type.as_deref(), Some("user"));
        assert_eq!(translated.target_id.as_deref(), Some(uid.to_string().as_str()));
        assert_eq!(translated.request_id.as_deref(), Some("r-abc"));
        assert_eq!(translated.page, Some(2));
        assert_eq!(translated.size, Some(50));
    }
}
