use crate::middleware::auth::AuthenticatedUser;
use crate::models::kline_entity::Entity as KlinePhase4;
use crate::models::schemas::{
    AtrQueryParams, AtrResponse, BollingerQueryParams, BollingerResponse, EmaQueryParams,
    EmaResponse, KdjParams, KdjQueryParams, KdjResponse, MaQueryParams, MaResponse,
    MacdQueryParams, MacdResponse, RsiQueryParams, RsiResponse, StochasticQueryParams,
    StochasticResponse,
};
use crate::services::indicator::{
    KlineInput, compute_atr, compute_bollinger, compute_ema, compute_kdj, compute_ma, compute_macd,
    compute_rsi, compute_stochastic, validate_kdj_params,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Query, State},
};
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use std::sync::Arc;

/// GET /api/v1/kline/kdj
pub async fn get_kdj(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<KdjQueryParams>,
) -> Result<Json<ApiResponse<KdjResponse>>, AppError> {
    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let n = params.n.unwrap_or(9);
    let m1 = params.m1.unwrap_or(3);
    let m2 = params.m2.unwrap_or(3);

    validate_kdj_params(n, m1, m2)?;

    // Query klines from klines_phase4
    use crate::models::kline_entity::Column as Phase4Col;

    let mut query = KlinePhase4::find()
        .filter(Phase4Col::Symbol.eq(symbol))
        .filter(Phase4Col::Interval.eq(interval))
        .order_by_asc(Phase4Col::OpenTime);

    if let Some(start) = params.start_time {
        let start_dt =
            chrono::DateTime::from_timestamp(start / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.gte(start_dt));
    }
    if let Some(end) = params.end_time {
        let end_dt =
            chrono::DateTime::from_timestamp(end / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.lte(end_dt));
    }

    let klines = query.all(db.as_ref()).await?;

    if klines.is_empty() {
        return Ok(Json(ApiResponse::success(KdjResponse {
            data: vec![],
            params: KdjParams { n, m1, m2 },
            symbol: symbol.clone(),
            interval: interval.clone(),
        })));
    }

    // Convert to KlineInput
    let inputs: Vec<KlineInput> = klines
        .iter()
        .map(|k| KlineInput {
            open_time: k.open_time.timestamp_millis(),
            high: k.high.to_f64().unwrap_or(0.0),
            low: k.low.to_f64().unwrap_or(0.0),
            close: k.close.to_f64().unwrap_or(0.0),
        })
        .collect();

    let kdj_bars = compute_kdj(&inputs, n, m1, m2);

    Ok(Json(ApiResponse::success(KdjResponse {
        data: kdj_bars,
        params: KdjParams { n, m1, m2 },
        symbol: symbol.clone(),
        interval: interval.clone(),
    })))
}

/// GET /api/v1/kline/ma
pub async fn get_ma(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<MaQueryParams>,
) -> Result<Json<ApiResponse<MaResponse>>, AppError> {
    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let period = params.period.unwrap_or(7);

    // Query klines from klines_phase4
    use crate::models::kline_entity::Column as Phase4Col;

    let mut query = KlinePhase4::find()
        .filter(Phase4Col::Symbol.eq(symbol))
        .filter(Phase4Col::Interval.eq(interval))
        .order_by_asc(Phase4Col::OpenTime);

    if let Some(start) = params.start_time {
        let start_dt =
            chrono::DateTime::from_timestamp(start / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.gte(start_dt));
    }
    if let Some(end) = params.end_time {
        let end_dt =
            chrono::DateTime::from_timestamp(end / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.lte(end_dt));
    }

    let klines = query.all(db.as_ref()).await?;

    if klines.is_empty() {
        return Ok(Json(ApiResponse::success(MaResponse {
            data: vec![],
            period,
            symbol: symbol.clone(),
            interval: interval.clone(),
        })));
    }

    let inputs: Vec<KlineInput> = klines
        .iter()
        .map(|k| KlineInput {
            open_time: k.open_time.timestamp_millis(),
            high: k.high.to_f64().unwrap_or(0.0),
            low: k.low.to_f64().unwrap_or(0.0),
            close: k.close.to_f64().unwrap_or(0.0),
        })
        .collect();

    let ma_bars = compute_ma(&inputs, period);

    Ok(Json(ApiResponse::success(MaResponse {
        data: ma_bars,
        period,
        symbol: symbol.clone(),
        interval: interval.clone(),
    })))
}

