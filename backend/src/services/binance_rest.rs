//! Binance REST API Client
//!
//! Fetches real-time market data from Binance public REST API.
//!
//! # API Endpoints
//! - Ticker: GET /api/v3/ticker/24hr?symbol={symbol}
//! - All Tickers: GET /api/v3/ticker/24hr
//! - Depth: GET /api/v3/depth?symbol={symbol}&limit={levels}
//!
//! # Error Handling
//! All reqwest errors are converted to AppError::Internal for consistent error handling.

use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, error, info};

use crate::models::schemas::{DepthLevel, DepthResponse, TickerResponse};
use crate::utils::error::AppError;

/// Binance REST API base URL
const BINANCE_API_BASE: &str = "https://api.binance.com";

/// Binance REST API client for fetching market data
#[derive(Clone)]
pub struct BinanceRestClient {
    client: Client,
    base_url: String,
}

impl BinanceRestClient {
    /// Create a new Binance REST API client
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: BINANCE_API_BASE.to_string(),
        }
    }

    /// Get 24hr ticker statistics for a single symbol
    ///
    /// # Data Flow
    /// 1. GET https://api.binance.com/api/v3/ticker/24hr?symbol={symbol}
    /// 2. Parse Binance response into internal TickerResponse format
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    ///
    /// # Returns
    /// * `Ok(TickerResponse)` - Normalized ticker data
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_ticker(&self, symbol: &str) -> Result<TickerResponse, AppError> {
        let url = format!("{}/api/v3/ticker/24hr", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("symbol", symbol)])
            .send()
            .await
            .map_err(|e| {
                error!(symbol = %symbol, error = %e, "Failed to fetch ticker from Binance");
                AppError::Internal(format!("Binance API error: {}", e))
            })?;

        let binance_ticker: BinanceTicker24hr = response.json().await.map_err(|e| {
            error!(symbol = %symbol, error = %e, "Failed to parse Binance ticker response");
            AppError::Internal(format!("Failed to parse Binance response: {}", e))
        })?;

        debug!(symbol = %symbol, price = %binance_ticker.last_price, "Fetched ticker from Binance");
        Ok(binance_ticker.into_ticker_response())
    }

    /// Get 24hr ticker statistics for all symbols
    ///
    /// # Data Flow
    /// 1. GET https://api.binance.com/api/v3/ticker/24hr
    /// 2. Parse all Binance responses into internal TickerResponse format
    /// 3. Filter to only supported symbols
    ///
    /// # Returns
    /// * `Ok(Vec<TickerResponse>)` - List of normalized ticker data
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_all_tickers(&self) -> Result<Vec<TickerResponse>, AppError> {
        let url = format!("{}/api/v3/ticker/24hr", self.base_url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            error!(error = %e, "Failed to fetch all tickers from Binance");
            AppError::Internal(format!("Binance API error: {}", e))
        })?;

        let binance_tickers: Vec<BinanceTicker24hr> = response.json().await.map_err(|e| {
            error!(error = %e, "Failed to parse Binance tickers response");
            AppError::Internal(format!("Failed to parse Binance response: {}", e))
        })?;

        // Supported symbols to filter
        let supported: std::collections::HashSet<&str> = [
            "BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "XRPUSDT", "DOGEUSDT", "ADAUSDT",
            "AVAXUSDT", "DOTUSDT", "LINKUSDT",
        ]
        .into();

        let tickers: Vec<TickerResponse> = binance_tickers
            .into_iter()
            .filter(|t| supported.contains(t.symbol.as_str()))
            .map(|t| t.into_ticker_response())
            .collect();

        info!(count = tickers.len(), "Fetched all tickers from Binance");
        Ok(tickers)
    }

    /// Get order book depth data
    ///
    /// # Data Flow
    /// 1. GET https://api.binance.com/api/v3/depth?symbol={symbol}&limit={levels}
    /// 2. Parse Binance response into internal DepthResponse format
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `levels` - Number of depth levels (e.g., 20, 50, 100, 500, 1000)
    ///
    /// # Returns
    /// * `Ok(DepthResponse)` - Normalized depth data with bids/asks
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_depth(&self, symbol: &str, levels: i32) -> Result<DepthResponse, AppError> {
        let url = format!("{}/api/v3/depth", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("symbol", symbol), ("limit", &levels.to_string())])
            .send()
            .await
            .map_err(|e| {
                error!(symbol = %symbol, error = %e, "Failed to fetch depth from Binance");
                AppError::Internal(format!("Binance API error: {}", e))
            })?;

        let binance_depth: BinanceDepth = response.json().await.map_err(|e| {
            error!(symbol = %symbol, error = %e, "Failed to parse Binance depth response");
            AppError::Internal(format!("Failed to parse Binance response: {}", e))
        })?;

        debug!(symbol = %symbol, bids = %binance_depth.bids.len(), asks = %binance_depth.asks.len(), "Fetched depth from Binance");
        Ok(binance_depth.into_depth_response())
    }
}

