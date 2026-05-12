use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use quant_trading_backend::db::{init_db, run_migrations, DbPool};
use quant_trading_backend::handlers;
use quant_trading_backend::CONFIG;
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
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&CONFIG.log_level)),
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
        .allow_origin(CONFIG.cors_allowed_origins.iter().map(|s| {
            s.parse::<axum::http::HeaderValue>().unwrap_or_else(|_| {
                axum::http::HeaderValue::from_static("http://localhost:5173")
            })
        }).collect::<Vec<_>>())
        .allow_methods(Any)
        .allow_headers(Any);

    // Build application
    let app = create_router(db, cors);

    // Start server
    let addr = CONFIG.server_addr();
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

fn create_router(db: DbPool, cors: CorsLayer) -> Router {
    let state = db.clone();

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
        .route("/users/{id}", delete_handler(handlers::users::admin_delete_user))
        .route("/roles", get(handlers::users::list_roles))
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
        .nest("/api/v1", public_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
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