/// GET /api/v1/kline/macd
pub async fn get_macd(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<MacdQueryParams>,
) -> Result<Json<ApiResponse<MacdResponse>>, AppError> {
    use crate::models::kline_entity::Column as Phase4Col;

    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let fast_period = params.fast_period.unwrap_or(12);
    let slow_period = params.slow_period.unwrap_or(26);
    let signal_period = params.signal_period.unwrap_or(9);

    if slow_period <= fast_period {
        return Err(AppError::Validation(
            "slow_period must be > fast_period".into(),
        ));
    }

    let mut query = KlinePhase4::find()
        .filter(Phase4Col::Symbol.eq(symbol))
        .filter(Phase4Col::Interval.eq(interval))
        .order_by_asc(Phase4Col::OpenTime);

    if let Some(start) = params.start_time {
        let start_dt =
            chrono::DateTime::from_timestamp(start / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.gte(start_dt));
    }
    if let Some(end) = params.end_time {
        let end_dt =
            chrono::DateTime::from_timestamp(end / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.lte(end_dt));
    }

    let klines = query.all(db.as_ref()).await?;

    if klines.is_empty() {
        return Ok(Json(ApiResponse::success(MacdResponse {
            data: vec![],
            params: crate::models::schemas::MacdParams {
                fast_period,
                slow_period,
                signal_period,
            },
            symbol: symbol.clone(),
            interval: interval.clone(),
        })));
    }

    let inputs: Vec<KlineInput> = klines
        .iter()
        .map(|k| KlineInput {
            open_time: k.open_time.timestamp_millis(),
            high: k.high.to_f64().unwrap_or(0.0),
            low: k.low.to_f64().unwrap_or(0.0),
            close: k.close.to_f64().unwrap_or(0.0),
        })
        .collect();

    let macd_bars = compute_macd(&inputs, fast_period, slow_period, signal_period);

    Ok(Json(ApiResponse::success(MacdResponse {
        data: macd_bars,
        params: crate::models::schemas::MacdParams {
            fast_period,
            slow_period,
            signal_period,
        },
        symbol: symbol.clone(),
        interval: interval.clone(),
    })))
}

/// GET /api/v1/kline/rsi
pub async fn get_rsi(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<RsiQueryParams>,
) -> Result<Json<ApiResponse<RsiResponse>>, AppError> {
    use crate::models::kline_entity::Column as Phase4Col;

    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let period = params.period.unwrap_or(14);

    let mut query = KlinePhase4::find()
        .filter(Phase4Col::Symbol.eq(symbol))
        .filter(Phase4Col::Interval.eq(interval))
        .order_by_asc(Phase4Col::OpenTime);

    if let Some(start) = params.start_time {
        let start_dt =
            chrono::DateTime::from_timestamp(start / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.gte(start_dt));
    }
    if let Some(end) = params.end_time {
        let end_dt =
            chrono::DateTime::from_timestamp(end / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.lte(end_dt));
    }

    let klines = query.all(db.as_ref()).await?;

    if klines.is_empty() {
        return Ok(Json(ApiResponse::success(RsiResponse {
            data: vec![],
            period,
            symbol: symbol.clone(),
            interval: interval.clone(),
        })));
    }

    let inputs: Vec<KlineInput> = klines
        .iter()
        .map(|k| KlineInput {
            open_time: k.open_time.timestamp_millis(),
            high: k.high.to_f64().unwrap_or(0.0),
            low: k.low.to_f64().unwrap_or(0.0),
            close: k.close.to_f64().unwrap_or(0.0),
        })
        .collect();

    let rsi_bars = compute_rsi(&inputs, period);

    Ok(Json(ApiResponse::success(RsiResponse {
        data: rsi_bars,
        period,
        symbol: symbol.clone(),
        interval: interval.clone(),
    })))
}

