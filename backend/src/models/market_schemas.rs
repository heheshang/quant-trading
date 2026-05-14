// ============ Market Schemas (新增) ============
// 添加到 models/schemas.rs 末尾

use serde::{Deserialize, Serialize};

// ---- Ticker ----

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct TickerQueryParams {
    pub symbol: String,
}

// ---- Depth ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthLevel {
    pub price: f64,
    pub quantity: f64,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthResponse {
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
    pub timestamp: i64,
}

#[derive(Debug, Deserialize)]
pub struct DepthQueryParams {
    pub symbol: String,
    pub levels: Option<i32>,
}

// ---- Ticker History (P1) ----

#[derive(Debug, Deserialize)]
pub struct TickerHistoryQueryParams {
    pub symbol: String,
    pub start: i64,
    pub end: i64,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct TickerHistoryMeta {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TickerHistoryResponse {
    pub items: Vec<TickerSnapshotResponse>,
    pub meta: TickerHistoryMeta,
}

// ---- Watchlist (P1) ----

#[derive(Debug, Deserialize)]
pub struct WatchlistUpdateRequest {
    pub symbols: Vec<String>,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize)]
pub struct WsInMessage {
    pub action: String,
    #[serde(default)]
    pub channels: Vec<String>,
}

// ---- Market Error Codes ----

pub const ERR_MARKET_SYMBOL_NOT_FOUND: i32 = 40401;
pub const ERR_MARKET_DEPTH_LEVELS_INVALID: i32 = 40001;
pub const ERR_MARKET_DEPTH_FORBIDDEN: i32 = 40301;
pub const ERR_MARKET_WS_AUTH_FAILED: i32 = 40101;
pub const ERR_MARKET_WS_CHANNEL_INVALID: i32 = 40002;
pub const ERR_MARKET_COLLECTOR_DOWN: i32 = 50301;
pub const ERR_MARKET_WATCHLIST_FULL: i32 = 42901;
pub const ERR_MARKET_WS_TOO_MANY_CHANNELS: i32 = 42902;

// ---- Tests ----

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
                DepthLevel { price: 50499.0, quantity: 1.5, total: 1.5 },
                DepthLevel { price: 50498.0, quantity: 2.3, total: 3.8 },
            ],
            asks: vec![
                DepthLevel { price: 50501.0, quantity: 1.2, total: 1.2 },
            ],
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
}
