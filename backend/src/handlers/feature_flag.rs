//! `handlers::feature_flag` — user-facing and admin HTTP endpoints for the
//! in-house feature flag system (PRD §3.17).
//!
//! Endpoints
//! ---------
//!   - `GET    /api/v1/feature-flags`            — bootstrap view: returns
//!                                                  the boolean evaluation of
//!                                                  every known flag for the
//!                                                  caller (frontend calls
//!                                                  this on app start).
//!   - `GET    /api/v1/admin/feature-flags`      — admin list of every
//!                                                  defined flag row.
//!   - `POST   /api/v1/admin/feature-flags`      — admin create / update
//!                                                  (upsert by `key`).
//!   - `DELETE /api/v1/admin/feature-flags/{key}`— admin delete.
//!
//! Auth model
//! ----------
//!   - The user bootstrap endpoint requires *any* authenticated user.
//!   - The admin endpoints require `role == "admin"`. The admin gate is
//!     applied in two places — a middleware on the route (fast reject
//!     before the handler body) and an in-handler re-check (defence in
//!     depth against a copy-paste routing mistake). See `require_admin`
//!     below.
//!
//! Layering
//! --------
//!   - `auth_middleware` injects `AuthenticatedUser` (the caller's id +
//!     role). The user bootstrap uses that id to compute the per-user
//!     `is_enabled` decision (whitelist, percentage rollout, etc.).
//!   - `require_admin_middleware` is layered *outside* `auth_middleware`
//!     so the role check sees the same `AuthenticatedUser` shape.
//!   - We thread the `DatabaseConnection` via `State`; the service-side
//!     cache (`FeatureFlagCache`) is process-wide and accessed via
//!     `services::feature_flag::is_enabled` etc.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::feature_flag as entity;
use crate::middleware::auth::AuthenticatedUser;
use crate::services::feature_flag as svc;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ─── DTOs ───────────────────────────────────────────────────────────────

/// Wire shape for a feature flag row. The `db::feature_flag::Model` is
/// the SeaORM-internal representation and uses a `Json` value for
/// `user_whitelist` / `metadata`; this DTO is what the admin UI sees and
/// what `openapi-typescript` consumes. All fields are camelCase on the
/// wire (`#[serde(rename_all = "camelCase")]`).
#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlag {
    /// Primary key, e.g. `"iceberg_order"`. VARCHAR(64) on the DB side.
    pub key: String,
    /// Free-text description shown in the admin UI.
    pub description: String,
    /// Global on/off switch. When `false`, no other column matters (other
    /// than whitelist overrides — see `services::feature_flag::evaluate_flag`).
    pub enabled: bool,
    /// JSON array of user UUIDs that always see the feature as on,
    /// regardless of `enabled` or `percentage_rollout`. Stored as JSONB.
    pub user_whitelist: serde_json::Value,
    /// 0..=100 — share of users (excluding whitelist) that see the feature.
    pub percentage_rollout: i32,
    /// Free-form JSON, e.g. `{"experiment": "v2"}`. Defaults to `{}`.
    pub metadata: serde_json::Value,
    /// DB-side row insert timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// DB-side row update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Admin user id (UUID) who last edited the row, if known.
    pub updated_by: Option<Uuid>,
}

impl From<entity::Model> for FeatureFlag {
    fn from(m: entity::Model) -> Self {
        Self {
            key: m.key,
            description: m.description,
            enabled: m.enabled,
            user_whitelist: m.user_whitelist,
            percentage_rollout: m.percentage_rollout,
            metadata: m.metadata,
            created_at: m.created_at,
            updated_at: m.updated_at,
            updated_by: m.updated_by,
        }
    }
}

/// Response shape for the user bootstrap endpoint: a flat
/// `{flag_key: bool}` map. The frontend renders UI affordances based on
/// this map; unknown keys are absent (i.e. missing → treat as off).
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FeatureFlagEvaluation {
    /// The `key` (string) of the flag. utoipa's `ToSchema` derive on a
    /// `HashMap<String, bool>` produces an open object with `additionalProperties:
    /// { type: boolean }`, which is exactly what the SPA expects.
    pub flags: std::collections::HashMap<String, bool>,
}

