//! handlers/position_alert.rs — P1-F2 实盘止盈止损 API
//!
//! PRD: P1-F2 实盘止盈止损
//! 端点：
//!   POST /api/v1/alerts           — 创建止盈/止损
//!   GET  /api/v1/alerts           — 列表（可按持仓筛选）
//!   GET  /api/v1/alerts/:id      — 详情
//!   PUT  /api/v1/alerts/:id      — 修改（价格/距离）
//!   DELETE /api/v1/alerts/:id    — 取消
//!   POST /api/v1/alerts/:id/trigger — 手动触发（测试用）
//!   POST /api/v1/alerts/batch-check — 批量检查（行情心跳调用）

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::db::position_alerts::{AlertStatus, AlertType, TriggerMode};
use crate::middleware::auth::AuthenticatedUser;
use crate::services::position_alert_service::{
    AlertResponse, CreateAlertRequest, PositionAlertService, UpdateAlertRequest,
};
use crate::utils::error::AppError;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

// ─── Schemas ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateAlertRequestSchema {
    pub position_id: String,               // UUID string
    pub alert_type: String,                // "take_profit" | "stop_loss" | "trailing_stop"
    pub trigger_price: String,             // f64 as string
    pub trigger_mode: Option<String>,      // "market" | "limit", default "market"
    pub limit_price: Option<String>,       // optional, for limit trigger
    pub trailing_distance: Option<String>, // e.g. "0.5" for 0.5%
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAlertRequestSchema {
    pub trigger_price: Option<String>,
    pub limit_price: Option<String>,
    pub trigger_mode: Option<String>,
    pub trailing_distance: Option<String>,
    pub status: Option<String>, // "active" | "paused" | "cancelled"
}

#[derive(Debug, Deserialize)]
pub struct ListAlertsQuery {
    pub symbol: Option<String>,
    pub position_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AlertCreatedResponse {
    pub alert_id: String,
    pub position_id: String,
    pub alert_type: String,
    pub trigger_price: String,
    pub trigger_mode: String,
    pub status: String,
    pub created_at: String,
}

// ─── Helpers ───────────────────────────────────────────────────

fn parse_alert_type(s: &str) -> Result<AlertType, AppError> {
    match s {
        "take_profit" => Ok(AlertType::TakeProfit),
        "stop_loss" => Ok(AlertType::StopLoss),
        "trailing_stop" => Ok(AlertType::TrailingStop),
        _ => Err(AppError::BadRequest(format!(
            "Invalid alert_type: {}. Expected: take_profit | stop_loss | trailing_stop",
            s
        ))),
    }
}

fn parse_trigger_mode(s: &Option<String>) -> TriggerMode {
    match s.as_deref() {
        Some("limit") => TriggerMode::Limit,
        _ => TriggerMode::Market,
    }
}

fn parse_alert_status(s: &str) -> Result<AlertStatus, AppError> {
    match s {
        "active" => Ok(AlertStatus::Active),
        "paused" => Ok(AlertStatus::Paused),
        "cancelled" => Ok(AlertStatus::Cancelled),
        _ => Err(AppError::BadRequest(format!(
            "Invalid status: {}. Expected: active | paused | cancelled",
            s
        ))),
    }
}

// ─── Handlers ──────────────────────────────────────────────────

/// POST /api/v1/alerts — 创建止盈/止损警戒
///
/// PRD: P1-F2 Scenario "开仓时附加止盈止损"
pub async fn create_alert(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<CreateAlertRequestSchema>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let service = PositionAlertService::new(db.clone());

    let position_id = req
        .position_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid position_id format".to_string()))?;

    let trigger_price = req
        .trigger_price
        .parse::<f64>()
        .map_err(|_| AppError::BadRequest("Invalid trigger_price format".to_string()))?;

    if trigger_price <= 0.0 {
        return Err(AppError::BadRequest(
            "trigger_price must be positive".to_string(),
        ));
    }

    let limit_price = match &req.limit_price {
        Some(p) => Some(
            p.parse::<f64>()
                .map_err(|_| AppError::BadRequest("Invalid limit_price format".to_string()))?,
        ),
        None => None,
    };

    let trailing_distance = match &req.trailing_distance {
        Some(d) => {
            let v = d.parse::<f64>().map_err(|_| {
                AppError::BadRequest("Invalid trailing_distance format".to_string())
            })?;
            if v <= 0.0 || v >= 100.0 {
                return Err(AppError::BadRequest(
                    "trailing_distance must be between 0 and 100".to_string(),
                ));
            }
            Some(v / 100.0) // 转换为小数
        }
        None => None,
    };

    let create_req = CreateAlertRequest {
        position_id,
        symbol: "BTCUSDT".to_string(), // 从持仓查询获得
        alert_type: parse_alert_type(&req.alert_type)?,
        trigger_price,
        trigger_mode: parse_trigger_mode(&req.trigger_mode),
        limit_price,
        trailing_distance,
        note: req.note,
    };

    // 获取持仓以确定 symbol
    let position = crate::db::order::positions::Entity::find()
        .filter(crate::db::order::positions::Column::UserId.eq(user.user_id))
        .filter(crate::db::order::positions::Column::Id.eq(position_id))
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Position {} not found", position_id)))?;

    let mut create_req = create_req;
    create_req.symbol = position.symbol.clone();

    let alert = service.create_alert(user.user_id, create_req).await?;

    let resp = AlertCreatedResponse {
        alert_id: alert.id.to_string(),
        position_id: alert.position_id.to_string(),
        alert_type: serde_json::to_value(&alert.alert_type)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        trigger_price: format!("{:.8}", alert.trigger_price),
        trigger_mode: serde_json::to_value(&alert.trigger_mode)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        status: serde_json::to_value(&alert.status)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        created_at: alert.created_at.to_rfc3339(),
    };

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(resp).unwrap()),
    ))
}

