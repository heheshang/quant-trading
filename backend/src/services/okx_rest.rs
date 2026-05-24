//! OKX REST API Client
//!
//! Fetches real-time market data from OKX public REST API.
//!
//! # API Endpoints
//! - Ticker: GET /api/v5/market/ticker?instId={symbol}-USDT
//! - All Tickers: GET /api/v5/market/tickers?instType=SPOT
//! - Depth: GET /api/v5/market/books?instId={symbol}-USDT&sz={levels}
//!
//! # Symbol Format
//! OKX uses hyphenated symbol format: "BTC-USDT" instead of Binance's "BTCUSDT"
//!
//! # Error Handling
//! All reqwest errors are converted to AppError::Internal for consistent error handling.

use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, error, info};

use crate::models::market_schemas::{DepthLevel, DepthResponse, TickerResponse};
use crate::utils::error::AppError;

/// OKX REST API base URL
const OKX_API_BASE: &str = "https://www.okx.com";

/// OKX REST API client for fetching market data
#[derive(Clone)]
pub struct OkxRestClient {
    client: Client,
    base_url: String,
}

impl OkxRestClient {
    /// Create a new OKX REST API client
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: OKX_API_BASE.to_string(),
        }
    }

    /// Convert internal symbol format to OKX format
    /// e.g., "BTCUSDT" -> "BTC-USDT"
    fn to_okx_symbol(symbol: &str) -> String {
        if symbol.contains('-') {
            symbol.to_string()
        } else if symbol.ends_with("USDT") {
            let prefix = &symbol[..symbol.len() - 4];
            format!("{}-USDT", prefix)
        } else {
            symbol.to_string()
        }
    }

    /// Get 24hr ticker statistics for a single symbol
    ///
    /// # Data Flow
    /// 1. GET https://www.okx.com/api/v5/market/ticker?instId={symbol}-USDT
    /// 2. Parse OKX response into internal TickerResponse format
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    ///
    /// # Returns
    /// * `Ok(TickerResponse)` - Normalized ticker data
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_ticker(&self, symbol: &str) -> Result<TickerResponse, AppError> {
        let okx_symbol = Self::to_okx_symbol(symbol);
        let url = format!("{}/api/v5/market/ticker", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("instId", &okx_symbol)])
            .send()
            .await
            .map_err(|e| {
                error!(symbol = %symbol, okx_symbol = %okx_symbol, error = %e, "Failed to fetch ticker from OKX");
                AppError::Internal(format!("OKX API error: {}", e))
            })?;

        let okx_ticker: OkxTickerData = response.json().await.map_err(|e| {
            error!(symbol = %symbol, error = %e, "Failed to parse OKX ticker response");
            AppError::Internal(format!("Failed to parse OKX response: {}", e))
        })?;

        debug!(symbol = %symbol, price = %okx_ticker.data.first().map(|d| d.last.as_str()).unwrap_or("?"), "Fetched ticker from OKX");
        Ok(okx_ticker.into_ticker_response(symbol))
    }

    /// Get 24hr ticker statistics for all symbols
    ///
    /// # Data Flow
    /// 1. GET https://www.okx.com/api/v5/market/tickers?instType=SPOT
    /// 2. Parse all OKX responses into internal TickerResponse format
    /// 3. Filter to only supported symbols
    ///
    /// # Returns
    /// * `Ok(Vec<TickerResponse>)` - List of normalized ticker data
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_all_tickers(&self) -> Result<Vec<TickerResponse>, AppError> {
        let url = format!("{}/api/v5/market/tickers", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("instType", "SPOT")])
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch all tickers from OKX");
                AppError::Internal(format!("OKX API error: {}", e))
            })?;

        let okx_data: OkxTickersResponse = response.json().await.map_err(|e| {
            error!(error = %e, "Failed to parse OKX tickers response");
            AppError::Internal(format!("Failed to parse OKX response: {}", e))
        })?;

        use crate::services::market_data::SUPPORTED_SYMBOLS;

        let tickers: Vec<TickerResponse> = okx_data
            .data
            .into_iter()
            .filter_map(|t| {
                // OKX returns symbols like "BTC-USDT", convert to "BTCUSDT"
                let normalized = t.normalize_symbol();
                if SUPPORTED_SYMBOLS.contains(&normalized.as_str()) {
                    Some(t.into_ticker_response(&normalized))
                } else {
                    None
                }
            })
            .collect();

        info!(count = tickers.len(), "Fetched all tickers from OKX");
        Ok(tickers)
    }

    /// Get order book depth data
    ///
    /// # Data Flow
    /// 1. GET https://www.okx.com/api/v5/market/books?instId={symbol}-USDT&sz={levels}
    /// 2. Parse OKX response into internal DepthResponse format
    ///
    /// # Arguments
    /// * `symbol` - Trading pair symbol (e.g., "BTCUSDT")
    /// * `levels` - Number of depth levels (e.g., 25, 50, 100, 500)
    ///
    /// # Returns
    /// * `Ok(DepthResponse)` - Normalized depth data with bids/asks
    /// * `Err(AppError::Internal)` - Network or parsing error
    pub async fn get_depth(&self, symbol: &str, levels: i32) -> Result<DepthResponse, AppError> {
        let okx_symbol = Self::to_okx_symbol(symbol);
        let url = format!("{}/api/v5/market/books-l1", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("instId", &okx_symbol), ("sz", &levels.to_string())])
            .send()
            .await
            .map_err(|e| {
                error!(symbol = %symbol, error = %e, "Failed to fetch depth from OKX");
                AppError::Internal(format!("OKX API error: {}", e))
            })?;

        let okx_depth: OkxDepthResponse = response.json().await.map_err(|e| {
            error!(symbol = %symbol, error = %e, "Failed to parse OKX depth response");
            AppError::Internal(format!("Failed to parse OKX response: {}", e))
        })?;

        debug!(symbol = %symbol, bids = %okx_depth.data.first().map(|d| d.bids.len()).unwrap_or(0),
              asks = %okx_depth.data.first().map(|d| d.asks.len()).unwrap_or(0), "Fetched depth from OKX");

        let depth = okx_depth.into_depth_response()?;
        Ok(depth)
    }
}

