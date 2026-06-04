use axum::{
    Extension, Router, middleware,
    routing::{delete, get, post, put},
};
use quant_trading_backend::CONFIG;
use quant_trading_backend::db::{DbPool, init_db, run_migrations};
use quant_trading_backend::state::AppState;
use quant_trading_backend::handlers;
use quant_trading_backend::services::ai::feature_engine::NormalizeMethod;
use quant_trading_backend::services::binance_rest::BinanceRestClient;
use quant_trading_backend::services::bybit_rest::BybitRestClient;
use quant_trading_backend::services::exchange::ws_hub::{WsHub, WsHubBuilder};
use quant_trading_backend::services::exchange::{
    api_keys::{ApiKeyStore, get_master_key},
    signed_client::SignedBinanceClient,
    SignedOkxClient,
};
use quant_trading_backend::services::gate_rest::GateRestClient;
use quant_trading_backend::services::kline_writer::KlineWriter;
use quant_trading_backend::services::matching_engine::MatchingEngine;
use quant_trading_backend::services::okx_rest::OkxRestClient;
use quant_trading_backend::services::order_rate_limiter::OrderRateLimiter;
use quant_trading_backend::services::redis_cache::RedisCache;
use quant_trading_backend::services::risk_manager::RiskManager;
use quant_trading_backend::services::strategy_state_manager::StrategyStateManager;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    // Load .env file
    dotenvy::dotenv().ok();

    // 运维-2: initialise Sentry (error aggregation) **before** tracing.
    //   - `SENTRY_DSN` 缺失时返回 Ok(None)，后续代码无 Sentry 开销。
    //   - 启用后 `sentry::init` 会装一个全局 Hub；
    //     `init_tracing` 会探测 `sentry::is_enabled()` 把 sentry-tracing
    //     layer 一起装到 Registry，覆盖 tracing::error!/warn! 事件。
    //   - guard 持有 sentry::Client，Drop 时 flush 客户端并释放资源。
    // 调用顺序：必须在 `init_tracing()` 之前。
    let _sentry_guard = match quant_trading_backend::observability::sentry::init_sentry() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Sentry init failed: {e}. Continuing without Sentry.");
            None
        }
    };

    // P0-运维: initialise OpenTelemetry tracing + JSON log layer.
    // Returns a guard whose Drop impl flushes pending spans to the
    // OTLP exporter (Jaeger :4317 by default). Hold it for the
    // entire process lifetime.
    let _tracing_guard =
        quant_trading_backend::observability::init_tracing();

    tracing::info!("Starting quant-trading backend...");

    // Initialize database
    let db: DbPool = init_db(&CONFIG.database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    run_migrations(&db)
        .await
        .expect("Failed to run database migrations");

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(
            CONFIG
                .cors_allowed_origins
                .iter()
                .map(|s| {
                    s.parse::<axum::http::HeaderValue>().unwrap_or_else(|_| {
                        axum::http::HeaderValue::from_static("http://localhost:5173")
                    })
                })
                .collect::<Vec<_>>(),
        )
        .allow_methods(Any)
        .allow_headers(Any);

    // Create matching engine
    let matching_engine = MatchingEngine::new(db.clone(), 200, 0.001);

    // Create order rate limiter (P1-F4)
    let order_rate_limiter = Arc::new(OrderRateLimiter::new());

    // Initialize Redis cache
    let redis_cache = Arc::new(
        RedisCache::new(&CONFIG.redis_url)
            .await
            .expect("Redis connect failed"),
    );

    // Initialize Binance REST client
    let binance_rest = Arc::new(BinanceRestClient::new());

    // Initialize OKX REST client
    let okx_rest = Arc::new(OkxRestClient::new());

    // Initialize Gate.io REST client
    let gate_rest = Arc::new(GateRestClient::new());

    // Initialize Bybit REST client
    let bybit_rest = Arc::new(BybitRestClient::new());

    // Initialize Signed Binance Client for authenticated API calls
    let master_key = get_master_key().unwrap_or([0u8; 32]);
    let key_store = Arc::new(ApiKeyStore::new(db.clone(), master_key));
    let signed_client = Arc::new(SignedBinanceClient::new(key_store.clone()));
    let okx_signed_client = Arc::new(SignedOkxClient::new(key_store.clone()));

    // Initialize KlineWriter background task
    let (kline_tx, kline_rx) = mpsc::channel(100);
    let mut kline_writer = KlineWriter::new(kline_rx, db.as_ref().clone());
    tokio::spawn(async move {
        kline_writer.run().await;
    });

    // Initialize WebSocket Hub (singleton) with KlineWriter + Redis cache
    let ws_hub = Arc::new(
        WsHubBuilder::new()
            .with_kline_writer_tx(kline_tx)
            .with_redis_cache((*redis_cache).clone())
            .build(),
    );
    ws_hub.start();

    // F6: Initialize RiskManager and StrategyStateManager (断线暂停监控)
    let risk_manager_inner = RiskManager::new(db.clone());

    // Build the AppState early so the P2-1 notifier setup can read its
    // `telegram_chat_ids` cache when wiring the per-user chat_id resolver.
    // (The first `app_state` carries a no-notifier RiskManager; we replace
    // it after the notifier is wired up. Functionally identical for downstream
    // because `risk_manager` is the same Arc handle at that point.)
    let app_state = AppState::new(
        db.clone(),
        (*redis_cache).clone(),
        (*binance_rest).clone(),
        (*ws_hub).clone(),
        risk_manager_inner.clone(),
    );

    // P2-1: Build the multi-channel alert notification service.
    // Channels added (in order, all no-op if env unconfigured):
    //   - EmailChannel (SMTP_HOST/PORT/USER/PASSWORD/FROM, ALERT_EMAIL_TO)
    //   - WeChatChannel (WECHAT_WEBHOOK_URL)
    //   - TelegramChannel (TELEGRAM_BOT_TOKEN) — uses a per-user chat_id
    //     resolver that reads `app_state.telegram_chat_ids` (populated by
    //     `bind_chat_id`).
    // All channels implement the `NotificationChannel` trait and the
    // service handles deduplication + fan-out.
    let alert_notifier = Arc::new(
        quant_trading_backend::services::alert_notification_service::AlertNotificationService::new(),
    );

    // Register the email channel. It is a no-op when SMTP_HOST/PORT/USER/PASSWORD
    // are not all set (see `EmailChannel::send` — returns Ok(()) silently).
    alert_notifier
        .add_channel(
            quant_trading_backend::services::notification::EmailChannel::from_env(),
        )
        .await;
    if std::env::var("SMTP_HOST").is_ok() {
        tracing::info!("Alert channel registered: email (SMTP configured)");
    }

    // Register the WeChat channel. No-op when WECHAT_WEBHOOK_URL is missing.
    alert_notifier
        .add_channel(
            quant_trading_backend::services::notification::WeChatChannel::from_env(),
        )
        .await;
    if std::env::var("WECHAT_WEBHOOK_URL").is_ok() {
        tracing::info!("Alert channel registered: wechat (webhook URL configured)");
    }

    // P2-1: Register the Telegram channel with a per-user chat_id resolver.
    // The resolver is sync (`Fn(String) -> Option<String>`) and reads from
    // the in-memory `TelegramChatIdCache` populated by `bind_chat_id`. This
    // avoids a per-send DB hit while still letting the per-user flow work.
    let chat_cache = app_state.telegram_chat_ids.clone();
    let chat_id_resolver: quant_trading_backend::services::notification::telegram::ChatIdResolver =
        std::sync::Arc::new(move |user_id: String| {
            // Parse user_id (UUID) and look up in the cache. None = not bound
            // → TelegramChannel::send will skip.
            match uuid::Uuid::parse_str(&user_id) {
                Ok(uid) => chat_cache.get_blocking(&uid),
                Err(_) => None,
            }
        });
    let telegram_channel =
        quant_trading_backend::services::notification::TelegramChannel::from_env()
            .with_resolver(chat_id_resolver);
    if std::env::var("TELEGRAM_BOT_TOKEN").is_ok() {
        tracing::info!("Alert channel registered: telegram");
    }
    alert_notifier.add_channel(telegram_channel).await;

    // Inject the notifier into RiskManager so risk events fan out to all channels.
    // 中文：先建一个"裸" RiskManager，调 with_notifier 注入，再 wrap 成 Arc 传给
    //   下游。这样保留 builder-style 的链式 API（see `with_notifier: mut self -> Self`）。
    // English: Build a bare RiskManager, call `with_notifier` to inject, then
    //   wrap in Arc for downstream consumers. Preserves the builder-style API
    //   (`with_notifier: mut self -> Self`).
    let risk_manager = Arc::new(risk_manager_inner.with_notifier(alert_notifier.clone()));

    // Re-build app_state now that risk_manager carries the notifier.
    // (app_state's `risk_manager: Arc<RiskManager>` clone happens lazily,
    // but risk_manager is the same Arc — no functional change. We rebuild
    // for explicitness.)
    let app_state = AppState::new(
        db.clone(),
        (*redis_cache).clone(),
        (*binance_rest).clone(),
        (*ws_hub).clone(),
        (*risk_manager).clone(),
    );

    let state_manager = Arc::new(StrategyStateManager::new(
        ws_hub.clone(),
        risk_manager.clone(),
    ));
    state_manager.start();
    let app = create_router(app_state,
        cors,
        matching_engine,
        order_rate_limiter,
        redis_cache,
        binance_rest,
        okx_rest,
        gate_rest,
        bybit_rest,
        ws_hub,
        signed_client,
        okx_signed_client,
        key_store,
        risk_manager.clone(),
        alert_notifier.clone(),
    );

    // Start server
    // P3-5: Spawn the feature flag cache refresh task. The first
    // refresh is eager (see `spawn_refresh_task`), so the first
    // post-boot `is_enabled` call is a cache hit. Failure to refresh
    // is logged but does not stop the loop — a transient DB hiccup
    // shouldn't disable the whole feature flag system.
    quant_trading_backend::services::feature_flag::spawn_refresh_task(
        app_state.db.as_ref().clone(),
    );

    let addr = CONFIG.server_addr();
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app).await.expect("Server failed");
}

