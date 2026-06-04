//! HTTP layer for the PAMM (Percent Allocation Management Module).
//!
//! 9 endpoints, all under `/api/v1/pamm/...`. Auth + role checks are
//! applied at the router layer in `main.rs`; the handlers themselves
//! are thin shims that translate HTTP into `services::pamm` calls.
//!
//! Routing layout:
//!
//!   GET    /api/v1/pamm/funds                  → list_active_funds (any auth user)
//!   POST   /api/v1/pamm/funds                  → create_fund      (any auth user — manager of own fund)
//!   GET    /api/v1/pamm/funds/{id}             → get_fund
//!   GET    /api/v1/pamm/funds/{id}/investments → list_fund_investments (manager + self)
//!   POST   /api/v1/pamm/funds/{id}/subscribe   → subscribe        (any auth user, not the manager)
//!   POST   /api/v1/pamm/funds/{id}/redeem      → redeem           (must have an investment)
//!   POST   /api/v1/pamm/funds/{id}/distribute  → distribute_profits (manager-only)
//!   POST   /api/v1/pamm/funds/{id}/liquidate   → liquidate         (manager-only)
//!   GET    /api/v1/pamm/my-investments         → list_my_investments
//!
//! Manager-only checks live in the service layer
//! (`PammError::NotManager`) and translate to HTTP 403.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::auth::AuthenticatedUser;
use crate::services::pamm as svc;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ─── Request DTOs ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateFundRequest {
    pub name: String,
    pub description: Option<String>,
    pub base_currency: String,
    /// Annualised management fee as a fraction (0.02 = 2%). Stored as
    /// string to preserve precision (JSON Decimal round-trip).
    pub management_fee_pct: String,
    /// Performance fee as a fraction of HWM-increment profit (0.20 = 20%).
    pub performance_fee_pct: String,
    pub high_water_mark: bool,
    pub strategy_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SubscribeRequest {
    /// Amount in `fund.base_currency`. Decimal string.
    pub amount: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RedeemRequest {
    pub amount: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DistributeRequest {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

// ─── Response DTOs ───────────────────────────────────────────────────

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateFundResponse {
    pub fund: svc::FundView,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListFundsResponse {
    pub items: Vec<svc::FundView>,
    pub total: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SubscribeResponse {
    pub subscription_id: Uuid,
    pub status: String,
    pub shares: String,
    pub share_value: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RedeemResponse {
    pub redemption_id: Uuid,
    pub status: String,
    pub amount_paid: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DistributeResponse {
    pub distributions: Vec<svc::DistributionView>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct InvestmentsResponse {
    pub items: Vec<svc::InvestmentView>,
    pub total: u64,
}

// ─── Handlers ────────────────────────────────────────────────────────

/// `GET /api/v1/pamm/funds` — list all `Active` funds.
#[utoipa::path(
    get,
    path = "/api/v1/pamm/funds",
    tag = "pamm",
    operation_id = "pamm_list_funds",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Page of active funds", body = ListFundsResponse),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn list_funds(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<ListFundsResponse>>, AppError> {
    let _ = user; // auth-only, no per-user logic
    let items = svc::list_active_funds(&db).await?;
    let total = items.len() as u64;
    Ok(Json(ApiResponse::success(ListFundsResponse { items, total })))
}

/// `POST /api/v1/pamm/funds` — open a new fund. The caller becomes
/// its manager.
#[utoipa::path(
    post,
    path = "/api/v1/pamm/funds",
    tag = "pamm",
    operation_id = "pamm_create_fund",
    security(("bearer_auth" = [])),
    request_body = CreateFundRequest,
    responses(
        (status = 200, description = "Fund created", body = CreateFundResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn create_fund(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<CreateFundRequest>,
) -> Result<Json<ApiResponse<CreateFundResponse>>, AppError> {
    let mgmt: Decimal = req
        .management_fee_pct
        .parse()
        .map_err(|_| AppError::Validation("management_fee_pct must be decimal".into()))?;
    let perf: Decimal = req
        .performance_fee_pct
        .parse()
        .map_err(|_| AppError::Validation("performance_fee_pct must be decimal".into()))?;
    let fund = svc::create_fund(
        &db,
        user.user_id,
        &req.name,
        req.description,
        &req.base_currency,
        mgmt,
        perf,
        req.high_water_mark,
        req.strategy_id,
    )
    .await?;
    let view = svc::FundView::from_fund(fund);
    Ok(Json(ApiResponse::success(CreateFundResponse { fund: view })))
}

/// `GET /api/v1/pamm/funds/{id}` — fund detail + current NAV.
#[utoipa::path(
    get,
    path = "/api/v1/pamm/funds/{id}",
    tag = "pamm",
    operation_id = "pamm_get_fund",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Fund id")),
    responses(
        (status = 200, description = "Fund detail", body = svc::FundView),
        (status = 401, description = "Unauthenticated"),
        (status = 404, description = "Fund not found"),
    )
)]
pub async fn get_fund(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<svc::FundView>>, AppError> {
    let f = svc::get_fund(&db, id).await?;
    Ok(Json(ApiResponse::success(svc::FundView::from_fund(f))))
}

/// `GET /api/v1/pamm/funds/{id}/investments` — manager + self can
/// see the investor list.
#[utoipa::path(
    get,
    path = "/api/v1/pamm/funds/{id}/investments",
    tag = "pamm",
    operation_id = "pamm_list_fund_investments",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Fund id")),
    responses(
        (status = 200, description = "Investor list (manager + self)", body = InvestmentsResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Not the fund manager"),
        (status = 404, description = "Fund not found"),
    )
)]
pub async fn list_fund_investments(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<InvestmentsResponse>>, AppError> {
    // Manager or self-investor can view.
    let is_manager = svc::get_fund(&db, id)
        .await
        .map(|f| f.manager_id == user.user_id)
        .unwrap_or(false);
    let has_self = svc::get_user_investment(&db, id, user.user_id)
        .await?
        .is_some();
    if !is_manager && !has_self {
        return Err(AppError::Forbidden(
            "must be the fund manager or an investor".into(),
        ));
    }
    let items = svc::list_fund_investments(&db, id).await?;
    let total = items.len() as u64;
    Ok(Json(ApiResponse::success(InvestmentsResponse { items, total })))
}

/// `POST /api/v1/pamm/funds/{id}/subscribe` — submit a subscription.
#[utoipa::path(
    post,
    path = "/api/v1/pamm/funds/{id}/subscribe",
    tag = "pamm",
    operation_id = "pamm_subscribe",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Fund id")),
    request_body = SubscribeRequest,
    responses(
        (status = 200, description = "Subscription accepted", body = SubscribeResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 404, description = "Fund not found"),
        (status = 409, description = "Fund not active"),
    )
)]
pub async fn subscribe(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
    Json(req): Json<SubscribeRequest>,
) -> Result<Json<ApiResponse<SubscribeResponse>>, AppError> {
    let amount: Decimal = req
        .amount
        .parse()
        .map_err(|_| AppError::Validation("amount must be decimal".into()))?;
    let sub = svc::subscribe(&db, user.user_id, id, amount).await?;
    let fund = svc::get_fund(&db, id).await?;
    let shares = amount / fund.share_value;
    Ok(Json(ApiResponse::success(SubscribeResponse {
        subscription_id: sub.id,
        status: sub.status,
        shares: shares.to_string(),
        share_value: fund.share_value.to_string(),
    })))
}

/// `POST /api/v1/pamm/funds/{id}/redeem` — submit a redemption.
#[utoipa::path(
    post,
    path = "/api/v1/pamm/funds/{id}/redeem",
    tag = "pamm",
    operation_id = "pamm_redeem",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Fund id")),
    request_body = RedeemRequest,
    responses(
        (status = 200, description = "Redemption accepted", body = RedeemResponse),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthenticated"),
        (status = 404, description = "Fund not found / no investment"),
    )
)]
pub async fn redeem(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
    Json(req): Json<RedeemRequest>,
) -> Result<Json<ApiResponse<RedeemResponse>>, AppError> {
    let amount: Decimal = req
        .amount
        .parse()
        .map_err(|_| AppError::Validation("amount must be decimal".into()))?;
    let red = svc::redeem(&db, user.user_id, id, amount).await?;
    Ok(Json(ApiResponse::success(RedeemResponse {
        redemption_id: red.id,
        status: red.status,
        amount_paid: red.amount_paid.to_string(),
    })))
}

/// `POST /api/v1/pamm/funds/{id}/distribute` — manager triggers a
/// profit distribution for the given period.
#[utoipa::path(
    post,
    path = "/api/v1/pamm/funds/{id}/distribute",
    tag = "pamm",
    operation_id = "pamm_distribute",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Fund id")),
    request_body = DistributeRequest,
    responses(
        (status = 200, description = "Distribution complete", body = DistributeResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Not the fund manager"),
        (status = 409, description = "Fund not active"),
    )
)]
pub async fn distribute(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
    Json(req): Json<DistributeRequest>,
) -> Result<Json<ApiResponse<DistributeResponse>>, AppError> {
    let distributions =
        svc::distribute_profits(&db, user.user_id, id, req.period_start, req.period_end).await?;
    Ok(Json(ApiResponse::success(DistributeResponse {
        distributions,
        period_start: req.period_start,
        period_end: req.period_end,
    })))
}

/// `POST /api/v1/pamm/funds/{id}/liquidate` — manager liquidates the
/// fund. Returns 204 No Content on success.
#[utoipa::path(
    post,
    path = "/api/v1/pamm/funds/{id}/liquidate",
    tag = "pamm",
    operation_id = "pamm_liquidate",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Fund id")),
    responses(
        (status = 204, description = "Fund liquidated"),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Not the fund manager"),
    )
)]
pub async fn liquidate(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    svc::liquidate(&db, user.user_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/pamm/my-investments` — the caller's investments.
#[utoipa::path(
    get,
    path = "/api/v1/pamm/my-investments",
    tag = "pamm",
    operation_id = "pamm_my_investments",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "My investments", body = InvestmentsResponse),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn my_investments(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<InvestmentsResponse>>, AppError> {
    let items = svc::list_my_investments(&db, user.user_id).await?;
    let total = items.len() as u64;
    Ok(Json(ApiResponse::success(InvestmentsResponse { items, total })))
}

// ─── Router factory ──────────────────────────────────────────────────

/// Build the **user-facing** PAMM sub-router. The caller in `main.rs`
/// layers `auth_middleware` (any authenticated user) on top.
///
/// Manager-only checks (distribute / liquidate) live in the manager
/// sub-router; non-manager tokens get 403 there. `create_fund` is
/// intentionally exposed here because any authenticated user can open a
/// fund (they become its manager on creation); the service layer
/// enforces that the caller == new manager.
pub fn router_user() -> Router<Arc<DatabaseConnection>> {
    Router::new()
        .route("/pamm/funds", get(list_funds))
        .route("/pamm/funds", post(create_fund))
        .route("/pamm/funds/{id}", get(get_fund))
        .route("/pamm/funds/{id}/investments", get(list_fund_investments))
        .route("/pamm/funds/{id}/subscribe", post(subscribe))
        .route("/pamm/funds/{id}/redeem", post(redeem))
        .route("/pamm/my-investments", get(my_investments))
}

/// Build the **manager-only** PAMM sub-router. The caller in `main.rs`
/// layers BOTH `auth_middleware` AND `require_admin_middleware` on top
/// (admin role required to distribute / liquidate).
///
/// Defence in depth: the service layer also checks `f.manager_id ==
/// user.user_id` and returns `PammError::NotManager` → 403, so even an
/// admin token that doesn't own the fund is rejected by the handler.
pub fn router_manager() -> Router<Arc<DatabaseConnection>> {
    Router::new()
        .route("/pamm/funds/{id}/distribute", post(distribute))
        .route("/pamm/funds/{id}/liquidate", post(liquidate))
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// `Router` builds without panicking; the surface is registered.
    /// Both the user-facing and manager-only sub-routers should build.
    #[test]
    fn router_builds() {
        let r = router_user();
        let _ = r.into_make_service();
        let m = router_manager();
        let _ = m.into_make_service();
    }

    /// The DTOs round-trip the expected fields. We test via JSON to
    /// catch a future field rename in the public surface.
    #[test]
    fn create_fund_request_serializes() {
        let v = CreateFundRequest {
            name: "Alpha".into(),
            description: Some("BTC momentum".into()),
            base_currency: "USDT".into(),
            management_fee_pct: "0.02".into(),
            performance_fee_pct: "0.20".into(),
            high_water_mark: true,
            strategy_id: None,
        };
        let json = serde_json::to_value(&v).expect("serialize");
        assert_eq!(json["name"], "Alpha");
        assert_eq!(json["management_fee_pct"], "0.02");
        assert_eq!(json["high_water_mark"], true);
    }

    /// `SubscribeRequest` accepts a string amount (no Decimal in JSON).
    #[test]
    fn subscribe_request_parses() {
        let json = r#"{"amount":"100.50"}"#;
        let req: SubscribeRequest = serde_json::from_str(json).expect("parse");
        assert_eq!(req.amount, "100.50");
    }

    /// `DistributeRequest` keeps the period as RFC3339 timestamps.
    #[test]
    fn distribute_request_parses() {
        let json = r#"{"period_start":"2026-06-01T00:00:00Z","period_end":"2026-06-30T23:59:59Z"}"#;
        let req: DistributeRequest = serde_json::from_str(json).expect("parse");
        assert_eq!(req.period_start.to_rfc3339(), "2026-06-01T00:00:00+00:00");
        assert_eq!(req.period_end.to_rfc3339(), "2026-06-30T23:59:59+00:00");
    }
}
