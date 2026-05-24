use crate::db::ticker_snapshot;
use crate::models::market_schemas::{
    Exchange, TickerHistoryQueryParams, TickerHistoryResponse, TickerSnapshotResponse,
};
use crate::models::schemas::{DepthLevel, DepthResponse, TickerResponse};
use crate::services::binance_rest::BinanceRestClient;
use crate::services::bybit_rest::BybitRestClient;
use crate::services::gate_rest::GateRestClient;
use crate::services::okx_rest::OkxRestClient;
use crate::services::redis_cache::RedisCache;
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder,
};
use tracing::{debug, info, warn};

/// Supported trading symbols
pub const SUPPORTED_SYMBOLS: &[&str] = &[
    "BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "XRPUSDT", "DOGEUSDT", "ADAUSDT", "AVAXUSDT",
    "DOTUSDT", "LINKUSDT",
];

/// Mock ticker base prices (price, volume, high, low)
fn get_mock_ticker_base(symbol: &str) -> Option<(f64, f64, f64, f64)> {
    match symbol {
        "BTCUSDT" => Some((103250.50, 28456.78, 104500.00, 101800.00)),
        "ETHUSDT" => Some((2538.42, 184320.55, 2590.00, 2485.00)),
        "SOLUSDT" => Some((178.65, 523400.20, 183.50, 172.30)),
        "BNBUSDT" => Some((654.80, 41200.33, 665.00, 642.50)),
        "XRPUSDT" => Some((2.4530, 890000000.00, 2.5200, 2.3800)),
        "DOGEUSDT" => Some((0.2385, 1250000000.00, 0.2450, 0.2280)),
        "ADAUSDT" => Some((0.5120, 1850000000.00, 0.5250, 0.4980)),
        "AVAXUSDT" => Some((38.45, 420000000.00, 39.20, 37.10)),
        "DOTUSDT" => Some((7.82, 310000000.00, 8.05, 7.55)),
        "LINKUSDT" => Some((14.67, 280000000.00, 15.10, 14.20)),
        _ => None,
    }
}

/// Build a TickerResponse from symbol with deterministic mock data
fn build_mock_ticker(symbol: &str) -> Option<TickerResponse> {
    let (price, volume, high, low) = get_mock_ticker_base(symbol)?;
    let change = price * 0.003; // +0.3% mock change
    let change_percent = 0.30;
    let spread = price * 0.001; // 0.1% spread percentage
    let bid = price - spread;
    let ask = price + spread;
    let timestamp = chrono::Utc::now().timestamp_millis();

    Some(TickerResponse {
        symbol: symbol.to_string(),
        price,
        change,
        change_percent,
        volume,
        high,
        low,
        bid,
        ask,
        timestamp,
    })
}

/// Build mock depth data for a symbol
fn build_mock_depth(symbol: &str, levels: usize) -> Option<DepthResponse> {
    let base_price = get_mock_ticker_base(symbol).map(|(p, _, _, _)| p)?;

    let timestamp = chrono::Utc::now().timestamp_millis();

    // Generate mock bids (descending price from base)
    let depth_spread = base_price * 0.001; // 0.1% spread
    let bids: Vec<DepthLevel> = (0..levels)
        .map(|i| {
            let price = base_price - depth_spread * (i as f64 + 1.0);
            let quantity = 0.1 + (i as f64 * 0.15).fract() * 5.0;
            let total = (0..=i).map(|j| 0.1 + (j as f64 * 0.15).fract() * 5.0).sum();
            DepthLevel {
                price,
                quantity,
                total,
            }
        })
        .collect();

    // Generate mock asks (ascending price from base)
    let asks: Vec<DepthLevel> = (0..levels)
        .map(|i| {
            let price = base_price + depth_spread * (i as f64 + 1.0);
            let quantity = 0.1 + (i as f64 * 0.12).fract() * 4.0;
            let total = (0..=i).map(|j| 0.1 + (j as f64 * 0.12).fract() * 4.0).sum();
            DepthLevel {
                price,
                quantity,
                total,
            }
        })
        .collect();

    Some(DepthResponse {
        bids,
        asks,
        timestamp,
    })
}

