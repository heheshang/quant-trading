//! Gate.io REST API Client
//!
//! Fetches real-time market data from Gate.io public REST API.
//!
//! # API Endpoints
//! - Ticker: GET /api/v4/spot/tickers?currency_pair={symbol}
//! - All Tickers: GET /api/v4/spot/tickers
//! - Depth: GET /api/v4/spot/order_book?currency_pair={symbol}&with_serialized=true&limit={levels}
//!
//! # Symbol Format
//! Gate.io uses plain symbol format like "BTCUSDT" (no hyphens), same as internal format.
//!
//! # Error Handling
//! All reqwest errors are converted to AppError::Internal for consistent error handling.

use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, error, info};

use crate::models::market_schemas::{DepthLevel, DepthResponse, TickerResponse};
use crate::utils::error::AppError;

/// Gate.io REST API base URL
const GATE_API_BASE: &str = "https://api.gateio.ws";

/// Gate.io REST API client for fetching market data
#[derive(Clone)]
pub struct GateRestClient {
    client: Client,
    base_url: String,
}

impl GateRestClient {
    /// Create a new Gate.io REST API client
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: GATE_API_BASE.to_string(),
        }
    }

    /// Get 24hr ticker statistics for a single symbol
    ///
    /// # Data Flow
    /// 1. GET https://api.gateio.ws/api/v4/spot/tickers?currency_pair={symbol}
    /// 2. Parse Gate response into internal TickerResponse format
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    ///
    /// # Returns
    /// * `Ok(TickerResponse)` - Normalized ticker data
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_ticker(&self, symbol: &str) -> Result<TickerResponse, AppError> {
        let url = format!("{}/api/v4/spot/tickers", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("currency_pair", symbol)])
            .send()
            .await
            .map_err(|e| {
                error!(symbol = %symbol, error = %e, "Failed to fetch ticker from Gate.io");
                AppError::Internal(format!("Gate.io API error: {}", e))
            })?;

        let gate_tickers: Vec<GateTickerItem> = response.json().await.map_err(|e| {
            error!(symbol = %symbol, error = %e, "Failed to parse Gate.io ticker response");
            AppError::Internal(format!("Failed to parse Gate.io response: {}", e))
        })?;

        // Gate returns an array, find the matching symbol
        let ticker = gate_tickers
            .into_iter()
            .find(|t| t.currency_pair == symbol)
            .ok_or_else(|| {
                error!(symbol = %symbol, "Symbol not found in Gate.io response");
                AppError::Internal(format!("Symbol {} not found in Gate.io response", symbol))
            })?;

        debug!(symbol = %symbol, price = %ticker.last, "Fetched ticker from Gate.io");
        Ok(ticker.into_ticker_response())
    }

    /// Get 24hr ticker statistics for all symbols
    ///
    /// # Data Flow
    /// 1. GET https://api.gateio.ws/api/v4/spot/tickers
    /// 2. Parse all Gate responses into internal TickerResponse format
    /// 3. Filter to only supported symbols
    ///
    /// # Returns
    /// * `Ok(Vec<TickerResponse>)` - List of normalized ticker data
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_all_tickers(&self) -> Result<Vec<TickerResponse>, AppError> {
        let url = format!("{}/api/v4/spot/tickers", self.base_url);

        let response = self.client.get(&url).send().await.map_err(|e| {
            error!(error = %e, "Failed to fetch all tickers from Gate.io");
            AppError::Internal(format!("Gate.io API error: {}", e))
        })?;

        let gate_tickers: Vec<GateTickerItem> = response.json().await.map_err(|e| {
            error!(error = %e, "Failed to parse Gate.io tickers response");
            AppError::Internal(format!("Failed to parse Gate.io response: {}", e))
        })?;

        use crate::services::market_data::SUPPORTED_SYMBOLS;

        let tickers: Vec<TickerResponse> = gate_tickers
            .into_iter()
            .filter(|t| SUPPORTED_SYMBOLS.contains(&t.currency_pair.as_str()))
            .map(|t| t.into_ticker_response())
            .collect();

        info!(count = tickers.len(), "Fetched all tickers from Gate.io");
        Ok(tickers)
    }

    /// Get order book depth data
    ///
    /// # Data Flow
    /// 1. GET https://api.gateio.ws/api/v4/spot/order_book?currency_pair={symbol}&with_serialized=true&limit={levels}
    /// 2. Parse Gate response into internal DepthResponse format
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `levels` - Number of depth levels (e.g., 25, 50, 100, 500)
    ///
    /// # Returns
    /// * `Ok(DepthResponse)` - Normalized depth data with bids/asks
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_depth(&self, symbol: &str, levels: i32) -> Result<DepthResponse, AppError> {
        let url = format!("{}/api/v4/spot/order_book", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[
                ("currency_pair", symbol),
                ("with_serialized", "true"),
                ("limit", &levels.to_string()),
            ])
            .send()
            .await
            .map_err(|e| {
                error!(symbol = %symbol, error = %e, "Failed to fetch depth from Gate.io");
                AppError::Internal(format!("Gate.io API error: {}", e))
            })?;

        let gate_depth: GateDepthResponse = response.json().await.map_err(|e| {
            error!(symbol = %symbol, error = %e, "Failed to parse Gate.io depth response");
            AppError::Internal(format!("Failed to parse Gate.io response: {}", e))
        })?;

        debug!(symbol = %symbol, bids = %gate_depth.bids.len(), asks = %gate_depth.asks.len(), "Fetched depth from Gate.io");

        Ok(gate_depth.into_depth_response())
    }
}

