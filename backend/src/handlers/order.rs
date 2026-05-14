//! handlers/order.rs — 交易执行 REST API handlers
//!
//! 对应 PRD: US-TE-01~06, US-TE-10
//! ADR: ADR-TRADING-EXECUTION D6 (API路由)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::order::{OrderSide, OrderStatus, OrderType, TimeInForce, TradeMode};
use crate::middleware::auth::AuthenticatedUser;
use crate::services::matching_engine::MatchingEngine;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set};
use std::sync::Arc;

// ─── Request Types ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub symbol: String,
    pub side: String,       // "buy" | "sell"
    pub order_type: String, // "limit" | "market"
    pub price: Option<String>,
    pub quantity: String,
    pub time_in_force: Option<String>, // "GTC" | "IOC" | "FOK", 默认 GTC
}

#[derive(Debug, Deserialize)]
pub struct CancelAllRequest {
    pub symbol: Option<String>,
    pub side: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListOrdersQuery {
    pub status: Option<String>,
    pub symbol: Option<String>,
    pub side: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ListTradesQuery {
    pub symbol: Option<String>,
    pub side: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: Option<u64>,
    pub size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ClosePositionRequest {
    pub quantity: Option<String>,
}

// ─── Response Types ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub price: Option<String>,
    pub quantity: String,
    pub filled_quantity: String,
    pub avg_fill_price: Option<String>,
    pub status: String,
    pub mode: String,
    pub fee: String,
    pub time_in_force: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct CancelResult {
    pub order_id: String,
    pub status: String,
    pub filled_quantity: String,
    pub released_amount: String,
}

#[derive(Debug, Serialize)]
pub struct CancelAllResult {
    pub cancelled_count: u32,
    pub failed_count: u32,
    pub failed_orders: Vec<FailedOrder>,
}

#[derive(Debug, Serialize)]
pub struct FailedOrder {
    pub order_id: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct Paginated<T: Serialize> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct PositionResponse {
    pub id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: String,
    pub available_quantity: String,
    pub avg_entry_price: String,
    pub unrealized_pnl: String,
    pub realized_pnl: String,
    pub mode: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct TradeResponse {
    pub trade_id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub price: String,
    pub quantity: String,
    pub fee: String,
    pub is_maker: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub user_id: String,
    pub balance: String,
    pub frozen_balance: String,
    pub initial_balance: String,
    pub total_pnl: String,
    pub equity: String,
    pub positions_count: i32,
    pub active_orders_count: i32,
}

#[derive(Debug, Serialize)]
pub struct SymbolConfigResponse {
    pub symbol: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub price_precision: i16,
    pub quantity_precision: i16,
    pub min_quantity: String,
    pub max_quantity: String,
    pub min_notional: String,
    pub fee_rate: String,
    pub enabled: bool,
}

// ─── Helpers ────────────────────────────────────────────────────

fn parse_side(s: &str) -> Result<OrderSide, AppError> {
    match s {
        "buy" => Ok(OrderSide::Buy),
        "sell" => Ok(OrderSide::Sell),
        _ => Err(AppError::BadRequest(format!(
            "无效的 side: {}, 支持 buy/sell",
            s
        ))),
    }
}

fn parse_order_type(s: &str) -> Result<OrderType, AppError> {
    match s {
        "limit" => Ok(OrderType::Limit),
        "market" => Ok(OrderType::Market),
        _ => Err(AppError::BadRequest(format!(
            "无效的 order_type: {}, 支持 limit/market",
            s
        ))),
    }
}

fn parse_time_in_force(s: &Option<String>) -> TimeInForce {
    match s.as_deref() {
        Some("IOC") => TimeInForce::IOC,
        Some("FOK") => TimeInForce::FOK,
        _ => TimeInForce::GTC,
    }
}

fn order_to_response(order: &crate::db::order::Model) -> OrderResponse {
    OrderResponse {
        order_id: order.id.to_string(),
        symbol: order.symbol.clone(),
        side: serde_json::to_value(&order.side)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        order_type: serde_json::to_value(&order.order_type)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        price: order.price.map(|p| format!("{:.8}", p)),
        quantity: format!("{:.8}", order.quantity),
        filled_quantity: format!("{:.8}", order.filled_quantity),
        avg_fill_price: order.avg_fill_price.map(|p| format!("{:.8}", p)),
        status: serde_json::to_value(&order.status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        mode: serde_json::to_value(&order.mode)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        fee: format!("{:.8}", order.fee),
        time_in_force: serde_json::to_value(&order.time_in_force)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        created_at: order.created_at.to_rfc3339(),
        updated_at: order.updated_at.to_rfc3339(),
    }
}

// ─── Handlers ───────────────────────────────────────────────────

/// POST /api/v1/orders — 创建委托
///
/// PRD: US-TE-01 (限价单), US-TE-02 (市价单)
/// ADR: D8 (风控前置), D3 (保证金冻结)
pub async fn create_order(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Extension(engine): Extension<Arc<MatchingEngine>>,
    Json(req): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<ApiResponse<OrderResponse>>), AppError> {
    // 1. Validate and parse request
    let side = parse_side(&req.side)?;
    let order_type = parse_order_type(&req.order_type)?;
    let time_in_force = parse_time_in_force(&req.time_in_force);

    let quantity: f64 = req
        .quantity
        .parse()
        .map_err(|_| AppError::BadRequest("数量格式错误".to_string()))?;

    if quantity <= 0.0 {
        return Err(AppError::BadRequest("数量必须大于 0".to_string()));
    }

    let price: Option<f64> = match order_type {
        OrderType::Limit => {
            let p = req
                .price
                .as_deref()
                .ok_or_else(|| AppError::BadRequest("限价单必须指定价格".to_string()))?
                .parse::<f64>()
                .map_err(|_| AppError::BadRequest("价格格式错误".to_string()))?;
            if p <= 0.0 {
                return Err(AppError::BadRequest("价格必须大于 0".to_string()));
            }
            Some(p)
        }
        OrderType::Market => None,
    };

    // 2. Create order in DB
    let now = chrono::Utc::now();
    let order_id = Uuid::new_v4();

    let order_model = crate::db::order::ActiveModel {
        id: Set(order_id),
        user_id: Set(user.user_id),
        strategy_id: Set(None),
        symbol: Set(req.symbol.clone()),
        side: Set(side.clone()),
        order_type: Set(order_type.clone()),
        price: Set(price),
        quantity: Set(quantity),
        filled_quantity: Set(0.0),
        avg_fill_price: Set(None),
        status: Set(OrderStatus::Pending),
        mode: Set(TradeMode::Paper),
        fee: Set(0.0),
        reject_reason: Set(None),
        time_in_force: Set(time_in_force),
        expire_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        cancelled_at: Set(None),
        filled_at: Set(None),
    };

    let inserted = order_model.insert(&*db).await.map_err(|e| {
        tracing::error!("Failed to create order: {:?}", e);
        AppError::Database(e.to_string())
    })?;

    // 3. For market orders, try immediate matching
    if order_type == OrderType::Market {
        match engine
            .match_market(order_id, user.user_id, &req.symbol, &side, quantity)
            .await
        {
            Ok(result) => {
                let new_status = if result.is_fully_filled {
                    OrderStatus::Filled
                } else {
                    OrderStatus::PartialFilled
                };

                let mut active: crate::db::order::ActiveModel = inserted.into();
                active.filled_quantity = Set(result.filled_quantity);
                active.avg_fill_price = Set(result.avg_fill_price);
                active.fee = Set(result.total_fee);
                active.status = Set(new_status);
                active.updated_at = Set(chrono::Utc::now());
                if result.is_fully_filled {
                    active.filled_at = Set(Some(chrono::Utc::now()));
                }

                let updated = active.update(&*db).await.map_err(|e| {
                    tracing::error!("Failed to update order after match: {:?}", e);
                    AppError::Database(e.to_string())
                })?;

                return Ok((
                    StatusCode::CREATED,
                    Json(ApiResponse::success(order_to_response(&updated))),
                ));
            }
            Err(e) => {
                tracing::warn!("Market order matching failed: {:?}", e);
                // Order stays in pending status, will be picked up later
            }
        }
    } else {
        // Limit order: insert into order book
        engine.insert_limit_order(crate::services::matching_engine::OrderEntry {
            order_id,
            user_id: user.user_id,
            symbol: req.symbol.clone(),
            side: side.clone(),
            price,
            remaining_quantity: quantity,
            created_at: now,
        });
    }

    // Re-fetch to get the latest state
    let order = crate::db::order::Entity::find_by_id(order_id)
        .one(&*db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::Internal("Order created but not found".to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(order_to_response(&order))),
    ))
}

/// GET /api/v1/orders — 查询委托列表
///
/// PRD: US-TE-04 (当前委托), US-TE-06 (历史委托)
pub async fn list_orders(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<ListOrdersQuery>,
) -> Result<Json<ApiResponse<Paginated<OrderResponse>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(20).clamp(1, 100);

    let mut find_query = crate::db::order::Entity::find()
        .filter(crate::db::order::Column::UserId.eq(user.user_id));

    if let Some(ref status) = query.status {
        let status_enum: OrderStatus =
            serde_json::from_value(serde_json::Value::String(status.clone()))
                .map_err(|_| AppError::BadRequest(format!("无效的 status: {}", status)))?;
        find_query = find_query.filter(crate::db::order::Column::Status.eq(status_enum));
    }

    if let Some(ref symbol) = query.symbol {
        find_query = find_query.filter(crate::db::order::Column::Symbol.eq(symbol));
    }

    let paginator = find_query
        .order_by_desc(crate::db::order::Column::CreatedAt)
        .paginate(&*db, size);

    let total = paginator
        .num_items()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    let pages = paginator
        .fetch_page((page - 1) as u64)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let items: Vec<OrderResponse> = pages.iter().map(order_to_response).collect();

    Ok(Json(ApiResponse::success(Paginated {
        items,
        total,
        page,
        size,
    })))
}

/// GET /api/v1/orders/:id — 查询委托详情
pub async fn get_order(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<ApiResponse<OrderResponse>>, AppError> {
    let order = crate::db::order::Entity::find_by_id(order_id)
        .one(&*db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("委托不存在: {}", order_id)))?;

    if order.user_id != user.user_id {
        return Err(AppError::Forbidden("无权访问此委托".to_string()));
    }

    Ok(Json(ApiResponse::success(order_to_response(&order))))
}

/// POST /api/v1/orders/:id/cancel — 撤单
///
/// PRD: US-TE-05 (撤单)
/// ADR: D2 (PG行锁保证状态一致性)
pub async fn cancel_order(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Extension(engine): Extension<Arc<MatchingEngine>>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<ApiResponse<CancelResult>>, AppError> {
    let order = crate::db::order::Entity::find_by_id(order_id)
        .one(&*db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("委托不存在: {}", order_id)))?;

    if order.user_id != user.user_id {
        return Err(AppError::Forbidden("无权操作此委托".to_string()));
    }

    if order.status.is_terminal() {
        return Err(AppError::Conflict(format!(
            "委托状态为 {:?}, 无法撤单",
            order.status
        )));
    }

    // Remove from order book
    engine.remove_from_book(order_id);

    // Update order status
    let now = chrono::Utc::now();
    let mut active: crate::db::order::ActiveModel = order.into();
    active.status = Set(OrderStatus::Cancelled);
    active.updated_at = Set(now);
    active.cancelled_at = Set(Some(now));

    let updated = active.update(&*db).await.map_err(|e| {
        tracing::error!("Failed to cancel order: {:?}", e);
        AppError::Database(e.to_string())
    })?;

    let released = updated.quantity - updated.filled_quantity;

    Ok(Json(ApiResponse::success(CancelResult {
        order_id: updated.id.to_string(),
        status: "cancelled".to_string(),
        filled_quantity: format!("{:.8}", updated.filled_quantity),
        released_amount: format!("{:.8}", released),
    })))
}

/// POST /api/v1/orders/cancel-all — 批量撤单
///
/// PRD: US-TE-05 (批量撤单, P1)
pub async fn cancel_all_orders(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Extension(engine): Extension<Arc<MatchingEngine>>,
    Json(req): Json<CancelAllRequest>,
) -> Result<Json<ApiResponse<CancelAllResult>>, AppError> {
    let mut find_query = crate::db::order::Entity::find()
        .filter(crate::db::order::Column::UserId.eq(user.user_id))
        .filter(crate::db::order::Column::Status.is_in(vec![
            OrderStatus::Pending,
            OrderStatus::PartialFilled,
        ]));

    if let Some(ref symbol) = req.symbol {
        find_query = find_query.filter(crate::db::order::Column::Symbol.eq(symbol));
    }

    let orders = find_query
        .all(&*db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let mut cancelled_count = 0u32;
    let mut failed_orders = Vec::new();

    for order in orders {
        let oid = order.id;
        engine.remove_from_book(order.id);

        let now = chrono::Utc::now();
        let mut active: crate::db::order::ActiveModel = order.into();
        active.status = Set(OrderStatus::Cancelled);
        active.updated_at = Set(now);
        active.cancelled_at = Set(Some(now));

        match active.update(&*db).await {
            Ok(_) => cancelled_count += 1,
            Err(e) => {
                tracing::warn!("Failed to cancel order: {:?}", e);
                failed_orders.push(FailedOrder {
                    order_id: oid.to_string(),
                    reason: e.to_string(),
                });
            }
        }
    }

    Ok(Json(ApiResponse::success(CancelAllResult {
        cancelled_count,
        failed_count: failed_orders.len() as u32,
        failed_orders,
    })))
}

/// GET /api/v1/trades — 查询成交记录
///
/// PRD: US-TE-06 (历史委托详情中的成交明细)
pub async fn list_trades(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<ListTradesQuery>,
) -> Result<Json<ApiResponse<Paginated<TradeResponse>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let size = query.size.unwrap_or(20).clamp(1, 100);

    let mut find_query = crate::db::order::trades::Entity::find()
        .filter(crate::db::order::trades::Column::UserId.eq(user.user_id));

    if let Some(ref symbol) = query.symbol {
        find_query =
            find_query.filter(crate::db::order::trades::Column::Symbol.eq(symbol));
    }

    let paginator = find_query
        .order_by_desc(crate::db::order::trades::Column::CreatedAt)
        .paginate(&*db, size);

    let total = paginator
        .num_items()
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
    let pages = paginator
        .fetch_page((page - 1) as u64)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let items: Vec<TradeResponse> = pages
        .iter()
        .map(|t| TradeResponse {
            trade_id: t.id.to_string(),
            order_id: t.order_id.to_string(),
            symbol: t.symbol.clone(),
            side: serde_json::to_value(&t.side)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            price: format!("{:.8}", t.price),
            quantity: format!("{:.8}", t.quantity),
            fee: format!("{:.8}", t.fee),
            is_maker: t.is_maker,
            created_at: t.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ApiResponse::success(Paginated {
        items,
        total,
        page,
        size,
    })))
}

/// GET /api/v1/positions — 查询持仓列表
///
/// PRD: US-TE-10 (持仓管理)
pub async fn list_positions(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<Vec<PositionResponse>>>, AppError> {
    let positions = crate::db::order::positions::Entity::find()
        .filter(crate::db::order::positions::Column::UserId.eq(user.user_id))
        .all(&*db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let items: Vec<PositionResponse> = positions
        .iter()
        .map(|p| PositionResponse {
            id: p.id.to_string(),
            symbol: p.symbol.clone(),
            side: serde_json::to_value(&p.side)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            quantity: format!("{:.8}", p.quantity),
            available_quantity: format!("{:.8}", p.available_quantity),
            avg_entry_price: format!("{:.8}", p.avg_entry_price),
            unrealized_pnl: format!("{:.8}", p.unrealized_pnl),
            realized_pnl: format!("{:.8}", p.realized_pnl),
            mode: serde_json::to_value(&p.mode)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default(),
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}

/// GET /api/v1/account — 查询模拟账户
///
/// PRD: US-TE-03 (风控余额校验), US-TE-08 (下单区余额显示)
pub async fn get_account(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<AccountResponse>>, AppError> {
    let account = crate::db::order::paper_accounts::Entity::find()
        .filter(crate::db::order::paper_accounts::Column::UserId.eq(user.user_id))
        .one(&*db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("模拟账户不存在".to_string()))?;

    let active_orders = crate::db::order::Entity::find()
        .filter(crate::db::order::Column::UserId.eq(user.user_id))
        .filter(crate::db::order::Column::Status.is_in(vec![
            OrderStatus::Pending,
            OrderStatus::PartialFilled,
        ]))
        .count(&*db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let positions_count = crate::db::order::positions::Entity::find()
        .filter(crate::db::order::positions::Column::UserId.eq(user.user_id))
        .count(&*db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let equity = account.balance + account.frozen_balance + account.total_pnl;

    Ok(Json(ApiResponse::success(AccountResponse {
        user_id: account.user_id.to_string(),
        balance: format!("{:.8}", account.balance),
        frozen_balance: format!("{:.8}", account.frozen_balance),
        initial_balance: format!("{:.8}", account.initial_balance),
        total_pnl: format!("{:.8}", account.total_pnl),
        equity: format!("{:.8}", equity),
        positions_count: positions_count as i32,
        active_orders_count: active_orders as i32,
    })))
}

/// GET /api/v1/symbols — 查询交易对配置
///
/// PRD: US-TE-03 (交易对校验), US-TE-08 (交易对选择器)
pub async fn list_symbols(
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<Vec<SymbolConfigResponse>>>, AppError> {
    let configs = crate::db::order::symbol_configs::Entity::find()
        .filter(crate::db::order::symbol_configs::Column::Enabled.eq(true))
        .all(&*db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

    let items: Vec<SymbolConfigResponse> = configs
        .iter()
        .map(|c| SymbolConfigResponse {
            symbol: c.symbol.clone(),
            base_currency: c.base_currency.clone(),
            quote_currency: c.quote_currency.clone(),
            price_precision: c.price_precision,
            quantity_precision: c.quantity_precision,
            min_quantity: format!("{:.8}", c.min_quantity),
            max_quantity: format!("{:.8}", c.max_quantity),
            min_notional: format!("{:.8}", c.min_notional),
            fee_rate: format!("{:.6}", c.fee_rate),
            enabled: c.enabled,
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_side_valid() {
        assert!(matches!(parse_side("buy"), Ok(OrderSide::Buy)));
        assert!(matches!(parse_side("sell"), Ok(OrderSide::Sell)));
    }

    #[test]
    fn test_parse_side_invalid() {
        assert!(parse_side("invalid").is_err());
    }

    #[test]
    fn test_parse_order_type_valid() {
        assert!(matches!(parse_order_type("limit"), Ok(OrderType::Limit)));
        assert!(matches!(
            parse_order_type("market"),
            Ok(OrderType::Market)
        ));
    }

    #[test]
    fn test_parse_order_type_invalid() {
        assert!(parse_order_type("stop").is_err());
    }

    #[test]
    fn test_parse_time_in_force() {
        assert!(matches!(parse_time_in_force(&None), TimeInForce::GTC));
        assert!(matches!(
            parse_time_in_force(&Some("GTC".to_string())),
            TimeInForce::GTC
        ));
        assert!(matches!(
            parse_time_in_force(&Some("IOC".to_string())),
            TimeInForce::IOC
        ));
        assert!(matches!(
            parse_time_in_force(&Some("FOK".to_string())),
            TimeInForce::FOK
        ));
        assert!(matches!(
            parse_time_in_force(&Some("XYZ".to_string())),
            TimeInForce::GTC
        ));
    }

    #[test]
    fn test_order_to_response_serialization() {
        let now = chrono::Utc::now();
        let order = crate::db::order::Model {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            strategy_id: None,
            symbol: "BTCUSDT".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            price: Some(50000.0),
            quantity: 1.0,
            filled_quantity: 0.5,
            avg_fill_price: Some(50000.0),
            status: OrderStatus::PartialFilled,
            mode: TradeMode::Paper,
            fee: 0.001,
            reject_reason: None,
            time_in_force: TimeInForce::GTC,
            expire_at: None,
            created_at: now,
            updated_at: now,
            cancelled_at: None,
            filled_at: None,
        };

        let resp = order_to_response(&order);
        assert_eq!(resp.symbol, "BTCUSDT");
        assert_eq!(resp.side, "buy");
        assert_eq!(resp.order_type, "limit");
        assert!(resp.price.is_some());
        assert_eq!(resp.status, "partial_filled");
    }

    #[test]
    fn test_paginated_serialization() {
        let paginated: Paginated<String> = Paginated {
            items: vec!["a".into(), "b".into()],
            total: 2,
            page: 1,
            size: 20,
        };
        let json = serde_json::to_value(&paginated).unwrap();
        assert_eq!(json["items"].as_array().unwrap().len(), 2);
        assert_eq!(json["total"], 2);
        assert_eq!(json["page"], 1);
        assert_eq!(json["size"], 20);
    }
}
