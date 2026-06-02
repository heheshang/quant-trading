//! Prometheus metrics for the quant-trading backend.
//!
//! Exposes a global [`REGISTRY`] plus a small set of opinionated
//! metrics covering:
//!
//! * HTTP request throughput & latency (per route + status)
//! * Order / trigger-order / position activity
//! * Risk-rule and rate-limit hits
//! * Matching-engine & WS-hub liveness
//!
//! The `/metrics` endpoint is wired in [`crate::handlers::metrics_handler::metrics`]
//! and emits the standard Prometheus text-exposition format
//! (`text/plain; version=0.0.4`).
//!
//! ## Usage
//!
//! ```ignore
//! use crate::metrics;
//! metrics::HTTP_REQUESTS_TOTAL
//!     .with_label_values(&["GET", "/api/v1/orders", "200"])
//!     .inc();
//! ```
//!
//! Adding new metrics: register the [`prometheus::core::Collector`] in
//! [`register`] (called from [`REGISTRY`] lazy init) and add a public
//! `LazyLock<...>` binding below.

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    register_int_gauge_vec_with_registry, Encoder, HistogramVec, IntCounterVec, IntGaugeVec,
    Registry, TextEncoder,
};
use std::sync::LazyLock;

/// Process-wide Prometheus registry. The `/metrics` handler encodes this
/// registry's collectors on every scrape.
pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

/// HTTP requests served, labelled by `(method, route, status)`.
pub static HTTP_REQUESTS_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "http_requests_total",
        "Total HTTP requests served, labelled by method, route, and status code.",
        &["method", "route", "status"],
        REGISTRY
    )
    .expect("register http_requests_total")
});

/// HTTP request latency in seconds, labelled by `(method, route)`.
pub static HTTP_REQUEST_DURATION_SECONDS: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec_with_registry!(
        "http_request_duration_seconds",
        "HTTP request latency in seconds, labelled by method and route.",
        &["method", "route"],
        REGISTRY
    )
    .expect("register http_request_duration_seconds")
});

// ---------------------------------------------------------------------------
// Orders / positions
// ---------------------------------------------------------------------------

/// Orders created, labelled by `(side, order_type, exchange)`.
pub static ORDERS_CREATED_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "orders_created_total",
        "Orders accepted by the matching engine, labelled by side, type, and exchange.",
        &["side", "order_type", "exchange"],
        REGISTRY
    )
    .expect("register orders_created_total")
});

/// Orders filled (entirely or partially), labelled by `(side, exchange)`.
pub static ORDERS_FILLED_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "orders_filled_total",
        "Orders that reached at least one fill, labelled by side and exchange.",
        &["side", "exchange"],
        REGISTRY
    )
    .expect("register orders_filled_total")
});

/// Orders cancelled by the user or by the risk manager, labelled by `(actor, exchange)`.
pub static ORDERS_CANCELLED_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "orders_cancelled_total",
        "Orders cancelled, labelled by actor (user|risk) and exchange.",
        &["actor", "exchange"],
        REGISTRY
    )
    .expect("register orders_cancelled_total")
});

/// P1-2.1: Iceberg child order lifecycle, labelled by `action` (created|filled|cancelled).
pub static ICEBERG_CHILD_ORDERS_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "iceberg_child_orders_total",
        "Iceberg child order lifecycle events.",
        &["action"],
        REGISTRY
    )
    .expect("register iceberg_child_orders_total")
});

/// Open positions currently tracked, labelled by `(exchange, symbol)`.
pub static POSITIONS_OPEN: LazyLock<IntGaugeVec> = LazyLock::new(|| {
    register_int_gauge_vec_with_registry!(
        "positions_open",
        "Open positions currently tracked, labelled by exchange and symbol.",
        &["exchange", "symbol"],
        REGISTRY
    )
    .expect("register positions_open")
});

// ---------------------------------------------------------------------------
// Trigger orders
// ---------------------------------------------------------------------------

/// Trigger orders created, labelled by `(trigger_type)`.
pub static TRIGGER_ORDERS_CREATED_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "trigger_orders_created_total",
        "Trigger orders created, labelled by trigger type (StopLoss|TakeProfit|OCO|TWAP).",
        &["trigger_type"],
        REGISTRY
    )
    .expect("register trigger_orders_created_total")
});

/// Trigger orders that fired, labelled by `(trigger_type)`.
pub static TRIGGER_ORDERS_FIRED_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "trigger_orders_fired_total",
        "Trigger orders that crossed their threshold and submitted a market order.",
        &["trigger_type"],
        REGISTRY
    )
    .expect("register trigger_orders_fired_total")
});

