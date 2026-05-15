use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use quant_trading_backend::db::{init_db, run_migrations, DbPool};
use quant_trading_backend::handlers;
use quant_trading_backend::services::matching_engine::MatchingEngine;
use quant_trading_backend::CONFIG;
use std::sync::Arc;
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

    // Build application
    let app = create_router(db, cors, matching_engine);

    // Start server
    let addr = CONFIG.server_addr();
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app).await.expect("Server failed");
}

fn create_router(db: DbPool, cors: CorsLayer, matching_engine: Arc<MatchingEngine>) -> Router {
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
        .route("/kline/import-history", get(handlers::kline::import_history))
        .route("/kline/quality", get(handlers::kline::quality_report))
        .route("/kline/clean", post(handlers::kline::clean_klines))
        .route("/kline/export", get(handlers::kline::export_klines))
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
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Order/Trading routes (authenticated, with matching engine)
    let order_routes = Router::new()
        .route("/orders", post(handlers::order::create_order))
        .route("/orders", get(handlers::order::list_orders))
        .route("/orders/{id}", get(handlers::order::get_order))
        .route("/orders/{id}/cancel", post(handlers::order::cancel_order))
        .route("/orders/cancel-all", post(handlers::order::cancel_all_orders))
        .route("/trades", get(handlers::order::list_trades))
        .route("/positions", get(handlers::order::list_positions))
        .route("/positions/{symbol}/close", post(handlers::order::close_position))
        .route("/account", get(handlers::order::get_account))
        .route("/account/init", post(handlers::order::init_account))
        .route("/symbols", get(handlers::order::list_symbols))
        .layer(axum::Extension(matching_engine))
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

    // Backtest routes (authenticated)
    let backtest_routes = Router::new()
        .route("/backtest", post(handlers::backtest::run_backtest))
        .route("/backtest/{id}", get(handlers::backtest::get_backtest))
        .route(
            "/backtest/{id}",
            delete_handler(handlers::backtest::delete_backtest),
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
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Cancel backtest endpoint (separate router to avoid matchit/axum handler type conflict)
    let cancel_routes = Router::new()
        .route(
            "/backtest/{id}/cancel",
            post(handlers::backtest::cancel_backtest),
        )
        .layer(middleware::from_fn(
            quant_trading_backend::middleware::auth::auth_middleware,
        ));

    // Public routes
    let public_routes = Router::new()
        .route("/health", get(handlers::ws::health_check))
        .route("/ws", get(handlers::ws::ws_handler));

    Router::new()
        .nest("/api/v1/auth", auth_routes)
        .nest("/api/v1/auth", auth_protected)
        .nest("/api/v1", user_routes)
        .nest("/api/v1", strategy_routes)
        .nest("/api/v1", kline_routes)
        .nest("/api/v1", market_routes)
        .nest("/api/v1", order_routes)
        .nest("/api/v1", portfolio_routes)
        .nest("/api/v1", backtest_routes)
        .nest("/api/v1", cancel_routes)
        .nest("/api/v1", public_routes)
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
