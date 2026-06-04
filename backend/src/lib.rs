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
        (name = "feature-flag", description = "Feature flag evaluation (user bootstrap) + admin CRUD"),
        (name = "export",      description = "CSV export for orders/trades/positions/account"),
        (name = "pamm",        description = "PAMM (Percent Allocation Management Module) funds (P3-1)"),
        (name = "copy-trading", description = "Copy trading: trader list, subscribe, on-order fan-out, profit shares (P6-2)"),
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
        // ── feature-flag ────────────────────────────────────────────
        handlers::feature_flag::evaluate_for_user,
        handlers::feature_flag::admin_list,
        handlers::feature_flag::admin_upsert,
        handlers::feature_flag::admin_delete,
        // ── audit ───────────────────────────────────────────────────
        handlers::audit_log::list_audit_logs,
        handlers::audit_log::get_audit_log,
        // ── pamm ───────────────────────────────────────────────────
        handlers::pamm::list_funds,
        handlers::pamm::create_fund,
        handlers::pamm::get_fund,
        handlers::pamm::list_fund_investments,
        handlers::pamm::subscribe,
        handlers::pamm::redeem,
        handlers::pamm::distribute,
        handlers::pamm::liquidate,
        handlers::pamm::my_investments,
        // ── copy-trading ──────────────────────────────────────────
        handlers::copy_trading::list_traders,
        handlers::copy_trading::get_trader,
        handlers::copy_trading::register,
        handlers::copy_trading::subscribe,
        handlers::copy_trading::unsubscribe,
        handlers::copy_trading::my_subscriptions,
        handlers::copy_trading::my_trader,
        handlers::copy_trading::trades,
        handlers::copy_trading::profit_shares,
        handlers::copy_trading::calculate_shares,
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
        crate::models::market_schemas::Exchange,
        crate::models::market_schemas::TickerResponse,
        crate::models::market_schemas::DepthResponse,
        // ── feature-flag ────────────────────────────────────────────
        handlers::feature_flag::FeatureFlag,
        handlers::feature_flag::FeatureFlagEvaluation,
        handlers::feature_flag::UpsertFeatureFlagRequest,
        // ── audit ───────────────────────────────────────────────────
        crate::services::audit_log::AuditLogView,
        // ── pamm ───────────────────────────────────────────────────
        handlers::pamm::CreateFundRequest,
        handlers::pamm::SubscribeRequest,
        handlers::pamm::RedeemRequest,
        handlers::pamm::DistributeRequest,
        handlers::pamm::CreateFundResponse,
        handlers::pamm::ListFundsResponse,
        handlers::pamm::SubscribeResponse,
        handlers::pamm::RedeemResponse,
        handlers::pamm::DistributeResponse,
        handlers::pamm::InvestmentsResponse,
        // ── copy-trading ──────────────────────────────────────────
        handlers::copy_trading::RegisterRequest,
        handlers::copy_trading::SubscribeRequest,
        handlers::copy_trading::UnsubscribeRequest,
        handlers::copy_trading::CalculateSharesRequest,
        handlers::copy_trading::ListTradersResponse,
        handlers::copy_trading::RegisterResponse,
        handlers::copy_trading::SubscribeResponse,
        handlers::copy_trading::UnsubscribeResponse,
        handlers::copy_trading::MySubscriptionsResponse,
        handlers::copy_trading::MyTraderResponse,
        handlers::copy_trading::TradesResponse,
        handlers::copy_trading::ProfitSharesResponse,
        handlers::copy_trading::CalculateSharesResponse,
        // ── copy-trading (view DTOs from the service layer) ───────
        crate::services::copy_trading::TraderView,
        crate::services::copy_trading::SubscriptionView,
        crate::services::copy_trading::CopyTradeView,
        crate::services::copy_trading::ProfitShareView,
        crate::services::copy_trading::FanOutSummary,
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
