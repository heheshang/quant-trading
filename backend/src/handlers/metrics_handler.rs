//! Prometheus `/metrics` endpoint.
//!
//! Emits all metrics registered in [`crate::metrics`] using the
//! standard Prometheus text-exposition format. Intended to be polled
//! by a Prometheus scraper (or scraped by Grafana Agent / Vector / etc.).
//!
//! The endpoint is **unauthenticated by design** — Prometheus scrapers
//! don't carry user credentials. In production it should be exposed
//! only on a private network or behind a firewall rule that restricts
//! source IPs to the monitoring infrastructure.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

/// `GET /metrics` — encode the global registry in text-exposition format.
#[tracing::instrument]
pub async fn metrics() -> Response {
    let (body, format_type) = crate::metrics::encode_text();
    let mut response = (StatusCode::OK, body).into_response();
    let mime: HeaderValue = format_type
        .parse()
        .unwrap_or_else(|_| HeaderValue::from_static("text/plain; version=0.0.4"));
    response.headers_mut().insert(header::CONTENT_TYPE, mime);
    // Disable caching — scrapers must see fresh values.
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::Request;
    use tower::ServiceExt;

    /// Build a router containing only the /metrics handler.
    fn router() -> axum::Router {
        axum::Router::new().route("/metrics", axum::routing::get(metrics))
    }

    #[tokio::test]
    async fn test_metrics_endpoint_returns_200() {
        let app = router();
        let req = Request::builder()
            .uri("/metrics")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.expect("oneshot");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_endpoint_content_type_is_prometheus_text() {
        let app = router();
        let req = Request::builder()
            .uri("/metrics")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.expect("oneshot");
        let ct = response
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("content-type present")
            .to_str()
            .unwrap();
        assert!(
            ct.starts_with("text/plain"),
            "content-type should start with text/plain, got {ct}"
        );
        assert!(
            ct.contains("version=0.0.4"),
            "content-type should declare Prometheus exposition version, got {ct}"
        );
    }

    #[tokio::test]
    async fn test_metrics_endpoint_disables_caching() {
        let app = router();
        let req = Request::builder()
            .uri("/metrics")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.expect("oneshot");
        let cache = response
            .headers()
            .get(header::CACHE_CONTROL)
            .expect("cache-control present")
            .to_str()
            .unwrap();
        assert_eq!(cache, "no-store");
    }

    #[tokio::test]
    async fn test_metrics_endpoint_body_contains_well_known_metrics() {
        // Touch every metric so the encoder emits at least one HELP+TYPE
        // line for each (Prometheus Vec metrics are lazy — collectors
        // without any sample don't appear in the text output).
        use crate::metrics as m;
        m::HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/test-endpoint", "200"])
            .inc();
        m::HTTP_REQUEST_DURATION_SECONDS
            .with_label_values(&["GET", "/test-endpoint"])
            .observe(0.001);
        m::ORDERS_CREATED_TOTAL
            .with_label_values(&["buy", "limit", "binance"])
            .inc();
        m::ORDERS_FILLED_TOTAL
            .with_label_values(&["buy", "binance"])
            .inc();
        m::ORDERS_CANCELLED_TOTAL
            .with_label_values(&["user", "binance"])
            .inc();
        m::POSITIONS_OPEN
            .with_label_values(&["binance", "BTCUSDT"])
            .set(1);
        m::TRIGGER_ORDERS_CREATED_TOTAL
            .with_label_values(&["StopLoss"])
            .inc();
        m::TRIGGER_ORDERS_FIRED_TOTAL
            .with_label_values(&["StopLoss"])
            .inc();
        m::RISK_RULES_TRIPPED_TOTAL
            .with_label_values(&["daily_loss"])
            .inc();
        m::RATE_LIMIT_DENIED_TOTAL
            .with_label_values(&["user"])
            .inc();
        m::WS_CONNECTIONS_ACTIVE
            .with_label_values(&["frontend"])
            .set(1);
        m::WS_MESSAGES_BROADCAST_TOTAL
            .with_label_values(&["Ticker"])
            .inc();
        m::KLINE_PERSIST_TOTAL.with_label_values(&["1m"]).inc();
        m::AI_PREDICTIONS_TOTAL
            .with_label_values(&["BTCUSDT"])
            .inc();
        m::init_build_info();

        let app = router();
        let req = Request::builder()
            .uri("/metrics")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(req).await.expect("oneshot");
        let body = to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("body");
        let text = String::from_utf8(body.to_vec()).expect("utf8");

        // Every well-known metric defined in crate::metrics should show up.
        for expected in [
            "# HELP http_requests_total",
            "# HELP http_request_duration_seconds",
            "# HELP orders_created_total",
            "# HELP orders_filled_total",
            "# HELP orders_cancelled_total",
            "# HELP positions_open",
            "# HELP trigger_orders_created_total",
            "# HELP trigger_orders_fired_total",
            "# HELP risk_rules_tripped_total",
            "# HELP rate_limit_denied_total",
            "# HELP ws_connections_active",
            "# HELP ws_messages_broadcast_total",
            "# HELP kline_persist_total",
            "# HELP ai_predictions_total",
            "# HELP build_info",
            "http_requests_total{method=\"GET\",route=\"/test-endpoint\",status=\"200\"} 1",
        ] {
            assert!(
                text.contains(expected),
                "metrics body missing: {expected}\n--- body ---\n{text}"
            );
        }
    }
}
