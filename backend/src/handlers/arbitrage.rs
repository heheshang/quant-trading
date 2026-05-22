//! 套利模块 HTTP API 处理器
//!
//! Routes:
//! POST/GET/PUT/DELETE /api/v1/arbitrage/pairs
//! GET /api/v1/arbitrage/spread/{pair_id}
//! GET /api/v1/arbitrage/positions
//! GET /api/v1/arbitrage/signals

use crate::db::DbPool;
use crate::db::arbitrage_entities::pair::{
    ActiveModel, Column as PairColumn, Entity as PairEntity,
};
use crate::db::arbitrage_entities::position::{Column as PositionColumn, Entity as PositionEntity};
use crate::db::arbitrage_entities::signal::{Column as SignalColumn, Entity as SignalEntity};
use crate::services::arbitrage::*;
use crate::utils::error::AppError;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use std::sync::{Arc, OnceLock};

// 模块级单例（OnceLock）
static SPREAD_CALCULATOR: OnceLock<Arc<SpreadCalculator>> = OnceLock::new();
static SIGNAL_GENERATOR: OnceLock<Arc<SignalGenerator>> = OnceLock::new();

fn get_spread_calculator() -> Arc<SpreadCalculator> {
    SPREAD_CALCULATOR
        .get_or_init(|| Arc::new(SpreadCalculator::new(20)))
        .clone()
}

fn get_signal_generator() -> Arc<SignalGenerator> {
    SIGNAL_GENERATOR
        .get_or_init(|| Arc::new(SignalGenerator::new(SignalGeneratorConfig::default())))
        .clone()
}

// ============================================================
// 创建套利对
// ============================================================
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePairRequest {
    pub pair_type: String,
    pub symbol_a: String,
    pub symbol_b: String,
    pub exchange: Option<String>,
    pub spread_entry_threshold: Decimal,
    pub spread_exit_threshold: Decimal,
    pub max_position_size: Option<Decimal>,
    pub calculation_mode: Option<String>,
    pub correlation_threshold: Option<Decimal>,
    pub z_score_entry: Option<Decimal>,
    pub z_score_exit: Option<Decimal>,
}

pub async fn create_pair(
    State(db): State<DbPool>,
    Json(req): Json<CreatePairRequest>,
) -> Result<impl IntoResponse, AppError> {
    let active = ActiveModel {
        pair_type: Set(req.pair_type),
        symbol_a: Set(req.symbol_a),
        symbol_b: Set(req.symbol_b),
        exchange: Set(req.exchange.unwrap_or_else(|| "binance".to_string())),
        status: Set("active".to_string()),
        spread_entry_threshold: Set(req.spread_entry_threshold),
        spread_exit_threshold: Set(req.spread_exit_threshold),
        max_position_size: Set(req.max_position_size.unwrap_or(Decimal::from(10000))),
        calculation_mode: Set(req
            .calculation_mode
            .unwrap_or_else(|| "percentage".to_string())),
        correlation_threshold: Set(req.correlation_threshold),
        z_score_entry: Set(req.z_score_entry),
        z_score_exit: Set(req.z_score_exit),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    let pair = PairEntity::insert(active).exec(&*db).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": pair.last_insert_id })),
    ))
}

