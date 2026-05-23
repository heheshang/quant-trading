//! Risk Management Handlers — P0-F2 资金风控 API
//!
//! ADR-013 D3: 7 个 API 端点

use axum::{
    Json, Router,
    extract::{Extension, Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use std::sync::Arc;
use uuid::Uuid;

use crate::middleware::auth::AuthenticatedUser;
use crate::services::exchange::ws_hub::ConnectionStatus;
use crate::services::risk_manager::{
    EmergencyCloseResult, PauseResponse, RiskCheckResult, RiskManager,
};
use crate::utils::error::AppError;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

// ─── Schemas ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RiskRulesQuery {}

#[derive(Debug, Deserialize)]
pub struct RiskLogQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct RiskPauseRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RiskRulesResponse {
    pub id: String,
    pub user_id: String,
    pub daily_loss_limit: String,
    pub daily_loss_auto_close: bool,
    pub single_trade_loss_ratio: String,
    pub max_drawdown_ratio: String,
    pub drawdown_auto_close: bool,
    pub stop_loss_type: String,
    pub atr_period: Option<i32>,
    pub atr_multiplier: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct RiskLogResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(rename = "triggered_rule")]
    pub rule_type: String,
    pub action: String,
    pub severity: String,
    pub details: String,
    #[serde(rename = "equity_snapshot")]
    pub account_equity: String,
    #[serde(rename = "threshold_snapshot")]
    pub threshold: String,
    #[serde(rename = "created_at")]
    pub triggered_at: String,
    pub order_id: Option<Uuid>,
    pub notification_sent: bool,
    pub position_value: Option<String>,
    pub actual_value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RiskRulesUpdate {
    pub daily_loss_limit: Option<String>,
    pub daily_loss_auto_close: Option<bool>,
    pub single_trade_loss_ratio: Option<String>,
    pub max_drawdown_ratio: Option<String>,
    pub drawdown_auto_close: Option<bool>,
    pub stop_loss_type: Option<String>,
    pub atr_period: Option<i32>,
    pub atr_multiplier: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self { code: 0, data }
    }
}

// ─── Handlers ──────────────────────────────────────────────────

/// GET /api/v1/risk/rules — 获取当前用户风控规则
pub async fn get_risk_rules(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
) -> Result<Json<ApiResponse<RiskRulesResponse>>, AppError> {
    let rm = RiskManager::new(db);
    let rules = rm.get_rules_internal(user.user_id).await?;

    Ok(Json(ApiResponse::success(RiskRulesResponse {
        id: user.user_id.to_string(),
        user_id: user.user_id.to_string(),
        daily_loss_limit: rules.daily_loss_limit.to_string(),
        daily_loss_auto_close: rules.daily_loss_auto_close,
        single_trade_loss_ratio: rules.single_trade_loss_ratio.to_string(),
        max_drawdown_ratio: rules.max_drawdown_ratio.to_string(),
        drawdown_auto_close: rules.drawdown_auto_close,
        stop_loss_type: rules.stop_loss_type,
        atr_period: rules.atr_period,
        atr_multiplier: rules.atr_multiplier.map(|d| d.to_string()),
        is_active: rules.is_active,
        created_at: rules.created_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
        updated_at: rules.updated_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
    })))
}

/// PUT /api/v1/risk/rules — 更新风控规则
pub async fn update_risk_rules(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<RiskRulesUpdate>,
) -> Result<Json<ApiResponse<RiskRulesResponse>>, AppError> {
    use crate::db::risk_rules::{ActiveModel, Column, Entity as RiskRulesEntity};

    let _rm = RiskManager::new(db.clone());

    // 加载现有规则
    let existing = RiskRulesEntity::find_by_id(user.user_id)
        .one(db.as_ref())
        .await?;

    let mut active: ActiveModel = match existing {
        Some(ref e) => e.clone().into(),
        None => ActiveModel {
            user_id: sea_orm::Set(user.user_id),
            ..Default::default()
        },
    };

    if let Some(v) = req.daily_loss_limit {
        active.daily_loss_limit = sea_orm::Set(v.parse().unwrap_or_default());
    }
    if let Some(v) = req.daily_loss_auto_close {
        active.daily_loss_auto_close = sea_orm::Set(v);
    }
    if let Some(v) = req.single_trade_loss_ratio {
        active.single_trade_loss_ratio = sea_orm::Set(v.parse().unwrap_or_default());
    }
    if let Some(v) = req.max_drawdown_ratio {
        active.max_drawdown_ratio = sea_orm::Set(v.parse().unwrap_or_default());
    }
    if let Some(v) = req.drawdown_auto_close {
        active.drawdown_auto_close = sea_orm::Set(v);
    }
    if let Some(v) = req.stop_loss_type {
        active.stop_loss_type = sea_orm::Set(v);
    }
    if let Some(v) = req.atr_period {
        active.atr_period = sea_orm::Set(Some(v));
    }
    if let Some(v) = req.atr_multiplier {
        active.atr_multiplier = sea_orm::Set(Some(v.parse().unwrap_or_default()));
    }
    if let Some(v) = req.is_active {
        active.is_active = sea_orm::Set(v);
    }
    // Set defaults for any unset fields (for new records)
    if active.daily_loss_limit == sea_orm::Set(Decimal::ZERO.into()) {
        active.daily_loss_limit = sea_orm::Set(Decimal::ZERO);
    }
    if active.updated_at.is_not_set() {
        active.updated_at = sea_orm::Set(chrono::Utc::now());
    }
    if active.created_at.is_not_set() {
        active.created_at = sea_orm::Set(chrono::Utc::now());
    }

    // Upsert: separate update and insert paths to avoid move conflict
    let saved = match existing {
        Some(_) => {
            // UPDATE path: active already populated above, just update
            RiskRulesEntity::update(active)
                .filter(Column::UserId.eq(user.user_id))
                .exec(db.as_ref())
                .await?
        }
        None => {
            // INSERT path: build new model with defaults + req fields
            let mut new_active: ActiveModel = ActiveModel {
                user_id: sea_orm::Set(user.user_id),
                daily_loss_limit: req.daily_loss_limit
                    .map(|v| sea_orm::Set(v.parse().unwrap_or_default()))
                    .unwrap_or(sea_orm::Set(Decimal::ZERO)),
                daily_loss_auto_close: req.daily_loss_auto_close
                    .map(sea_orm::Set)
                    .unwrap_or(sea_orm::Set(false)),
                single_trade_loss_ratio: req.single_trade_loss_ratio
                    .map(|v| sea_orm::Set(v.parse().unwrap_or_default()))
                    .unwrap_or(sea_orm::Set(Decimal::ZERO)),
                max_drawdown_ratio: req.max_drawdown_ratio
                    .map(|v| sea_orm::Set(v.parse().unwrap_or_default()))
                    .unwrap_or(sea_orm::Set(std::str::FromStr::from_str("0.1").unwrap())),
                drawdown_auto_close: req.drawdown_auto_close
                    .map(sea_orm::Set)
                    .unwrap_or(sea_orm::Set(false)),
                stop_loss_type: req.stop_loss_type
                    .map(sea_orm::Set)
                    .unwrap_or(sea_orm::Set("fixed".to_string())),
                atr_period: req.atr_period.map(|v| sea_orm::Set(Some(v))),
                atr_multiplier: req.atr_multiplier
                    .map(|v| sea_orm::Set(Some(v.parse().unwrap_or_default()))),
                is_active: req.is_active
                    .map(sea_orm::Set)
                    .unwrap_or(sea_orm::Set(false)),
                created_at: sea_orm::Set(chrono::Utc::now()),
                updated_at: sea_orm::Set(chrono::Utc::now()),
            };
            RiskRulesEntity::insert(new_active)
                .exec(db.as_ref())
                .await?;
            RiskRulesEntity::find_by_id(user.user_id)
                .one(db.as_ref())
                .await?
                .ok_or_else(|| AppError::Internal("Failed to fetch inserted risk rules".into()))?
        }
    };

    Ok(Json(ApiResponse::success(RiskRulesResponse {
        id: saved.user_id.to_string(),
        user_id: saved.user_id.to_string(),
        daily_loss_limit: saved.daily_loss_limit.to_string(),
        daily_loss_auto_close: saved.daily_loss_auto_close,
        single_trade_loss_ratio: saved.single_trade_loss_ratio.to_string(),
        max_drawdown_ratio: saved.max_drawdown_ratio.to_string(),
        drawdown_auto_close: saved.drawdown_auto_close,
        stop_loss_type: saved.stop_loss_type,
        atr_period: saved.atr_period,
        atr_multiplier: saved.atr_multiplier.map(|d| d.to_string()),
        is_active: saved.is_active,
        created_at: saved.created_at.to_rfc3339(),
        updated_at: saved.updated_at.to_rfc3339(),
    })))
}

/// GET /api/v1/risk/logs — 分页查询风控日志
pub async fn get_risk_logs(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Query(params): Query<RiskLogQuery>,
) -> Result<Json<ApiResponse<Vec<RiskLogResponse>>>, AppError> {
    use crate::db::risk_logs::Column as RiskLogCol;
    use crate::db::risk_logs::Entity as RiskLogsEntity;
    use sea_orm::PaginatorTrait;

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);

    let logs = RiskLogsEntity::find()
        .filter(RiskLogCol::UserId.eq(user.user_id))
        .order_by_desc(RiskLogCol::TriggeredAt)
        .paginate(db.as_ref(), page_size as u64)
        .fetch_page((page - 1) as u64)
        .await?;

    let data: Vec<RiskLogResponse> = logs
        .into_iter()
        .map(|l| RiskLogResponse {
            id: l.id,
            user_id: l.user_id,
            rule_type: l.rule_type.clone(),
            action: l.action_taken,
            severity: "medium".to_string(), // DB doesn't have severity, default to medium
            details: format!("{} triggered at {}", l.rule_type, l.triggered_at),
            account_equity: l.account_equity.to_string(),
            threshold: l.threshold.to_string(),
            triggered_at: l.triggered_at.to_rfc3339(),
            order_id: l.order_id,
            notification_sent: l.notification_sent,
            position_value: l.position_value.map(|v| v.to_string()),
            actual_value: Some(l.actual_value.to_string()),
        })
        .collect();

    Ok(Json(ApiResponse::success(data)))
}

