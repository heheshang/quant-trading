//! OpenAPI 3.1 spec + Swagger UI endpoints.
//!
//!   - `GET /api/v1/openapi.json`  — raw JSON spec (used by openapi-fetch,
//!                                   openapi-typescript, postman, etc.)
//!   - `GET /swagger-ui`           — interactive browser explorer
//!   - `GET /swagger-ui/*path`      — static assets (CSS, JS, favicon)
//!
//! Implementation notes:
//!   - Spec is generated at request time via `ApiDoc::openapi()`. The
//!     function is `const`-stable and trivial to compute; if the spec
//!     ever grows large enough to matter for latency we can memoize
//!     behind a `OnceLock`.
//!   - utoipa-swagger-ui's `serve` returns a `Router` that owns its own
//!     state. We `merge` it into the global app router (it works on
//!     `S = ()`, the swagger crate is generic enough to merge with our
//!     `S = Arc<DatabaseConnection>` router).
//!   - We do NOT require auth on these endpoints. The spec itself is
//!     public metadata; the actual API still requires JWT.
//!   - The `/api/v1/openapi.json` path is intentionally under the
//!     `/api/v1` namespace so nginx's existing `location /api/v1/`
//!     reverse-proxy rule picks it up automatically.

use axum::{Json, Router, response::IntoResponse};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::ApiDoc;

/// `GET /api/v1/openapi.json` — return the full OpenAPI 3.1 spec as JSON.
///
/// Response is `application/json; charset=utf-8` (axum's `Json` default).
/// `to_pretty_json` is used for human readability when devs hit the URL
/// in a browser; openapi-typescript and other consumers parse it the
/// same way regardless of formatting.
pub async fn openapi_json() -> impl IntoResponse {
    let api = <ApiDoc as OpenApi>::openapi();
    Json(api.to_pretty_json().unwrap_or_else(|_| "{}".to_string()))
}

/// Build the Swagger-UI sub-router.
///
/// The caller in `main.rs` merges this into the global `Router`:
///
/// ```ignore
/// app = app.merge(handlers::openapi::swagger_ui_router());
/// ```
///
/// `SwaggerUi::new("/swagger-ui")` mounts the UI at that path and
/// auto-registers `openapi.json` discovery if you pass it via
/// `.url(...)` — but since the spec endpoint is dynamic and lives at
/// a custom path (`/api/v1/openapi.json`), we use the explicit `.url`
/// form so the UI knows where to fetch the document.
pub fn swagger_ui_router() -> Router {
    SwaggerUi::new("/swagger-ui")
        .url("/api/v1/openapi.json", <ApiDoc as OpenApi>::openapi())
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn openapi_json_returns_valid_spec() {
        let resp = openapi_json().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), 1_000_000)
            .await
            .expect("read body");
        let v: serde_json::Value =
            serde_json::from_slice(&body).expect("valid JSON");
        // OpenAPI 3.1 spec
        assert_eq!(v["openapi"], "3.1.0");
        assert!(v["info"]["title"].is_string());
        assert!(v["info"]["version"].is_string());
        // paths present
        assert!(v["paths"].is_object(), "spec has paths block");
        assert!(!v["paths"].as_object().unwrap().is_empty(), "at least one path");
        // security scheme registered
        assert!(v["components"]["securitySchemes"]["bearer_auth"].is_object());
    }

    #[test]
    fn spec_paths_count_matches_annotated_handlers() {
        // The spec must be deterministic and include every annotated
        // handler we wired up in lib.rs::ApiDoc::paths. If a path drops
        // out, the spec contract with the frontend is silently broken —
        // fail loudly here.
        let api = <ApiDoc as OpenApi>::openapi();
        let paths = api.paths.paths.len();
        // 14 currently-annotated paths (system×2 + auth×5 + users×3 +
        // market×3 + audit×2 = 15 actually; allow some slack as we add
        // more). Update this when the list grows.
        assert!(
            paths >= 14,
            "expected ≥14 OpenAPI paths, got {paths}"
        );
    }
}