impl Default for BinanceRestClient {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Binance API Response Types ==========

/// Binance 24hr Ticker Response
/// See: https://developers.binance.com/docs/derivatives/coin-margined-futures/market-data/GetTicker
#[derive(Debug, Deserialize)]
struct BinanceTicker24hr {
    #[serde(alias = "symbol")]
    symbol: String,
    #[serde(alias = "lastPrice")]
    last_price: String,
    #[serde(alias = "priceChange")]
    price_change: String,
    #[serde(alias = "priceChangePercent")]
    price_change_percent: String,
    #[serde(alias = "volume")]
    volume: String,
    #[serde(alias = "highPrice")]
    high_price: String,
    #[serde(alias = "lowPrice")]
    low_price: String,
    #[serde(alias = "bidPrice")]
    bid_price: String,
    #[serde(alias = "askPrice")]
    ask_price: String,
    #[serde(alias = "openTime")]
    open_time: i64,
    #[serde(alias = "closeTime")]
    close_time: i64,
}

impl BinanceTicker24hr {
    /// Convert Binance response to internal TickerResponse format
    fn into_ticker_response(self) -> TickerResponse {
        let price: f64 = self.last_price.parse().unwrap_or(0.0);
        let change: f64 = self.price_change.parse().unwrap_or(0.0);
        let change_percent: f64 = self.price_change_percent.parse().unwrap_or(0.0);
        let volume: f64 = self.volume.parse().unwrap_or(0.0);
        let high: f64 = self.high_price.parse().unwrap_or(0.0);
        let low: f64 = self.low_price.parse().unwrap_or(0.0);
        let bid: f64 = self.bid_price.parse().unwrap_or(0.0);
        let ask: f64 = self.ask_price.parse().unwrap_or(0.0);

        // Use closeTime as timestamp (milliseconds)
        let timestamp = self.close_time;

        TickerResponse {
            symbol: self.symbol,
            price,
            change,
            change_percent,
            volume,
            high,
            low,
            bid,
            ask,
            timestamp,
        }
    }
}

/// Binance Order Book Depth Response
/// See: https://developers.binance.com/docs/derivatives/coin-margined-futures/market-data/OrderBook
#[derive(Debug, Deserialize)]
struct BinanceDepth {
    #[serde(alias = "lastUpdateId")]
    last_update_id: i64,
    #[serde(alias = "bids")]
    bids: Vec<BinanceDepthLevel>,
    #[serde(alias = "asks")]
    asks: Vec<BinanceDepthLevel>,
}

#[derive(Debug, Deserialize)]
struct BinanceDepthLevel {
    #[serde(alias = "price")]
    price: String,
    #[serde(alias = "qty")]
    quantity: String,
}

impl BinanceDepth {
    /// Convert Binance response to internal DepthResponse format
    fn into_depth_response(self) -> DepthResponse {
        // Calculate cumulative totals for bids (descending price order)
        let mut bid_total = 0.0;
        let bids: Vec<DepthLevel> = self
            .bids
            .into_iter()
            .map(|level| {
                let qty: f64 = level.quantity.parse().unwrap_or(0.0);
                bid_total += qty;
                DepthLevel {
                    price: level.price.parse().unwrap_or(0.0),
                    quantity: qty,
                    total: bid_total,
                }
            })
            .collect();

        // Calculate cumulative totals for asks (ascending price order)
        let mut ask_total = 0.0;
        let asks: Vec<DepthLevel> = self
            .asks
            .into_iter()
            .map(|level| {
                let qty: f64 = level.quantity.parse().unwrap_or(0.0);
                ask_total += qty;
                DepthLevel {
                    price: level.price.parse().unwrap_or(0.0),
                    quantity: qty,
                    total: ask_total,
                }
            })
            .collect();

        DepthResponse {
            bids,
            asks,
            timestamp: self.last_update_id,
        }
    }
}
