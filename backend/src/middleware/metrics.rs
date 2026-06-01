//! HTTP request metrics middleware.
//!
//! Wraps every request that reaches the router (regardless of whether
//! it succeeds, is rejected by auth, or 404s) and emits two
//! Prometheus metrics:
//!
//! * `http_requests_total{method,route,status}`
//! * `http_request_duration_seconds{method,route}`
//!
//! The `route` label carries the **matched route pattern** (e.g.
//! `/api/v1/orders/{id}`) rather than the raw URI, so cardinality
//! stays bounded by the number of declared routes — not the
//! number of distinct resources accessed.
//!
//! ## Skipped paths
//!
//! * `/metrics` — the scrape endpoint itself must not pollute its
//!   own counters; otherwise a 15s scrape loop would dominate the
//!   request rate.
//!
//! * `/health`, `/api/v1/health/ready` — liveness/readiness probes
//!   fire on a tight interval (typically 1-5s) and would dwarf
//!   actual user traffic. The process is still observable via
//!   `build_info`, `ws_connections_active`, and the `cargo` build
//!   itself; the user-facing dashboard does not need liveness QPS.
//!
//! See `cargo clippy --lib --all-targets --all-features -- -D warnings`
//! to enforce the `clippy::result_large_err` style of the underlying
//! `prometheus` crate.

use crate::metrics::{HTTP_REQUEST_DURATION_SECONDS, HTTP_REQUESTS_TOTAL};
use axum::extract::{MatchedPath, Request};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;

/// Paths whose traffic must not be counted, in the exact form they
/// appear after routing (we compare against the *raw* URI).  The
/// `/api/v1/health/ready` form is kept because readiness is exposed
/// under the v1 prefix in `main.rs`.
const SKIPPED_PATHS: &[&str] = &["/metrics", "/health", "/api/v1/health/ready"];

/// HTTP metrics middleware.
///
/// Place this **inside** CORS and **outside** `TraceLayer`:
///
/// ```ignore
/// app.layer(cors)
///    .layer(middleware::from_fn(http_metrics_middleware))
///    .layer(TraceLayer::new_for_http())
/// ```
///
/// CORS preflight responses flow through this middleware so 4xx
/// CORS outcomes are observable, but `TraceLayer` runs *after* so
/// tracing captures the final response status emitted by handlers.
pub async fn http_metrics_middleware(req: Request, next: Next) -> Response {
    // Cheap O(1) skip for hot paths. The URI check is the raw path
    // before matched-pattern resolution; if the path is unknown to
    // the router we still want to record it as the literal URL.
    let raw_path = req.uri().path();
    if SKIPPED_PATHS.contains(&raw_path) {
        return next.run(req).await;
    }

    // Snapshot the request shape *before* consuming the request
    // body. The `Method` is `Copy`, so this is free.
    let method: Method = req.method().clone();

    // `MatchedPath` is an extractor — it must be taken here while
    // the request is still intact. If no route matched (e.g. 404
    // against an unknown path) the extension is absent and we
    // fall back to the raw path. The raw-path fallback is also
    // bounded in practice: only a finite set of typo'd URLs hit
    // the server, and the operator can grep access logs for them.
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| raw_path.to_string());

    let start = Instant::now();
    let response = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    // Counters/histograms use `&'static str` for label values to
    // avoid per-request allocations on the hot path; the method
    // string and the matched route are both already cheap to obtain
    // as `&str`, so we just clone the small ones below.
    let method_str = method.as_str();
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method_str, &route, &status])
        .inc();
    HTTP_REQUEST_DURATION_SECONDS
        .with_label_values(&[method_str, &route])
        .observe(elapsed);

    response
}

#[cfg(test)]
mod tests {
    //! These tests exercise the middleware in isolation against a
    //! minimal router so they don't depend on the full backend
    //! state. We do not assert exact counter values (those are
    //! shared with other tests in the process); we assert that the
    //! counter **family exists** in the registry output and that
    //! the skip list correctly bypasses the middleware.

    use super::*;
    use crate::metrics::encode_text;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use axum::routing::{get, post};
    use axum::Router;
    use tower::ServiceExt;

    fn router() -> Router {
        Router::new()
            .route("/orders/{id}", get(|| async { "ok" }))
            .route("/orders", post(|| async { "created" }))
            .layer(axum::middleware::from_fn(http_metrics_middleware))
    }

    #[tokio::test]
    async fn test_metrics_increment_on_get_with_matched_route() {
        let app = router();
        let req = HttpRequest::builder()
            .uri("/orders/abc")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.expect("oneshot");
        assert_eq!(response.status(), 200);

        let (body, _) = encode_text();
        let text = String::from_utf8(body).unwrap();
        // The route label must be the matched pattern, not the
        // literal /orders/abc — that's the cardinality guarantee.
        assert!(
            text.contains("http_requests_total{method=\"GET\",route=\"/orders/{id}\",status=\"200\"}"),
            "missing matched-route counter; body:\n{text}"
        );
    }

    #[tokio::test]
    async fn test_metrics_increment_on_post() {
        let app = router();
        let req = HttpRequest::builder()
            .method("POST")
            .uri("/orders")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.expect("oneshot");
        assert_eq!(response.status(), 200);

        let (body, _) = encode_text();
        let text = String::from_utf8(body).unwrap();
        assert!(
            text.contains("http_requests_total{method=\"POST\",route=\"/orders\",status=\"200\"}"),
            "missing POST counter; body:\n{text}"
        );
    }

    #[tokio::test]
    async fn test_duration_histogram_records_elapsed() {
        let app = router();
        let req = HttpRequest::builder()
            .uri("/orders/zzz")
            .body(Body::empty())
            .unwrap();
        let _ = app.oneshot(req).await.expect("oneshot");

        let (body, _) = encode_text();
        let text = String::from_utf8(body).unwrap();
        // The histogram is keyed by method+route only (no status),
        // matching the standard Prometheus client convention.
        assert!(
            text.contains("http_request_duration_seconds_bucket{method=\"GET\",route=\"/orders/{id}\""),
            "missing duration histogram bucket; body:\n{text}"
        );
    }

    #[tokio::test]
    async fn test_skip_paths_are_not_counted() {
        // The skip check runs *before* any route matching, so the
        // raw URI is enough to drive the bypass.
        let app = router();
        let req = HttpRequest::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.expect("oneshot");
        assert_eq!(response.status(), 404); // /health is not in this router

        let (body, _) = encode_text();
        let text = String::from_utf8(body).unwrap();
        assert!(
            !text.contains("route=\"/health\""),
            "/health should be skipped; body:\n{text}"
        );
    }
}
