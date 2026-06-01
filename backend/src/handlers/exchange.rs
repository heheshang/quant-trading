//! Exchange API Handlers — 交易所 API 签名认证
//!
//! ADR-012 D5: 签名端点映射
//! - GET  /api/v1/exchange/ping       — 测试连通性（JWT）
//! - GET  /api/v1/exchange/account     — 账户余额（JWT+HMAC）
//! - POST /api/v1/exchange/order      — 下单（JWT+HMAC）
//! - DELETE /api/v1/exchange/order/{orderId} — 撤单（JWT+HMAC）
//! - GET  /api/v1/exchange/rate-limit — 频率限制状态（JWT+HMAC）
//! - GET  /api/v1/exchange/okx/ping    — OKX 连通性测试
//! - GET  /api/v1/exchange/okx/account  — OKX 账户余额
//! - POST /api/v1/exchange/okx/order    — OKX 下单
//! - DELETE /api/v1/exchange/okx/order/{orderId} — OKX 撤单
//! - GET  /api/v1/exchange/okx/orders/pending — OKX 挂单查询

use axum::{Extension, Json, extract::Path, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::middleware::auth::AuthenticatedUser;
use crate::services::exchange::signed_client::{NewOrder as BinanceNewOrder, SignedBinanceClient};
use crate::services::exchange::SignedOkxClient;
use crate::services::okx_signed_client::NewOrder as OkxNewOrder;
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

// ─── OKX Request Types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateOkxOrderRequest {
    pub symbol: String,      // BTC-USDT (OKX format)
    pub side: String,       // buy/sell
    pub order_type: String,  // limit/market
    pub quantity: Option<String>,
    pub price: Option<String>,
    pub time_in_force: Option<String>, // GTC, IOC, FOK (for limit orders)
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

// ─── OKX Response Types ─────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OkxPendingOrdersResponse {
    pub orders: Vec<OkxPendingOrder>,
}

#[derive(Debug, Serialize)]
pub struct OkxPendingOrder {
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub price: String,
    pub qty: String,
    pub status: String,
}

// ─── Binance → OKX Symbol Conversion ────────────────────────────

/// Convert Binance symbol format to OKX format (BTCUSDT → BTC-USDT).
///
/// Reserved for the OKX signed-client multi-exchange adapter. Not yet wired
/// into a handler — kept here for the next OKX symbol conversion task.
#[allow(dead_code)]
fn binance_to_okx_symbol(binance_symbol: &str) -> String {
    // BTCUSDT -> BTC-USDT, ETHUSDT -> ETH-USDT
    // Strategy: insert '-' before last 4 chars (USDT)
    if binance_symbol.ends_with("USDT") {
        let len = binance_symbol.len();
        format!("{}-{}", &binance_symbol[..len - 4], &binance_symbol[len - 4..])
    } else {
        binance_symbol.to_string()
    }
}

// ─── OKX Handlers ───────────────────────────────────────────────

/// GET /api/v1/exchange/okx/ping
pub async fn exchange_okx_ping(
    Extension(okx_client): Extension<Arc<SignedOkxClient>>,
) -> Result<(StatusCode, Json<ApiResponse<ExchangePingResponse>>), AppError> {
    let ping_resp = okx_client.ping().await?;
    let response = ExchangePingResponse {
        server_time: ping_resp.server_time,
        status: ping_resp.status,
    };
    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

/// GET /api/v1/exchange/okx/account
pub async fn exchange_okx_account(
    user: AuthenticatedUser,
    Extension(okx_client): Extension<Arc<SignedOkxClient>>,
) -> Result<(StatusCode, Json<ApiResponse<ExchangeAccountResponse>>), AppError> {
    let account = okx_client.get_account(user.user_id).await?;
    let response = ExchangeAccountResponse {
        balances: account
            .balances
            .into_iter()
            .map(|b| ExchangeBalance {
                asset: b.asset,
                free: b.available,
                locked: b.frozen,
            })
            .collect(),
    };
    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

/// POST /api/v1/exchange/okx/order
pub async fn exchange_okx_create_order(
    user: AuthenticatedUser,
    Extension(okx_client): Extension<Arc<SignedOkxClient>>,
    Json(req): Json<CreateOkxOrderRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ExchangeOrderResponse>>), AppError> {
    if req.symbol.is_empty() {
        return Err(AppError::BadRequest("symbol is required".into()));
    }
    if req.side.is_empty() {
        return Err(AppError::BadRequest("side is required".into()));
    }
    if req.order_type.is_empty() {
        return Err(AppError::BadRequest("order_type is required".into()));
    }

    let side_lower = req.side.to_lowercase();
    match side_lower.as_str() {
        "buy" | "sell" => {}
        _ => return Err(AppError::BadRequest("side must be BUY or SELL".into())),
    }

    let order_type_lower = req.order_type.to_lowercase();
    match order_type_lower.as_str() {
        "limit" | "market" => {}
        _ => return Err(AppError::BadRequest(
            "order_type must be LIMIT or MARKET".into(),
        )),
    }

    // Build OKX NewOrder (instId format: BTC-USDT)
    let okx_order = OkxNewOrder {
        inst_id: req.symbol.clone(),
        td_mode: "cash".to_string(), //现货 cash 模式
        side: side_lower,
        ord_type: order_type_lower,
        sz: req.quantity.clone().unwrap_or_default(),
        px: req.price.clone(),
    };

    let order_resp = okx_client.place_order(okx_order, user.user_id).await?;

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
                commission: f.fee,
            })
            .collect(),
    };

    tracing::info!(
        user_id = %user.user_id,
        order_id = %response.order_id,
        symbol = %response.symbol,
        "OKX order created"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

/// DELETE /api/v1/exchange/okx/order/{orderId}
pub async fn exchange_okx_cancel_order(
    user: AuthenticatedUser,
    Extension(okx_client): Extension<Arc<SignedOkxClient>>,
    Path((order_id,)): Path<(String,)>,
    Json(req): Json<CancelExchangeOrderRequest>,
) -> Result<(StatusCode, Json<ApiResponse<ExchangeCancelResponse>>), AppError> {
    if order_id.is_empty() {
        return Err(AppError::BadRequest("order_id is required".into()));
    }
    if req.symbol.is_empty() {
        return Err(AppError::BadRequest("symbol is required".into()));
    }

    // symbol comes as BTC-USDT from frontend
    let cancel_resp = okx_client
        .cancel_order(&order_id, &req.symbol, user.user_id)
        .await?;

    let response = ExchangeCancelResponse {
        order_id: cancel_resp.order_id,
        status: cancel_resp.status,
    };

    tracing::info!(
        user_id = %user.user_id,
        order_id = %response.order_id,
        "OKX order cancelled"
    );

    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

/// GET /api/v1/exchange/okx/orders/pending
pub async fn exchange_okx_pending_orders(
    user: AuthenticatedUser,
    Extension(okx_client): Extension<Arc<SignedOkxClient>>,
) -> Result<(StatusCode, Json<ApiResponse<OkxPendingOrdersResponse>>), AppError> {
    let orders = okx_client.get_pending_orders(user.user_id).await?;
    let response = OkxPendingOrdersResponse {
        orders: orders
            .into_iter()
            .map(|o| OkxPendingOrder {
                order_id: o.order_id,
                symbol: o.symbol,
                side: o.side,
                order_type: o.order_type,
                price: o.price,
                qty: o.qty,
                status: o.status,
            })
            .collect(),
    };
    Ok((StatusCode::OK, Json(ApiResponse::success(response))))
}

// ─── Binance Handlers (existing) ───────────────────────────────

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

    let new_order = BinanceNewOrder {
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
