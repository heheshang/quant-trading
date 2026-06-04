//! HTTP layer for the Copy Trading module — PRD §6 commercialization-2.
//!
//! 10 endpoints under `/api/v1/copy-trading/...`. Auth + role checks
//! are applied at the router layer in `main.rs`; the handlers
//! themselves are thin shims that translate HTTP into
//! `services::copy_trading` calls.
//!
//! Routing layout:
//!
//!   GET    /api/v1/copy-trading/traders          → list_traders   (any auth user)
//!   GET    /api/v1/copy-trading/traders/{id}     → get_trader     (any auth user)
//!   POST   /api/v1/copy-trading/register         → register_as_trader
//!   POST   /api/v1/copy-trading/subscribe        → subscribe
//!   POST   /api/v1/copy-trading/unsubscribe      → unsubscribe
//!   GET    /api/v1/copy-trading/my-subscriptions → list_my_subscriptions
//!   GET    /api/v1/copy-trading/my-trader        → find_trader_for_user + list_trader_subscriptions
//!   GET    /api/v1/copy-trading/trades           → trades_for_follower
//!   GET    /api/v1/copy-trading/profit-shares    → profit_shares_for_user
//!   POST   /api/v1/copy-trading/calculate-shares → calculate_profit_share
//!
//! The order path triggers fan-out directly via
//! `services::copy_trading::spawn_on_trader_order` from
//! `handlers/order.rs` — there is no public endpoint for that.
//!
//! All numeric decimals are serialised as strings (rust_decimal precision).

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::auth::AuthenticatedUser;
use crate::services::copy_trading as svc;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ─── Request DTOs ───────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    pub display_name: String,
    pub bio: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SubscribeRequest {
    pub trader_id: Uuid,
    /// Copy ratio in (0, 1]. String to preserve precision.
    pub ratio: String,
    /// Per-trade cap. 0 = no cap.
    pub max_position_size: String,
    /// Daily loss cap. 0 = no cap.
    pub max_loss_per_day: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UnsubscribeRequest {
    pub subscription_id: Uuid,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CalculateSharesRequest {
    pub subscription_id: Uuid,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
    pub trader_fee_pct: Option<String>,
}

// ─── Response DTOs ──────────────────────────────────────────────────

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListTradersResponse {
    pub items: Vec<svc::TraderView>,
    pub total: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RegisterResponse {
    pub trader: svc::TraderView,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SubscribeResponse {
    pub subscription_id: Uuid,
    pub status: String,
    pub ratio: String,
    pub max_position_size: String,
    pub max_loss_per_day: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UnsubscribeResponse {
    pub subscription_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MySubscriptionsResponse {
    pub items: Vec<svc::SubscriptionView>,
    pub total: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MyTraderResponse {
    pub trader: Option<svc::TraderView>,
    pub subscribers: Vec<svc::SubscriptionView>,
    pub total_subscribers: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TradesResponse {
    pub items: Vec<svc::CopyTradeView>,
    pub total: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ProfitSharesResponse {
    pub items: Vec<svc::ProfitShareView>,
    pub total: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CalculateSharesResponse {
    pub share: svc::ProfitShareView,
}

// ─── Handlers ───────────────────────────────────────────────────────

/// `GET /api/v1/copy-trading/traders` — list active traders, sorted by
/// monthly_pnl DESC.
#[utoipa::path(
    get,
    path = "/api/v1/copy-trading/traders",
    tag = "copy-trading",
    operation_id = "copy_trading_list_traders",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of active copy traders", body = ListTradersResponse),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn list_traders(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<ListTradersResponse>>, AppError> {
    let items = svc::list_traders(&db, 200).await?;
    let total = items.len() as u64;
    Ok(Json(ApiResponse::success(ListTradersResponse { items, total })))
}

/// `GET /api/v1/copy-trading/traders/{id}` — trader detail.
#[utoipa::path(
    get,
    path = "/api/v1/copy-trading/traders/{id}",
    tag = "copy-trading",
    operation_id = "copy_trading_get_trader",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Trader id")),
    responses(
        (status = 200, description = "Trader detail", body = svc::TraderView),
        (status = 401, description = "Unauthenticated"),
        (status = 404, description = "Trader not found"),
    )
)]
pub async fn get_trader(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<svc::TraderView>>, AppError> {
    let t = svc::get_trader(&db, id).await?;
    Ok(Json(ApiResponse::success(svc::TraderView::from_trader(t, None))))
}

/// `POST /api/v1/copy-trading/register` — register the caller as a
/// copy trader.
#[utoipa::path(
    post,
    path = "/api/v1/copy-trading/register",
    tag = "copy-trading",
    operation_id = "copy_trading_register",
    security(("bearer_auth" = [])),
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registered as a copy trader", body = RegisterResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 409, description = "Already a copy trader"),
    )
)]
pub async fn register(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<RegisterResponse>>, AppError> {
    let t = svc::register_as_trader(&db, user.user_id, &req.display_name, req.bio).await?;
    Ok(Json(ApiResponse::success(RegisterResponse {
        trader: svc::TraderView::from_trader(t, None),
    })))
}

/// `POST /api/v1/copy-trading/subscribe` — subscribe to a trader.
#[utoipa::path(
    post,
    path = "/api/v1/copy-trading/subscribe",
    tag = "copy-trading",
    operation_id = "copy_trading_subscribe",
    security(("bearer_auth" = [])),
    request_body = SubscribeRequest,
    responses(
        (status = 200, description = "Subscription created", body = SubscribeResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 404, description = "Trader not found"),
        (status = 409, description = "Already subscribed / trader not active"),
    )
)]
pub async fn subscribe(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<SubscribeRequest>,
) -> Result<Json<ApiResponse<SubscribeResponse>>, AppError> {
    let ratio: Decimal = req
        .ratio
        .parse()
        .map_err(|_| AppError::Validation("ratio must be decimal".into()))?;
    let max_pos: Decimal = req
        .max_position_size
        .parse()
        .map_err(|_| AppError::Validation("max_position_size must be decimal".into()))?;
    let max_loss: Decimal = req
        .max_loss_per_day
        .parse()
        .map_err(|_| AppError::Validation("max_loss_per_day must be decimal".into()))?;
    let sub = svc::subscribe(&db, user.user_id, req.trader_id, ratio, max_pos, max_loss).await?;
    Ok(Json(ApiResponse::success(SubscribeResponse {
        subscription_id: sub.id,
        status: sub.status,
        ratio: sub.ratio.to_string(),
        max_position_size: sub.max_position_size.to_string(),
        max_loss_per_day: sub.max_loss_per_day.to_string(),
    })))
}

/// `POST /api/v1/copy-trading/unsubscribe` — cancel a subscription.
#[utoipa::path(
    post,
    path = "/api/v1/copy-trading/unsubscribe",
    tag = "copy-trading",
    operation_id = "copy_trading_unsubscribe",
    security(("bearer_auth" = [])),
    request_body = UnsubscribeRequest,
    responses(
        (status = 200, description = "Subscription cancelled", body = UnsubscribeResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Not the subscription owner"),
        (status = 404, description = "Subscription not found"),
    )
)]
pub async fn unsubscribe(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<UnsubscribeRequest>,
) -> Result<Json<ApiResponse<UnsubscribeResponse>>, AppError> {
    let sub = svc::unsubscribe(&db, user.user_id, req.subscription_id).await?;
    Ok(Json(ApiResponse::success(UnsubscribeResponse {
        subscription_id: sub.id,
        status: sub.status,
    })))
}

/// `GET /api/v1/copy-trading/my-subscriptions` — subscriptions the
/// caller has as a follower.
#[utoipa::path(
    get,
    path = "/api/v1/copy-trading/my-subscriptions",
    tag = "copy-trading",
    operation_id = "copy_trading_my_subscriptions",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Caller's subscriptions", body = MySubscriptionsResponse),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn my_subscriptions(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<MySubscriptionsResponse>>, AppError> {
    let items = svc::list_my_subscriptions(&db, user.user_id).await?;
    let total = items.len() as u64;
    Ok(Json(ApiResponse::success(MySubscriptionsResponse { items, total })))
}

/// `GET /api/v1/copy-trading/my-trader` — caller's trader profile +
/// subscribers.
#[utoipa::path(
    get,
    path = "/api/v1/copy-trading/my-trader",
    tag = "copy-trading",
    operation_id = "copy_trading_my_trader",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Caller's trader profile and subscribers", body = MyTraderResponse),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn my_trader(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<MyTraderResponse>>, AppError> {
    let trader = svc::find_trader_for_user(&db, user.user_id).await?;
    let (trader_view, subscribers) = match trader {
        Some(t) => {
            let v = svc::TraderView::from_trader(t.clone(), None);
            let subs = svc::list_trader_subscriptions(&db, t.id).await?;
            (Some(v), subs)
        }
        None => (None, vec![]),
    };
    let total = subscribers.len() as u64;
    Ok(Json(ApiResponse::success(MyTraderResponse {
        trader: trader_view,
        subscribers,
        total_subscribers: total,
    })))
}

/// `GET /api/v1/copy-trading/trades` — the caller's audit trail of
/// copied trades.
#[utoipa::path(
    get,
    path = "/api/v1/copy-trading/trades",
    tag = "copy-trading",
    operation_id = "copy_trading_trades",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Caller's copied-trade audit", body = TradesResponse),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn trades(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<TradesResponse>>, AppError> {
    let items = svc::trades_for_follower(&db, user.user_id, 200).await?;
    let total = items.len() as u64;
    Ok(Json(ApiResponse::success(TradesResponse { items, total })))
}