#[allow(clippy::too_many_arguments)]
fn create_router(
    app_state: AppState,
    cors: CorsLayer,
    matching_engine: Arc<MatchingEngine>,
    order_rate_limiter: Arc<OrderRateLimiter>,
    redis_cache: Arc<RedisCache>,
    binance_rest: Arc<BinanceRestClient>,
    okx_rest: Arc<OkxRestClient>,
    gate_rest: Arc<GateRestClient>,
    bybit_rest: Arc<BybitRestClient>,
    ws_hub: Arc<WsHub>,
    signed_client: Arc<SignedBinanceClient>,
    okx_signed_client: Arc<SignedOkxClient>,
    key_store: Arc<ApiKeyStore>,
    risk_manager: Arc<RiskManager>,
    alert_notifier: Arc<
        quant_trading_backend::services::alert_notification_service::AlertNotificationService,
    >,
) -> Router {
    #[allow(unused_assignments)]
    let mut app = Router::new();
    // Auth routes (no auth required)
    let auth_routes = Router::new()
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login))
        .route("/refresh", post(handlers::auth::refresh));

    // Protected auth routes
    let auth_protected = Router::new()
        .route("/logout", post(handlers::auth::logout))
        .route("/me", get(handlers::auth::me_route))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // User management routes (authenticated)
    let user_routes = Router::new()
        .route("/users/me", get(handlers::users::get_me))
        .route("/users/me", post(handlers::users::update_me))
        .route("/users/me/password", post(handlers::users::change_password))
        .route("/users", get(handlers::users::list_users))
        .route("/users/{id}", post(handlers::users::admin_update_user))
        .route(
            "/users/{id}",
            delete_handler(handlers::users::admin_delete_user),
        )
        .route("/roles", get(handlers::users::list_roles))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Strategy routes (authenticated)
    let strategy_routes = Router::new()
        .route(
            "/strategies/templates",
            get(handlers::strategy::list_templates),
        )
        .route("/strategies", get(handlers::strategy::list_strategies))
        .route("/strategies", post(handlers::strategy::create_strategy))
        .route(
            "/strategies/bulk/status",
            post(handlers::strategy::bulk_update_status),
        )
        .route("/strategies/{id}", get(handlers::strategy::get_strategy))
        .route(
            "/strategies/{id}",
            post(handlers::strategy::update_strategy),
        )
        .route(
            "/strategies/{id}",
            delete_handler(handlers::strategy::delete_strategy),
        )
        .route(
            "/strategies/{id}/status",
            post(handlers::strategy::update_status),
        )
        .route(
            "/strategies/export",
            get(handlers::strategy::export_strategies),
        )
        .route(
            "/strategies/bulk/delete",
            post(handlers::strategy::bulk_delete_strategies),
        )
        .route(
            "/strategies/code/upload",
            post(handlers::strategy::upload_strategy_code),
        )
        .route(
            "/strategies/import",
            post(handlers::strategy::import_strategies_batch),
        )
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));
    // Kline routes (authenticated)
    let kline_routes = Router::new()
        .route("/kline/query", get(handlers::kline::query_klines))
        .route("/kline/import", post(handlers::kline::import_klines))
        .route(
            "/kline/import-history",
            get(handlers::kline::import_history),
        )
        .route("/kline/quality", get(handlers::kline::quality_report))
        .route("/kline/clean", post(handlers::kline::clean_klines))
        .route("/kline/export", get(handlers::kline::export_klines))
        .route("/kline/latest", get(handlers::kline::get_latest_kline))
        .route("/kline/symbols", get(handlers::kline::list_symbols))
        .route("/kline/fetch", post(handlers::kline::fetch_klines))
        .route(
            "/kline/clean/rollback",
            delete(handlers::kline::rollback_clean),
        )
        .route("/kline/import/csv", post(handlers::kline::import_csv))
        // P3-6: TimescaleDB continuous aggregate (1m/5m/1h) — 走 klines_1m/5m/1h 视图
        .route("/kline/aggregate", get(handlers::kline::get_aggregate_klines))
        .route("/kline/kdj", get(handlers::indicator::get_kdj))
        .route("/kline/ma", get(handlers::indicator::get_ma))
        .route("/kline/macd", get(handlers::indicator::get_macd))
        .route("/kline/rsi", get(handlers::indicator::get_rsi))
        .route("/kline/bollinger", get(handlers::indicator::get_bollinger))
        .route("/kline/ema", get(handlers::indicator::get_ema))
        .route("/kline/atr", get(handlers::indicator::get_atr))
        .route(
            "/kline/stochastic",
            get(handlers::indicator::get_stochastic),
        )
        .route("/kline/obv", get(handlers::indicator::get_obv))
        .route("/kline/pivot", get(handlers::indicator::get_pivot))
        .route("/kline/fib", get(handlers::indicator::get_fib))
        .route("/kline/hurst", get(handlers::indicator::get_hurst))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Market routes (authenticated)
    let market_routes = Router::new()
        .route("/market/tickers", get(handlers::market::get_tickers))
        .route("/market/ticker", get(handlers::market::get_ticker))
        .route("/market/depth", get(handlers::market::get_depth))
        .route(
            "/market/ticker/history",
            get(handlers::market::get_ticker_history),
        )
        .route("/market/kline", get(handlers::market::get_kline))
        .layer(Extension(redis_cache.clone()))
        .layer(Extension(binance_rest.clone()))
        .layer(Extension(okx_rest.clone()))
        .layer(Extension(gate_rest.clone()))
        .layer(Extension(bybit_rest.clone()))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Order/Trading routes (authenticated, with matching engine)
    // Both /orders and /orders/ paths registered to avoid nginx 301 redirect issues
    let order_routes = Router::new()
        .route("/orders", post(handlers::order::create_order))
        .route("/orders", get(handlers::order::list_orders))
        .route("/orders/", post(handlers::order::create_order))
        .route("/orders/", get(handlers::order::list_orders))
        .route("/orders/{id}", get(handlers::order::get_order))
        .route("/orders/{id}/cancel", post(handlers::order::cancel_order))
        .route(
            "/orders/cancel-all",
            post(handlers::order::cancel_all_orders),
        )
        .route("/trades", get(handlers::order::list_trades))
        .route("/positions", get(handlers::order::list_positions))
        .route(
            "/positions/{symbol}/close",
            post(handlers::order::close_position),
        )
        .route("/account", get(handlers::order::get_account))
        .route("/account/init", post(handlers::order::init_account))
        .route("/symbols", get(handlers::order::list_symbols))
        // Export routes
        .route("/exports/orders", get(handlers::export::export_orders))
        .route("/exports/trades", get(handlers::export::export_trades))
        .route(
            "/exports/positions",
            get(handlers::export::export_positions),
        )
        .route("/exports/account", get(handlers::export::export_account))
        .layer(axum::Extension(matching_engine))
        .layer(axum::Extension(order_rate_limiter))
        .layer(axum::Extension(ws_hub.clone()))
        .layer(axum::Extension(risk_manager.clone()))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Risk management routes (authenticated)
    let risk_routes = Router::new()
        .route("/risk/rules", get(handlers::risk::get_risk_rules))
        .route("/risk/rules", put(handlers::risk::update_risk_rules))
        .route("/risk/logs", get(handlers::risk::get_risk_logs))
        .route(
            "/risk/emergency-close",
            post(handlers::risk::emergency_close),
        )
        .route("/risk/pause", post(handlers::risk::pause_trading))
        .route("/risk/resume", post(handlers::risk::resume_trading))
        .route(
            "/risk/connection-status",
            get(handlers::risk::connection_status),
        )
        .route("/risk/check", post(handlers::risk::manual_risk_check))
        .layer(Extension(ws_hub.clone()))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Dashboard routes (authenticated)
    let dashboard_routes = Router::new()
        .route("/dashboard/stats", get(handlers::dashboard::get_stats))
        .route("/dashboard/pnl", get(handlers::dashboard::get_pnl))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Portfolio routes (authenticated)
    let portfolio_routes = Router::new()
        .route(
            "/portfolio/summary",
            get(handlers::portfolio::get_portfolio_summary),
        )
        .route(
            "/portfolio/positions",
            get(handlers::portfolio::list_portfolio_positions),
        )
        .route(
            "/portfolio/performance",
            get(handlers::portfolio::get_portfolio_performance),
        )
        .route(
            "/portfolio/equity_curve",
            get(handlers::portfolio::get_equity_curve),
        )
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Backtest routes (authenticated, cancel merged to avoid double-nest conflict)
    let backtest_routes = Router::new()
        .route("/backtest", post(handlers::backtest::run_backtest))
        .route("/backtest/{id}", get(handlers::backtest::get_backtest))
        .route(
            "/backtest/{id}",
            delete_handler(handlers::backtest::delete_backtest),
        )
        .route(
            "/backtest/{id}/cancel",
            post(handlers::backtest::cancel_backtest),
        )
        .route(
            "/backtest/{id}/trades",
            get(handlers::backtest::get_backtest_trades),
        )
        .route(
            "/backtest/{id}/equity",
            get(handlers::backtest::get_backtest_equity),
        )
        .route(
            "/backtest/history",
            get(handlers::backtest::list_backtest_history),
        )
        .layer(Extension(ws_hub.clone()))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // API Key management routes (authenticated)
    let api_key_routes = Router::new()
        .route("/api-keys", get(handlers::api_key::list_api_keys))
        .route("/api-keys", post(handlers::api_key::create_api_key))
        .route("/api-keys/{id}", get(handlers::api_key::get_api_key))
        .route(
            "/api-keys/{id}",
            axum::routing::put(handlers::api_key::update_api_key),
        )
        .route("/api-keys/{id}", delete(handlers::api_key::delete_api_key))
        .route("/api-keys/{id}/test", post(handlers::api_key::test_api_key))
        .layer(Extension(key_store.clone()))
        .layer(Extension(signed_client.clone()))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.db.clone());

    // Review routes (authenticated)
    let review_routes = Router::new()
        .route("/reviews/submit", post(handlers::review::submit_for_review))
        .route("/reviews/approve", post(handlers::review::approve_strategy))
        .route("/reviews/reject", post(handlers::review::reject_strategy))
        .route(
            "/reviews/pending",
            get(handlers::review::list_pending_reviews),
        )
        .route(
            "/reviews/{strategy_id}",
            get(handlers::review::get_strategy_review),
        )
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Admin API Key routes (authenticated + admin role check inside handler)
    let admin_api_key_routes = Router::new()
        .route(
            "/api-keys",
            get(handlers::api_key::admin_list_api_keys),
        )
        .layer(Extension(key_store.clone()))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.db.clone());

    // P3-A: Admin IP whitelist routes (authenticated + admin role check + IP
    // gate). The IP gate is layered AFTER auth so we have an
    // AuthenticatedUser to scope the allow-list to. We do NOT apply the IP
    // gate to the existing admin_api_key_routes — that's a deliberate
    // staging decision: existing admins need to add their IP entries via
    // a manual SQL insert or env-bypass first, then enable the gate. P3-B
    // (DB seeding script) will document this path.
    let admin_ip_routes = Router::new()
        .route(
            "/ip-whitelist",
            get(handlers::admin_ip_whitelist::list_ip_whitelist),
        )
        .route(
            "/ip-whitelist",
            post(handlers::admin_ip_whitelist::add_ip_whitelist),
        )
        .route(
            "/ip-whitelist/{id}",
            delete(handlers::admin_ip_whitelist::delete_ip_whitelist),
        )
        .layer(middleware::from_fn_with_state(
            app_state.db.clone(),
            quant_trading_backend::middleware::admin_ip_check::admin_ip_check,
        ))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.db.clone());

    // Exchange routes (authenticated, with signed Binance client)
    let exchange_routes = Router::new()
        .route("/exchange/ping", get(handlers::exchange::exchange_ping))
        .route(
            "/exchange/account",
            get(handlers::exchange::exchange_account),
        )
        .route(
            "/exchange/order",
            post(handlers::exchange::exchange_create_order),
        )
        .route(
            "/exchange/rate-limit",
            get(handlers::exchange::exchange_rate_limit),
        )
        .layer(Extension(signed_client.clone()))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.db.clone());

    // OKX Exchange routes (authenticated, with signed OKX client)
    let okx_exchange_routes = Router::new()
        .route("/exchange/okx/ping", get(handlers::exchange::exchange_okx_ping))
        .route(
            "/exchange/okx/account",
            get(handlers::exchange::exchange_okx_account),
        )
        .route(
            "/exchange/okx/order",
            post(handlers::exchange::exchange_okx_create_order),
        )
        .route(
            "/exchange/okx/order/{orderId}",
            delete(handlers::exchange::exchange_okx_cancel_order),
        )
        .route(
            "/exchange/okx/orders/pending",
            get(handlers::exchange::exchange_okx_pending_orders),
        )
        .layer(Extension(okx_signed_client.clone()))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.db.clone());

    // Public routes — registered as direct path to avoid /api/v1 nesting shadowing
    // Both /ws and /api/v1/ws registered (nginx regex proxy_pass preserves full path)
    // Also /api/v1/openapi.json and /swagger-ui — public metadata, no auth.
    let public_routes = Router::new()
        .route("/health", get(handlers::health::liveness))
        .route("/api/v1/health/ready", get(handlers::health::readiness))
        .route(
            "/api/v1/openapi.json",
            get(handlers::openapi::openapi_json),
        )
        .route("/ws", get(handlers::ws::ws_handler))
        .route("/api/v1/ws", get(handlers::ws::ws_handler))
        .route("/metrics", get(handlers::metrics_handler::metrics))
        .with_state(app_state.clone())
        .layer(Extension(ws_hub.clone()));

    // P3-4: 审计日志查询端点（admin only）。
    //   - `auth_middleware`  注入 `AuthenticatedUser` 到 extensions
    //   - `require_admin_middleware` 校验 role == "admin"
    //   - `with_state(db)` 给 list_audit_logs / get_audit_log 传 DbPool
    //   - `nest("/api/v1/admin", ...)` 让路径是 `/api/v1/admin/audit-logs`
    //     —— 与 admin_api_key_routes / admin_ip_routes 的命名风格一致。
    let audit_log_routes = handlers::audit_log::router()
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::admin_only::require_admin_middleware,
        ))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.db.clone());

    app = Router::new()
        .nest("/api/v1/auth", auth_routes)
        .nest("/api/v1/auth", auth_protected)
        .nest("/api/v1", user_routes)
        .nest("/api/v1", strategy_routes)
        .nest("/api/v1", kline_routes)
        .nest("/api/v1", market_routes)
        .nest("/api/v1", order_routes)
        .nest("/api/v1", portfolio_routes)
        .nest("/api/v1", risk_routes)
        .nest(
            "/api/v1",
            handlers::position_alert::router().layer(middleware::from_fn(
                quant_trading_backend::middleware::auth::auth_middleware,
            )),
        )
        .nest(
            "/api/v1",
            handlers::trigger_order::router().layer(middleware::from_fn(
                quant_trading_backend::middleware::auth::auth_middleware,
            )),
        )
        .nest(
            "/api/v1",
            handlers::bracket::router().layer(middleware::from_fn(
                quant_trading_backend::middleware::auth::auth_middleware,
            )),
        )
        .merge(handlers::arbitrage::router().layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        )))
        .nest("/api/v1", dashboard_routes)
        .nest("/api/v1", backtest_routes)
        .nest("/api/v1", exchange_routes)
        .nest("/api/v1", okx_exchange_routes)
        .nest("/api/v1", api_key_routes)
        .nest("/api/v1/admin", admin_api_key_routes)
        .nest("/api/v1/admin", admin_ip_routes)
        .nest("/api/v1", review_routes);

    // §6-1 PAMM (Percent Allocation Management Module) closing.
    //   - `pamm_user_routes`  : any authenticated user (list / detail /
    //     subscribe / redeem / my-investments / create-fund). The
    //     service layer enforces that only the fund's manager can
    //     trigger distribute / liquidate; here we only gate auth.
    //   - `pamm_manager_routes`: admin role required (the admin_only
    //     middleware is layered AFTER auth so the role check sees the
    //     same `AuthenticatedUser` shape). Even an admin token that
    //     doesn't own the fund is rejected by the service layer
    //     (PammError::NotManager → 403) — defence in depth.
    let pamm_user_routes = handlers::pamm::router_user()
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.db.clone());

    let pamm_manager_routes = handlers::pamm::router_manager()
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::admin_only::require_admin_middleware,
        ))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.db.clone());

    // P3-4: 审计日志查询端点（admin only）。restore
    let audit_log_routes = handlers::audit_log::router()
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::admin_only::require_admin_middleware,
        ))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.db.clone());

    let feature_flag_routes = handlers::feature_flag::router()
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    let feature_flag_admin_routes = handlers::feature_flag::router()
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::admin_only::require_admin_middleware,
        ))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    app = app
        // P3-5: user bootstrap endpoint (any authenticated user).
        .merge(feature_flag_routes)
        // P3-5: admin CRUD endpoints (admin role required).
        .merge(feature_flag_admin_routes);

    // §6-1 PAMM: merge the user-facing + manager-only sub-routers into
    // the global `app`. Both have already been layered with their own
    // auth middleware (manager router also has require_admin), so
    // `merge` is enough — no extra layer needed here.
    app = app
        .merge(pamm_user_routes)
        .merge(pamm_manager_routes);

    // §6-2 Copy Trading — user-facing sub-router. Auth required for all
    // endpoints; manager-only actions (none in v0.1) would go in a
    // separate sub-router with `require_admin_middleware` if/when
    // added. Mirrors the PAMM user-router wiring pattern.
    let copy_trading_routes = handlers::copy_trading::router()
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(app_state.db.clone());

    app = app.merge(copy_trading_routes);

    // P2-1: Telegram notification routes (authenticated).
    // 中文：bind/test 都需要 `DbPool` 状态 + `TelegramChatIdCache` 与
    //   `AlertNotificationService` Extension；auth_middleware 在 router 之前套上。
    //   `telegram_chat_ids` 缓存还通过 Extension 注入到全局 app，让
    //   PositionAlertMonitor / 其他服务按需读取。
    // English: bind/test need `DbPool` state + `TelegramChatIdCache` and
    //   `AlertNotificationService` Extension; auth_middleware is applied
    //   before the router. The chat_id cache is also surfaced as a global
    //   Extension so PositionAlertMonitor and other services can read it.
    let telegram_chat_cache = app_state.telegram_chat_ids.clone();
    let telegram_routes = handlers::telegram::router()
        .layer(Extension(alert_notifier.clone()))
        .layer(Extension(telegram_chat_cache.clone()))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // P3-F3 AI Quant services — injected via Extension into AI handlers
    let ai_services = Arc::new(handlers::ai::AiServices {
        model_client: quant_trading_backend::services::ai::ModelClient::new(
            std::env::var("AI_MODEL_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8001".to_string()),
            2,
        ),
        feature_engine: quant_trading_backend::services::ai::FeatureEngine::new(
            100,
            NormalizeMethod::ZScore,
        ),
    });

    // Spawn AI prediction broadcaster — polls AI service every 60s, broadcasts to all WS clients
    {
        let hub = ws_hub.clone();
        let ai = ai_services.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let features = vec![
                    0.01, 0.02, 0.015, 45.0, 0.5, 0.1, 0.5, 0.02, 20.0, 50.0,
                    0.5, 0.01, 1.2, 0.02, 0.01, 0.01, 0.5, 1.5, 1.2, 0.01,
                ];
                match ai.model_client.predict_price_direction(&features, "BTCUSDT", "1h", None).await {
                    Ok(p) => {
                        let _ = hub.broadcast(quant_trading_backend::services::exchange::ws_hub::HubMessage::AIPredict {
                            symbol: "BTCUSDT".to_string(),
                            interval: "1h".to_string(),
                            direction: p.direction.clone(),
                            confidence: p.confidence,
                            signal: p.signal.unwrap_or_else(|| "neutral".to_string()),
                            price_target: p.price_target,
                            analysis: p.analysis.unwrap_or_default(),
                            indicators: p.indicators.unwrap_or(serde_json::Value::Null),
                            generated_at: p.generated_at.clone(),
                        });
                    }
                    Err(e) => {
                        tracing::warn!("AI prediction broadcast failed: {}", e);
                    }
                }
            }
        });
    }

    // P1-2.3: Trailing stop poll loop — every 2s, fetch price via the in-process
    // RedisCache which is already fed by Binance WS. This avoids:
    //   1. Outbound HTTPS restriction to api.binance.com in this environment
    //   2. Auth requirement on /api/v1/market/tickers (price loop runs in background)
    {
        let db_for_trailing = app_state.db.clone();
        let redis_for_trailing = redis_cache.clone();
        let _join = quant_trading_backend::services::trailing_stop::spawn_poll_loop(
            db_for_trailing,
            2, // poll every 2 seconds
            move |symbol: &str| -> futures::future::BoxFuture<'static, Result<f64, String>> {
                let redis = redis_for_trailing.clone();
                let sym = symbol.to_string();
                Box::pin(async move {
                    // Both "BTCUSDT" and "BTC/USDT" forms accepted; WS writes "BTCUSDT" without slash
                    let naked = sym.replace('/', "");
                    match redis.get_ticker(&naked).await {
                        Ok(Some(t)) => Ok(t.price),
                        Ok(None) => Err(format!("price not found in Redis for {}", sym)),
                        Err(e) => Err(format!("Redis get_ticker error for {}: {}", sym, e)),
                    }
                })
            },
        );
    }

    app.merge(
        handlers::ai::router()
            .layer(axum::Extension(ai_services))
            .layer(middleware::from_fn(
                quant_trading_backend::middleware::auth::auth_middleware,
            )),
    )
    .merge(public_routes)
    // P3-3: Swagger UI at /swagger-ui. Public metadata, no auth.
    // The .url("/api/v1/openapi.json", ...) form is set inside
    // `swagger_ui_router()` so the UI auto-fetches the live spec.
    .merge(handlers::openapi::swagger_ui_router())
    // P2-1: Telegram notification routes — already layered with auth middleware
    // + Extension(alert_notifier) + Extension(telegram_chat_cache) at the top
    // of `create_router`. State type `Arc<DatabaseConnection>` matches the
    // global state (`app_state.db.clone()` set at the end of `create_router`).
    .merge(telegram_routes)
    .layer(cors)
    // P0-2: HTTP metrics middleware (P0-1 metric definitions)
    // Placed inside CORS but outside TraceLayer so:
    //   - CORS preflight responses are still counted
    //   - TraceLayer sees the real status from inner handlers
    //   - The middleware itself can read response.status() cheaply
    .layer(middleware::from_fn(
        quant_trading_backend::middleware::metrics::http_metrics_middleware,
    ))
    .layer(TraceLayer::new_for_http())
    .with_state(app_state.db.clone())
}

// Helper: Since axum 0.8 uses method routing differently for DELETE
fn delete_handler<H, T, S>(handler: H) -> axum::routing::MethodRouter<S>
where
    H: axum::handler::Handler<T, S> + Clone + Send + 'static,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    axum::routing::delete(handler)
}
