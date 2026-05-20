//! Exchange API Handlers — 交易所 API 签名认证
//!
//! ADR-012 D5: 签名端点映射
//! - GET  /api/v1/exchange/ping       — 测试连通性（JWT）
//! - GET  /api/v1/exchange/account     — 账户余额（JWT+HMAC）
//! - POST /api/v1/exchange/order      — 下单（JWT+HMAC）
//! - DELETE /api/v1/exchange/order/{orderId} — 撤单（JWT+HMAC）
//! - GET  /api/v1/exchange/rate-limit — 频率限制状态（JWT+HMAC）

use axum::{Extension, Json, extract::Path, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::middleware::auth::AuthenticatedUser;
use crate::services::exchange::signed_client::{NewOrder, SignedBinanceClient};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use std::sync::Arc;

// ─── Request Types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateExchangeOrderRequest {
    pub symbol: String,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub quantity: Option<String>,
    pub price: Option<String>,
    pub time_in_force: Option<String>,
    pub quote_order_qty: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CancelExchangeOrderRequest {
    pub symbol: String,
}

// ─── Response Types ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ExchangePingResponse {
    pub server_time: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ExchangeAccountResponse {
    pub balances: Vec<ExchangeBalance>,
}

#[derive(Debug, Serialize)]
pub struct ExchangeBalance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

#[derive(Debug, Serialize)]
pub struct ExchangeOrderResponse {
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub status: String,
    pub executed_qty: String,
    pub fills: Vec<ExchangeFill>,
}

#[derive(Debug, Serialize)]
pub struct ExchangeFill {
    pub price: String,
    pub qty: String,
    pub commission: String,
}

#[derive(Debug, Serialize)]
pub struct ExchangeCancelResponse {
    pub order_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ExchangeRateLimitResponse {
    pub user_id: String,
    pub local_remaining: u64,
    pub local_reset_at_ms: i64,
    pub binance_limit: BinanceLimitInfo,
}

#[derive(Debug, Serialize)]
pub struct BinanceLimitInfo {
    pub rate_limit_type: String,
    pub interval: String,
    pub interval_num: i32,
    pub limit: i64,
    pub count: i64,
}

// ─── Handlers ───────────────────────────────────────────────────

/// GET /api/v1/exchange/ping
///
/// 测试连通性，返回 Binance 服务器时间
/// 认证: JWT
pub async fn exchange_ping(
    Extension(signed_client): Extension<Arc<SignedBinanceClient>>,
) -> Result<(StatusCode, Json<ApiResponse<ExchangePingResponse>>), AppError> {
    let ping_resp = signed_client.ping().await?;

    let response = ExchangePingResponse {
        server_time: ping_resp.server_time,
        status: ping_resp.status,
    };

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

/// GET /api/v1/exchange/account
///
/// 获取账户余额
/// 认证: JWT + HMAC
pub async fn exchange_account(
    user: AuthenticatedUser,
    Extension(signed_client): Extension<Arc<SignedBinanceClient>>,
) -> Result<(StatusCode, Json<ApiResponse<ExchangeAccountResponse>>), AppError> {
    let account = signed_client.get_account(user.user_id).await?;

    let response = ExchangeAccountResponse {
        // account_id removed — not in AccountInfo
        balances: account
            .balances
            .into_iter()
            .map(|b| ExchangeBalance {
                asset: b.asset,
                free: b.free,
                locked: b.locked,
            })
            .collect(),
    };

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

/// POST /api/v1/exchange/order
///
/// 下单
/// 认证: JWT + HMAC
pub async fn exchange_create_order(
    user: AuthenticatedUser,
    Extension(signed_client): Extension<Arc<SignedBinanceClient>>,
    Json(req): Json<CreateExchangeOrderRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ExchangeOrderResponse>>), AppError> {
    // 验证请求参数
    if req.symbol.is_empty() {
        return Err(AppError::BadRequest("symbol is required".into()));
    }
    if req.side.is_empty() {
        return Err(AppError::BadRequest("side is required".into()));
    }
    if req.order_type.is_empty() {
        return Err(AppError::BadRequest("order type is required".into()));
    }

    // 验证 side
    match req.side.to_uppercase().as_str() {
        "BUY" | "SELL" => {}
        _ => {
            return Err(AppError::BadRequest("side must be BUY or SELL".into()));
        }
    }

    // 验证 order_type
    match req.order_type.to_uppercase().as_str() {
        "LIMIT" | "MARKET" => {}
        _ => {
            return Err(AppError::BadRequest(
                "order type must be LIMIT or MARKET".into(),
            ));
        }
    }

    let new_order = NewOrder {
        symbol: req.symbol.to_uppercase(),
        side: req.side.to_uppercase(),
        order_type: req.order_type.to_uppercase(),
        quantity: req.quantity,
        price: req.price,
        quote_order_qty: req.quote_order_qty,
        time_in_force: req.time_in_force,
    };

    let order_resp = signed_client.place_order(new_order, user.user_id).await?;

    let response = ExchangeOrderResponse {
        order_id: order_resp.order_id,
        symbol: order_resp.symbol,
        side: order_resp.side,
        order_type: order_resp.order_type,
        status: order_resp.status,
        executed_qty: order_resp.executed_qty,
        fills: order_resp
            .fills
            .into_iter()
            .map(|f| ExchangeFill {
                price: f.price,
                qty: f.qty,
                commission: f.commission,
            })
            .collect(),
    };

    tracing::info!(
        user_id = %user.user_id,
        order_id = %response.order_id,
        symbol = %response.symbol,
        "Exchange order created"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

/// DELETE /api/v1/exchange/order/{orderId}
///
/// 撤单
/// 认证: JWT + HMAC
pub async fn exchange_cancel_order(
    user: AuthenticatedUser,
    Extension(signed_client): Extension<Arc<SignedBinanceClient>>,
    Path(order_id): Path<String>,
    Json(req): Json<CancelExchangeOrderRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ExchangeCancelResponse>>), AppError> {
    if order_id.is_empty() {
        return Err(AppError::BadRequest("order_id is required".into()));
    }
    if req.symbol.is_empty() {
        return Err(AppError::BadRequest("symbol is required".into()));
    }

    let cancel_resp = signed_client
        .cancel_order(&order_id, &req.symbol.to_uppercase(), user.user_id)
        .await?;

    let response = ExchangeCancelResponse {
        order_id: cancel_resp.order_id,
        status: cancel_resp.status,
    };

    tracing::info!(
        user_id = %user.user_id,
        order_id = %response.order_id,
        "Exchange order cancelled"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

/// GET /api/v1/exchange/rate-limit
///
/// 获取频率限制状态
/// 认证: JWT + HMAC
pub async fn exchange_rate_limit(
    user: AuthenticatedUser,
    Extension(signed_client): Extension<Arc<SignedBinanceClient>>,
) -> Result<(StatusCode, Json<ApiResponse<ExchangeRateLimitResponse>>), AppError> {
    let (local_remaining, local_reset_at_ms) =
        signed_client.get_rate_limit_status(user.user_id).await?;
    let binance_limit = signed_client.get_rate_limit(user.user_id).await?;

    let response = ExchangeRateLimitResponse {
        user_id: user.user_id.to_string(),
        local_remaining,
        local_reset_at_ms,
        binance_limit: BinanceLimitInfo {
            rate_limit_type: binance_limit.rate_limit_type,
            interval: binance_limit.interval,
            interval_num: binance_limit.interval_num,
            limit: binance_limit.limit,
            count: binance_limit.count,
        },
    };

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}