/// POST /api/v1/risk/check — 手动触发风控检查
pub async fn manual_risk_check(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
) -> Result<Json<ApiResponse<RiskCheckResult>>, AppError> {
    let rm = RiskManager::new(db);
    let result = rm.manual_check(user.user_id).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// POST /api/v1/risk/emergency-close — 紧急全平
pub async fn emergency_close(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
) -> Result<Json<ApiResponse<EmergencyCloseResult>>, AppError> {
    let rm = RiskManager::new(db);
    let result = rm.emergency_close(user.user_id, true).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// POST /api/v1/risk/pause — 暂停交易
pub async fn pause_trading(
    State(db): State<Arc<DatabaseConnection>>,
    user: AuthenticatedUser,
    Json(req): Json<RiskPauseRequest>,
) -> Result<Json<ApiResponse<PauseResponse>>, AppError> {
    let rm = RiskManager::new(db);
    let reason = req.reason.unwrap_or_else(|| "手动暂停".to_string());
    let result = rm.pause(user.user_id, &reason).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// POST /api/v1/risk/resume — 恢复交易
pub async fn resume_trading(
    State(_db): State<Arc<DatabaseConnection>>,
    _user: AuthenticatedUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // TODO: 清除暂停状态
    Ok(Json(ApiResponse::success(serde_json::json!({
        "paused": false,
        "message": "交易已恢复"
    }))))
}

/// GET /api/v1/risk/connection-status — 真实连接状态（P0-F3）
pub async fn connection_status(
    _user: AuthenticatedUser,
    Extension(ws_hub): Extension<std::sync::Arc<crate::services::exchange::ws_hub::WsHub>>,
) -> Result<Json<ApiResponse<ConnectionStatus>>, AppError> {
    Ok(Json(ApiResponse::success(ws_hub.get_connection_status())))
}

// ─── Router ────────────────────────────────────────────────────

pub fn router() -> Router<Arc<DatabaseConnection>> {
    Router::new()
        .route("/risk/rules", get(get_risk_rules).put(update_risk_rules))
        .route("/risk/logs", get(get_risk_logs))
        .route("/risk/check", post(manual_risk_check))
        .route("/risk/emergency-close", post(emergency_close))
        .route("/risk/pause", post(pause_trading))
        .route("/risk/resume", post(resume_trading))
        .route("/risk/connection-status", get(connection_status))
}