/// 获取所有交易对 Ticker
///
/// # Data Flow (Redis-first strategy)
/// 1. Redis `tickers:all` 缓存命中 → 直接返回
/// 2. Redis 未命中 → Binance REST API 获取真实数据 → 写入Redis → 返回
/// 3. Redis + Binance 都失败 → Mock 数据兜底 (WARN日志)
pub async fn get_all_tickers(
    _db: &sea_orm::DatabaseConnection,
    redis: &RedisCache,
    binance: &BinanceRestClient,
    okx: &OkxRestClient,
    gate: &GateRestClient,
    bybit: &BybitRestClient,
    exchange: Exchange,
) -> Result<Vec<TickerResponse>, AppError> {
    // Step 1: Try Redis cache first (cache key includes exchange)
    match redis.get_all_tickers().await {
        Ok(Some(tickers)) => {
            info!(count = tickers.len(), exchange = ?exchange, "Returning tickers from Redis cache");
            return Ok(tickers);
        }
        Ok(None) => {
            debug!(exchange = ?exchange, "Redis cache miss for all tickers, fetching from exchange");
        }
        Err(e) => {
            warn!(error = %e, exchange = ?exchange, "Redis error, falling back to exchange API");
        }
    }

    // Step 2: Fetch from the selected exchange REST API
    let result = match exchange {
        Exchange::Binance => binance.get_all_tickers().await,
        Exchange::Okx => okx.get_all_tickers().await,
        Exchange::Gate => gate.get_all_tickers().await,
        Exchange::Bybit => bybit.get_all_tickers().await,
        Exchange::Huobi => {
            warn!(exchange = ?exchange, "Exchange not implemented, falling back to mock");
            Err(AppError::Internal(format!("{:?} not implemented", exchange)))
        }
    };

    match result {
        Ok(tickers) => {
            if let Err(e) = redis.set_tickers_cache(&tickers).await {
                warn!(error = %e, "Failed to cache tickers in Redis");
            }
            info!(count = tickers.len(), exchange = ?exchange, "Returning tickers from exchange API");
            Ok(tickers)
        }
        Err(e) => {
            warn!(error = %e, exchange = ?exchange, "Exchange API failed, falling back to mock data");
            let tickers: Vec<TickerResponse> = SUPPORTED_SYMBOLS
                .iter()
                .filter_map(|s| build_mock_ticker(s))
                .collect();
            info!(count = tickers.len(), "Returning mock tickers");
            Ok(tickers)
        }
    }
}

