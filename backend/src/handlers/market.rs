use crate::middleware::auth::AuthenticatedUser;
use crate::models::schemas::{
    DepthQueryParams, DepthResponse, KlineQueryParams, KlineQueryResponse,
    TickerHistoryQueryParams, TickerHistoryResponse, TickerQueryParams, TickerResponse,
};
use crate::services::binance_rest::BinanceRestClient;
use crate::services::market_data;
use crate::services::redis_cache::RedisCache;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Extension, Json,
    extract::{Query, State},
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// Valid depth levels
const VALID_DEPTH_LEVELS: [i32; 4] = [5, 10, 20, 50];
const DEFAULT_DEPTH_LEVELS: i32 = 10;
const MAX_FREE_DEPTH_LEVELS: i32 = 20;

/// GET /api/v1/market/tickers — 获取所有交易对 Ticker
pub async fn get_tickers(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Extension(redis): Extension<Arc<RedisCache>>,
    Extension(binance): Extension<Arc<BinanceRestClient>>,
) -> Result<Json<ApiResponse<Vec<TickerResponse>>>, AppError> {
    let tickers = market_data::get_all_tickers(&db, &redis, &binance).await?;
    Ok(Json(ApiResponse::success(tickers)))
}

/// GET /api/v1/market/ticker — 获取单个交易对 Ticker
pub async fn get_ticker(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Extension(redis): Extension<Arc<RedisCache>>,
    Extension(binance): Extension<Arc<BinanceRestClient>>,
    Query(params): Query<TickerQueryParams>,
) -> Result<Json<ApiResponse<TickerResponse>>, AppError> {
    let ticker = market_data::get_ticker_by_symbol(&db, &redis, &binance, &params.symbol).await?;
    Ok(Json(ApiResponse::success(ticker)))
}

/// GET /api/v1/market/depth — 获取深度数据
pub async fn get_depth(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Extension(redis): Extension<Arc<RedisCache>>,
    Extension(binance): Extension<Arc<BinanceRestClient>>,
    Query(params): Query<DepthQueryParams>,
) -> Result<Json<ApiResponse<DepthResponse>>, AppError> {
    // Validate levels parameter
    let levels = params.levels.unwrap_or(DEFAULT_DEPTH_LEVELS);

    if !VALID_DEPTH_LEVELS.contains(&levels) {
        return Err(AppError::BadRequest(format!(
            "levels 参数必须在 {:?} 中选择",
            VALID_DEPTH_LEVELS
        )));
    }

    // RBAC check: trader can only access up to 20 levels
    if levels > MAX_FREE_DEPTH_LEVELS && user.role != "pro-trader" && user.role != "admin" {
        return Err(AppError::Forbidden(
            "当前角色仅支持 20 档深度，升级至 pro-trader 可查看 50 档".into(),
        ));
    }

    let depth = market_data::get_depth(&db, &redis, &binance, &params.symbol, levels).await?;
    Ok(Json(ApiResponse::success(depth)))
}

/// GET /api/v1/market/ticker/history — Ticker 历史快照查询 (P1)
pub async fn get_ticker_history(
    _user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<TickerHistoryQueryParams>,
) -> Result<Json<ApiResponse<TickerHistoryResponse>>, AppError> {
    let result = market_data::get_ticker_history(&db, params).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/market/kline — K线数据查询（复用 kline::query_klines）
pub async fn get_kline(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<KlineQueryParams>,
) -> Result<Json<ApiResponse<KlineQueryResponse>>, AppError> {
    let result = crate::services::kline::query_klines(&db, user.user_id, params).await?;
    Ok(Json(ApiResponse::success(KlineQueryResponse(result))))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create an authenticated user
    fn make_auth_user(role: &str) -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: uuid::Uuid::new_v4(),
            username: "testuser".to_string(),
            role: role.to_string(),
            jti: "test-jti".to_string(),
        }
    }

    #[tokio::test]
    async fn test_get_tickers_handler() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("trader");
        let result = get_tickers(user, State(db)).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        let body = serde_json::to_value(&*resp).unwrap();
        assert_eq!(body["code"], 0);
        let data = body["data"].as_array().unwrap();
        assert!(!data.is_empty());
    }

    #[tokio::test]
    async fn test_get_ticker_handler_found() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("trader");
        let params = TickerQueryParams {
            symbol: "BTCUSDT".to_string(),
        };
        let result = get_ticker(user, State(db), Query(params)).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        let body = serde_json::to_value(&*resp).unwrap();
        assert_eq!(body["code"], 0);
        assert_eq!(body["data"]["symbol"], "BTCUSDT");
    }

    #[tokio::test]
    async fn test_get_ticker_handler_not_found() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("trader");
        let params = TickerQueryParams {
            symbol: "INVALID99".to_string(),
        };
        let result = get_ticker(user, State(db), Query(params)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_get_depth_handler_default_levels() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("trader");
        let params = DepthQueryParams {
            symbol: "BTCUSDT".to_string(),
            levels: None, // default 10
        };
        let result = get_depth(user, State(db), Query(params)).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        let body = serde_json::to_value(&*resp).unwrap();
        assert_eq!(body["code"], 0);
        assert_eq!(body["data"]["bids"].as_array().unwrap().len(), 10);
        assert_eq!(body["data"]["asks"].as_array().unwrap().len(), 10);
    }

    #[tokio::test]
    async fn test_get_depth_handler_custom_levels() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("pro-trader");
        let params = DepthQueryParams {
            symbol: "ETHUSDT".to_string(),
            levels: Some(20),
        };
        let result = get_depth(user, State(db), Query(params)).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        let body = serde_json::to_value(&*resp).unwrap();
        assert_eq!(body["code"], 0);
        assert_eq!(body["data"]["bids"].as_array().unwrap().len(), 20);
    }

    #[tokio::test]
    async fn test_get_depth_invalid_levels() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("trader");
        let params = DepthQueryParams {
            symbol: "BTCUSDT".to_string(),
            levels: Some(0),
        };
        let result = get_depth(user, State(db), Query(params)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_get_depth_trader_50_forbidden() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("trader");
        let params = DepthQueryParams {
            symbol: "BTCUSDT".to_string(),
            levels: Some(50),
        };
        let result = get_depth(user, State(db), Query(params)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn test_get_depth_pro_trader_50_allowed() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("pro-trader");
        let params = DepthQueryParams {
            symbol: "BTCUSDT".to_string(),
            levels: Some(50),
        };
        let result = get_depth(user, State(db), Query(params)).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        let body = serde_json::to_value(&*resp).unwrap();
        assert_eq!(body["data"]["bids"].as_array().unwrap().len(), 50);
    }

    #[tokio::test]
    async fn test_get_depth_admin_50_allowed() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("admin");
        let params = DepthQueryParams {
            symbol: "BTCUSDT".to_string(),
            levels: Some(50),
        };
        let result = get_depth(user, State(db), Query(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_depth_invalid_levels_15() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("trader");
        let params = DepthQueryParams {
            symbol: "BTCUSDT".to_string(),
            levels: Some(15), // not in [5,10,20,50]
        };
        let result = get_depth(user, State(db), Query(params)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_get_ticker_history_not_implemented() {
        let db = Arc::new(sea_orm::DatabaseConnection::Disconnected);
        let user = make_auth_user("trader");
        let params = TickerHistoryQueryParams {
            symbol: "BTCUSDT".to_string(),
            start: 0,
            end: 0,
            page: None,
            page_size: None,
        };
        let result = get_ticker_history(user, State(db), Query(params)).await;
        assert!(result.is_err());
    }
}
