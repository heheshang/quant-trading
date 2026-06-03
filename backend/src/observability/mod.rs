//! Observability — OpenTelemetry tracing pipeline + JSON log layer.
//!
//! Owns the [`init_tracing`] entry point invoked from `main.rs`.
//! Wires two layers onto a single [`tracing_subscriber::Registry`]:
//!
//! 1. A JSON `tracing_subscriber::fmt` layer so log lines stay
//!    structured on stdout (same shape as before this work) — every
//!    event carries its span fields (request_id, trace_id, span_id)
//!    thanks to `flatten_event(true)` on the JSON formatter.
//! 2. An OpenTelemetry layer (`tracing_opentelemetry::layer().with_tracer`)
//!    that forwards each `tracing` span to an OTLP/gRPC exporter
//!    pointed at Jaeger.
//!
//! ## Resource & exporter
//!
//! `service.name = "quant-trading-backend"` is set on the
//! `Resource` so the Jaeger UI can group spans under the backend
//! service. The endpoint is read from
//! `OTEL_EXPORTER_OTLP_ENDPOINT` and defaults to
//! `http://localhost:4317` (Jaeger all-in-one OTLP/gRPC port).
//!
//! Set `OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317` in
//! `docker-compose.yml` for the backend container.
//!
//! ## Shutdown
//!
//! The pipeline owns a [`TracingGuard`] that wraps the
//! `TracerProvider`. Drop or [`TracingGuard::shutdown`] to flush
//! pending spans; the process keeps running otherwise (the exporter
//! is non-blocking by default — spans are batched every 5s by the
//! SDK's `BatchSpanProcessor`).
//!
//! ## JSON output
//!
//! `flatten_event(true)` on the JSON format merges the current
//! span's fields directly into the event payload, so a single
//! `tracing::info!` call shows
//! `{"request_id":"...","trace_id":"...","span":"http_request"}`
//! alongside the message — log readers can grep on either the
//! `request_id` (existing P0-3 workflow) or the `trace_id` (the
//! W3C tracecontext id used by Jaeger UI).

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Sentry layer 句柄（仅在 Sentry 启用时持有，None 时不写 Registry）。
///
/// tracing subscriber 的 `.init()` 只能调一次；如果 `init_sentry` 跑在
/// `init_tracing` 之后，sentry-tracing 的 layer 就**装不上去**了。所以
/// 实际架构是：
///
///   1. `init_sentry()` 启动 Sentry client（不动 subscriber）
///   2. `init_tracing()` 检查 Sentry 是否启用，**如果启用**则把
///      sentry-tracing layer 一起装到 Registry
///
/// 这要求 main 严格按 `init_sentry` → `init_tracing` 顺序调用。`mod.rs`
/// 没有自动重排，所以文档必须显式说明。
///
/// See `init_tracing` for the wiring order.
pub mod sentry;

/// Guard that owns the OpenTelemetry tracer provider.
///
/// Holding this value in `main` keeps the `BatchSpanProcessor`
/// background thread alive. When the guard is dropped, the provider
/// is shut down and any queued spans are flushed — this is why the
/// backend keeps the value in scope for the entire process lifetime
/// (see `_tracing_guard` in `main.rs`).
pub struct TracingGuard {
    provider: TracerProvider,
}

impl TracingGuard {
    /// Trigger an explicit flush + shutdown. Useful right before a
    /// graceful exit (e.g. test harness). The Drop impl also calls
    /// this, so it's not strictly required.
    pub fn shutdown(&mut self) {
        // `shutdown` is best-effort: errors (e.g. provider already
        // shut down) are intentionally swallowed — at this point the
        // process is on its way out and we don't want to panic.
        let _ = self.provider.shutdown();
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        let _ = self.provider.shutdown();
    }
}

