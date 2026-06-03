//! middleware/admin_only.rs — admin role gate
//!
//! Tiny middleware that 403s any request whose `AuthenticatedUser`
//! extension is not role="admin". Use after `auth_middleware` on
//! `/api/v1/admin/...` sub-routers.
//!
//! Why a separate middleware from `admin_ip_check`:
//!   - role check is unconditional (always admin role required);
//!   - IP check is a CIDR allow-list (per-user, fail-closed default).
//!   Both are independently useful; admin-only routes get role-only,
//!   admin-IP-gated routes (legacy P3-A wiring) get the IP check on top.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use crate::middleware::auth::AuthenticatedUser;

/// Admin role gate. Must be applied AFTER `auth_middleware`.
pub async fn require_admin_middleware(req: Request, next: Next) -> Result<Response, Response> {
    let user = req
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned();

    match user {
        Some(u) if u.role == "admin" => Ok(next.run(req).await),
        Some(_) => Err(admin_only_response(
            StatusCode::FORBIDDEN,
            "Admin role required",
        )),
        None => Err(admin_only_response(
            StatusCode::UNAUTHORIZED,
            "Authentication required",
        )),
    }
}

fn admin_only_response(code: StatusCode, message: &str) -> Response {
    use axum::Json;
    Json(serde_json::json!({
        "code": code.as_u16() as i32 * 100,
        "message": message,
    }))
    .into_response_with_status(code)
}

/// Tiny extension trait so we can set both the body and status without
/// a manual `Response::builder()`. Avoids the `axum::http::Response`
/// boilerplate at the call sites.
trait IntoResponseWithStatus {
    fn into_response_with_status(self, status: StatusCode) -> Response;
}

impl IntoResponseWithStatus for axum::Json<serde_json::Value> {
    fn into_response_with_status(self, status: StatusCode) -> Response {
        use axum::response::IntoResponse;
        let mut resp = self.into_response();
        *resp.status_mut() = status;
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use uuid::Uuid;

    fn fake_user(role: &str) -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: Uuid::new_v4(),
            username: "u".into(),
            role: role.into(),
            jti: "j".into(),
        }
    }

    #[tokio::test]
    async fn admin_role_passes_through() {
        let mut req = HttpRequest::new(Body::empty()).into_parts().0;
        // We need a Request, not parts; build the full Request.
        let mut req: Request = HttpRequest::new(Body::empty());
        req.extensions_mut().insert(fake_user("admin"));
        // The next handler never runs in this unit test — we only assert
        // the gate does not 403. If it 403s we'd get a Response back,
        // not Ok.
        let res = require_admin_middleware(req, axum::middleware::from_fn(|_r, n| async move {
            Ok::<_, Response>(n.run(_r).await)
        }).call)
        .await;
        // We expect Ok; if it's Err we know the gate fired.
        assert!(res.is_ok(), "admin should pass through");
    }

    #[tokio::test]
    async fn non_admin_role_gets_403() {
        let mut req: Request = HttpRequest::new(Body::empty());
        req.extensions_mut().insert(fake_user("user"));
        let next = axum::middleware::Next::new(|_r| async {
            Ok::<_, Response>(axum::response::Response::new(axum::body::Body::empty()))
        });
        let res = require_admin_middleware(req, next).await;
        assert!(res.is_err(), "non-admin should be 403");
    }

    #[tokio::test]
    async fn missing_user_gets_401() {
        let req: Request = HttpRequest::new(Body::empty());
        let next = axum::middleware::Next::new(|_r| async {
            Ok::<_, Response>(axum::response::Response::new(axum::body::Body::empty()))
        });
        let res = require_admin_middleware(req, next).await;
        assert!(res.is_err(), "missing user should be 401");
    }
}