/// `GET /api/v1/copy-trading/profit-shares` — the caller's profit-share
/// history (as either follower or trader).
#[utoipa::path(
    get,
    path = "/api/v1/copy-trading/profit-shares",
    tag = "copy-trading",
    operation_id = "copy_trading_profit_shares",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Caller's profit-share history", body = ProfitSharesResponse),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn profit_shares(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<ProfitSharesResponse>>, AppError> {
    let items = svc::profit_shares_for_user(&db, user.user_id).await?;
    let total = items.len() as u64;
    Ok(Json(ApiResponse::success(ProfitSharesResponse { items, total })))
}

/// `POST /api/v1/copy-trading/calculate-shares` — manager/trader
/// triggers a profit-share settlement for one subscription across the
/// given period. Idempotent on (subscription, period_start, period_end).
#[utoipa::path(
    post,
    path = "/api/v1/copy-trading/calculate-shares",
    tag = "copy-trading",
    operation_id = "copy_trading_calculate_shares",
    security(("bearer_auth" = [])),
    request_body = CalculateSharesRequest,
    responses(
        (status = 200, description = "Profit share calculated", body = CalculateSharesResponse),
        (status = 400, description = "Validation failed / no trades in period"),
        (status = 401, description = "Unauthenticated"),
        (status = 409, description = "Already distributed for this period"),
    )
)]
pub async fn calculate_shares(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<CalculateSharesRequest>,
) -> Result<Json<ApiResponse<CalculateSharesResponse>>, AppError> {
    let _ = user; // any auth user can request; service layer would gate
                  // tighter for v0.2.
    let fee_pct = match req.trader_fee_pct.as_deref() {
        Some(s) => Some(
            s.parse::<Decimal>()
                .map_err(|_| AppError::Validation("trader_fee_pct must be decimal".into()))?,
        ),
        None => None,
    };
    let share =
        svc::calculate_profit_share(&db, req.subscription_id, req.period_start, req.period_end, fee_pct)
            .await?;
    Ok(Json(ApiResponse::success(CalculateSharesResponse {
        share: share.into(),
    })))
}