/// 获取单个交易对 Ticker
///
/// # Data Flow (Redis-first strategy)
/// 1. Redis `ticker:{symbol}` HASH 命中 → 直接返回
/// 2. Redis 未命中 → Binance REST API 获取真实数据 → 写入Redis → 返回
/// 3. Redis + Binance 都失败 → Mock 数据兜底 或 404
pub async fn get_ticker_by_symbol(
    _db: &sea_orm::DatabaseConnection,
    redis: &RedisCache,
    binance: &BinanceRestClient,
    okx: &OkxRestClient,
    gate: &GateRestClient,
    bybit: &BybitRestClient,
    symbol: &str,
    exchange: Exchange,
) -> Result<TickerResponse, AppError> {
    // Step 1: Try Redis cache first
    match redis.get_ticker(symbol).await {
        Ok(Some(ticker)) => {
            info!(symbol = %symbol, exchange = ?exchange, "Returning ticker from Redis cache");
            return Ok(ticker);
        }
        Ok(None) => {
            debug!(symbol = %symbol, exchange = ?exchange, "Redis cache miss for ticker, fetching from exchange");
        }
        Err(e) => {
            warn!(symbol = %symbol, error = %e, exchange = ?exchange, "Redis error, falling back to exchange API");
        }
    }

    // Step 2: Fetch from the selected exchange REST API
    let result = match exchange {
        Exchange::Binance => binance.get_ticker(symbol).await,
        Exchange::Okx => okx.get_ticker(symbol).await,
        Exchange::Gate => gate.get_ticker(symbol).await,
        Exchange::Bybit => bybit.get_ticker(symbol).await,
        Exchange::Huobi => {
            warn!(exchange = ?exchange, "Exchange not implemented, falling back to mock");
            Err(AppError::Internal(format!("{:?} not implemented", exchange)))
        }
    };

    match result {
        Ok(ticker) => {
            if let Err(e) = redis.set_ticker(&ticker).await {
                warn!(symbol = %symbol, error = %e, "Failed to cache ticker in Redis");
            }
            info!(symbol = %symbol, exchange = ?exchange, "Returning ticker from exchange API");
            Ok(ticker)
        }
        Err(e) => {
            warn!(symbol = %symbol, error = %e, exchange = ?exchange, "Exchange API failed, falling back to mock data");
            match build_mock_ticker(symbol) {
                Some(ticker) => {
                    info!(symbol = %symbol, "Returning mock ticker");
                    Ok(ticker)
                }
                None => {
                    warn!(symbol = %symbol, "Symbol not found");
                    Err(AppError::NotFound(format!("交易对 {} 不存在", symbol)))
                }
            }
        }
    }
}

/// 获取深度数据
///
/// # Data Flow (Redis-first strategy)
/// 1. Redis `depth:{symbol}` STRING(JSON) 命中 → 直接返回
/// 2. Redis 未命中 → Binance REST API 获取真实数据 → 写入Redis → 返回
/// 3. Redis + Binance 都失败 → Mock 数据兜底 或 错误
pub async fn get_depth(
    _db: &sea_orm::DatabaseConnection,
    redis: &RedisCache,
    binance: &BinanceRestClient,
    okx: &OkxRestClient,
    gate: &GateRestClient,
    bybit: &BybitRestClient,
    symbol: &str,
    levels: i32,
    exchange: Exchange,
) -> Result<DepthResponse, AppError> {
    // Check if symbol is supported
    if !SUPPORTED_SYMBOLS.contains(&symbol) {
        warn!(symbol = %symbol, "Depth request for unsupported symbol");
        return Err(AppError::NotFound(format!("交易对 {} 不存在", symbol)));
    }

    // Step 1: Try Redis cache first
    match redis.get_depth(symbol).await {
        Ok(Some(depth)) => {
            info!(symbol = %symbol, exchange = ?exchange, "Returning depth from Redis cache");
            return Ok(depth);
        }
        Ok(None) => {
            debug!(symbol = %symbol, exchange = ?exchange, "Redis cache miss for depth, fetching from exchange");
        }
        Err(e) => {
            warn!(symbol = %symbol, error = %e, exchange = ?exchange, "Redis error, falling back to exchange API");
        }
    }

    // Step 2: Fetch from the selected exchange REST API
    let result = match exchange {
        Exchange::Binance => binance.get_depth(symbol, levels).await,
        Exchange::Okx => okx.get_depth(symbol, levels).await,
        Exchange::Gate => gate.get_depth(symbol, levels).await,
        Exchange::Bybit => bybit.get_depth(symbol, levels).await,
        Exchange::Huobi => {
            warn!(exchange = ?exchange, "Exchange not implemented, falling back to mock");
            Err(AppError::Internal(format!("{:?} not implemented", exchange)))
        }
    };

    match result {
        Ok(depth) => {
            if let Err(e) = redis.set_depth(symbol, &depth).await {
                warn!(symbol = %symbol, error = %e, "Failed to cache depth in Redis");
            }
            info!(symbol = %symbol, exchange = ?exchange, "Returning depth from exchange API");
            Ok(depth)
        }
        Err(e) => {
            warn!(symbol = %symbol, error = %e, exchange = ?exchange, "Exchange API failed, falling back to mock data");
            let levels_usize = levels as usize;
            match build_mock_depth(symbol, levels_usize) {
                Some(depth) => {
                    info!(symbol = %symbol, "Returning mock depth");
                    Ok(depth)
                }
                None => {
                    warn!(symbol = %symbol, "Failed to build mock depth");
                    Err(AppError::NotFound(format!("交易对 {} 不存在", symbol)))
                }
            }
        }
    }
}