/// Request body for the admin upsert. Mirrors `entity::Model` but takes
/// only the editable fields — the server fills in timestamps, and
/// `updated_by` from the caller.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpsertFeatureFlagRequest {
    /// Primary key, e.g. `"iceberg_order"`. VARCHAR(64) on the DB side;
    /// the handler validates length before forwarding.
    #[schema(example = "iceberg_order")]
    pub key: String,
    /// Free-text description shown in the admin UI.
    #[schema(example = "Iceberg order type (advanced parent/child)")]
    pub description: String,
    /// Global on/off switch. When `false`, no other column matters (other
    /// than whitelist overrides — see `services::feature_flag::evaluate_flag`).
    pub enabled: bool,
    /// JSON array of user UUIDs that always see the feature as on,
    /// regardless of `enabled` or `percentage_rollout`. Stored as JSONB.
    #[serde(default)]
    pub user_whitelist: serde_json::Value,
    /// 0..=100 — share of users (excluding whitelist) that see the
    /// feature. Server-side clamped on read; we also clamp on write to
    /// keep the DB clean.
    #[schema(value_type = i32, minimum = 0, maximum = 100, example = 25)]
    pub percentage_rollout: i32,
    /// Free-form JSON, e.g. `{"experiment": "v2"}`. Defaults to `{}`
    /// when the admin doesn't supply one.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl UpsertFeatureFlagRequest {
    /// Convert to the DB model, clamping the rollout percentage. Used by
    /// the admin upsert handler.
    fn into_model(self, updated_by: Uuid) -> Result<entity::Model, AppError> {
        if self.key.is_empty() || self.key.len() > 64 {
            return Err(AppError::BadRequest(
                "key must be 1..=64 chars".into(),
            ));
        }
        // Coerce whitelist to a JSON array. Accepting any JSON shape here
        // would let the admin break the evaluator; reject anything that
        // isn't a JSON array (or null).
        let whitelist = match self.user_whitelist {
            serde_json::Value::Null => serde_json::json!([]),
            serde_json::Value::Array(_) => self.user_whitelist,
            other => {
                return Err(AppError::BadRequest(format!(
                    "user_whitelist must be a JSON array, got {}",
                    other
                )));
            }
        };
        let metadata = if self.metadata.is_null() {
            serde_json::json!({})
        } else if !self.metadata.is_object() {
            return Err(AppError::BadRequest(
                "metadata must be a JSON object".into(),
            ));
        } else {
            self.metadata
        };
        let now = chrono::Utc::now();
        Ok(entity::Model {
            key: self.key,
            description: self.description,
            enabled: self.enabled,
            user_whitelist: whitelist,
            percentage_rollout: self.percentage_rollout.clamp(0, 100),
            metadata,
            created_at: now,
            updated_at: now,
            updated_by: Some(updated_by),
        })
    }
}

// ─── Admin guard ────────────────────────────────────────────────────────

/// In-handler re-check. The router layer in `main.rs` already enforces
/// admin role via `require_admin_middleware`; this is the defence in
/// depth.
fn require_admin(user: &AuthenticatedUser) -> Result<(), AppError> {
    if user.role != "admin" {
        return Err(AppError::Forbidden(
            "Admin privileges required to manage feature flags".into(),
        ));
    }
    Ok(())
}

// ─── Handlers ───────────────────────────────────────────────────────────

