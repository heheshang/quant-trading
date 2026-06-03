// ============ Market Schemas (新增) ============
// 添加到 models/schemas.rs 末尾

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ---- Exchange ----

/// Supported exchange enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Exchange {
    #[default]
    Binance,
    Okx,
    Gate,
    Bybit,
    Huobi,
}

// ---- Ticker ----

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TickerResponse {
    pub symbol: String,
    pub price: f64,
    pub change: f64,
    pub change_percent: f64,
    pub volume: f64,
    pub high: f64,
    pub low: f64,
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64,
}

/// Valid trading symbol pattern: 2-10 uppercase letters followed by USDT
/// Examples: BTCUSDT, ETHUSDT, SOLUSDT, etc.
const SYMBOL_SUFFIX: &str = "USDT";
const SYMBOL_MIN_LEN: usize = 2; // e.g., "osusdt" (actually 2-10 letters)
const SYMBOL_MAX_LEN: usize = 10;

#[derive(Debug, Deserialize, ToSchema)]
pub struct TickerQueryParams {
    pub symbol: Option<String>,
    pub exchange: Option<Exchange>,
}

impl TickerQueryParams {
    /// Validates that the symbol matches the expected format (e.g., BTCUSDT, ETHUSDT)
    pub fn validate_symbol(&self) -> Result<(), String> {
        let s = match &self.symbol {
            Some(symbol) => symbol,
            None => return Ok(()), // No symbol provided is valid (will return all tickers)
        };
        // Check minimum length (prefix + suffix)
        if s.len() < SYMBOL_MIN_LEN + SYMBOL_SUFFIX.len() {
            return Err(format!(
                "Invalid symbol format '{}': must be 2-10 uppercase letters followed by USDT (e.g., BTCUSDT)",
                s
            ));
        }
        // Check suffix
        if !s.ends_with(SYMBOL_SUFFIX) {
            return Err(format!(
                "Invalid symbol format '{}': must end with USDT (e.g., BTCUSDT)",
                s
            ));
        }
        // Check prefix is 2-10 uppercase letters
        let prefix = &s[..s.len() - SYMBOL_SUFFIX.len()];
        if prefix.len() < SYMBOL_MIN_LEN || prefix.len() > SYMBOL_MAX_LEN {
            return Err(format!(
                "Invalid symbol prefix '{}': must be 2-10 uppercase letters",
                prefix
            ));
        }
        if !prefix.chars().all(|c| c.is_ascii_uppercase()) {
            return Err(format!(
                "Invalid symbol '{}': prefix must be uppercase letters only",
                s
            ));
        }
        Ok(())
    }
}

