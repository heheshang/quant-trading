use crate::middleware::auth::AuthenticatedUser;
use crate::models::kline_entity::Entity as KlinePhase4;
use crate::models::schemas::{KdjParams, KdjQueryParams, KdjResponse};
use crate::services::indicator::{KlineInput, compute_kdj, validate_kdj_params};
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
    user: AuthenticatedUser,
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