/// GET /api/v1/alerts — 列表（可按 symbol / position_id 筛选）
pub async fn list_alerts(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<ListAlertsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    

    let service = PositionAlertService::new(db);

    let alerts = if let Some(position_id) = &params.position_id {
        let pid = position_id
            .parse::<Uuid>()
            .map_err(|_| AppError::BadRequest("Invalid position_id".to_string()))?;
        service.list_alerts_by_position(user.user_id, pid).await?
    } else {
        service
            .list_active_alerts(user.user_id, params.symbol.clone())
            .await?
    };

    // 如果指定了 status 过滤
    let alerts: Vec<_> = if let Some(status_str) = &params.status {
        let target_status = parse_alert_status(status_str)?;
        alerts
            .into_iter()
            .filter(|a| a.status == target_status)
            .collect()
    } else {
        alerts
    };

    let items: Vec<AlertResponse> = alerts.iter().map(AlertResponse::from_model).collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": items
    })))
}

/// GET /api/v1/alerts/:id — 详情
pub async fn get_alert(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    use crate::db::position_alerts::Column as AlertCol;
    use crate::db::position_alerts::Entity as AlertEntity;

    let alert = AlertEntity::find()
        .filter(AlertCol::UserId.eq(user.user_id))
        .filter(AlertCol::Id.eq(alert_id))
        .one(db.as_ref())
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Alert {} not found", alert_id)))?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": AlertResponse::from_model(&alert)
    })))
}

/// PUT /api/v1/alerts/:id — 修改止盈止损
///
/// PRD: P1-F2 Scenario "手动修改止盈止损"
pub async fn update_alert(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(alert_id): Path<Uuid>,
    Json(req): Json<UpdateAlertRequestSchema>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = PositionAlertService::new(db);

    let trigger_price = req
        .trigger_price
        .as_ref()
        .map(|p| p.parse::<f64>())
        .transpose()
        .map_err(|_| AppError::BadRequest("Invalid trigger_price".to_string()))?;

    let limit_price = req
        .limit_price
        .as_ref()
        .map(|p| p.parse::<f64>())
        .transpose()
        .map_err(|_| AppError::BadRequest("Invalid limit_price".to_string()))?;

    let trigger_mode = req
        .trigger_mode
        .as_ref()
        .map(|m| match m.as_str() {
            "limit" => Ok(TriggerMode::Limit),
            "market" => Ok(TriggerMode::Market),
            _ => Err(AppError::BadRequest(format!("Invalid trigger_mode: {}", m))),
        })
        .transpose()?;

    let trailing_distance = req
        .trailing_distance
        .as_ref()
        .map(|d| {
            let v: f64 = d
                .parse()
                .map_err(|_| AppError::BadRequest("Invalid trailing_distance".to_string()))?;
            if v <= 0.0 || v >= 100.0 {
                return Err(AppError::BadRequest(
                    "trailing_distance must be between 0 and 100".to_string(),
                ));
            }
            Ok(v / 100.0)
        })
        .transpose()?;

    let status = req.status.as_deref().map(parse_alert_status).transpose()?;

    let update_req = UpdateAlertRequest {
        trigger_price,
        limit_price,
        trigger_mode,
        trailing_distance,
        status,
    };

    let updated = service
        .update_alert(user.user_id, alert_id, update_req)
        .await?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": AlertResponse::from_model(&updated)
    })))
}

/// DELETE /api/v1/alerts/:id — 取消止盈止损
pub async fn cancel_alert(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let service = PositionAlertService::new(db);
    service.cancel_alert(user.user_id, alert_id).await?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "message": "Alert cancelled successfully" }
    })))
}

// ─── Router ────────────────────────────────────────────────────

pub fn router() -> Router<Arc<DatabaseConnection>> {
    Router::new()
        .route("/", axum::routing::post(create_alert))
        .route("/", axum::routing::get(list_alerts))
        .route("/{id}", axum::routing::get(get_alert))
        .route("/{id}", axum::routing::put(update_alert))
        .route("/{id}", axum::routing::delete(cancel_alert))
}