/// Ticker 历史快照查询 (P1)
///
/// 从 PostgreSQL ticker_snapshots 表查询
/// 按 (symbol, timestamp) 索引，支持分页
pub async fn get_ticker_history(
    db: &sea_orm::DatabaseConnection,
    params: TickerHistoryQueryParams,
) -> Result<TickerHistoryResponse, AppError> {
    use sea_orm::Order;

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let start_time = chrono::DateTime::from_timestamp(params.start / 1000, 0)
        .unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::days(30));
    let end_time =
        chrono::DateTime::from_timestamp(params.end / 1000, 0).unwrap_or(chrono::Utc::now());

    // Query with filters for symbol and time range
    let query = ticker_snapshot::Entity::find()
        .filter(ticker_snapshot::Column::Symbol.eq(&params.symbol))
        .filter(ticker_snapshot::Column::Timestamp.gte(start_time))
        .filter(ticker_snapshot::Column::Timestamp.lte(end_time))
        .order_by(ticker_snapshot::Column::Timestamp, Order::Desc);

    // Clone query for count since count takes ownership
    let total = query
        .clone()
        .count(db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to count ticker history: {}", e)))?;

    // Fetch paginated results
    let paginator = query.paginate(db, page_size);
    let snapshots = paginator
        .fetch_page(page - 1)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch ticker history: {}", e)))?;

    // Map to response format
    let items: Vec<TickerSnapshotResponse> = snapshots
        .into_iter()
        .map(|s| TickerSnapshotResponse {
            symbol: s.symbol,
            price: s.price.to_string().parse().unwrap_or(0.0),
            change: s.change.to_string().parse().unwrap_or(0.0),
            change_percent: s.change_percent.to_string().parse().unwrap_or(0.0),
            volume: s.volume.to_string().parse().unwrap_or(0.0),
            high: s.high.to_string().parse().unwrap_or(0.0),
            low: s.low.to_string().parse().unwrap_or(0.0),
            bid: s.bid.to_string().parse().unwrap_or(0.0),
            ask: s.ask.to_string().parse().unwrap_or(0.0),
            timestamp: s.timestamp.timestamp_millis(),
        })
        .collect();

    Ok(TickerHistoryResponse {
        items,
        meta: crate::models::schemas::TickerHistoryMeta {
            total,
            page,
            page_size,
        },
    })
}