/// GET /api/v1/kline/bollinger
pub async fn get_bollinger(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<BollingerQueryParams>,
) -> Result<Json<ApiResponse<BollingerResponse>>, AppError> {
    use crate::models::kline_entity::Column as Phase4Col;

    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let period = params.period.unwrap_or(20);
    let std_dev = params.std_dev.unwrap_or(2.0);

    let mut query = KlinePhase4::find()
        .filter(Phase4Col::Symbol.eq(symbol))
        .filter(Phase4Col::Interval.eq(interval))
        .order_by_asc(Phase4Col::OpenTime);

    if let Some(start) = params.start_time {
        let start_dt =
            chrono::DateTime::from_timestamp(start / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.gte(start_dt));
    }
    if let Some(end) = params.end_time {
        let end_dt =
            chrono::DateTime::from_timestamp(end / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.lte(end_dt));
    }

    let klines = query.all(db.as_ref()).await?;

    if klines.is_empty() {
        return Ok(Json(ApiResponse::success(BollingerResponse {
            data: vec![],
            params: crate::models::schemas::BollingerParams { period, std_dev },
            symbol: symbol.clone(),
            interval: interval.clone(),
        })));
    }

    let inputs: Vec<KlineInput> = klines
        .iter()
        .map(|k| KlineInput {
            open_time: k.open_time.timestamp_millis(),
            high: k.high.to_f64().unwrap_or(0.0),
            low: k.low.to_f64().unwrap_or(0.0),
            close: k.close.to_f64().unwrap_or(0.0),
        })
        .collect();

    let bb_bars = compute_bollinger(&inputs, period, std_dev);

    Ok(Json(ApiResponse::success(BollingerResponse {
        data: bb_bars,
        params: crate::models::schemas::BollingerParams { period, std_dev },
        symbol: symbol.clone(),
        interval: interval.clone(),
    })))
}

/// GET /api/v1/kline/ema
pub async fn get_ema(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<EmaQueryParams>,
) -> Result<Json<ApiResponse<EmaResponse>>, AppError> {
    use crate::models::kline_entity::Column as Phase4Col;

    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let period = params.period.unwrap_or(21);

    let mut query = KlinePhase4::find()
        .filter(Phase4Col::Symbol.eq(symbol))
        .filter(Phase4Col::Interval.eq(interval))
        .order_by_asc(Phase4Col::OpenTime);

    if let Some(start) = params.start_time {
        let start_dt =
            chrono::DateTime::from_timestamp(start / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.gte(start_dt));
    }
    if let Some(end) = params.end_time {
        let end_dt =
            chrono::DateTime::from_timestamp(end / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.lte(end_dt));
    }

    let klines = query.all(db.as_ref()).await?;

    if klines.is_empty() {
        return Ok(Json(ApiResponse::success(EmaResponse {
            data: vec![],
            period,
            symbol: symbol.clone(),
            interval: interval.clone(),
        })));
    }

    let inputs: Vec<KlineInput> = klines
        .iter()
        .map(|k| KlineInput {
            open_time: k.open_time.timestamp_millis(),
            high: k.high.to_f64().unwrap_or(0.0),
            low: k.low.to_f64().unwrap_or(0.0),
            close: k.close.to_f64().unwrap_or(0.0),
        })
        .collect();

    let ema_bars = compute_ema(&inputs, period);

    Ok(Json(ApiResponse::success(EmaResponse {
        data: ema_bars,
        period,
        symbol: symbol.clone(),
        interval: interval.clone(),
    })))
}

