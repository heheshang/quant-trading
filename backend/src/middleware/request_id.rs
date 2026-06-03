//! Request-ID propagation middleware.
//!
//! Adopts an incoming `X-Request-Id` header (or mints a fresh UUID v4
//! when the client did not provide one) and:
//!
//! 1. Stores it on the request via `req.extensions_mut().insert(...)`
//!    so any downstream handler can pull it out with the
//!    [`RequestId`] extractor.
//! 2. Echoes the value back to the client as `X-Request-Id` on the
//!    response, so the client can correlate its call with the server
//!    log line.
//! 3. Wraps the inner future in a `tracing::info_span!` carrying the
//!    `request_id` and `trace_id` fields, so every `tracing::info!` /
//!    `warn!` / `error!` emitted while handling the request — including
//!    `tower_http::trace::TraceLayer` output — picks the fields up
//!    automatically when the JSON formatter flattens the span into
//!    the log event.
//!
//! ## OpenTelemetry integration
//!
//! The `trace_id` is read from the **current OpenTelemetry span**
//! (W3C tracecontext, 32 lowercase-hex chars). For inbound requests
//! the upstream `tracing_opentelemetry::OpenTelemetrySpanExt` layer
//! extracts the `traceparent` header (set by an upstream gateway / the
//! frontend's axios interceptor in a future patch) and starts a span
//! whose trace_id matches the caller's. For outbound calls the same
//! extension is used to inject — see
//! `http_client_inject_traceparent` helper (future).
//!
//! When no OTel span is active (e.g. tests, or when OTel is
//! disabled), `trace_id` falls back to `"none"`. The `request_id` is
//! always present. JSON log readers can pivot on either key.
//!
//! ## Layering
//!
//! Apply the layer in `main.rs` with
//!
//! ```ignore
//! use quant_trading_backend::middleware::request_id::request_id_middleware;
//!
//! app.layer(cors)
//!     .layer(middleware::from_fn(http_metrics_middleware))
//!     .layer(middleware::from_fn(request_id_middleware))   // ← here
//!     .layer(TraceLayer::new_for_http())
//! ```
//!
//! The layer is registered **before** `TraceLayer::new_for_http()` so
//! trace events emitted by `TraceLayer` (started/response/error)
//! inherit the `http_request` span and therefore carry the
//! `request_id` field.

use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use opentelemetry::trace::TraceContextExt as _;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt as _;
use uuid::Uuid;

/// HTTP header used both for ingestion (client → server) and
/// response echo (server → client).
pub const X_REQUEST_ID: &str = "x-request-id";

/// W3C `traceparent` header — set by an upstream service / load
/// balancer to propagate the trace id. The OpenTelemetry layer
/// reads this and creates a span with the same trace id, which
/// we then surface on every log line and propagate to outbound
/// HTTP calls.
pub const TRACEPARENT: &str = "traceparent";

/// Newtype wrapper around the request ID string.
///
/// Stored in `req.extensions()` by [`request_id_middleware`]; pulled
/// out by handlers via `RequestId: FromRequestParts`. Clone is cheap
/// (an `Arc<String>` internally) so handlers that need to log it can
/// do so without a heap copy.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<S> axum::extract::FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // The middleware always inserts a value before forwarding
        // the request downstream, so a missing extension is a
        // programming error (someone bypassed the layer). We still
        // fall back to a freshly-minted id rather than panicking,
        // because panicking inside an extractor would 500 the
        // request with a less actionable error.
        Ok(parts
            .extensions
            .get::<RequestId>()
            .cloned()
            .unwrap_or_else(|| RequestId(Uuid::new_v4().to_string())))
    }
}

/// Maximum length we will accept from a client-supplied
/// `X-Request-Id` header. Anything longer is almost certainly
/// abusive or a probe — we discard and mint a new one.
const MAX_INCOMING_ID_LEN: usize = 128;

/// Validate a client-supplied request ID. Returns the trimmed string
/// if it is non-empty and within the length cap, otherwise `None`.
fn sanitize_incoming(value: &HeaderValue) -> Option<String> {
    let s = value.to_str().ok()?.trim();
    if s.is_empty() || s.len() > MAX_INCOMING_ID_LEN {
        return None;
    }
    // Header values are already constrained to visible-ASCII, but
    // belt-and-braces: reject anything that could break JSON
    // encoding inside a span field.
    if s.chars().any(|c| c.is_control()) {
        return None;
    }
    Some(s.to_owned())
}