impl Default for GateRestClient {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Gate.io API Response Types ==========

/// Gate.io Ticker Item (from /spot/tickers endpoint)
#[derive(Debug, Deserialize)]
pub struct GateTickerItem {
    /// Currency pair, e.g., "BTCUSDT"
    #[serde(alias = "currency_pair")]
    pub currency_pair: String,
    /// Last traded price
    #[serde(alias = "last")]
    pub last: String,
    /// 24h trading volume (quote currency)
    #[serde(alias = "quote_volume")]
    pub quote_volume: String,
    /// 24h high price
    #[serde(alias = "high_24h")]
    pub high_24h: String,
    /// 24h low price
    #[serde(alias = "low_24h")]
    pub low_24h: String,
    /// Open price at UTC 00:00
    #[serde(alias = "open_24h")]
    pub open_24h: String,
    /// 24h price change percentage (string like "2.5" for 2.5%)
    #[serde(alias = "change_24h_percent")]
    pub change_24h_percent: String,
    /// Best bid price (if available)
    #[serde(alias = "bid")]
    pub bid: Option<String>,
    /// Best ask price (if available)
    #[serde(alias = "ask")]
    pub ask: Option<String>,
}

impl GateTickerItem {
    fn into_ticker_response(self) -> TickerResponse {
        let price: f64 = self.last.parse().unwrap_or(0.0);
        let open_price: f64 = self.open_24h.parse().unwrap_or(price);
        let change: f64 = price - open_price;
        let change_percent: f64 = self.change_24h_percent.parse().unwrap_or(0.0);
        let volume: f64 = self.quote_volume.parse().unwrap_or(0.0);
        let high: f64 = self.high_24h.parse().unwrap_or(0.0);
        let low: f64 = self.low_24h.parse().unwrap_or(0.0);
        let bid: f64 = self.bid.and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let ask: f64 = self.ask.and_then(|s| s.parse().ok()).unwrap_or(0.0);

        TickerResponse {
            symbol: self.currency_pair,
            price,
            change,
            change_percent,
            volume,
            high,
            low,
            bid,
            ask,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Gate.io Depth Response (from /spot/order_book endpoint)
#[derive(Debug, Deserialize)]
pub struct GateDepthResponse {
    /// Currency pair
    #[serde(alias = "currency_pair")]
    pub currency_pair: String,
    /// Asks: [[price, quantity], ...]
    pub asks: Vec<Vec<String>>,
    /// Bids: [[price, quantity], ...]
    pub bids: Vec<Vec<String>>,
    /// Last update timestamp (UTC milliseconds)
    #[serde(alias = "ts")]
    pub ts: Option<String>,
}

impl GateDepthResponse {
    fn into_depth_response(self) -> DepthResponse {
        // Calculate cumulative totals for bids (descending price order)
        let mut bid_total = 0.0;
        let bids: Vec<DepthLevel> = self
            .bids
            .into_iter()
            .take(500)
            .map(|level| {
                let price: f64 = level[0].parse().unwrap_or(0.0);
                let qty: f64 = level[1].parse().unwrap_or(0.0);
                bid_total += qty;
                DepthLevel {
                    price,
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
            .take(500)
            .map(|level| {
                let price: f64 = level[0].parse().unwrap_or(0.0);
                let qty: f64 = level[1].parse().unwrap_or(0.0);
                ask_total += qty;
                DepthLevel {
                    price,
                    quantity: qty,
                    total: ask_total,
                }
            })
            .collect();

        let timestamp: i64 = self
            .ts
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        DepthResponse {
            bids,
            asks,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_ticker_item_to_response() {
        let item = GateTickerItem {
            currency_pair: "BTCUSDT".to_string(),
            last: "50000".to_string(),
            quote_volume: "1000000".to_string(),
            high_24h: "51000".to_string(),
            low_24h: "49000".to_string(),
            open_24h: "49000".to_string(),
            change_24h_percent: "2.04".to_string(),
            bid: Some("49999".to_string()),
            ask: Some("50001".to_string()),
        };

        let response = item.into_ticker_response();
        assert_eq!(response.symbol, "BTCUSDT");
        assert_eq!(response.price, 50000.0);
        assert_eq!(response.change, 1000.0);
        assert_eq!(response.change_percent, 2.04);
        assert_eq!(response.volume, 1000000.0);
        assert_eq!(response.high, 51000.0);
        assert_eq!(response.low, 49000.0);
    }

    #[test]
    fn test_gate_depth_response_to_response() {
        let depth = GateDepthResponse {
            currency_pair: "BTCUSDT".to_string(),
            asks: vec![
                vec!["50001".to_string(), "1.5".to_string()],
                vec!["50002".to_string(), "2.0".to_string()],
            ],
            bids: vec![
                vec!["49999".to_string(), "1.0".to_string()],
                vec!["49998".to_string(), "1.5".to_string()],
            ],
            ts: Some("1715500000000".to_string()),
        };

        let response = depth.into_depth_response();
        assert_eq!(response.bids.len(), 2);
        assert_eq!(response.asks.len(), 2);
        assert_eq!(response.timestamp, 1715500000000);
        // Check cumulative totals
        assert_eq!(response.bids[0].total, 1.0);
        assert_eq!(response.bids[1].total, 2.5);
    }
}