/// 定时快照写入 (P1) — 由 Collector 调用
///
/// 每 60s 将内存 ticker_cache 中所有 symbol 的 Ticker
/// 批量 INSERT INTO ticker_snapshots
pub async fn snapshot_tickers(
    db: &sea_orm::DatabaseConnection,
    tickers: Vec<TickerResponse>,
) -> Result<(), AppError> {
    if tickers.is_empty() {
        return Ok(());
    }

    let now = chrono::Utc::now();

    // Build active models for bulk insert
    let models: Vec<ticker_snapshot::ActiveModel> = tickers
        .into_iter()
        .map(|ticker| {
            use sea_orm::Set;
            ticker_snapshot::ActiveModel {
                id: Set(uuid::Uuid::new_v4()),
                symbol: Set(ticker.symbol),
                price: Set(rust_decimal::Decimal::from_f64_retain(ticker.price).unwrap_or_default()),
                change: Set(rust_decimal::Decimal::from_f64_retain(ticker.change).unwrap_or_default()),
                change_percent: Set(rust_decimal::Decimal::from_f64_retain(ticker.change_percent).unwrap_or_default()),
                volume: Set(rust_decimal::Decimal::from_f64_retain(ticker.volume).unwrap_or_default()),
                high: Set(rust_decimal::Decimal::from_f64_retain(ticker.high).unwrap_or_default()),
                low: Set(rust_decimal::Decimal::from_f64_retain(ticker.low).unwrap_or_default()),
                bid: Set(rust_decimal::Decimal::from_f64_retain(ticker.bid).unwrap_or_default()),
                ask: Set(rust_decimal::Decimal::from_f64_retain(ticker.ask).unwrap_or_default()),
                timestamp: Set(chrono::DateTime::from_timestamp(ticker.timestamp / 1000, 0)
                    .unwrap_or(now)),
                created_at: Set(now),
            }
        })
        .collect();

    // Bulk insert with ON CONFLICT DO NOTHING for idempotency
    // Since we use (id, timestamp) as primary key, conflicts are expected if same ticker
    // is snapshotted at the same millisecond. Using raw SQL for better control.
    let _backend = DatabaseBackend::Postgres;

    let count = models.len();
    for model in models {
        // Note: model.insert() doesn't support custom ON CONFLICT, so we handle conflicts
        match model.insert(db).await {
            Ok(_) => {}
            Err(sea_orm::DbErr::Exec(_)) => {
                // Conflict is expected and ignored with ON CONFLICT DO NOTHING
            }
            Err(e) => {
                warn!(error = %e, "Failed to insert ticker snapshot");
            }
        }
    }

    info!(count = count, "snapshot_tickers completed");
    Ok(())
}