/// Read the W3C trace id (32 lowercase hex chars) from the current
/// OpenTelemetry span, or "none" if no span is active. Cheap (no
/// allocations beyond the returned String).
fn current_trace_id() -> String {
    let span = tracing::Span::current();
    let cx = span.context();
    let span_ref = cx.span();
    let sc = span_ref.span_context();
    sc.is_valid()
        .then(|| sc.trace_id().to_string())
        .unwrap_or_else(|| "none".to_string())
}

/// Core request-id middleware. Use [`request_id_layer`] from
/// `main.rs` — that helper wraps this in `axum::middleware::from_fn`
/// and exposes the conventional `tower::Layer` shape.
pub async fn request_id_middleware(mut req: Request, next: Next) -> Response {
    // 1. Resolve the ID: header → fallback UUID.
    let request_id = req
        .headers()
        .get(X_REQUEST_ID)
        .and_then(sanitize_incoming)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // 2. Make it available to handlers.
    req.extensions_mut().insert(RequestId(request_id.clone()));

    // 3. Build the tracing span that all downstream events will
    //    inherit. `http.request` keeps us close to the convention
    //    that `tower_http::trace::TraceLayer` already uses, so log
    //    readers see a single, well-known span name.
    //
    //    `trace_id` is captured here (not via a closure) so the
    //    value is fixed at span entry — even if the inner future
    //    spawns child tasks with their own spans, the parent
    //    `http_request` span carries the same trace id.
    let method = req.method().clone();
    let uri = req.uri().clone();
    let trace_id = current_trace_id();
    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        trace_id = %trace_id,
        method = %method,
        path = %uri.path(),
    );

    // 4. Run the inner future inside the span so every
    //    `tracing::*!` event picked up by the JSON formatter carries
    //    the `request_id` field via the `span` key.
    let mut response = async { next.run(req).await }.instrument(span).await;

    // 5. Echo the id back on the response, so the client (and any
    //    intermediate proxy) can correlate with the server log.
    //    `append` is used so we don't blow away an existing
    //    `X-Request-Id` header that a downstream layer might have
    //    already set deliberately.
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .append(HeaderName::from_static(X_REQUEST_ID), value);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request as HttpRequest, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    async fn echo_handler(
        axum::extract::Extension(id): axum::extract::Extension<RequestId>,
    ) -> String {
        id.0
    }

    async fn assert_id_extractor(
        axum::extract::Extension(id): axum::extract::Extension<RequestId>,
    ) -> String {
        id.0
    }

    fn build_app() -> Router {
        Router::new()
            .route("/echo", get(echo_handler))
            .route("/extract", get(assert_id_extractor))
            .layer(axum::middleware::from_fn(request_id_middleware))
    }

    #[tokio::test]
    async fn honors_incoming_header() {
        let app = build_app();
        let req = HttpRequest::builder()
            .uri("/echo")
            .header(X_REQUEST_ID, "test-abc-123")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get(X_REQUEST_ID).unwrap(),
            "test-abc-123"
        );
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"test-abc-123");
    }

    #[tokio::test]
    async fn mints_uuid_when_header_missing() {
        let app = build_app();
        let req = HttpRequest::builder()
            .uri("/echo")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let echoed = res
            .headers()
            .get(X_REQUEST_ID)
            .expect("response should carry X-Request-Id")
            .to_str()
            .unwrap()
            .to_owned();
        // UUID v4 strings parse back as Uuid
        assert!(Uuid::parse_str(&echoed).is_ok(), "{echoed} should parse as UUID");
    }

    #[tokio::test]
    async fn rejects_overlong_incoming_id() {
        let app = build_app();
        let long = "x".repeat(MAX_INCOMING_ID_LEN + 1);
        let req = HttpRequest::builder()
            .uri("/echo")
            .header(X_REQUEST_ID, &long)
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        let echoed = res
            .headers()
            .get(X_REQUEST_ID)
            .unwrap()
            .to_str()
            .unwrap();
        // Long input is discarded; a fresh UUID is used instead.
        assert_ne!(echoed, long);
        assert!(Uuid::parse_str(echoed).is_ok());
    }

    #[tokio::test]
    async fn extractor_yields_same_id_as_response_header() {
        let app = Router::new()
            .route("/extract", get(assert_id_extractor))
            .layer(axum::middleware::from_fn(request_id_middleware));
        let req = HttpRequest::builder()
            .uri("/extract")
            .header(X_REQUEST_ID, "abc")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        // Read the header *before* consuming the body so the
        // `assert_eq!` below still compiles.
        assert_eq!(res.headers().get(X_REQUEST_ID).unwrap(), "abc");
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"abc");
    }
}
