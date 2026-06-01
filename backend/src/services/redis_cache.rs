//! Redis Cache Layer for Market Data
//!
//! Data flow:
//! 1. `get_ticker` / `set_ticker` → HASH `ticker:{symbol}`
//! 2. `get_all_tickers` / `set_tickers_cache` → STRING `tickers:all` (TTL=5s)
//! 3. `get_depth` / `set_depth` → STRING `depth:{symbol}` (TTL=2s)
//!
//! Degradation strategy: Redis connection failure returns Ok(None), allowing
//! callers to fall back to Binance REST API or mock data.

use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use tracing::{debug, warn};

use crate::models::schemas::{DepthResponse, TickerResponse};
use crate::utils::error::AppError;

/// Redis Key prefixes
const TICKER_KEY_PREFIX: &str = "ticker:";
const ALL_TICKERS_KEY: &str = "tickers:all";
const DEPTH_KEY_PREFIX: &str = "depth:";

/// TTL settings (in seconds)
const ALL_TICKERS_TTL: u64 = 5;
const DEPTH_TTL: u64 = 2;

/// Redis cache wrapper for market data
#[derive(Clone)]
pub struct RedisCache {
    conn: ConnectionManager,
}

impl RedisCache {
    /// Create a new RedisCache instance
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL (e.g., "redis://127.0.0.1:6379")
    ///
    /// # Returns
    /// * `Ok(Self)` - Successfully created cache instance
    /// * `Err(AppError::Internal)` - Failed to connect to Redis
    pub async fn new(redis_url: &str) -> Result<Self, AppError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| AppError::Internal(format!("Failed to create Redis client: {}", e)))?;

        let conn = ConnectionManager::new(client)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to connect to Redis: {}", e)))?;

        debug!("Redis cache connected successfully");
        Ok(Self { conn })
    }

    // ========== Ticker Operations ==========

    /// Get a single ticker from Redis HASH
    ///
    /// # Data Flow
    /// 1. Get all fields from `ticker:{symbol}` HASH
    /// 2. Deserialize into TickerResponse
    /// 3. Return None if key doesn't exist or error occurs
    pub async fn get_ticker(&self, symbol: &str) -> Result<Option<TickerResponse>, AppError> {
        let key = format!("{}{}", TICKER_KEY_PREFIX, symbol);
        let mut conn = self.conn.clone();

        let result: Result<std::collections::HashMap<String, String>, _> = conn.hgetall(&key).await;

        match result {
            Ok(fields) if !fields.is_empty() => match deserialize_ticker_from_hash(&fields) {
                Some(ticker) => {
                    debug!(symbol = %symbol, "Redis cache hit for ticker");
                    Ok(Some(ticker))
                }
                None => {
                    warn!(symbol = %symbol, "Redis cache hit but failed to deserialize ticker");
                    Ok(None)
                }
            },
            Ok(_) => {
                debug!(symbol = %symbol, "Redis cache miss for ticker");
                Ok(None)
            }
            Err(e) => {
                warn!(symbol = %symbol, error = %e, "Redis error getting ticker, degrading gracefully");
                Ok(None)
            }
        }
    }

    /// Store a single ticker into Redis HASH
    ///
    /// # Data Flow
    /// 1. Serialize TickerResponse into field-value pairs
    /// 2. HSET each field into `ticker:{symbol}` HASH
    pub async fn set_ticker(&self, ticker: &TickerResponse) -> Result<(), AppError> {
        let key = format!("{}{}", TICKER_KEY_PREFIX, ticker.symbol);
        let mut conn = self.conn.clone();

        // Use hset_multiple to set all fields at once (HMSET)
        let fields: Vec<(&str, String)> = vec![
            ("symbol", ticker.symbol.clone()),
            ("price", ticker.price.to_string()),
            ("change", ticker.change.to_string()),
            ("change_percent", ticker.change_percent.to_string()),
            ("volume", ticker.volume.to_string()),
            ("high", ticker.high.to_string()),
            ("low", ticker.low.to_string()),
            ("bid", ticker.bid.to_string()),
            ("ask", ticker.ask.to_string()),
            ("timestamp", ticker.timestamp.to_string()),
        ];

        let result: Result<(), _> = conn.hset_multiple(&key, &fields).await;

        match result {
            Ok(_) => {
                debug!(symbol = %ticker.symbol, "Cached ticker to Redis");
                Ok(())
            }
            Err(e) => {
                warn!(symbol = %ticker.symbol, error = %e, "Redis error setting ticker, degrading gracefully");
                Ok(())
            }
        }
    }

    /// Get all tickers from Redis STRING (JSON)
    ///
    /// # Data Flow
    /// 1. GET `tickers:all` STRING
    /// 2. Deserialize JSON into Vec<TickerResponse>
    /// 3. Return None if key doesn't exist or error occurs
    pub async fn get_all_tickers(&self) -> Result<Option<Vec<TickerResponse>>, AppError> {
        let mut conn = self.conn.clone();

        let result: Result<Option<String>, _> = conn.get(ALL_TICKERS_KEY).await;

        match result {
            Ok(Some(json)) => match serde_json::from_str::<Vec<TickerResponse>>(&json) {
                Ok(tickers) => {
                    debug!(count = tickers.len(), "Redis cache hit for all tickers");
                    Ok(Some(tickers))
                }
                Err(e) => {
                    warn!(error = %e, "Redis cache hit but failed to deserialize tickers");
                    Ok(None)
                }
            },
            Ok(None) => {
                debug!("Redis cache miss for all tickers");
                Ok(None)
            }
            Err(e) => {
                warn!(error = %e, "Redis error getting all tickers, degrading gracefully");
                Ok(None)
            }
        }
    }

    /// Store all tickers to Redis STRING (JSON) with TTL
    ///
    /// # Data Flow
    /// 1. Serialize Vec<TickerResponse> to JSON
    /// 2. SET `tickers:all` with 5 second TTL
    pub async fn set_tickers_cache(&self, tickers: &[TickerResponse]) -> Result<(), AppError> {
        let mut conn = self.conn.clone();

        let json = serde_json::to_string(tickers)
            .map_err(|e| AppError::Internal(format!("Failed to serialize tickers: {}", e)))?;

        let result: Result<(), _> = conn.set_ex(ALL_TICKERS_KEY, json, ALL_TICKERS_TTL).await;

        match result {
            Ok(_) => {
                debug!(
                    count = tickers.len(),
                    ttl = ALL_TICKERS_TTL,
                    "Cached all tickers to Redis"
                );
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "Redis error setting all tickers, degrading gracefully");
                Ok(())
            }
        }
    }

    // ========== Depth Operations ==========

    /// Get depth data from Redis STRING (JSON)
    ///
    /// # Data Flow
    /// 1. GET `depth:{symbol}` STRING
    /// 2. Deserialize JSON into DepthResponse
    /// 3. Return None if key doesn't exist or error occurs
    pub async fn get_depth(&self, symbol: &str) -> Result<Option<DepthResponse>, AppError> {
        let key = format!("{}{}", DEPTH_KEY_PREFIX, symbol);
        let mut conn = self.conn.clone();

        let result: Result<Option<String>, _> = conn.get(&key).await;

        match result {
            Ok(Some(json)) => match serde_json::from_str::<DepthResponse>(&json) {
                Ok(depth) => {
                    debug!(symbol = %symbol, "Redis cache hit for depth");
                    Ok(Some(depth))
                }
                Err(e) => {
                    warn!(symbol = %symbol, error = %e, "Redis cache hit but failed to deserialize depth");
                    Ok(None)
                }
            },
            Ok(None) => {
                debug!(symbol = %symbol, "Redis cache miss for depth");
                Ok(None)
            }
            Err(e) => {
                warn!(symbol = %symbol, error = %e, "Redis error getting depth, degrading gracefully");
                Ok(None)
            }
        }
    }

    /// Store depth data to Redis STRING (JSON) with TTL
    ///
    /// # Data Flow
    /// 1. Serialize DepthResponse to JSON
    /// 2. SET `depth:{symbol}` with 2 second TTL
    pub async fn set_depth(&self, symbol: &str, depth: &DepthResponse) -> Result<(), AppError> {
        let key = format!("{}{}", DEPTH_KEY_PREFIX, symbol);
        let mut conn = self.conn.clone();

        let json = serde_json::to_string(depth)
            .map_err(|e| AppError::Internal(format!("Failed to serialize depth: {}", e)))?;

        let result: Result<(), _> = conn.set_ex(&key, json, DEPTH_TTL).await;

        match result {
            Ok(_) => {
                debug!(symbol = %symbol, ttl = DEPTH_TTL, "Cached depth to Redis");
                Ok(())
            }
            Err(e) => {
                warn!(symbol = %symbol, error = %e, "Redis error setting depth, degrading gracefully");
                Ok(())
            }
        }
    }
}

// ========== Deserialization Helper ==========

/// Deserialize HashMap from Redis HASH into TickerResponse
fn deserialize_ticker_from_hash(
    fields: &std::collections::HashMap<String, String>,
) -> Option<TickerResponse> {
    Some(TickerResponse {
        symbol: fields.get("symbol")?.clone(),
        price: fields.get("price")?.parse().ok()?,
        change: fields.get("change")?.parse().ok()?,
        change_percent: fields.get("change_percent")?.parse().ok()?,
        volume: fields.get("volume")?.parse().ok()?,
        high: fields.get("high")?.parse().ok()?,
        low: fields.get("low")?.parse().ok()?,
        bid: fields.get("bid")?.parse().ok()?,
        ask: fields.get("ask")?.parse().ok()?,
        timestamp: fields.get("timestamp")?.parse().ok()?,
    })
}

impl RedisCache {
    /// Borrow the underlying `ConnectionManager` for direct commands
    /// (e.g. `PING` for the readiness probe).
    pub fn conn(&self) -> ConnectionManager {
        self.conn.clone()
    }
}