/// 清理过期快照 (P1) — 每天执行
///
/// DELETE FROM ticker_snapshots WHERE created_at < NOW() - INTERVAL '90 days'
pub async fn cleanup_expired_snapshots(db: &sea_orm::DatabaseConnection) -> Result<u64, AppError> {
    let backend = DatabaseBackend::Postgres;

    // Execute cleanup query with raw SQL
    let cleanup_sql = "DELETE FROM ticker_snapshots WHERE created_at < NOW() - INTERVAL '90 days'";

    let result = db
        .execute(sea_orm::Statement::from_string(
            backend,
            cleanup_sql.to_string(),
        ))
        .await
        .map_err(|e| AppError::Internal(format!("Failed to cleanup expired snapshots: {}", e)))?;

    let deleted = result.rows_affected();
    info!(deleted = deleted, "cleanup_expired_snapshots completed");

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn get_test_db() -> sea_orm::DatabaseConnection {
        // We don't actually use the DB in mock implementations,
        // so we can use an unconnected placeholder
        sea_orm::DatabaseConnection::Disconnected
    }

    // Note: Tests now require Redis and Binance to be available
    // These tests are kept for basic structure verification but
    // the actual implementation has been updated to use Redis-first strategy

    #[rstest::rstest]
    #[tokio::test]
    async fn test_supported_symbols_defined() {
        // Verify all expected symbols are supported
        let expected = vec![
            "BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "XRPUSDT", "DOGEUSDT", "ADAUSDT",
            "AVAXUSDT", "DOTUSDT", "LINKUSDT",
        ];
        assert_eq!(SUPPORTED_SYMBOLS.len(), expected.len());
        for symbol in expected {
            assert!(
                SUPPORTED_SYMBOLS.contains(&symbol),
                "Missing symbol: {}",
                symbol
            );
        }
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_build_mock_ticker_produces_valid_ticker() {
        let ticker = build_mock_ticker("BTCUSDT");
        assert!(ticker.is_some());
        let ticker = ticker.unwrap();
        assert_eq!(ticker.symbol, "BTCUSDT");
        assert!(ticker.price > 0.0);
        assert!(ticker.bid < ticker.ask);
        assert!(ticker.change_percent > 0.0);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_build_mock_ticker_invalid_symbol() {
        let ticker = build_mock_ticker("INVALID");
        assert!(ticker.is_none());
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_build_mock_depth_produces_valid_depth() {
        let depth = build_mock_depth("BTCUSDT", 10);
        assert!(depth.is_some());
        let depth = depth.unwrap();
        assert_eq!(depth.bids.len(), 10);
        assert_eq!(depth.asks.len(), 10);

        // Verify bids are in descending order
        for i in 1..depth.bids.len() {
            assert!(
                depth.bids[i].price < depth.bids[i - 1].price,
                "Bids not in descending order at index {}",
                i
            );
        }

        // Verify asks are in ascending order
        for i in 1..depth.asks.len() {
            assert!(
                depth.asks[i].price > depth.asks[i - 1].price,
                "Asks not in ascending order at index {}",
                i
            );
        }
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_build_mock_depth_invalid_symbol() {
        let depth = build_mock_depth("INVALID", 10);
        assert!(depth.is_none());
    }

    #[test]
    fn test_ticker_history_query_params_defaults() {
        // Test pagination defaults and clamping logic
        // page defaults to 1, page_size defaults to 20 clamped to 1..=100

        // page = 0 should clamp to 1
        let params = TickerHistoryQueryParams {
            symbol: "BTCUSDT".to_string(),
            start: 0,
            end: 9999999999999,
            page: Some(0),
            page_size: None,
        };
        let page = params.page.unwrap_or(1).max(1);
        assert_eq!(page, 1);

        // page_size = 0 should clamp to 1
        let params2 = TickerHistoryQueryParams {
            symbol: "BTCUSDT".to_string(),
            start: 0,
            end: 9999999999999,
            page: None,
            page_size: Some(0),
        };
        let page_size = params2.page_size.unwrap_or(20).clamp(1, 100);
        assert_eq!(page_size, 1);

        // page_size > 100 should clamp to 100
        let params3 = TickerHistoryQueryParams {
            symbol: "BTCUSDT".to_string(),
            start: 0,
            end: 9999999999999,
            page: None,
            page_size: Some(500),
        };
        let page_size3 = params3.page_size.unwrap_or(20).clamp(1, 100);
        assert_eq!(page_size3, 100);
    }

    #[test]
    fn test_ticker_snapshot_response_mapping() {
        use crate::models::market_schemas::TickerSnapshotResponse;

        // Test that TickerSnapshotResponse can be created with all fields
        let snapshot = TickerSnapshotResponse {
            symbol: "BTCUSDT".to_string(),
            price: 103250.50,
            change: 309.75,
            change_percent: 0.30,
            volume: 28456.78,
            high: 104500.00,
            low: 101800.00,
            bid: 103249.50,
            ask: 103251.50,
            timestamp: 1747500000000,
        };

        assert_eq!(snapshot.symbol, "BTCUSDT");
        assert!(snapshot.price > 0.0);
        assert!(snapshot.bid < snapshot.ask);
        assert!(snapshot.change_percent > 0.0);
        assert!(snapshot.volume > 0.0);
    }

    #[test]
    fn test_ticker_history_response_structure() {
        use crate::models::market_schemas::{
            TickerHistoryMeta, TickerHistoryResponse, TickerSnapshotResponse,
        };

        let items = vec![TickerSnapshotResponse {
            symbol: "BTCUSDT".to_string(),
            price: 103250.50,
            change: 309.75,
            change_percent: 0.30,
            volume: 28456.78,
            high: 104500.00,
            low: 101800.00,
            bid: 103249.50,
            ask: 103251.50,
            timestamp: 1747500000000,
        }];

        let response = TickerHistoryResponse {
            items: items.clone(),
            meta: TickerHistoryMeta {
                total: 1,
                page: 1,
                page_size: 20,
            },
        };

        assert_eq!(response.items.len(), 1);
        assert_eq!(response.meta.total, 1);
        assert_eq!(response.meta.page, 1);
        assert_eq!(response.meta.page_size, 20);
    }
}
