//! Bybit REST API Client
//!
//! Fetches real-time market data from Bybit public REST API.
//!
//! # API Endpoints
//! - Ticker: GET /v5/market/tickers?category=spot&symbol={symbol}
//! - All Tickers: GET /v5/market/tickers?category=spot
//! - Depth: GET /v5/market/orderbook?category=spot&symbol={symbol}&limit={levels}
//!
//! # Symbol Format
//! Bybit uses the same format as internal symbols: "BTCUSDT" (no hyphen)
//!
//! # Error Handling
//! All reqwest errors are converted to AppError::Internal for consistent error handling.

use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, error, info};

use crate::models::market_schemas::{DepthLevel, DepthResponse, TickerResponse};
use crate::services::market_data::SUPPORTED_SYMBOLS;
use crate::utils::error::AppError;

/// Bybit REST API base URL
const BYBIT_API_BASE: &str = "https://api.bybit.com";

/// Bybit REST API client for fetching market data
#[derive(Clone)]
pub struct BybitRestClient {
    client: Client,
    base_url: String,
}

impl BybitRestClient {
    /// Create a new Bybit REST API client
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: BYBIT_API_BASE.to_string(),
        }
    }

    /// Get 24hr ticker statistics for a single symbol
    ///
    /// # Data Flow
    /// 1. GET https://api.bybit.com/v5/market/tickers?category=spot&symbol={symbol}
    /// 2. Parse Bybit response into internal TickerResponse format
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    ///
    /// # Returns
    /// * `Ok(TickerResponse)` - Normalized ticker data
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_ticker(&self, symbol: &str) -> Result<TickerResponse, AppError> {
        let url = format!("{}/v5/market/tickers", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("category", "spot"), ("symbol", symbol)])
            .send()
            .await
            .map_err(|e| {
                error!(symbol = %symbol, error = %e, "Failed to fetch ticker from Bybit");
                AppError::Internal(format!("Bybit API error: {}", e))
            })?;

        let bybit_ticker: BybitTickersResponse = response.json().await.map_err(|e| {
            error!(symbol = %symbol, error = %e, "Failed to parse Bybit ticker response");
            AppError::Internal(format!("Failed to parse Bybit response: {}", e))
        })?;

        // Find the ticker for our symbol in the list
        let ticker_data = bybit_ticker
            .result
            .list
            .into_iter()
            .find(|t| t.symbol == symbol)
            .ok_or_else(|| {
                error!(symbol = %symbol, "Symbol not found in Bybit response");
                AppError::Internal(format!("Symbol {} not found in Bybit response", symbol))
            })?;

        debug!(symbol = %symbol, price = %ticker_data.last_price, "Fetched ticker from Bybit");
        Ok(ticker_data.into_ticker_response())
    }

    /// Get 24hr ticker statistics for all symbols
    ///
    /// # Data Flow
    /// 1. GET https://api.bybit.com/v5/market/tickers?category=spot
    /// 2. Parse all Bybit responses into internal TickerResponse format
    /// 3. Filter to only supported symbols
    ///
    /// # Returns
    /// * `Ok(Vec<TickerResponse>)` - List of normalized ticker data
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_all_tickers(&self) -> Result<Vec<TickerResponse>, AppError> {
        let url = format!("{}/v5/market/tickers", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("category", "spot")])
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch all tickers from Bybit");
                AppError::Internal(format!("Bybit API error: {}", e))
            })?;

        let bybit_data: BybitTickersResponse = response.json().await.map_err(|e| {
            error!(error = %e, "Failed to parse Bybit tickers response");
            AppError::Internal(format!("Failed to parse Bybit response: {}", e))
        })?;

        let tickers: Vec<TickerResponse> = bybit_data
            .result
            .list
            .into_iter()
            .filter(|t| SUPPORTED_SYMBOLS.contains(&t.symbol.as_str()))
            .map(|t| t.into_ticker_response())
            .collect();

        info!(count = tickers.len(), "Fetched all tickers from Bybit");
        Ok(tickers)
    }

    /// Get order book depth data
    ///
    /// # Data Flow
    /// 1. GET https://api.bybit.com/v5/market/orderbook?category=spot&symbol={symbol}&limit={levels}
    /// 2. Parse Bybit response into internal DepthResponse format
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `levels` - Number of depth levels (e.g., 5, 10, 20, 50, 100, 500)
    ///
    /// # Returns
    /// * `Ok(DepthResponse)` - Normalized depth data with bids/asks
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_depth(&self, symbol: &str, levels: i32) -> Result<DepthResponse, AppError> {
        let url = format!("{}/v5/market/orderbook", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[
                ("category", "spot"),
                ("symbol", symbol),
                ("limit", &levels.to_string()),
            ])
            .send()
            .await
            .map_err(|e| {
                error!(symbol = %symbol, error = %e, "Failed to fetch depth from Bybit");
                AppError::Internal(format!("Bybit API error: {}", e))
            })?;

        let bybit_depth: BybitDepthResponse = response.json().await.map_err(|e| {
            error!(symbol = %symbol, error = %e, "Failed to parse Bybit depth response");
            AppError::Internal(format!("Failed to parse Bybit response: {}", e))
        })?;

        debug!(symbol = %symbol, bids = %bybit_depth.result.bid.len(),
              asks = %bybit_depth.result.ask.len(), "Fetched depth from Bybit");

        let depth = bybit_depth.result.into_depth_response()?;
        Ok(depth)
    }
}