impl Default for OkxRestClient {
    fn default() -> Self {
        Self::new()
    }
}

// ========== OKX API Response Types ==========

/// OKX Tickers Response (for multiple symbols)
#[derive(Debug, Deserialize)]
pub struct OkxTickersResponse {
    pub code: String,
    pub msg: String,
    pub data: Vec<OkxTickerItem>,
}

#[derive(Debug, Deserialize)]
pub struct OkxTickerItem {
    /// Instrument ID, e.g., "BTC-USDT"
    #[serde(alias = "instId")]
    inst_id: String,
    /// Last traded price
    #[serde(alias = "last")]
    last: String,
    /// 24h price change
    #[serde(alias = "price24h")]
    price_24h: String,
    /// 24h trading volume (quote currency)
    #[serde(alias = "vol24h")]
    vol_24h: String,
    /// 24h high price
    #[serde(alias = "high24h")]
    high_24h: String,
    /// 24h low price
    #[serde(alias = "low24h")]
    low_24h: String,
    /// Best bid price
    #[serde(alias = "bidPx")]
    bid_price: Option<String>,
    /// Best ask price
    #[serde(alias = "askPx")]
    ask_price: Option<String>,
    /// Open price at UTC 00:00
    #[serde(alias = "openUtc0")]
    open_utc0: String,
}

impl OkxTickerItem {
    /// Convert OKX symbol format to internal format
    /// e.g., "BTC-USDT" -> "BTCUSDT"
    fn normalize_symbol(&self) -> String {
        self.inst_id.replace("-", "")
    }