// ---------------------------------------------------------------------------
// Risk / rate-limit
// ---------------------------------------------------------------------------

/// Risk-rule violations that blocked an order, labelled by `(rule)`.
pub static RISK_RULES_TRIPPED_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "risk_rules_tripped_total",
        "Risk-rule violations that blocked or modified an order, labelled by rule name.",
        &["rule"],
        REGISTRY
    )
    .expect("register risk_rules_tripped_total")
});

/// Rate-limit denials, labelled by `(scope)` where scope ∈ {user,symbol,global}.
pub static RATE_LIMIT_DENIED_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "rate_limit_denied_total",
        "Order submissions rejected by the rate limiter, labelled by scope.",
        &["scope"],
        REGISTRY
    )
    .expect("register rate_limit_denied_total")
});

// ---------------------------------------------------------------------------
// Matching engine / WS
// ---------------------------------------------------------------------------

/// Currently active WS clients, labelled by `(kind)` where kind ∈ {frontend,ai}.
pub static WS_CONNECTIONS_ACTIVE: LazyLock<IntGaugeVec> = LazyLock::new(|| {
    register_int_gauge_vec_with_registry!(
        "ws_connections_active",
        "Currently active WebSocket connections, labelled by client kind.",
        &["kind"],
        REGISTRY
    )
    .expect("register ws_connections_active")
});

/// Total messages broadcast by the WS hub, labelled by `(kind)` matching
/// the [`crate::services::exchange::ws_hub::HubMessage`] variant.
pub static WS_MESSAGES_BROADCAST_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "ws_messages_broadcast_total",
        "Total WebSocket messages broadcast by the hub, labelled by message kind.",
        &["kind"],
        REGISTRY
    )
    .expect("register ws_messages_broadcast_total")
});

/// Total kline messages written to Postgres by the KlineWriter.
pub static KLINE_PERSIST_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "kline_persist_total",
        "Total kline rows written to the database, labelled by interval.",
        &["interval"],
        REGISTRY
    )
    .expect("register kline_persist_total")
});

// ---------------------------------------------------------------------------
// AI service
// ---------------------------------------------------------------------------

/// AI predictions served, labelled by `(symbol)`.
pub static AI_PREDICTIONS_TOTAL: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec_with_registry!(
        "ai_predictions_total",
        "AI predictions served by the /ws/predict endpoint, labelled by symbol.",
        &["symbol"],
        REGISTRY
    )
    .expect("register ai_predictions_total")
});

// ---------------------------------------------------------------------------
// Process / build
// ---------------------------------------------------------------------------

/// Static gauge reporting the crate version. Useful for confirming
/// Grafana is scraping the binary the operator expects.
pub static BUILD_INFO: LazyLock<IntGaugeVec> = LazyLock::new(|| {
    register_int_gauge_vec_with_registry!(
        "build_info",
        "Static build metadata. The `version` label carries the cargo package version; the value is always 1.",
        &["version"],
        REGISTRY
    )
    .expect("register build_info")
});

/// Initialise the process-info gauges. Safe to call repeatedly — the
/// `with_label_values` call is idempotent for the same label set.
pub fn init_build_info() {
    BUILD_INFO
        .with_label_values(&[env!("CARGO_PKG_VERSION")])
        .set(1);
}