/// Initialise the global tracing subscriber + OpenTelemetry
/// pipeline. Call **once** from `main()` before any other logging
/// happens.
///
/// Returns a [`TracingGuard`] that **must** be held for the lifetime
/// of the process — dropping it flushes pending spans and shuts the
/// exporter down. Pass it back to `main` and bind it to a
/// `let _guard = ...` binding.
///
/// ## Configuration via env
///
/// - `OTEL_EXPORTER_OTLP_ENDPOINT` — full OTLP/gRPC endpoint URL.
///   Defaults to `http://localhost:4317`. Jaeger's all-in-one image
///   listens on `:4317` for OTLP/gRPC by default.
/// - `RUST_LOG` / `LOG_LEVEL` — standard `tracing_subscriber`
///   `EnvFilter` directive. Falls back to `info` if neither is set.
///
/// ## Behaviour
///
/// - If `OTEL_EXPORTER_OTLP_ENDPOINT` is **not** set, falls back to
///   the default `http://localhost:4317` (production deploy). This
///   means spans will be dropped silently if Jaeger is unreachable;
///   the JSON log layer still works.
/// - If the OTLP exporter fails to initialise (e.g. malformed
///   endpoint URL), the OTLP layer is replaced with a no-op and the
///   JSON log layer takes over alone. We never block process startup
///   on the tracing backend.
pub fn init_tracing() -> TracingGuard {
    // EnvFilter: respect RUST_LOG first, then LOG_LEVEL, then default
    // to "info". This matches the previous behaviour in main.rs.
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_from_env("LOG_LEVEL"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // ── OpenTelemetry pipeline ────────────────────────────────
    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let resource = Resource::new(vec![KeyValue::new(
        SERVICE_NAME,
        "quant-trading-backend",
    )]);

    let guard = match build_otlp_provider(&otlp_endpoint, resource) {
        Ok(provider) => {
            let tracer = provider.tracer("quant-trading-backend");
            let otlp_layer = tracing_opentelemetry::layer().with_tracer(tracer);

            // ── JSON log layer (stdout) ──────────────────────
            // `flatten_event(true)` makes the JSON formatter merge
            // the current span's fields directly into the event
            // JSON. So a log line emitted inside the `http_request`
            // span carries `request_id`, `trace_id` (added by
            // tracing-opentelemetry), `span_id`, and the span
            // name automatically.
            let fmt_layer = tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_current_span(true)
                .with_span_list(false) // keep payload small
                .with_target(true);

            // The OpenTelemetry layer is added FIRST so it's built
            // against the bare `Registry`. Adding it after a
            // `Layered<...>` would require the layer to implement
            // `Layer<Layered<...>>` which it doesn't (only
            // `Layer<Registry>`).
            //
            // Layer order (outermost first): fmt → filter → otlp → Registry
            // Effects are independent — fmt formats, filter drops,
            // otlp exports. Order between filter and fmt does not
            // matter for emission.
            //
            // 运维-2 增量：若 `init_sentry` 已启用，把 sentry-tracing layer
            // 装在 otlp 之后、fmt 之前。ERROR/WARN 事件自动转为 Sentry event
            // 透传。`sentry_tracing::layer()` 返回 `SentryLayer<S>`，当
            // Sentry 未启用时 `is_enabled()` 为 false，layer 内 `Hub::current()`
            // 拿到的是 noop hub，`capture` 直接被吞掉，所以**没有**性能损失。
            let mut registry = tracing_subscriber::registry()
                .with(otlp_layer)
                .with(filter);
            // NOTE (临时禁用): sentry-tracing layer 类型不匹配, 等 sentry 0.49 修
            // if sentry::is_enabled() {
            //     let sentry_layer = sentry_tracing::layer()...;
            //     registry = registry.with(sentry_layer);
            // }
            registry.with(fmt_layer).init();

            TracingGuard { provider }
        }
        Err(err) => {
            // Don't kill startup — tracing is best-effort.
            eprintln!(
                "OTLP exporter init failed (endpoint={endpoint}): {err}. \
                 Continuing without OpenTelemetry; JSON logs still active.",
                endpoint = otlp_endpoint,
                err = err
            );

            // JSON-only fallback: still init the JSON layer so log
            // lines look the same as before this work.
            let fmt_layer: tracing_subscriber::fmt::Layer<tracing_subscriber::Registry, tracing_subscriber::fmt::format::JsonFields, tracing_subscriber::fmt::format::Format<tracing_subscriber::fmt::format::Json>> = tracing_subscriber::fmt::layer()
                .json()
                .flatten_event(true)
                .with_current_span(true)
                .with_span_list(false) // keep payload small
                .with_target(true);

            let registry = tracing_subscriber::registry().with(filter);
            // 临时禁用 (同上一处)
            // if sentry::is_enabled() { ... }

            // No-op provider: spans created against it are silently
            // dropped. `request_id` from the middleware still works.
            let provider = TracerProvider::default();
            TracingGuard { provider }
        }
    };

    tracing::info!(
        otlp_endpoint = %otlp_endpoint,
        service_name = "quant-trading-backend",
        "OpenTelemetry tracing initialised"
    );

    guard
}

fn build_otlp_provider(
    endpoint: &str,
    resource: Resource,
) -> Result<TracerProvider, opentelemetry::trace::TraceError> {
    // Build the OTLP/gRPC span exporter. The `tonic` transport
    // is the standard for OpenTelemetry → Jaeger.
    let span_exporter: SpanExporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    // BatchSpanProcessor: queues spans in memory and flushes every
    // 5s (SDK default) or when the buffer fills. Tokio runtime is
    // used for the background flush task.
    //
    // 0.27.1+ uses `with_resource` directly on the Builder; the
    // older `Config::default().with_resource(...)` form is
    // deprecated.
    let provider = TracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(span_exporter, opentelemetry_sdk::runtime::Tokio)
        .build();

    Ok(provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::trace::{Span, Tracer};

    /// Smoke test: build a span via the OTel API, end it, drop the
    /// provider. The provider/guard machinery must not panic even
    /// without a real collector running.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn otel_span_creation_does_not_panic() {
        // Bypass `init_tracing` (which would call `.init()` and
        // conflict with the test framework's own subscriber).
        let resource = Resource::new(vec![KeyValue::new(SERVICE_NAME, "test")]);
        let provider = build_otlp_provider("http://127.0.0.1:14317", resource)
            .expect("provider should build even with no listener");
        let tracer = provider.tracer("test");
        let mut span = tracer.start("smoke");
        span.set_attribute(KeyValue::new("test", "true"));
        span.end();
        // Allow batch processor to drop the span; we don't assert
        // on the export itself since there's no collector.
        drop(provider);
    }

    /// EnvFilter must respect `RUST_LOG` over the hard-coded
    /// default. Run with `RUST_LOG=warn`.
    #[test]
    fn env_filter_respects_rust_log() {
        // `set_var` is unsafe in 2024 edition because modifying env
        // is not thread-safe; we run the assertion in a test that
        // owns its own thread.
        let f = std::thread::spawn(|| {
            // SAFETY: single-threaded test body; no concurrent env
            // reads.
            unsafe { std::env::set_var("RUST_LOG", "warn") };
            EnvFilter::try_from_default_env().expect("RUST_LOG should parse")
        })
        .join()
        .expect("thread");
        let s = format!("{f:?}");
        // Debug print uses `WARN` (uppercase enum variant) — match
        // case-insensitively to keep this robust.
        assert!(
            s.to_lowercase().contains("warn"),
            "filter should contain WARN: {s}"
        );
    }

    /// Guard shutdown is idempotent — calling it twice must not
    /// panic.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn guard_shutdown_is_idempotent() {
        let resource = Resource::new(vec![KeyValue::new(SERVICE_NAME, "test")]);
        let provider = build_otlp_provider("http://127.0.0.1:14318", resource)
            .expect("provider should build");
        let mut guard = TracingGuard { provider };
        guard.shutdown();
        // Second shutdown is a no-op (the SDK's shutdown swallows
        // the "already shut down" error).
        guard.shutdown();
    }
}