/// GET /api/v1/kline/atr
pub async fn get_atr(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<AtrQueryParams>,
) -> Result<Json<ApiResponse<AtrResponse>>, AppError> {
    use crate::models::kline_entity::Column as Phase4Col;

    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let period = params.period.unwrap_or(14);

    let mut query = KlinePhase4::find()
        .filter(Phase4Col::Symbol.eq(symbol))
        .filter(Phase4Col::Interval.eq(interval))
        .order_by_asc(Phase4Col::OpenTime);

    if let Some(start) = params.start_time {
        let start_dt =
            chrono::DateTime::from_timestamp(start / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.gte(start_dt));
    }
    if let Some(end) = params.end_time {
        let end_dt =
            chrono::DateTime::from_timestamp(end / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.lte(end_dt));
    }

    let klines = query.all(db.as_ref()).await?;

    if klines.is_empty() {
        return Ok(Json(ApiResponse::success(AtrResponse {
            data: vec![],
            period,
            symbol: symbol.clone(),
            interval: interval.clone(),
        })));
    }

    let inputs: Vec<KlineInput> = klines
        .iter()
        .map(|k| KlineInput {
            open_time: k.open_time.timestamp_millis(),
            high: k.high.to_f64().unwrap_or(0.0),
            low: k.low.to_f64().unwrap_or(0.0),
            close: k.close.to_f64().unwrap_or(0.0),
        })
        .collect();

    let atr_bars = compute_atr(&inputs, period);

    Ok(Json(ApiResponse::success(AtrResponse {
        data: atr_bars,
        period,
        symbol: symbol.clone(),
        interval: interval.clone(),
    })))
}

/// GET /api/v1/kline/stochastic
pub async fn get_stochastic(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<StochasticQueryParams>,
) -> Result<Json<ApiResponse<StochasticResponse>>, AppError> {
    use crate::models::kline_entity::Column as Phase4Col;

    let symbol = params
        .symbol
        .as_ref()
        .ok_or_else(|| AppError::Validation("symbol is required".into()))?;
    let interval = params
        .interval
        .as_ref()
        .ok_or_else(|| AppError::Validation("interval is required".into()))?;

    let k_period = params.k_period.unwrap_or(14);
    let d_period = params.d_period.unwrap_or(3);
    let smooth_k = params.smooth_k.unwrap_or(3);

    let mut query = KlinePhase4::find()
        .filter(Phase4Col::Symbol.eq(symbol))
        .filter(Phase4Col::Interval.eq(interval))
        .order_by_asc(Phase4Col::OpenTime);

    if let Some(start) = params.start_time {
        let start_dt =
            chrono::DateTime::from_timestamp(start / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.gte(start_dt));
    }
    if let Some(end) = params.end_time {
        let end_dt =
            chrono::DateTime::from_timestamp(end / 1000, 0).unwrap_or_else(chrono::Utc::now);
        query = query.filter(Phase4Col::OpenTime.lte(end_dt));
    }

    let klines = query.all(db.as_ref()).await?;

    if klines.is_empty() {
        return Ok(Json(ApiResponse::success(StochasticResponse {
            data: vec![],
            params: crate::models::schemas::StochasticParams {
                k_period,
                d_period,
                smooth_k,
            },
            symbol: symbol.clone(),
            interval: interval.clone(),
        })));
    }

    let inputs: Vec<KlineInput> = klines
        .iter()
        .map(|k| KlineInput {
            open_time: k.open_time.timestamp_millis(),
            high: k.high.to_f64().unwrap_or(0.0),
            low: k.low.to_f64().unwrap_or(0.0),
            close: k.close.to_f64().unwrap_or(0.0),
        })
        .collect();

    let stoch_bars = compute_stochastic(&inputs, k_period, d_period, smooth_k);

    // Map BollingerBar (upper=k, middle=d, lower=unused) to StochasticBar
    let stoch_data: Vec<crate::models::schemas::StochasticBar> = stoch_bars
        .into_iter()
        .map(|b| crate::models::schemas::StochasticBar {
            open_time: b.open_time,
            k: b.upper,
            d: b.middle,
        })
        .collect();

    Ok(Json(ApiResponse::success(StochasticResponse {
        data: stoch_data,
        params: crate::models::schemas::StochasticParams {
            k_period,
            d_period,
            smooth_k,
        },
        symbol: symbol.clone(),
        interval: interval.clone(),
    })))
}
