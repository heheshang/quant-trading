//! handlers/trigger_order.rs — Trigger Order HTTP Handlers (条件触发单接口)
//!
//! P1-F3: 条件触发单 - 止损单/止盈单/OCO/TWAP HTTP API

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter, QueryOrder, ActiveModelTrait};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::auth::AuthenticatedUser;
use crate::services::trigger_order::TriggerOrderService;
use crate::utils::error::AppError;

/// ==================== 请求/响应结构 ====================

/// 创建止损单请求
#[derive(Debug, Deserialize)]
pub struct CreateStopLossRequest {
    pub position_id: Uuid,
    pub symbol: String,
    pub trigger_price: f64,
    pub base_price: Option<f64>,
    pub quantity: f64,
}

/// 创建止盈单请求
#[derive(Debug, Deserialize)]
pub struct CreateTakeProfitRequest {
    pub position_id: Uuid,
    pub symbol: String,
    pub trigger_price: f64,
    pub base_price: Option<f64>,
    pub quantity: f64,
}

/// 创建OCO单请求
#[derive(Debug, Deserialize)]
pub struct CreateOcoRequest {
    pub position_id: Uuid,
    pub symbol: String,
    pub stop_loss_price: f64,
    pub take_profit_price: f64,
    pub base_price: Option<f64>,
    pub quantity: f64,
}

/// 创建TWAP单请求
#[derive(Debug, Deserialize)]
pub struct CreateTwapRequest {
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub slice_quantity: f64,
    pub interval_secs: i32,
    pub duration_secs: i32,
}

/// 查询条件单列表参数
#[derive(Debug, Deserialize)]
pub struct ListTriggerOrdersQuery {
    pub status: Option<String>,
    pub symbol: Option<String>,
}

/// 条件单响应
#[derive(Debug, Serialize)]
pub struct TriggerOrderResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub position_id: Option<Uuid>,
    pub symbol: String,
    pub trigger_type: String,
    pub status: String,
    pub trigger_direction: String,
    pub trigger_price: f64,
    pub trigger_price_upper: Option<f64>,
    pub trigger_price_lower: Option<f64>,
    pub base_price: Option<f64>,
    pub side: String,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub avg_fill_price: Option<f64>,
    pub oco_pair_id: Option<Uuid>,
    pub triggered_order_id: Option<Uuid>,
    pub trigger_reason: Option<String>,
    pub triggered_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    // TWAP特定字段
    pub twap_slice_quantity: f64,
    pub twap_interval_secs: i32,
    pub twap_executed_slices: i32,
    pub twap_max_slices: i32,
}

impl From<crate::db::trigger_order::Model> for TriggerOrderResponse {
    fn from(m: crate::db::trigger_order::Model) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            position_id: m.position_id,
            symbol: m.symbol,
            trigger_type: m.trigger_type.to_string(),
            status: m.status.to_string(),
            trigger_direction: m.trigger_direction.to_string(),
            trigger_price: m.trigger_price,
            trigger_price_upper: m.trigger_price_upper,
            trigger_price_lower: m.trigger_price_lower,
            base_price: m.base_price,
            side: m.side.to_string(),
            quantity: m.quantity,
            filled_quantity: m.filled_quantity,
            avg_fill_price: m.avg_fill_price,
            oco_pair_id: m.oco_pair_id,
            triggered_order_id: m.triggered_order_id,
            trigger_reason: m.trigger_reason,
            triggered_at: m.triggered_at.map(|t| t.to_rfc3339()),
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
            twap_slice_quantity: m.twap_slice_quantity,
            twap_interval_secs: m.twap_interval_secs,
            twap_executed_slices: m.twap_executed_slices,
            twap_max_slices: m.twap_max_slices,
        }
    }
}

/// OCO订单对响应
#[derive(Debug, Serialize)]
pub struct OcoPairResponse {
    pub stop_loss: TriggerOrderResponse,
    pub take_profit: TriggerOrderResponse,
}

/// 取消条件单请求
#[derive(Debug, Deserialize)]
pub struct CancelTriggerOrderRequest {
    pub reason: Option<String>,
}

/// ==================== HTTP Handlers ====================