// ─── Router factory ────────────────────────────────────────────────

/// User-facing copy-trading sub-router. Manager-only actions (none in
/// v0.1 — the trader themselves is the only actor) live behind a
/// future `router_manager` if/when admin override is needed.
pub fn router() -> Router<Arc<DatabaseConnection>> {
    Router::new()
        .route("/copy-trading/traders", get(list_traders))
        .route("/copy-trading/traders/{id}", get(get_trader))
        .route("/copy-trading/register", post(register))
        .route("/copy-trading/subscribe", post(subscribe))
        .route("/copy-trading/unsubscribe", post(unsubscribe))
        .route("/copy-trading/my-subscriptions", get(my_subscriptions))
        .route("/copy-trading/my-trader", get(my_trader))
        .route("/copy-trading/trades", get(trades))
        .route("/copy-trading/profit-shares", get(profit_shares))
        .route("/copy-trading/calculate-shares", post(calculate_shares))
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// The router builds without panicking; the surface is registered.
    #[test]
    fn router_builds() {
        let r = router();
        let _ = r.into_make_service();
    }

    /// `SubscribeRequest` parses ratio as a string (no Decimal in JSON).
    #[test]
    fn subscribe_request_parses() {
        let json = r#"{"trader_id":"00000000-0000-0000-0000-000000000001","ratio":"0.5","max_position_size":"1000","max_loss_per_day":"100"}"#;
        let req: SubscribeRequest = serde_json::from_str(json).expect("parse");
        assert_eq!(req.ratio, "0.5");
        assert_eq!(req.max_position_size, "1000");
    }

    /// `CalculateSharesRequest` keeps the period as RFC3339 timestamps.
    #[test]
    fn calculate_shares_request_parses() {
        let json = r#"{"subscription_id":"00000000-0000-0000-0000-000000000001","period_start":"2026-06-01T00:00:00Z","period_end":"2026-06-30T23:59:59Z","trader_fee_pct":null}"#;
        let req: CalculateSharesRequest = serde_json::from_str(json).expect("parse");
        assert_eq!(req.trader_fee_pct, None);
    }
}