/// `GET /api/v1/feature-flags`
///
/// User bootstrap: returns the per-user evaluation of every known flag.
/// Frontend caches this map and uses it via `useFeatureFlag(key)`. We
/// evaluate against the *current* caller's `user_id` so the response
/// naturally captures whitelist + percentage rollout.
#[utoipa::path(
    get,
    path = "/api/v1/feature-flags",
    tag = "feature-flag",
    operation_id = "feature_flags_evaluate",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Per-user boolean evaluation of every known flag",
         body = FeatureFlagEvaluation),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn evaluate_for_user(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<FeatureFlagEvaluation>>, AppError> {
    let flags = svc::evaluate_all(svc::init_cache().as_ref(), db.as_ref(), Some(&user.user_id)).await;
    Ok(Json(ApiResponse::success(FeatureFlagEvaluation { flags })))
}

/// `GET /api/v1/admin/feature-flags`
///
/// Admin list: every flag row (raw, no per-user evaluation). The admin
/// UI reads this to populate the toggle table.
#[utoipa::path(
    get,
    path = "/api/v1/admin/feature-flags",
    tag = "feature-flag",
    operation_id = "feature_flags_admin_list",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All defined feature flag rows"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Admin only"),
    )
)]
pub async fn admin_list(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<Vec<FeatureFlag>>>, AppError> {
    require_admin(&user)?;
    let rows = svc::list(db.as_ref())
        .await?
        .into_iter()
        .map(FeatureFlag::from)
        .collect();
    Ok(Json(ApiResponse::success(rows)))
}

/// `POST /api/v1/admin/feature-flags`
///
/// Admin upsert: create or update a flag row by `key`. The response is
/// the persisted row. The cache is invalidated automatically by
/// `svc::upsert`.
#[utoipa::path(
    post,
    path = "/api/v1/admin/feature-flags",
    tag = "feature-flag",
    operation_id = "feature_flags_admin_upsert",
    security(("bearer_auth" = [])),
    request_body = UpsertFeatureFlagRequest,
    responses(
        (status = 200, description = "Persisted flag row", body = FeatureFlag),
        (status = 400, description = "Validation error (key length, whitelist shape, ...)"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Admin only"),
    )
)]
pub async fn admin_upsert(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(body): Json<UpsertFeatureFlagRequest>,
) -> Result<Json<ApiResponse<FeatureFlag>>, AppError> {
    require_admin(&user)?;
    let model = body.into_model(user.user_id)?;
    let persisted = svc::upsert(db.as_ref(), model).await?;
    Ok(Json(ApiResponse::success(FeatureFlag::from(persisted))))
}

/// `DELETE /api/v1/admin/feature-flags/{key}`
///
/// Admin delete. Returns 204 if a row was actually removed, 404
/// otherwise. The cache is invalidated automatically.
#[utoipa::path(
    delete,
    path = "/api/v1/admin/feature-flags/{key}",
    tag = "feature-flag",
    operation_id = "feature_flags_admin_delete",
    security(("bearer_auth" = [])),
    params(
        ("key" = String, Path, description = "Flag primary key to delete"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Admin only"),
        (status = 404, description = "Flag with that key not found"),
    )
)]
pub async fn admin_delete(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(key): Path<String>,
) -> Result<StatusCode, AppError> {
    require_admin(&user)?;
    let removed = svc::delete_flag(svc::init_cache().as_ref(), db.as_ref(), &key).await?;
    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("feature flag {key} not found")))
    }
}

// ─── Router factory ─────────────────────────────────────────────────────

