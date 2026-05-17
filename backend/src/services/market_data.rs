use crate::models::schemas::{
    DepthLevel, DepthResponse, TickerHistoryQueryParams, TickerHistoryResponse, TickerResponse,
};
use crate::services::binance_rest::BinanceRestClient;
use crate::services::redis_cache::RedisCache;
use crate::utils::error::AppError;
use tracing::{debug, info, warn};

/// Supported trading symbols
const SUPPORTED_SYMBOLS: &[&str] = &[
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
) -> Result<Vec<TickerResponse>, AppError> {
    // Step 1: Try Redis cache first
    match redis.get_all_tickers().await {
        Ok(Some(tickers)) => {
            info!(count = tickers.len(), "Returning tickers from Redis cache");
            return Ok(tickers);
        }
        Ok(None) => {
            // Cache miss, continue to fetch from Binance
            debug!("Redis cache miss for all tickers, fetching from Binance");
        }
        Err(e) => {
            warn!(error = %e, "Redis error, falling back to Binance");
        }
    }

    // Step 2: Fetch from Binance REST API
    match binance.get_all_tickers().await {
        Ok(tickers) => {
            // Cache the result in Redis (best effort, don't fail if Redis write fails)
            if let Err(e) = redis.set_tickers_cache(&tickers).await {
                warn!(error = %e, "Failed to cache tickers in Redis");
            }
            info!(count = tickers.len(), "Returning tickers from Binance API");
            Ok(tickers)
        }
        Err(e) => {
            warn!(error = %e, "Binance API failed, falling back to mock data");
            // Step 3: Fall back to mock data
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
    symbol: &str,
) -> Result<TickerResponse, AppError> {
    // Step 1: Try Redis cache first
    match redis.get_ticker(symbol).await {
        Ok(Some(ticker)) => {
            info!(symbol = %symbol, "Returning ticker from Redis cache");
            return Ok(ticker);
        }
        Ok(None) => {
            // Cache miss, continue to fetch from Binance
            debug!(symbol = %symbol, "Redis cache miss for ticker, fetching from Binance");
        }
        Err(e) => {
            warn!(symbol = %symbol, error = %e, "Redis error, falling back to Binance");
        }
    }

    // Step 2: Fetch from Binance REST API
    match binance.get_ticker(symbol).await {
        Ok(ticker) => {
            // Cache the result in Redis (best effort, don't fail if Redis write fails)
            if let Err(e) = redis.set_ticker(&ticker).await {
                warn!(symbol = %symbol, error = %e, "Failed to cache ticker in Redis");
            }
            info!(symbol = %symbol, "Returning ticker from Binance API");
            Ok(ticker)
        }
        Err(e) => {
            warn!(symbol = %symbol, error = %e, "Binance API failed, falling back to mock data");
            // Step 3: Fall back to mock data
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
    symbol: &str,
    levels: i32,
) -> Result<DepthResponse, AppError> {
    // Check if symbol is supported
    if !SUPPORTED_SYMBOLS.contains(&symbol) {
        warn!(symbol = %symbol, "Depth request for unsupported symbol");
        return Err(AppError::NotFound(format!("交易对 {} 不存在", symbol)));
    }

    // Step 1: Try Redis cache first
    match redis.get_depth(symbol).await {
        Ok(Some(depth)) => {
            info!(symbol = %symbol, "Returning depth from Redis cache");
            return Ok(depth);
        }
        Ok(None) => {
            // Cache miss, continue to fetch from Binance
            debug!(symbol = %symbol, "Redis cache miss for depth, fetching from Binance");
        }
        Err(e) => {
            warn!(symbol = %symbol, error = %e, "Redis error, falling back to Binance");
        }
    }

    // Step 2: Fetch from Binance REST API
    match binance.get_depth(symbol, levels).await {
        Ok(depth) => {
            // Cache the result in Redis (best effort, don't fail if Redis write fails)
            if let Err(e) = redis.set_depth(symbol, &depth).await {
                warn!(symbol = %symbol, error = %e, "Failed to cache depth in Redis");
            }
            info!(symbol = %symbol, "Returning depth from Binance API");
            Ok(depth)
        }
        Err(e) => {
            warn!(symbol = %symbol, error = %e, "Binance API failed, falling back to mock data");
            // Step 3: Fall back to mock data
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
    _db: &sea_orm::DatabaseConnection,
    _params: TickerHistoryQueryParams,
) -> Result<TickerHistoryResponse, AppError> {
    // TODO: P1 implementation
    Err(AppError::Internal(
        "market_data::get_ticker_history not implemented (P1)".into(),
    ))
}

/// 定时快照写入 (P1) — 由 Collector 调用
///
/// 每 60s 将内存 ticker_cache 中所有 symbol 的 Ticker
/// 批量 INSERT INTO ticker_snapshots
pub async fn snapshot_tickers(_db: &sea_orm::DatabaseConnection) -> Result<(), AppError> {
    // TODO: P1 implementation
    info!("snapshot_tickers called (TODO: P1 implementation)");
    Ok(())
}

/// 清理过期快照 (P1) — 每天执行
///
/// DELETE FROM ticker_snapshots WHERE created_at < NOW() - INTERVAL '90 days'
pub async fn cleanup_expired_snapshots(_db: &sea_orm::DatabaseConnection) -> Result<u64, AppError> {
    // TODO: P1 implementation
    info!("cleanup_expired_snapshots called (TODO: P1 implementation)");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
