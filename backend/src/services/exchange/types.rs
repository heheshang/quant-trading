//! Binance WebSocket Stream Message Types
//!
//! Reference: https://github.com/binance/binance-connector-java/blob/master/docs/WebSocket_API.md

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Supported trading symbols (USDT perpetual).
pub const SUPPORTED_SYMBOLS: &[&str] = &[
    "btcusdt", "ethusdt", "solusdt", "bnbusdt", "xrpusdt", "dogeusdt", "adausdt", "avaxusdt",
    "dotusdt", "linkusdt",
];

/// Binance WebSocket stream message wrapper.
/// WS URL: `wss://stream.binance.com:9443/stream`
/// Streams: `<symbol>@ticker`, `<symbol>@depth20@100ms`, `<symbol>@kline_1m`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BinanceStreamMessage {
    /// Stream name, e.g. "btcusdt@ticker"
    pub stream: String,
    /// Message data (varies by stream type)
    #[serde(flatten)]
    pub data: BinanceData,
}

/// Union type for all Binance stream message data.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BinanceData {
    /// 24hr Ticker: <symbol>@ticker
    Ticker(TickerData),
    /// Depth 20 levels: <symbol>@depth20@100ms
    Depth(DepthData),
    /// Kline/Candlestick: <symbol>@kline_1m
    Kline(KlineData),
    /// Unknown or raw JSON
    Unknown(JsonValue),
}

/// 24hr Ticker Update Data
///
/// WS stream: `<symbol>@ticker`
/// API: https://developers.binance.com/docs/websocket_api/streams/ticker
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct TickerData {
    /// Symbol (uppercase), e.g. "BTCUSDT"
    pub s: String,
    /// Price change (last 24h)
    pub p: String,
    /// Price change percent (last 24h)
    pub P: String,
    /// Last trade price
    pub c: String,
    /// Last bid price
    pub b: String,
    /// Last ask price
    pub a: String,
    /// Volume (last 24h)
    pub v: String,
    /// Quote volume (last 24h)
    pub q: String,
    /// High price (last 24h)
    pub h: String,
    /// Low price (last 24h)
    pub l: String,
    /// Total number of trades
    pub n: u64,
    /// Event time (Unix timestamp in ms)
    pub E: u64,
    /// Open time (Unix timestamp in ms)
    pub O: Option<u64>,
    /// Close time (Unix timestamp in ms)
    pub C: Option<u64>,
}

impl TickerData {
    /// Parse price as f64
    pub fn price(&self) -> Option<f64> {
        self.c.parse().ok()
    }

    /// Parse bid as f64
    pub fn bid(&self) -> Option<f64> {
        self.b.parse().ok()
    }

    /// Parse ask as f64
    pub fn ask(&self) -> Option<f64> {
        self.a.parse().ok()
    }

    /// Parse volume as f64
    pub fn volume(&self) -> Option<f64> {
        self.v.parse().ok()
    }
}

/// Depth Update Data (Order Book)
///
/// WS stream: `<symbol>@depth20@100ms`
/// API: https://developers.binance.com/docs/websocket_api/streams/depth
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct DepthData {
    /// Symbol (uppercase)
    pub s: String,
    /// Last update ID
    pub u: u64,
    /// Bids: [price, quantity]
    pub b: Vec<(String, String)>,
    /// Asks: [price, quantity]
    pub a: Vec<(String, String)>,
    /// Event time (Unix timestamp in ms)
    pub E: u64,
}

/// Kline/Candlestick Data
///
/// WS stream: `<symbol>@kline_1m`
/// API: https://developers.binance.com/docs/websocket_api/streams/kline
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct KlineData {
    /// Symbol (uppercase)
    pub s: String,
    /// Kline interval, e.g. "1m"
    pub i: String,
    /// Kline data
    pub k: Kline,
}

/// Kline/Candlestick
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct Kline {
    /// Kline open time
    pub t: u64,
    /// Kline close time
    pub T: u64,
    /// Symbol
    pub s: String,
    /// Kline interval
    pub i: String,
    /// First trade ID
    pub f: u64,
    /// Last trade ID
    pub L: u64,
    /// Open price
    pub o: String,
    /// Close price
    pub c: String,
    /// High price
    pub h: String,
    /// Low price
    pub l: String,
    /// Base asset volume
    pub v: String,
    /// Number of trades
    pub n: u64,
    /// Is this kline closed?
    pub x: bool,
    /// Quote asset volume
    pub q: String,
    /// Taker buy base asset volume
    pub V: String,
    /// Taker buy quote asset volume
    pub Q: String,
    /// Ignore
    pub B: String,
}

impl Kline {
    /// Parse close price as f64
    pub fn close_price(&self) -> Option<f64> {
        self.c.parse().ok()
    }
}

/// Internal market message types (normalized from Binance format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketMessage {
    /// Ticker update (24hr rolling)
    Ticker {
        symbol: String,
        price: f64,
        change: f64,
        change_percent: f64,
        volume: f64,
        high: f64,
        low: f64,
        bid: f64,
        ask: f64,
        timestamp: u64,
    },
    /// Depth update (order book)
    Depth {
        symbol: String,
        bids: Vec<(f64, f64)>, // (price, quantity)
        asks: Vec<(f64, f64)>,
        timestamp: u64,
    },
    /// Kline update (candlestick)
    Kline {
        symbol: String,
        interval: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        close_time: u64,
        timestamp: u64,
    },
}

impl MarketMessage {
    /// Get channel name for this message type
    pub fn channel_name(&self) -> &'static str {
        match self {
            MarketMessage::Ticker { .. } => "market:ticker",
            MarketMessage::Depth { .. } => "market:depth",
            MarketMessage::Kline { .. } => "market:kline",
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticker_data_parse() {
        let json = r#"{
            "s": "BTCUSDT",
            "p": "-1000.00",
            "P": "-1.50",
            "c": "77950.00",
            "b": "77949.00",
            "a": "77951.00",
            "v": "12345.6789",
            "q": "123456789.00",
            "h": "79000.00",
            "l": "77000.00",
            "n": 12345,
            "E": 1747452000000,
            "O": 1747365600000,
            "C": null
        }"#;

        let ticker: TickerData = serde_json::from_str(json).unwrap();
        assert_eq!(ticker.s, "BTCUSDT");
        assert_eq!(ticker.price(), Some(77950.0));
        assert_eq!(ticker.bid(), Some(77949.0));
        assert_eq!(ticker.ask(), Some(77951.0));
    }

    #[test]
    fn test_market_message_channel() {
        let msg = MarketMessage::Ticker {
            symbol: "BTCUSDT".to_string(),
            price: 77950.0,
            change: -1000.0,
            change_percent: -1.5,
            volume: 12345.6789,
            high: 79000.0,
            low: 77000.0,
            bid: 77949.0,
            ask: 77951.0,
            timestamp: 1747452000000,
        };
        assert_eq!(msg.channel_name(), "market:ticker");
    }
}