impl Default for BybitRestClient {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Bybit API Response Types ==========

/// Bybit API Response wrapper (common structure)
#[derive(Debug, Deserialize)]
pub struct BybitApiResponse<T> {
    #[serde(rename = "retCode")]
    pub ret_code: i32,
    #[serde(rename = "retMsg")]
    pub ret_msg: String,
    pub result: T,
}

/// Bybit Tickers Response
#[derive(Debug, Deserialize)]
pub struct BybitTickersResponse {
    pub result: BybitTickersResult,
}

#[derive(Debug, Deserialize)]
pub struct BybitTickersResult {
    pub list: Vec<BybitTickerItem>,
}

#[derive(Debug, Deserialize)]
pub struct BybitTickerItem {
    pub symbol: String,
    #[serde(rename = "lastPrice")]
    pub last_price: String,
    #[serde(rename = "price24hPcnt")]
    pub price_24h_pcnt: String,
    #[serde(rename = "volume24h")]
    pub volume_24h: String,
    #[serde(rename = "highPrice24h")]
    pub high_price_24h: String,
    #[serde(rename = "lowPrice24h")]
    pub low_price_24h: String,
    #[serde(rename = "bid1Price")]
    pub bid1_price: Option<String>,
    #[serde(rename = "ask1Price")]
    pub ask1_price: Option<String>,
}

impl BybitTickerItem {
    fn into_ticker_response(self) -> TickerResponse {
        let price: f64 = self.last_price.parse().unwrap_or(0.0);
        let change_percent: f64 = self.price_24h_pcnt.parse().unwrap_or(0.0) * 100.0;
        let change = price * change_percent / 100.0;
        let volume: f64 = self.volume_24h.parse().unwrap_or(0.0);
        let high: f64 = self.high_price_24h.parse().unwrap_or(0.0);
        let low: f64 = self.low_price_24h.parse().unwrap_or(0.0);
        let bid: f64 = self
            .bid1_price
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let ask: f64 = self
            .ask1_price
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

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
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Bybit Depth Response
#[derive(Debug, Deserialize)]
pub struct BybitDepthResponse {
    pub result: BybitDepthResult,
}

#[derive(Debug, Deserialize)]
pub struct BybitDepthResult {
    pub bid: Vec<BybitDepthLevel>,
    pub ask: Vec<BybitDepthLevel>,
}

#[derive(Debug, Deserialize)]
pub struct BybitDepthLevel {
    #[serde(alias = "price")]
    price: String,
    #[serde(alias = "qty")]
    qty: String,
}

impl BybitDepthResult {
    fn into_depth_response(self) -> Result<DepthResponse, AppError> {
        if self.bid.is_empty() && self.ask.is_empty() {
            return Err(AppError::Internal("Bybit returned empty depth data".to_string()));
        }

        // Calculate cumulative totals for bids (descending price order)
        let mut bid_total = 0.0;
        let bids: Vec<DepthLevel> = self
            .bid
            .into_iter()
            .take(500)
            .map(|level| {
                let price: f64 = level.price.parse().unwrap_or(0.0);
                let qty: f64 = level.qty.parse().unwrap_or(0.0);
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
            .ask
            .into_iter()
            .take(500)
            .map(|level| {
                let price: f64 = level.price.parse().unwrap_or(0.0);
                let qty: f64 = level.qty.parse().unwrap_or(0.0);
                ask_total += qty;
                DepthLevel {
                    price,
                    quantity: qty,
                    total: ask_total,
                }
            })
            .collect();

        Ok(DepthResponse {
            bids,
            asks,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bybit_ticker_item_conversion() {
        let item = BybitTickerItem {
            symbol: "BTCUSDT".to_string(),
            last_price: "50000.00".to_string(),
            price_24h_pcnt: "0.05".to_string(),
            volume_24h: "12345.67".to_string(),
            high_price_24h: "51000.00".to_string(),
            low_price_24h: "49000.00".to_string(),
            bid1_price: Some("49999.00".to_string()),
            ask1_price: Some("50001.00".to_string()),
        };

        let response = item.into_ticker_response();
        assert_eq!(response.symbol, "BTCUSDT");
        assert_eq!(response.price, 50000.0);
        assert_eq!(response.change_percent, 5.0); // 0.05 * 100
        assert!(response.change > 0.0);
        assert_eq!(response.bid, 49999.0);
        assert_eq!(response.ask, 50001.0);
    }
}