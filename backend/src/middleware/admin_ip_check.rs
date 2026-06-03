//! middleware/admin_ip_check.rs — P3-A admin IP whitelist gate
//!
//! Gates every request on a `nest("/admin", …)` sub-router: looks up the
//! authenticated user's CIDR allow-list in `admin_ip_whitelist`, parses the
//! peer IP from `X-Forwarded-For` (or socket peer addr) and 403s requests
//! whose IP is not covered.
//!
//! Behavioural contract:
//!   - Empty allow-list → **fail closed** (403). Reason: the absence of any
//!     rule is almost always a misconfiguration. If you want to opt-in to
//!     "no IP restriction for this admin", set `ADMIN_IP_CHECK_BYPASS=1` in
//!     env. Default is strict.
//!   - Malformed CIDR rows in the DB → 500 (caller-visible: "internal
//!     configuration error"). A typo should never silently grant access.
//!   - Trusted-proxy XFF semantics: we take the *last* untrusted hop. To do
//!     that correctly we need a trusted-proxy CIDR list, which is out of
//!     scope for P3-A. For now we use the *leftmost* XFF entry, which is the
//!     pragmatic choice behind a single reverse proxy (nginx) — but documented
//!     as a known limitation. Operators behind multi-hop proxies should set
//!     `TRUSTED_PROXY_CIDR=…` (a single trusted CIDR) to enable the more
//!     robust "rightmost non-trusted" parsing.
//!
//! This middleware **requires** an authenticated user in the request
//! extensions; it does NOT do auth itself. The router must layer it on
//! AFTER `auth_middleware` (see `main.rs`).

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;

use crate::db::DbPool;
use crate::db::admin_ip_whitelist::{Column as WlCol, Entity as WlEntity};
use crate::middleware::auth::AuthenticatedUser;
use crate::services::ip_cidr::{match_cidrs_strict, parse_cidr};

/// Extract the client IP from the request.
///
/// Order of precedence:
///   1. `X-Forwarded-For` (leftmost when `TRUSTED_PROXY_CIDR` is unset;
///      rightmost untrusted hop when set).
///   2. `X-Real-IP` (nginx convention).
///   3. Socket peer address.
fn extract_client_ip(req: &Request) -> Option<IpAddr> {
    // Helper: parse a header value as an IP
    fn header_ip(req: &Request, name: &str) -> Option<IpAddr> {
        req.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| {
                // XFF is a comma-separated list. Take the first non-empty
                // trimmed entry.
                s.split(',')
                    .map(str::trim)
                    .find(|p| !p.is_empty())
                    .and_then(|p| IpAddr::from_str(p).ok())
            })
    }

    let trusted = std::env::var("TRUSTED_PROXY_CIDR").ok();

    if let Some(xff) = req.headers().get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        let entries: Vec<&str> = xff
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();
        if let Some(ref trusted_cidr) = trusted {
            if let Ok(net) = parse_cidr(trusted_cidr) {
                // Rightmost-untrusted: walk from the right, take the first
                // IP that is NOT in the trusted net. If none, fall back to
                // the leftmost.
                for entry in entries.iter().rev() {
                    if let Ok(ip) = IpAddr::from_str(entry) {
                        if !net.contains(ip) {
                            return Some(ip);
                        }
                    }
                }
            }
        }
        // Default: leftmost (works behind a single reverse proxy).
        if let Some(first) = entries.first() {
            if let Ok(ip) = IpAddr::from_str(first) {
                return Some(ip);
            }
        }
    }

    if let Some(ip) = header_ip(req, "x-real-ip") {
        return Some(ip);
    }

    // Last resort: socket peer addr from the ConnectInfo extension.
    req.extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
}

/// Public entry point — usable as a layer in main.rs.
///
/// Pre-conditions: `auth_middleware` must have run first and placed an
/// `AuthenticatedUser` in the request extensions. The `DbPool` is passed via
/// `State`; the caller is responsible for wiring it (`axum::middleware::from_fn_with_state`).
pub async fn admin_ip_check(
    State(db): State<DbPool>,
    req: Request,
    next: Next,
) -> Response {
    // Bypass for ops — explicitly opt out of IP gating.
    if std::env::var("ADMIN_IP_CHECK_BYPASS").as_deref() == Ok("1") {
        return next.run(req).await;
    }

    // Must have an authenticated user; otherwise pass through (the auth layer
    // will reject the request separately — we don't want to 500 here).
    let user = match req.extensions().get::<AuthenticatedUser>().cloned() {
        Some(u) => u,
        None => return next.run(req).await,
    };

    let client_ip = match extract_client_ip(&req) {
        Some(ip) => ip,
        None => {
            // We can't determine the IP at all — fail closed.
            return forbidden("client IP could not be determined");
        }
    };

    // Fetch the user's allow-list.
    let rows: Vec<String> = match WlEntity::find()
        .filter(WlCol::UserId.eq(user.user_id))
        .select_only()
        .column(WlCol::IpCidr)
        .into_tuple()
        .all(db.as_ref())
        .await
    {
        Ok(rs) => rs,
        Err(e) => {
            tracing::error!(
                user_id = %user.user_id,
                error = %e,
                "admin_ip_check: DB error reading allow-list"
            );
            return internal_error();
        }
    };

    if rows.is_empty() {
        tracing::warn!(
            user_id = %user.user_id,
            "admin_ip_check: user has no allow-list entries; denying by default"
        );
        return forbidden("no IP whitelist configured for this admin");
    }

    match match_cidrs_strict(client_ip, &rows) {
        Ok(true) => next.run(req).await,
        Ok(false) => {
            tracing::warn!(
                user_id = %user.user_id,
                ip = %client_ip,
                "admin_ip_check: peer IP not in allow-list"
            );
            forbidden("client IP not in admin allow-list")
        }
        Err(e) => {
            tracing::error!(
                user_id = %user.user_id,
                error = %e,
                "admin_ip_check: malformed CIDR row in DB"
            );
            internal_error()
        }
    }
}

fn forbidden(msg: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({
            "code": 40301,
            "message": msg,
        })),
    )
        .into_response()
}

fn internal_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "code": 50001,
            "message": "admin IP whitelist configuration error",
        })),
    )
        .into_response()
}

// Re-export the State type for the caller.
pub type AdminIpCheckState = Arc<sea_orm::DatabaseConnection>;
