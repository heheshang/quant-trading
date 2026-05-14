use crate::models::schemas::{
    DepthLevel, DepthResponse, TickerHistoryQueryParams, TickerHistoryResponse, TickerResponse,
};
use crate::utils::error::AppError;
use tracing::{info, warn};

/// Supported trading symbols
const SUPPORTED_SYMBOLS: &[&str] = &[
    "BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "XRPUSDT", "DOGEUSDT",
    "ADAUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT",
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
fn build_ticker(symbol: &str) -> Option<TickerResponse> {
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

/// 获取所有交易对 Ticker
///
/// 数据获取策略 (当前使用 mock 数据，Redis 缓存为 P1):
/// 1. Redis `tickers:all` 缓存命中 → 直接返回
/// 2. Redis 未命中 → SCAN `ticker:*` keys → 重建 `tickers:all` 缓存
/// 3. Redis 不可用 → Mock 数据兜底 (当前实现)
pub async fn get_all_tickers(
    _db: &sea_orm::DatabaseConnection,
) -> Result<Vec<TickerResponse>, AppError> {
    // TODO: P1 - Implement Redis-first strategy
    // For now, return mock data for all supported symbols
    let tickers: Vec<TickerResponse> = SUPPORTED_SYMBOLS
        .iter()
        .filter_map(|s| build_ticker(s))
        .collect();

    info!(count = tickers.len(), "Returning mock tickers");
    Ok(tickers)
}

/// 获取单个交易对 Ticker
///
/// 数据获取策略 (当前使用 mock 数据，Redis 缓存为 P1):
/// 1. Redis `ticker:{symbol}` HASH 命中 → 反序列化返回
/// 2. Redis 未命中 → MarketCollector 内存缓存
/// 3. 均不可用 → 404 "交易对不存在" 或 503 "采集器离线"
pub async fn get_ticker_by_symbol(
    _db: &sea_orm::DatabaseConnection,
    symbol: &str,
) -> Result<TickerResponse, AppError> {
    // TODO: P1 - Implement Redis-first strategy
    match build_ticker(symbol) {
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

/// 获取深度数据
///
/// 数据获取策略 (当前使用 mock 数据，Redis 缓存为 P1):
/// 1. Redis `depth:{symbol}` STRING(JSON) 命中 → 解析截取 levels 档返回
/// 2. Redis 未命中 → MarketCollector 内存缓存
/// 3. 均不可用 → 503
pub async fn get_depth(
    _db: &sea_orm::DatabaseConnection,
    symbol: &str,
    levels: i32,
) -> Result<DepthResponse, AppError> {
    // Check if symbol is supported
    if !SUPPORTED_SYMBOLS.contains(&symbol) {
        warn!(symbol = %symbol, "Depth request for unsupported symbol");
        return Err(AppError::NotFound(format!("交易对 {} 不存在", symbol)));
    }

    // TODO: P1 - Implement Redis-first strategy
    let base_price = get_mock_ticker_base(symbol)
        .map(|(p, _, _, _)| p)
        .ok_or_else(|| AppError::NotFound(format!("交易对 {} 不存在", symbol)))?;

    let levels_usize = levels as usize;
    let timestamp = chrono::Utc::now().timestamp_millis();

    // Generate mock bids (descending price from base)
    let depth_spread = base_price * 0.001; // 0.1% spread
    let bids: Vec<DepthLevel> = (0..levels_usize)
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
    let asks: Vec<DepthLevel> = (0..levels_usize)
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

    info!(symbol = %symbol, levels = levels, "Returning mock depth");
    Ok(DepthResponse {
        bids,
        asks,
        timestamp,
    })
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
pub async fn snapshot_tickers(
    _db: &sea_orm::DatabaseConnection,
) -> Result<(), AppError> {
    // TODO: P1 implementation
    info!("snapshot_tickers called (TODO: P1 implementation)");
    Ok(())
}

/// 清理过期快照 (P1) — 每天执行
///
/// DELETE FROM ticker_snapshots WHERE created_at < NOW() - INTERVAL '90 days'
pub async fn cleanup_expired_snapshots(
    _db: &sea_orm::DatabaseConnection,
) -> Result<u64, AppError> {
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

    #[rstest::rstest]
    #[tokio::test]
    async fn test_get_all_tickers_returns_list() {
        let db = get_test_db();
        let result = get_all_tickers(&db).await;
        assert!(result.is_ok());
        let tickers = result.unwrap();
        assert_eq!(tickers.len(), SUPPORTED_SYMBOLS.len());
        // Verify all symbols are present
        let symbols: Vec<&str> = tickers.iter().map(|t| t.symbol.as_str()).collect();
        for s in SUPPORTED_SYMBOLS {
            assert!(symbols.contains(s), "Missing symbol: {}", s);
        }
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_get_ticker_by_symbol_found() {
        let db = get_test_db();
        let result = get_ticker_by_symbol(&db, "BTCUSDT").await;
        assert!(result.is_ok());
        let ticker = result.unwrap();
        assert_eq!(ticker.symbol, "BTCUSDT");
        assert!(ticker.price > 0.0);
        assert!(ticker.volume > 0.0);
        assert!(ticker.bid < ticker.ask);
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_get_ticker_by_symbol_not_found() {
        let db = get_test_db();
        let result = get_ticker_by_symbol(&db, "INVALID99").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_get_ticker_each_symbol() {
        let db = get_test_db();
        for symbol in SUPPORTED_SYMBOLS {
            let result = get_ticker_by_symbol(&db, symbol).await;
            assert!(result.is_ok(), "Failed for symbol: {}", symbol);
            let ticker = result.unwrap();
            assert_eq!(ticker.symbol, *symbol);
            assert!(ticker.price > 0.0);
        }
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_get_depth_default_levels() {
        let db = get_test_db();
        let result = get_depth(&db, "BTCUSDT", 10).await;
        assert!(result.is_ok());
        let depth = result.unwrap();
        assert_eq!(depth.bids.len(), 10);
        assert_eq!(depth.asks.len(), 10);
        assert!(depth.timestamp > 0);

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
    async fn test_get_depth_levels_param() {
        let db = get_test_db();

        for level in [5, 10, 20, 50] {
            let result = get_depth(&db, "ETHUSDT", level).await;
            assert!(result.is_ok(), "Failed for level: {}", level);
            let depth = result.unwrap();
            assert_eq!(depth.bids.len(), level as usize);
            assert_eq!(depth.asks.len(), level as usize);
        }
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_get_depth_symbol_not_found() {
        let db = get_test_db();
        let result = get_depth(&db, "FOOBAR", 10).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_get_depth_has_total_field() {
        let db = get_test_db();
        let result = get_depth(&db, "BTCUSDT", 5).await;
        assert!(result.is_ok());
        let depth = result.unwrap();
        // Verify cumulative total increases
        for i in 1..depth.bids.len() {
            assert!(
                depth.bids[i].total >= depth.bids[i - 1].total,
                "Cumulative total not increasing at bid index {}",
                i
            );
        }
        for i in 1..depth.asks.len() {
            assert!(
                depth.asks[i].total >= depth.asks[i - 1].total,
                "Cumulative total not increasing at ask index {}",
                i
            );
        }
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_get_ticker_history_not_implemented() {
        let db = get_test_db();
        let params = TickerHistoryQueryParams {
            symbol: "BTCUSDT".to_string(),
            start: 0,
            end: 0,
            page: None,
            page_size: None,
        };
        let result = get_ticker_history(&db, params).await;
        assert!(result.is_err());
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_snapshot_tickers_ok() {
        let db = get_test_db();
        let result = snapshot_tickers(&db).await;
        assert!(result.is_ok());
    }

    #[rstest::rstest]
    #[tokio::test]
    async fn test_cleanup_expired_snapshots_ok() {
        let db = get_test_db();
        let result = cleanup_expired_snapshots(&db).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
