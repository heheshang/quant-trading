use axum::{
    Extension, Router, middleware,
    routing::{delete, get, post},
};
use quant_trading_backend::CONFIG;
use quant_trading_backend::db::{DbPool, init_db, run_migrations};
use quant_trading_backend::handlers;
use quant_trading_backend::services::binance_rest::BinanceRestClient;
use quant_trading_backend::services::exchange::ws_hub::{WsHub, WsHubBuilder};
use quant_trading_backend::services::exchange::{
    api_keys::{ApiKeyStore, get_master_key},
    signed_client::SignedBinanceClient,
};
use quant_trading_backend::services::kline_writer::KlineWriter;
use quant_trading_backend::services::matching_engine::MatchingEngine;
use quant_trading_backend::services::order_rate_limiter::OrderRateLimiter;
use quant_trading_backend::services::redis_cache::RedisCache;
use quant_trading_backend::services::risk_manager::RiskManager;
use quant_trading_backend::services::strategy_state_manager::StrategyStateManager;
use quant_trading_backend::services::ai::feature_engine::NormalizeMethod;
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&CONFIG.log_level)),
        )
        .json()
        .init();

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
    let matching_engine = Arc::new(MatchingEngine::new(db.clone(), 200, 0.001));

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

    // Initialize Signed Binance Client for authenticated API calls
    let master_key = get_master_key().unwrap_or([0u8; 32]);
    let key_store = Arc::new(ApiKeyStore::new(db.clone(), master_key));
    let signed_client = Arc::new(SignedBinanceClient::new(key_store.clone()));

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
    let risk_manager = Arc::new(RiskManager::new(db.clone()));
    let state_manager = Arc::new(StrategyStateManager::new(
        ws_hub.clone(),
        risk_manager.clone(),
    ));
    state_manager.start();

    // Build application
    let app = create_router(
        db,
        cors,
        matching_engine,
        order_rate_limiter,
        redis_cache,
        binance_rest,
        ws_hub,
        signed_client,
        key_store,
    );

    // Start server
    let addr = CONFIG.server_addr();
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app).await.expect("Server failed");
}

fn create_router(
    db: DbPool,
    cors: CorsLayer,
    matching_engine: Arc<MatchingEngine>,
    order_rate_limiter: Arc<OrderRateLimiter>,
    redis_cache: Arc<RedisCache>,
    binance_rest: Arc<BinanceRestClient>,
    ws_hub: Arc<WsHub>,
    signed_client: Arc<SignedBinanceClient>,
    key_store: Arc<ApiKeyStore>,
) -> Router {
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
        .route("/kline/kdj", get(handlers::indicator::get_kdj))
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
        .with_state(db.clone());

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
            "/admin/api-keys",
            get(handlers::api_key::admin_list_api_keys),
        )
        .layer(Extension(key_store.clone()))
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ))
        .with_state(db.clone());

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
        .with_state(db.clone());

    // Public routes — registered as direct path to avoid /api/v1 nesting shadowing
    // Both /ws and /api/v1/ws registered (nginx regex proxy_pass preserves full path)
    let public_routes = Router::new()
        .route("/health", get(handlers::ws::health_check))
        .route("/ws", get(handlers::ws::ws_handler))
        .route("/api/v1/ws", get(handlers::ws::ws_handler))
        .layer(Extension(ws_hub.clone()));

    app = Router::new()
        .nest("/api/v1/auth", auth_routes)
        .nest("/api/v1/auth", auth_protected)
        .nest("/api/v1", user_routes)
        .nest("/api/v1", strategy_routes)
        .nest("/api/v1", kline_routes)
        .nest("/api/v1", market_routes)
        .nest("/api/v1", order_routes)
        .nest("/api/v1", portfolio_routes)
        .nest(
            "/api/v1",
            handlers::risk::router()
                .layer(Extension(ws_hub.clone()))
                .with_state(db.clone()),
        )
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
        .merge(handlers::arbitrage::router().layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        )))
        .nest("/api/v1", dashboard_routes)
        .nest("/api/v1", backtest_routes)
        .nest("/api/v1", exchange_routes)
        .nest("/api/v1", api_key_routes)
        .nest("/api/v1/admin", admin_api_key_routes)
        .nest("/api/v1", review_routes);

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

    app
        .merge(
            handlers::ai::router()
                .layer(axum::Extension(ai_services))
                .layer(middleware::from_fn(
                    quant_trading_backend::middleware::auth::auth_middleware,
                )),
        )
        .merge(public_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(db)
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
