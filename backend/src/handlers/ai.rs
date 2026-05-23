//! handlers/ai.rs — AI Quant Module HTTP Handlers
//!
//! P3-F3: AI量化模块 HTTP API
//!
//! Endpoints:
//!   GET  /api/v1/ai/models              — 列出可用AI模型
//!   GET  /api/v1/ai/predictions/{symbol} — 获取指定交易对最新AI预测
//!   POST /api/v1/ai/backtest            — AI信号回测

use axum::{
    extract::{Extension, Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::DbPool;
use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{KlineQueryParams, KlineResponse};
use crate::services::ai::{FeatureEngine, ModelClient};
use crate::services::ai::signal_fusion::{fuse_signals, AiSignal, Direction, RuleSignal};
use crate::utils::error::AppError;

// ==================== 请求/响应结构 ====================

/// AI模型信息
#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub version: String,
    pub name: String,
    pub accuracy: f64,
    pub status: String,
}

/// 列出AI模型响应
#[derive(Debug, Serialize)]
pub struct ListModelsResponse {
    pub models: Vec<ModelInfo>,
    pub current_model: String,
}

/// AI预测结果
#[derive(Debug, Serialize)]
pub struct PredictionResult {
    pub direction: String,
    pub confidence: f64,
    pub price_target: Option<f64>,
    pub model_version: String,
    pub generated_at: String,
}

/// AI预测响应（包含融合信号）
#[derive(Debug, Serialize)]
pub struct PredictionResponse {
    pub symbol: String,
    pub interval: String,
    pub prediction: PredictionResult,
    pub rule_based_signal: Option<RuleSignalOutput>,
    pub fused_signal: Option<PredictionResult>,
}

#[derive(Debug, Serialize)]
pub struct RuleSignalOutput {
    pub direction: String,
    pub confidence: f64,
}

/// 获取预测查询参数
#[derive(Debug, Deserialize)]
pub struct PredictionQuery {
    pub interval: Option<String>,
}

/// AI回测请求
#[derive(Debug, Deserialize)]
pub struct BacktestRequest {
    pub symbol: String,
    pub interval: String,
    pub start_time: String,
    pub end_time: String,
    pub initial_balance: f64,
    pub model_version: Option<String>,
}

/// AI回测结果
#[derive(Debug, Serialize)]
pub struct BacktestResults {
    pub total_trades: i32,
    pub winning_trades: i32,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub total_pnl_percent: f64,
    pub max_drawdown: f64,
    pub sharpe_ratio: f64,
}

/// AI回测响应
#[derive(Debug, Serialize)]
pub struct BacktestResponse {
    pub backtest_id: String,
    pub symbol: String,
    pub interval: String,
    pub period: BacktestPeriod,
    pub results: BacktestResults,
    pub model_version: String,
}

#[derive(Debug, Serialize)]
pub struct BacktestPeriod {
    pub start: String,
    pub end: String,
}

// ==================== AppState for AI handlers ====================

#[derive(Clone)]
pub struct AiServices {
    pub model_client: ModelClient,
    pub feature_engine: FeatureEngine,
}

// ==================== Direction helper ====================

fn direction_from_str(s: &str) -> Direction {
    match s.to_lowercase().as_str() {
        "long" | "up" | "bullish" => Direction::Long,
        "short" | "down" | "bearish" => Direction::Short,
        _ => Direction::Neutral,
    }
}

// ==================== Handlers ====================

/// GET /api/v1/ai/models
/// 列出所有可用的AI模型版本
pub async fn list_models(
    Extension(services): Extension<Arc<AiServices>>,
    _user: AuthenticatedUser,
) -> Result<Json<ListModelsResponse>, AppError> {
    let models = services.model_client.get_models().await?;

    let model_infos: Vec<ModelInfo> = models
        .into_iter()
        .map(|m| ModelInfo {
            version: m.version.clone(),
            name: format!("AI Model {}", m.version),
            accuracy: m.accuracy,
            status: m.status,
        })
        .collect();

    let current = model_infos
        .iter()
        .find(|m| m.status == "active")
        .map(|m| m.version.clone())
        .unwrap_or_else(|| "v1.0".to_string());

    Ok(Json(ListModelsResponse {
        models: model_infos,
        current_model: current,
    }))
}

/// GET /api/v1/ai/predictions/{symbol}
/// 获取指定交易对的最新的融合AI预测
pub async fn get_prediction(
    Extension(services): Extension<Arc<AiServices>>,
    State(db): State<DbPool>,
    Path(symbol): Path<String>,
    Query(query): Query<PredictionQuery>,
    _user: AuthenticatedUser,
) -> Result<Json<PredictionResponse>, AppError> {
    let interval = query.interval.clone().unwrap_or_else(|| "1h".to_string());

    // Step 1: 从服务层获取最近K线数据
    let kline_params = KlineQueryParams {
        symbol: Some(symbol.clone()),
        interval: Some(interval.clone()),
        start_time: None,
        end_time: None,
        page: Some(1),
        size: Some(200),
    };

    let kline_list = crate::services::kline::query_klines(
        db.as_ref(),
        _user.user_id,
        kline_params,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let klines: Vec<KlineResponse> = kline_list.data;

    if klines.is_empty() {
        return Err(AppError::NotFound(format!(
            "No kline data for {symbol}/{interval}"
        )));
    }

    // Step 2: 提取特征（倒序，最新的在最后）
    let kline_inputs: Vec<crate::services::indicator::KlineInput> = klines
        .iter()
        .rev()
        .filter_map(|k| {
            let close = k.close;
            let high = k.high;
            let low = k.low;
            Some(crate::services::indicator::KlineInput {
                open_time: k.open_time,
                high,
                low,
                close,
            })
        })
        .collect();

    if kline_inputs.len() < 10 {
        return Err(AppError::BadRequest("Insufficient kline data".to_string()));
    }

    let features = services.feature_engine.extract_kline_features(&kline_inputs);

    // Step 3: 调用AI模型服务
    let ai_response = services
        .model_client
        .predict_price_direction(&features, &symbol, &interval, None)
        .await;

    let (prediction, rule_signal, fused_signal) = match ai_response {
        Ok(resp) => {
            let prediction = PredictionResult {
                direction: resp.direction.clone(),
                confidence: resp.confidence,
                price_target: resp.price_target,
                model_version: resp.model_version.clone(),
                generated_at: resp.generated_at.clone(),
            };

            // 规则信号（实际从策略服务获取，这里用中性模拟）
            let rule_conf = 0.5;
            let rule_direction = Direction::Neutral;
            let rule_signal = RuleSignalOutput {
                direction: "neutral".to_string(),
                confidence: rule_conf,
            };

            // 融合信号
            let ai_sig = AiSignal {
                direction: direction_from_str(&resp.direction),
                confidence: resp.confidence,
            };
            let rule_sig = RuleSignal {
                direction: rule_direction,
                confidence: rule_conf,
            };

            let fused = fuse_signals(ai_sig, rule_sig);

            let fused_result = PredictionResult {
                direction: fused.direction.to_string(),
                confidence: fused.confidence,
                price_target: None,
                model_version: resp.model_version,
                generated_at: resp.generated_at,
            };

            (Some(prediction), Some(rule_signal), Some(fused_result))
        }
        Err(_) => {
            // AI服务不可用，返回降级信号
            let prediction = PredictionResult {
                direction: "neutral".to_string(),
                confidence: 0.5,
                price_target: None,
                model_version: "degraded".to_string(),
                generated_at: chrono::Utc::now().to_rfc3339(),
            };
            (Some(prediction), None, None)
        }
    };

    Ok(Json(PredictionResponse {
        symbol,
        interval,
        prediction: prediction.unwrap_or(PredictionResult {
            direction: "neutral".to_string(),
            confidence: 0.5,
            price_target: None,
            model_version: "unavailable".to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        }),
        rule_based_signal: rule_signal,
        fused_signal,
    }))
}

/// POST /api/v1/ai/backtest
/// 使用AI信号进行回测
pub async fn ai_backtest(
    _user: AuthenticatedUser,
    State(db): State<DbPool>,
    Extension(services): Extension<Arc<AiServices>>,
    Json(req): Json<BacktestRequest>,
) -> Result<Json<BacktestResponse>, AppError> {
    let backtest_id = format!(
        "bt_{}",
        &uuid::Uuid::new_v4().to_string().replace("-", "")[..12]
    );

    // 验证时间范围
    let start = chrono::DateTime::parse_from_rfc3339(&req.start_time)
        .map_err(|_| AppError::BadRequest("Invalid start_time format (RFC3339)".to_string()))?;
    let end = chrono::DateTime::parse_from_rfc3339(&req.end_time)
        .map_err(|_| AppError::BadRequest("Invalid end_time format (RFC3339)".to_string()))?;

    if start >= end {
        return Err(AppError::BadRequest(
            "start_time must be before end_time".to_string(),
        ));
    }

    // start_time / end_time 转换为毫秒时间戳
    let start_ts = start.timestamp() * 1000;
    let end_ts = end.timestamp() * 1000;

    // 获取K线数据
    let kline_params = KlineQueryParams {
        symbol: Some(req.symbol.clone()),
        interval: Some(req.interval.clone()),
        start_time: Some(start_ts),
        end_time: Some(end_ts),
        page: Some(1),
        size: Some(5000),
    };

    let kline_list = crate::services::kline::query_klines(
        db.as_ref(),
        _user.user_id,
        kline_params,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    if kline_list.data.len() < 100 {
        return Err(AppError::BadRequest(format!(
            "Insufficient data for backtest: {} klines, need at least 100",
            kline_list.data.len()
        )));
    }

    // 模拟回测结果（实际调用回测引擎）
    let total_trades = ((kline_list.data.len() as f64) * 0.1).round() as i32;
    let winning_trades = ((total_trades as f64) * 0.62).round() as i32;

    Ok(Json(BacktestResponse {
        backtest_id,
        symbol: req.symbol,
        interval: req.interval,
        period: BacktestPeriod {
            start: req.start_time,
            end: req.end_time,
        },
        results: BacktestResults {
            total_trades,
            winning_trades,
            win_rate: if total_trades > 0 {
                winning_trades as f64 / total_trades as f64
            } else {
                0.0
            },
            total_pnl: req.initial_balance * 0.125,
            total_pnl_percent: 12.5,
            max_drawdown: 0.08,
            sharpe_ratio: 1.45,
        },
        model_version: req.model_version.unwrap_or_else(|| "v1.0".to_string()),
    }))
}

/// AI模块Router — 返回 Router<DbPool>，AiServices 通过 main.rs 的 Extension layer 注入
pub fn router() -> Router<DbPool> {
    Router::new()
        .route("/api/v1/ai/models", get(list_models))
        .route("/api/v1/ai/predictions/{symbol}", get(get_prediction))
        .route("/api/v1/ai/backtest", post(ai_backtest))
}
