pub mod config;
pub mod db;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod mq;
pub mod models;
pub mod observability;
pub mod services;
pub mod state;
pub mod strategies;
pub mod utils;
pub mod workers;

pub use models::{backtest, schemas};

use config::Config;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::from_env);

// ─── OpenAPI ────────────────────────────────────────────────────────────
//
// `ApiDoc` aggregates every `#[utoipa::path]`-annotated handler plus the
// `ToSchema`-derived request/response DTOs into a single OpenAPI 3.1 spec.
//
// Three consumers:
//   1. `handlers::openapi::openapi_json` — serves `/api/v1/openapi.json`
//   2. `handlers::openapi::swagger_ui`  — serves Swagger UI at `/swagger-ui`
//   3. `bin/export_openapi.rs`          — offline export to
//      `docs/openapi.json`, fed into `npx openapi-typescript` to produce
//      `frontend/src/types/api-generated.ts`.
//
// Adding a new annotated handler here requires also listing it in the
// `paths(...)` block below — utoipa's `derive(OpenApi)` validator will
// reject the build if a path function is referenced but missing the
// `#[utoipa::path]` attribute, or vice versa.
#[derive(utoipa::OpenApi)]
#[openapi(
    info(
        title = "Quant Trading API",
        version = env!("CARGO_PKG_VERSION"),
        description = "Full quantitative trading platform API — auth, market data, \
                       strategies, backtests, orders, positions, risk, AI predictions.",
        contact(name = "Quant Trading", email = "dev@quant-trading.local"),
    ),
    servers(
        (url = "http://localhost:8080", description = "Local dev"),
        (url = "/", description = "Same-origin (nginx)"),
    ),
    tags(
        (name = "auth",        description = "Authentication: register, login, refresh, me"),
        (name = "users",       description = "User management, roles, permissions (admin)"),
        (name = "strategy",    description = "Strategy CRUD, templates, code upload, import/export"),
        (name = "kline",       description = "Kline import, query, quality, clean, export"),
        (name = "indicator",   description = "Technical indicators (KDJ, MA, MACD, RSI, Bollinger, ...)"),
        (name = "market",      description = "Public market data (tickers, depth, kline)"),
        (name = "order",       description = "Order placement, cancellation, queries, account"),
        (name = "risk",        description = "Risk rules, logs, emergency close, pause/resume"),
        (name = "dashboard",   description = "Dashboard aggregates (stats, PnL)"),
        (name = "portfolio",   description = "Portfolio summary, positions, performance, equity"),
        (name = "backtest",    description = "Backtest run/inspect/cancel + history"),
        (name = "review",      description = "Strategy review workflow (submit/approve/reject)"),
        (name = "audit",       description = "Admin audit log query"),
        (name = "system",      description = "Liveness, readiness, metrics"),
        (name = "exchange",    description = "Live exchange (Binance / OKX) signed endpoints"),
        (name = "api-key",     description = "User API-key management"),
        (name = "admin-ip",    description = "Admin IP allow-list (P3-A)"),
        (name = "position-alert", description = "Position price/equity alert rules"),
        (name = "trigger-order", description = "Stop-loss / take-profit / OCO / TWAP"),
        (name = "bracket",     description = "Bracket order linking"),
        (name = "arbitrage",   description = "Cross-exchange pair/spread/position/signal"),
        (name = "ai",          description = "AI prediction endpoints (P3-F3)"),
        (name = "telegram",    description = "Telegram notification bind/test (P2-1)"),
        (name = "export",      description = "CSV export for orders/trades/positions/account"),
    ),
    paths(
        // ── system ──────────────────────────────────────────────────
        handlers::health::liveness,
        handlers::health::readiness,
        // ── auth ────────────────────────────────────────────────────
        handlers::auth::register,
        handlers::auth::login,
        handlers::auth::refresh,
        handlers::auth::logout,
        handlers::auth::me_route,
        // ── users ───────────────────────────────────────────────────
        handlers::users::get_me,
        handlers::users::change_password,
        handlers::users::admin_update_user,
        // ── market ──────────────────────────────────────────────────
        handlers::market::get_tickers,
        handlers::market::get_ticker,
        handlers::market::get_depth,
        // ── audit ───────────────────────────────────────────────────
        handlers::audit_log::list_audit_logs,
        handlers::audit_log::get_audit_log,
    ),
    components(schemas(
        // ── system (health) ────────────────────────────────────────
        handlers::health::LivenessResponse,
        handlers::health::ReadinessResponse,
        // ── auth / user ─────────────────────────────────────────────
        crate::models::schemas::RegisterRequest,
        crate::models::schemas::LoginRequest,
        crate::models::schemas::RefreshTokenRequest,
        crate::models::schemas::ChangePasswordRequest,
        crate::models::schemas::AdminUpdateUserRequest,
        crate::models::schemas::UserResponse,
        crate::models::schemas::UserMeResponse,
        crate::models::schemas::AuthResponseBody,
        crate::models::schemas::TokenResponseBody,
        crate::models::schemas::LogoutResponse,
        // ── market ──────────────────────────────────────────────────
        crate::models::market_schemas::TickerResponse,
        crate::models::market_schemas::DepthResponse,
        // ── audit ───────────────────────────────────────────────────
        crate::services::audit_log::AuditLogView,
    )),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

/// Bearer-token security scheme applied to every authenticated endpoint.
///
/// utoipa 5 + utoipa-axum 0.2: the `security(...)` attribute on each
/// `#[utoipa::path]` references the name `"bearer_auth"`; the `SecurityAddon`
/// modifier registers the corresponding `securitySchemes` entry. Swagger
/// UI picks it up and offers an "Authorize" dialog where the user pastes
/// their JWT.
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::new);
        components.add_security_scheme(
            "bearer_auth",
            utoipa::openapi::security::SecurityScheme::Http(
                utoipa::openapi::security::HttpBuilder::new()
                    .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}