/// Build the feature-flag sub-router. The caller in `main.rs` layers
/// `auth_middleware` and (for admin routes) `require_admin_middleware`
/// on top.
///
/// We register both prefixes here so the same `router()` can be mounted
/// twice — once under `/api/v1` (for the user bootstrap) and once under
/// `/api/v1/admin` (for the admin endpoints). The router then owns the
/// paths and the caller only adds auth layers.
pub fn router() -> axum::Router<std::sync::Arc<sea_orm::DatabaseConnection>> {
    use axum::routing::{delete, get, post};
    axum::Router::new()
        // User bootstrap (any authenticated user).
        .route("/feature-flags", get(evaluate_for_user))
        // Admin surface.
        .route("/admin/feature-flags", get(admin_list))
        .route("/admin/feature-flags", post(admin_upsert))
        .route("/admin/feature-flags/{key}", delete(admin_delete))
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn admin() -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "admin".into(),
            role: "admin".into(),
            jti: "jti".into(),
        }
    }
    fn regular_user() -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "alice".into(),
            role: "user".into(),
            jti: "jti".into(),
        }
    }

    #[test]
    fn require_admin_allows_admin_role() {
        assert!(require_admin(&admin()).is_ok());
    }

    #[test]
    fn require_admin_rejects_user_role() {
        let r = require_admin(&regular_user());
        match r {
            Err(AppError::Forbidden(_)) => {}
            other => panic!("expected Forbidden, got {other:?}"),
        }
    }

    #[test]
    fn upsert_request_rejects_empty_key() {
        let r = UpsertFeatureFlagRequest {
            key: "".into(),
            description: "x".into(),
            enabled: true,
            user_whitelist: serde_json::json!([]),
            percentage_rollout: 50,
            metadata: serde_json::json!({}),
        };
        let err = r.into_model(Uuid::new_v4()).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn upsert_request_rejects_oversized_key() {
        let r = UpsertFeatureFlagRequest {
            key: "a".repeat(65),
            description: "x".into(),
            enabled: true,
            user_whitelist: serde_json::json!([]),
            percentage_rollout: 50,
            metadata: serde_json::json!({}),
        };
        let err = r.into_model(Uuid::new_v4()).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn upsert_request_clamps_rollout() {
        let r = UpsertFeatureFlagRequest {
            key: "x".into(),
            description: "x".into(),
            enabled: true,
            user_whitelist: serde_json::json!([]),
            percentage_rollout: 250,
            metadata: serde_json::json!({}),
        };
        let m = r.into_model(Uuid::new_v4()).unwrap();
        assert_eq!(m.percentage_rollout, 100);
    }

    #[test]
    fn upsert_request_rejects_non_array_whitelist() {
        let r = UpsertFeatureFlagRequest {
            key: "x".into(),
            description: "x".into(),
            enabled: true,
            user_whitelist: serde_json::json!("not-an-array"),
            percentage_rollout: 50,
            metadata: serde_json::json!({}),
        };
        let err = r.into_model(Uuid::new_v4()).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn upsert_request_rejects_non_object_metadata() {
        let r = UpsertFeatureFlagRequest {
            key: "x".into(),
            description: "x".into(),
            enabled: true,
            user_whitelist: serde_json::json!([]),
            percentage_rollout: 50,
            metadata: serde_json::json!([]),
        };
        let err = r.into_model(Uuid::new_v4()).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn upsert_request_null_metadata_defaults_to_empty_object() {
        let r = UpsertFeatureFlagRequest {
            key: "x".into(),
            description: "x".into(),
            enabled: true,
            user_whitelist: serde_json::json!([]),
            percentage_rollout: 50,
            metadata: serde_json::Value::Null,
        };
        let m = r.into_model(Uuid::new_v4()).unwrap();
        assert_eq!(m.metadata, serde_json::json!({}));
    }

    #[test]
    fn router_wires_all_four_routes() {
        // Constructing the router is enough to assert the type is
        // well-formed. We don't actually exercise the routes — that
        // needs a DB and a real request.
        let _r: axum::Router<std::sync::Arc<sea_orm::DatabaseConnection>> = router();
    }

    #[test]
    fn feature_flag_dto_round_trip() {
        // The wire DTO is what the admin UI sees — make sure
        // camelCase + Value shapes survive a JSON round-trip.
        let m = entity::Model {
            key: "iceberg_order".into(),
            description: "iceberg".into(),
            enabled: true,
            user_whitelist: serde_json::json!([]),
            percentage_rollout: 50,
            metadata: serde_json::json!({"experiment": "v2"}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            updated_by: Some(Uuid::new_v4()),
        };
        let dto: FeatureFlag = m.into();
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"percentageRollout\""));
        assert!(json.contains("\"userWhitelist\""));
        assert!(json.contains("\"updatedBy\""));
    }
}
