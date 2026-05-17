//! Application shared state for all request handlers
//!
//! This module defines the `AppState` struct that holds all shared resources
//! accessible by route handlers via Axum's `State` extractor.

use std::sync::Arc;

use crate::db::DbPool;
use crate::services::binance_rest::BinanceRestClient;
use crate::services::redis_cache::RedisCache;

/// Application shared state
///
/// Contains all resources needed by request handlers:
/// - `db`: PostgreSQL database connection pool
/// - `redis`: Redis cache for market data
/// - `binance`: Binance REST API client
#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub redis: Arc<RedisCache>,
    pub binance: Arc<BinanceRestClient>,
}

impl AppState {
    /// Create a new AppState instance
    pub fn new(db: DbPool, redis: RedisCache, binance: BinanceRestClient) -> Self {
        Self {
            db,
            redis: Arc::new(redis),
            binance: Arc::new(binance),
        }
    }
}