    fn into_ticker_response(self, normalized_symbol: &str) -> TickerResponse {
        let price: f64 = self.last.parse().unwrap_or(0.0);
        let open_price: f64 = self.open_utc0.parse().unwrap_or(price);
        let change: f64 = price - open_price;
        let change_percent: f64 = if open_price > 0.0 {
            (price - open_price) / open_price * 100.0
        } else {
            0.0
        };
        let volume: f64 = self.vol_24h.parse().unwrap_or(0.0);
        let high: f64 = self.high_24h.parse().unwrap_or(0.0);
        let low: f64 = self.low_24h.parse().unwrap_or(0.0);
        let bid: f64 = self
            .bid_price
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let ask: f64 = self
            .ask_price
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        TickerResponse {
            symbol: normalized_symbol.to_string(),
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

/// OKX single ticker data (for /ticker endpoint)
#[derive(Debug, Deserialize)]
pub struct OkxTickerData {
    pub code: String,
    pub msg: String,
    pub data: Vec<OkxTickerItem>,
}

impl OkxTickerData {
    fn into_ticker_response(self, symbol: &str) -> TickerResponse {
        let item = self
            .data
            .into_iter()
            .next()
            .unwrap_or_else(|| OkxTickerItem {
                inst_id: symbol.to_string(),
                last: "0".to_string(),
                price_24h: "0".to_string(),
                vol_24h: "0".to_string(),
                high_24h: "0".to_string(),
                low_24h: "0".to_string(),
                bid_price: None,
                ask_price: None,
                open_utc0: "0".to_string(),
            });
        let normalized = item.normalize_symbol();
        item.into_ticker_response(&normalized)
    }
}

/// OKX Depth Response
#[derive(Debug, Deserialize)]
pub struct OkxDepthResponse {
    pub code: String,
    pub msg: String,
    pub data: Vec<OkxDepthData>,
}

#[derive(Debug, Deserialize)]
pub struct OkxDepthData {
    /// Instrument ID
    #[serde(alias = "instId")]
    inst_id: String,
    /// Last update timestamp
    #[serde(alias = "ts")]
    ts: String,
    /// Bids [price, quantity, liquidated quantity, price]
    #[serde(alias = "bids")]
    bids: Vec<Vec<String>>,
    /// Asks [price, quantity, liquidated quantity, price]
    #[serde(alias = "asks")]
    asks: Vec<Vec<String>>,
}

impl OkxDepthResponse {
    fn into_depth_response(self) -> Result<DepthResponse, AppError> {
        let data = self
            .data
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Internal("OKX returned empty depth data".to_string()))?;

        // Calculate cumulative totals for bids (descending price order)
        let mut bid_total = 0.0;
        let bids: Vec<DepthLevel> = data
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
        let asks: Vec<DepthLevel> = data
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

        let timestamp: i64 = data.ts.parse().unwrap_or_else(|_| chrono::Utc::now().timestamp_millis());

        Ok(DepthResponse {
            bids,
            asks,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_okx_symbol_conversion() {
        assert_eq!(OkxRestClient::to_okx_symbol("BTCUSDT"), "BTC-USDT");
        assert_eq!(OkxRestClient::to_okx_symbol("ETHUSDT"), "ETH-USDT");
        assert_eq!(OkxRestClient::to_okx_symbol("BTC-USDT"), "BTC-USDT");
    }

    #[test]
    fn test_normalize_symbol() {
        let item = OkxTickerItem {
            inst_id: "BTC-USDT".to_string(),
            last: "50000".to_string(),
            price_24h: "49000".to_string(),
            vol_24h: "1000000".to_string(),
            high_24h: "51000".to_string(),
            low_24h: "49000".to_string(),
            bid_price: Some("49999".to_string()),
            ask_price: Some("50001".to_string()),
            open_utc0: "49000".to_string(),
        };
        assert_eq!(item.normalize_symbol(), "BTCUSDT");
    }
}