/// 创建止损单
/// POST /api/v1/trigger-orders/stop-loss
pub async fn create_stop_loss(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<CreateStopLossRequest>,
) -> Result<impl IntoResponse, AppError> {
    let service = TriggerOrderService::new(db);
    let order = service
        .create_stop_loss(
            user.user_id,
            req.position_id,
            &req.symbol,
            req.trigger_price,
            req.base_price,
            req.quantity,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(TriggerOrderResponse::from(order))))
}

/// 创建止盈单
/// POST /api/v1/trigger-orders/take-profit
pub async fn create_take_profit(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<CreateTakeProfitRequest>,
) -> Result<impl IntoResponse, AppError> {
    let service = TriggerOrderService::new(db);
    let order = service
        .create_take_profit(
            user.user_id,
            req.position_id,
            &req.symbol,
            req.trigger_price,
            req.base_price,
            req.quantity,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(TriggerOrderResponse::from(order))))
}

/// 创建OCO单
/// POST /api/v1/trigger-orders/oco
pub async fn create_oco(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<CreateOcoRequest>,
) -> Result<impl IntoResponse, AppError> {
    let service = TriggerOrderService::new(db);
    let (stop_loss, take_profit) = service
        .create_oco(
            user.user_id,
            req.position_id,
            &req.symbol,
            req.stop_loss_price,
            req.take_profit_price,
            req.base_price,
            req.quantity,
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(OcoPairResponse {
            stop_loss: TriggerOrderResponse::from(stop_loss),
            take_profit: TriggerOrderResponse::from(take_profit),
        }),
    ))
}

/// 创建TWAP单
/// POST /api/v1/trigger-orders/twap
pub async fn create_twap(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<CreateTwapRequest>,
) -> Result<impl IntoResponse, AppError> {
    let service = TriggerOrderService::new(db);
    let order = service
        .create_twap(
            user.user_id,
            &req.symbol,
            &req.side,
            req.quantity,
            req.slice_quantity,
            req.interval_secs,
            req.duration_secs,
        )
        .await?;

    Ok((StatusCode::CREATED, Json(TriggerOrderResponse::from(order))))
}

/// 查询条件单列表
/// GET /api/v1/trigger-orders
pub async fn list_trigger_orders(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Query(query): Query<ListTriggerOrdersQuery>,
) -> Result<impl IntoResponse, AppError> {
    let service = TriggerOrderService::new(db);
    let orders = service
        .list_trigger_orders(user.user_id, query.status, query.symbol)
        .await?;

    let response: Vec<TriggerOrderResponse> =
        orders.into_iter().map(TriggerOrderResponse::from).collect();

    Ok(Json(response))
}

/// 查询单个条件单
/// GET /api/v1/trigger-orders/:id
pub async fn get_trigger_order(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Path(order_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let service = TriggerOrderService::new(db);
    let order = service.get_trigger_order(user.user_id, order_id).await?;

    Ok(Json(TriggerOrderResponse::from(order)))
}

/// 取消条件单
/// DELETE /api/v1/trigger-orders/:id
pub async fn cancel_trigger_order(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Path(order_id): Path<Uuid>,
    Json(req): Json<CancelTriggerOrderRequest>,
) -> Result<impl IntoResponse, AppError> {
    let service = TriggerOrderService::new(db);

    // 先验证权限
    service.get_trigger_order(user.user_id, order_id).await?;

    // 取消订单
    let reason = req.reason.unwrap_or_else(|| "user_cancelled".to_string());
    service.cancel_trigger_order(order_id, &reason).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// ==================== 路由注册 ====================

/// 注册触发订单路由
pub fn router() -> Router<Arc<DatabaseConnection>> {
    Router::new()
        .route("/api/v1/trigger-orders/stop-loss", post(create_stop_loss))
        .route("/api/v1/trigger-orders/take-profit", post(create_take_profit))
        .route("/api/v1/trigger-orders/oco", post(create_oco))
        .route("/api/v1/trigger-orders/twap", post(create_twap))
        .route("/api/v1/trigger-orders", get(list_trigger_orders))
        .route("/api/v1/trigger-orders/:id", get(get_trigger_order))
        .route("/api/v1/trigger-orders/:id", delete(cancel_trigger_order))
}