/// Prometheus exposition format MIME type. `prometheus::TextEncoder::format_type()`
/// returns this same string; we hard-code the literal here so the helper
/// signature can return a `&'static str` without tying it to a local
/// `TextEncoder` value's borrow lifetime.
const PROMETHEUS_TEXT_FORMAT: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Encode the full registry to Prometheus text-exposition format.
///
/// The returned tuple is `(body, content_type)`; pass both to the
/// `/metrics` handler.
pub fn encode_text() -> (Vec<u8>, &'static str) {
    let metric_families = REGISTRY.gather();
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();
    // Encoding failure is non-actionable for the metrics path (the registry
    // is in-memory and collectors are unit-tested). Propagate via empty
    // body so the scrape still returns 200.
    if encoder.encode(&metric_families, &mut buf).is_err() {
        buf.clear();
    }
    (buf, PROMETHEUS_TEXT_FORMAT)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Touching every LazyLock in one test forces initialisation and
    /// catches registration panics early.
    #[test]
    fn test_all_metrics_register_without_panic() {
        // Force LazyLock evaluation by calling `.get()` (or a method
        // that depends on internal state) on each.
        HTTP_REQUESTS_TOTAL.with_label_values(&["GET", "/x", "200"]).inc();
        HTTP_REQUEST_DURATION_SECONDS
            .with_label_values(&["GET", "/x"])
            .observe(0.001);
        ORDERS_CREATED_TOTAL
            .with_label_values(&["buy", "limit", "binance"])
            .inc();
        ORDERS_FILLED_TOTAL.with_label_values(&["buy", "binance"]).inc();
        ORDERS_CANCELLED_TOTAL
            .with_label_values(&["user", "binance"])
            .inc();
        POSITIONS_OPEN.with_label_values(&["binance", "BTCUSDT"]).set(3);
        TRIGGER_ORDERS_CREATED_TOTAL
            .with_label_values(&["StopLoss"])
            .inc();
        TRIGGER_ORDERS_FIRED_TOTAL
            .with_label_values(&["StopLoss"])
            .inc();
        RISK_RULES_TRIPPED_TOTAL
            .with_label_values(&["daily_loss"])
            .inc();
        RATE_LIMIT_DENIED_TOTAL.with_label_values(&["user"]).inc();
        WS_CONNECTIONS_ACTIVE.with_label_values(&["frontend"]).set(7);
        WS_MESSAGES_BROADCAST_TOTAL
            .with_label_values(&["Ticker"])
            .inc();
        KLINE_PERSIST_TOTAL.with_label_values(&["1m"]).inc();
        AI_PREDICTIONS_TOTAL.with_label_values(&["BTCUSDT"]).inc();
        init_build_info();
    }

    #[test]
    fn test_encode_text_returns_prometheus_format() {
        // Touch a metric so the output is non-empty.
        HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/probe", "200"])
            .inc();

        let (body, content_type) = encode_text();
        let text = String::from_utf8(body).expect("metrics body is utf-8");
        assert!(
            text.contains("# HELP http_requests_total"),
            "missing HELP line: {text}"
        );
        assert!(
            text.contains("# TYPE http_requests_total counter"),
            "missing TYPE line: {text}"
        );
        assert!(
            text.contains("http_requests_total{method=\"GET\",route=\"/probe\",status=\"200\"} 1"),
            "missing sample line: {text}"
        );
        assert_eq!(
            content_type,
            "text/plain; version=0.0.4; charset=utf-8",
            "content type must match Prometheus text-exposition spec"
        );
    }

    #[test]
    fn test_counter_increments_independently_per_label_set() {
        // Use unique label values that no other test touches, so the
        // absolute counts are deterministic.
        let c = &HTTP_REQUESTS_TOTAL;
        c.with_label_values(&["TEST_INCR", "/x1", "200"]).inc();
        c.with_label_values(&["TEST_INCR", "/x1", "200"]).inc();
        c.with_label_values(&["TEST_INCR", "/x2", "500"]).inc();

        let (body, _) = encode_text();
        let text = String::from_utf8(body).unwrap();
        // 2 incs for /x1 → 2; 1 inc for /x2 → 1
        assert!(text.contains("http_requests_total{method=\"TEST_INCR\",route=\"/x1\",status=\"200\"} 2"));
        assert!(text.contains("http_requests_total{method=\"TEST_INCR\",route=\"/x2\",status=\"500\"} 1"));
    }

    #[test]
    fn test_gauge_set_and_decrement() {
        let g = &POSITIONS_OPEN;
        g.with_label_values(&["binance", "ETHUSDT"]).set(5);
        g.with_label_values(&["binance", "ETHUSDT"]).dec();
        g.with_label_values(&["binance", "ETHUSDT"]).dec();

        let (body, _) = encode_text();
        let text = String::from_utf8(body).unwrap();
        assert!(text.contains("positions_open{exchange=\"binance\",symbol=\"ETHUSDT\"} 3"));
    }

    #[test]
    fn test_histogram_records_buckets() {
        let h = &HTTP_REQUEST_DURATION_SECONDS;
        for v in [0.001, 0.01, 0.1, 1.0] {
            h.with_label_values(&["POST", "/api/v1/orders"]).observe(v);
        }
        let (body, _) = encode_text();
        let text = String::from_utf8(body).unwrap();
        // Histogram emits `le` buckets + `_sum` + `_count`.
        assert!(text.contains("http_request_duration_seconds_bucket"));
        assert!(text.contains("http_request_duration_seconds_sum"));
        assert!(text.contains("http_request_duration_seconds_count"));
        assert!(text.contains("http_request_duration_seconds_count{method=\"POST\",route=\"/api/v1/orders\"} 4"));
    }
}