// ---- Depth ----

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DepthLevel {
    pub price: f64,
    pub quantity: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DepthResponse {
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
    pub timestamp: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DepthQueryParams {
    pub symbol: String,
    pub levels: Option<i32>,
    pub exchange: Option<Exchange>,
}

// ---- Ticker History (P1) ----

/// Maximum allowed time range for ticker history queries (90 days in milliseconds)
const MAX_TICKER_HISTORY_RANGE_MS: i64 = 90 * 24 * 60 * 60 * 1000;

#[derive(Debug, Deserialize, ToSchema)]
pub struct TickerHistoryQueryParams {
    pub symbol: String,
    pub start: i64,
    pub end: i64,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

impl TickerHistoryQueryParams {
    /// Validates that start < end and the range doesn't exceed 90 days
    pub fn validate_time_range(&self) -> Result<(), String> {
        if self.start >= self.end {
            return Err(format!(
                "Invalid time range: start ({}) must be less than end ({})",
                self.start, self.end
            ));
        }
        let range = self.end - self.start;
        if range > MAX_TICKER_HISTORY_RANGE_MS {
            return Err(format!(
                "Time range exceeds maximum of 90 days ({}ms): requested {}ms",
                MAX_TICKER_HISTORY_RANGE_MS, range
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TickerSnapshotResponse {
    pub symbol: String,
    pub price: f64,
    pub change: f64,
    pub change_percent: f64,
    pub volume: f64,
    pub high: f64,
    pub low: f64,
    pub bid: f64,
    pub ask: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TickerHistoryMeta {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TickerHistoryResponse {
    pub items: Vec<TickerSnapshotResponse>,
    pub meta: TickerHistoryMeta,
}

// ---- Watchlist (P1) ----

#[derive(Debug, Deserialize, ToSchema)]
pub struct WatchlistUpdateRequest {
    pub symbols: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WatchlistResponse {
    pub symbols: Vec<String>,
    pub count: usize,
}

// ---- WebSocket Messages ----

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WsOutMessage {
    #[serde(rename = "ticker")]
    Ticker {
        symbol: String,
        data: TickerResponse,
        ts: i64,
    },
    #[serde(rename = "depth")]
    Depth {
        symbol: String,
        data: DepthResponse,
        ts: i64,
    },
    #[serde(rename = "depth_update")]
    DepthUpdate {
        symbol: String,
        data: DepthResponse,
        ts: i64,
    },
    #[serde(rename = "kline")]
    Kline {
        symbol: String,
        data: serde_json::Value,
        ts: i64,
    },
    #[serde(rename = "heartbeat")]
    Heartbeat { ts: i64 },
    #[serde(rename = "subscribed")]
    Subscribed { channel: String },
    #[serde(rename = "unsubscribed")]
    Unsubscribed { channel: String },
    #[serde(rename = "kick")]
    Kick { reason: String },
    #[serde(rename = "error")]
    Error { code: i32, message: String },
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WsInMessage {
    pub action: String,
    #[serde(default)]
    pub channels: Vec<String>,
}

// (Error codes removed — use AppError::with_code() for market-specific errors)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticker_response_serialization() {
        let ticker = TickerResponse {
            symbol: "BTCUSDT".into(),
            price: 50500.0,
            change: 150.0,
            change_percent: 0.30,
            volume: 123456.7,
            high: 51000.0,
            low: 49000.0,
            bid: 50499.0,
            ask: 50501.0,
            timestamp: 1715500000000,
        };
        let json = serde_json::to_value(&ticker).unwrap();
        assert_eq!(json["symbol"], "BTCUSDT");
        assert_eq!(json["price"], 50500.0);
        assert_eq!(json["bid"], 50499.0);
        assert_eq!(json["ask"], 50501.0);
        assert_eq!(json["timestamp"], 1715500000000_i64);
    }

    #[test]
    fn test_depth_response_serialization() {
        let depth = DepthResponse {
            bids: vec![
                DepthLevel {
                    price: 50499.0,
                    quantity: 1.5,
                    total: 1.5,
                },
                DepthLevel {
                    price: 50498.0,
                    quantity: 2.3,
                    total: 3.8,
                },
            ],
            asks: vec![DepthLevel {
                price: 50501.0,
                quantity: 1.2,
                total: 1.2,
            }],
            timestamp: 1715500000000,
        };
        let json = serde_json::to_value(&depth).unwrap();
        assert_eq!(json["bids"].as_array().unwrap().len(), 2);
        assert_eq!(json["bids"][0]["price"], 50499.0);
        assert_eq!(json["bids"][0]["quantity"], 1.5);
        assert_eq!(json["bids"][0]["total"], 1.5);
    }

    #[test]
    fn test_ws_out_message_ticker_serialization() {
        let msg = WsOutMessage::Ticker {
            symbol: "BTCUSDT".into(),
            data: TickerResponse {
                symbol: "BTCUSDT".into(),
                price: 50500.0,
                change: 150.0,
                change_percent: 0.30,
                volume: 123456.7,
                high: 51000.0,
                low: 49000.0,
                bid: 50499.0,
                ask: 50501.0,
                timestamp: 1715500000000,
            },
            ts: 1715500000000,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "ticker");
        assert_eq!(json["symbol"], "BTCUSDT");
        assert_eq!(json["data"]["price"], 50500.0);
    }

    #[test]
    fn test_ws_in_message_deserialization() {
        let json = r#"{"action":"subscribe","channels":["ticker:BTCUSDT","depth:ETHUSDT"]}"#;
        let msg: WsInMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.action, "subscribe");
        assert_eq!(msg.channels.len(), 2);
        assert_eq!(msg.channels[0], "ticker:BTCUSDT");
    }

    #[test]
    fn test_depth_query_params_defaults() {
        let json = r#"{"symbol":"BTCUSDT"}"#;
        let params: DepthQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.symbol, "BTCUSDT");
        assert!(params.levels.is_none());
    }

    #[test]
    fn test_depth_query_params_with_levels() {
        let json = r#"{"symbol":"BTCUSDT","levels":20}"#;
        let params: DepthQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.levels, Some(20));
    }

    #[test]
    fn test_depth_query_params_with_exchange() {
        let json = r#"{"symbol":"BTCUSDT","levels":20,"exchange":"okx"}"#;
        let params: DepthQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.levels, Some(20));
        assert_eq!(params.exchange, Some(Exchange::Okx));
    }

    // === F-08: Symbol format validation tests ===

    #[test]
    fn test_symbol_validation_valid_btc() {
        let params = TickerQueryParams {
            symbol: Some("BTCUSDT".to_string()),
            exchange: None,
        };
        assert!(params.validate_symbol().is_ok());
    }

    #[test]
    fn test_symbol_validation_valid_eth() {
        let params = TickerQueryParams {
            symbol: Some("ETHUSDT".to_string()),
            exchange: None,
        };
        assert!(params.validate_symbol().is_ok());
    }

    #[test]
    fn test_symbol_validation_valid_long_prefix() {
        let params = TickerQueryParams {
            symbol: Some("SOLANAUSDT".to_string()),
            exchange: None,
        };
        assert!(params.validate_symbol().is_ok());
    }

    #[test]
    fn test_symbol_validation_invalid_lowercase() {
        // "BtcUSDT" has uppercase USDT suffix but lowercase 'Btc' prefix
        let params = TickerQueryParams {
            symbol: Some("BtcUSDT".to_string()),
            exchange: None,
        };
        let result = params.validate_symbol();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("uppercase"));
    }

    #[test]
    fn test_symbol_validation_invalid_wrong_suffix() {
        let params = TickerQueryParams {
            symbol: Some("BTCUSD".to_string()),
            exchange: None,
        };
        let result = params.validate_symbol();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("USDT"));
    }

    #[test]
    fn test_symbol_validation_invalid_too_short() {
        let params = TickerQueryParams {
            symbol: Some("BUSDT".to_string()),
            exchange: None,
        }; // 1 letter prefix
        let result = params.validate_symbol();
        assert!(result.is_err());
    }

    #[test]
    fn test_symbol_validation_invalid_too_long() {
        let params = TickerQueryParams {
            symbol: Some("VERYLONGCOINNAMEUSDT".to_string()),
            exchange: None,
        }; // 15 letter prefix
        let result = params.validate_symbol();
        assert!(result.is_err());
    }

    #[test]
    fn test_symbol_validation_invalid_with_numbers() {
        let params = TickerQueryParams {
            symbol: Some("BTC123USDT".to_string()),
            exchange: None,
        };
        let result = params.validate_symbol();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("uppercase"));
    }

    // === F-09: TickerHistory time range validation tests ===

    #[test]
    fn test_ticker_history_valid_range() {
        let params = TickerHistoryQueryParams {
            symbol: "BTCUSDT".into(),
            start: 1747400000000,
            end: 1747500000000,
            page: Some(1),
            page_size: Some(20),
        };
        assert!(params.validate_time_range().is_ok());
    }

    #[test]
    fn test_ticker_history_invalid_start_after_end() {
        let params = TickerHistoryQueryParams {
            symbol: "BTCUSDT".into(),
            start: 1747500000000,
            end: 1747400000000, // end before start
            page: Some(1),
            page_size: Some(20),
        };
        let result = params.validate_time_range();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be less than"));
    }

    #[test]
    fn test_ticker_history_invalid_start_equals_end() {
        let params = TickerHistoryQueryParams {
            symbol: "BTCUSDT".into(),
            start: 1747400000000,
            end: 1747400000000, // same as start
            page: Some(1),
            page_size: Some(20),
        };
        let result = params.validate_time_range();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be less than"));
    }

    #[test]
    fn test_ticker_history_invalid_range_exceeds_90_days() {
        // 100 days = 100 * 24 * 60 * 60 * 1000 = 8640000000ms
        let start = 1747400000000i64;
        let end = start + (100i64 * 24 * 60 * 60 * 1000); // exactly 100 days
        let params = TickerHistoryQueryParams {
            symbol: "BTCUSDT".into(),
            start,
            end,
            page: Some(1),
            page_size: Some(20),
        };
        let result = params.validate_time_range();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("90 days"));
    }
}