// ============================================================
// 列出所有活跃套利对
// ============================================================
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairListItem {
    pub id: i32,
    pub pair_type: String,
    pub symbol_a: String,
    pub symbol_b: String,
    pub exchange: String,
    pub status: String,
    pub spread_entry_threshold: Decimal,
    pub spread_exit_threshold: Decimal,
    pub max_position_size: Decimal,
    pub calculation_mode: String,
    pub correlation_threshold: Option<Decimal>,
    pub z_score_entry: Option<Decimal>,
    pub z_score_exit: Option<Decimal>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list_pairs(State(db): State<DbPool>) -> Result<Json<Vec<PairListItem>>, AppError> {
    let pairs = PairEntity::find()
        .filter(PairColumn::Status.eq("active"))
        .all(&*db)
        .await?;

    let items: Vec<PairListItem> = pairs
        .into_iter()
        .map(|p| PairListItem {
            id: p.id,
            pair_type: p.pair_type,
            symbol_a: p.symbol_a,
            symbol_b: p.symbol_b,
            exchange: p.exchange,
            status: p.status,
            spread_entry_threshold: p.spread_entry_threshold,
            spread_exit_threshold: p.spread_exit_threshold,
            max_position_size: p.max_position_size,
            calculation_mode: p.calculation_mode,
            correlation_threshold: p.correlation_threshold,
            z_score_entry: p.z_score_entry,
            z_score_exit: p.z_score_exit,
            created_at: p.created_at.to_rfc3339(),
            updated_at: p.updated_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(items))
}

// ============================================================
// 获取单个套利对
// ============================================================
pub async fn get_pair(
    State(db): State<DbPool>,
    Path(pair_id): Path<i32>,
) -> Result<Json<PairListItem>, AppError> {
    let pair = PairEntity::find_by_id(pair_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Pair {} not found", pair_id)))?;

    Ok(Json(PairListItem {
        id: pair.id,
        pair_type: pair.pair_type,
        symbol_a: pair.symbol_a,
        symbol_b: pair.symbol_b,
        exchange: pair.exchange,
        status: pair.status,
        spread_entry_threshold: pair.spread_entry_threshold,
        spread_exit_threshold: pair.spread_exit_threshold,
        max_position_size: pair.max_position_size,
        calculation_mode: pair.calculation_mode,
        correlation_threshold: pair.correlation_threshold,
        z_score_entry: pair.z_score_entry,
        z_score_exit: pair.z_score_exit,
        created_at: pair.created_at.to_rfc3339(),
        updated_at: pair.updated_at.to_rfc3339(),
    }))
}

// ============================================================
// 更新套利对
// ============================================================
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePairRequest {
    pub pair_type: Option<String>,
    pub symbol_a: Option<String>,
    pub symbol_b: Option<String>,
    pub exchange: Option<String>,
    pub status: Option<String>,
    pub spread_entry_threshold: Option<Decimal>,
    pub spread_exit_threshold: Option<Decimal>,
    pub max_position_size: Option<Decimal>,
    pub calculation_mode: Option<String>,
    pub correlation_threshold: Option<Decimal>,
    pub z_score_entry: Option<Decimal>,
    pub z_score_exit: Option<Decimal>,
}

pub async fn update_pair(
    State(db): State<DbPool>,
    Path(pair_id): Path<i32>,
    Json(req): Json<UpdatePairRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pair = PairEntity::find_by_id(pair_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Pair {} not found", pair_id)))?;

    let mut active: ActiveModel = pair.into();

    if let Some(v) = req.pair_type {
        active.pair_type = Set(v);
    }
    if let Some(v) = req.symbol_a {
        active.symbol_a = Set(v);
    }
    if let Some(v) = req.symbol_b {
        active.symbol_b = Set(v);
    }
    if let Some(v) = req.exchange {
        active.exchange = Set(v);
    }
    if let Some(v) = req.status {
        active.status = Set(v);
    }
    if let Some(v) = req.spread_entry_threshold {
        active.spread_entry_threshold = Set(v);
    }
    if let Some(v) = req.spread_exit_threshold {
        active.spread_exit_threshold = Set(v);
    }
    if let Some(v) = req.max_position_size {
        active.max_position_size = Set(v);
    }
    if let Some(v) = req.calculation_mode {
        active.calculation_mode = Set(v);
    }
    if let Some(v) = req.correlation_threshold {
        active.correlation_threshold = Set(Some(v));
    }
    if let Some(v) = req.z_score_entry {
        active.z_score_entry = Set(Some(v));
    }
    if let Some(v) = req.z_score_exit {
        active.z_score_exit = Set(Some(v));
    }
    active.updated_at = Set(chrono::Utc::now());

    active.update(&*db).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================
// 删除套利对（软删除）
// ============================================================
pub async fn delete_pair(
    State(db): State<DbPool>,
    Path(pair_id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pair = PairEntity::find_by_id(pair_id)
        .one(&*db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Pair {} not found", pair_id)))?;

    let mut active: ActiveModel = pair.into();
    active.status = Set("inactive".to_string());
    active.updated_at = Set(chrono::Utc::now());
    active.update(&*db).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================
// 计算当前价差
// ============================================================
#[derive(Debug, serde::Deserialize)]
pub struct SpreadQuery {
    pub price_a: Option<Decimal>,
    pub price_b: Option<Decimal>,
}

pub async fn get_spread(
    State(_db): State<DbPool>,
    Path(pair_id): Path<i32>,
    Query(query): Query<SpreadQuery>,
) -> Result<Json<SpreadData>, AppError> {
    if query.price_a.is_none() || query.price_b.is_none() {
        return Ok(Json(SpreadData {
            pair_id,
            spread: Decimal::ZERO,
            spread_pct: Decimal::ZERO,
            z_score: None,
            historical_mean: None,
            historical_std: None,
            signal: SignalDirection::Neutral,
            timestamp: chrono::Utc::now(),
        }));
    }

    let price_a = query.price_a.unwrap();
    let price_b = query.price_b.unwrap();

    let mode = SpreadCalculationMode::Percentage;

    let calc = get_spread_calculator();
    let signal_gen = get_signal_generator();

    let live = calc.calculate_spread(pair_id, price_a, price_b, &mode)?;

    let signal = signal_gen
        .generate_signal(&live, Decimal::ZERO, Decimal::ZERO, None)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let direction = signal
        .as_ref()
        .map(SignalGenerator::signal_to_direction)
        .unwrap_or(SignalDirection::Neutral);

    let hist = calc
        .windows
        .read()
        .map_err(|e| AppError::LockError(e.to_string()))?
        .get(&pair_id)
        .map(|win| (win.mean(), win.std()));

    Ok(Json(SpreadData {
        pair_id,
        spread: live.spread,
        spread_pct: live.spread_pct,
        z_score: live.z_score,
        historical_mean: hist.and_then(|(m, _)| m),
        historical_std: hist.and_then(|(_, s)| s),
        signal: direction,
        timestamp: live.timestamp,
    }))
}

// ============================================================
// 获取所有套利持仓
// ============================================================
pub async fn list_positions(
    State(db): State<DbPool>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let positions = PositionEntity::find()
        .filter(PositionColumn::Status.eq("open"))
        .all(&*db)
        .await?;

    let items: Vec<serde_json::Value> = positions
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "pair_id": p.pair_id,
                "direction": p.direction,
                "size_a": p.size_a,
                "size_b": p.size_b,
                "entry_spread": p.entry_spread,
                "current_spread": p.current_spread,
                "unrealized_pnl": p.unrealized_pnl,
                "status": p.status,
                "opened_at": p.opened_at.to_rfc3339(),
                "closed_at": p.closed_at.map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(Json(items))
}

// ============================================================
// 获取最近的信号记录
// ============================================================
#[derive(Debug, serde::Deserialize)]
pub struct SignalsQuery {
    pub pair_id: Option<i32>,
    pub limit: Option<i64>,
}

pub async fn list_signals(
    State(db): State<DbPool>,
    Query(query): Query<SignalsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let limit = query.limit.unwrap_or(50);

    let mut sel = SignalEntity::find()
        .order_by(SignalColumn::CreatedAt, sea_orm::Order::Desc)
        .limit(limit as u64);

    if let Some(pid) = query.pair_id {
        sel = sel.filter(SignalColumn::PairId.eq(pid));
    }

    let signals = sel.all(&*db).await?;

    let items: Vec<serde_json::Value> = signals
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "pair_id": s.pair_id,
                "signal_type": s.signal_type,
                "spread": s.spread,
                "z_score": s.z_score,
                "confidence": s.confidence,
                "executed": s.executed,
                "created_at": s.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(items))
}

// ============================================================
// 路由
// ============================================================
pub fn router() -> Router<DbPool> {
    Router::new()
        .route("/api/v1/arbitrage/pairs", axum::routing::post(create_pair))
        .route("/api/v1/arbitrage/pairs", axum::routing::get(list_pairs))
        .route("/api/v1/arbitrage/pairs/{pair_id}", axum::routing::get(get_pair))
        .route("/api/v1/arbitrage/pairs/{pair_id}", axum::routing::put(update_pair))
        .route("/api/v1/arbitrage/pairs/{pair_id}", axum::routing::delete(delete_pair))
        .route("/api/v1/arbitrage/spread/{pair_id}", axum::routing::get(get_spread))
        .route("/api/v1/arbitrage/positions", axum::routing::get(list_positions))
        .route("/api/v1/arbitrage/signals", axum::routing::get(list_signals))
}
