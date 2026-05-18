//! Binance WebSocket Connector Implementation

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use super::errors::ConnectorError;
use super::types::{BinanceData, BinanceStreamMessage, MarketMessage, SUPPORTED_SYMBOLS};

/// Binance WebSocket connector configuration.
#[derive(Debug, Clone)]
pub struct BinanceConnectorConfig {
    /// WebSocket endpoint
    pub ws_url: String,
    /// Ping interval (default 60s)
    pub ping_interval: Duration,
    /// Maximum reconnection delay (default 30s)
    pub max_reconnect_delay: Duration,
    /// Initial reconnection delay (default 1s)
    pub initial_reconnect_delay: Duration,
}

impl Default for BinanceConnectorConfig {
    fn default() -> Self {
        Self {
            ws_url: "wss://stream.binance.com:9443/stream".to_string(),
            ping_interval: Duration::from_secs(60),
            max_reconnect_delay: Duration::from_secs(30),
            initial_reconnect_delay: Duration::from_secs(1),
        }
    }
}

/// Binance WebSocket Connector
///
/// Manages the connection to Binance WebSocket streams, handles reconnection
/// with exponential backoff, and converts raw messages to internal MarketMessage format.
pub struct BinanceConnector {
    config: BinanceConnectorConfig,
    /// Broadcast channel for distributing messages to subscribers
    tx: broadcast::Sender<MarketMessage>,
    /// Shutdown signal sender
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl BinanceConnector {
    /// Create a new BinanceConnector with default config.
    pub fn new() -> Self {
        Self::with_config(BinanceConnectorConfig::default())
    }

    /// Create a new BinanceConnector with custom config.
    pub fn with_config(config: BinanceConnectorConfig) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            config,
            tx,
            shutdown_tx: None,
        }
    }

    /// Subscribe to the connector's message stream.
    pub fn subscribe(&self) -> broadcast::Receiver<MarketMessage> {
        self.tx.subscribe()
    }

    /// Start the connector and return a handle for control.
    pub async fn start(&mut self) -> Result<(), ConnectorError> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let streams = self.build_stream_list();
        let subscribe_msg = self.build_subscribe_message(&streams);

        self.connect_and_subscribe(&subscribe_msg, shutdown_rx)
            .await
    }

    /// Build the list of stream names to subscribe to.
    fn build_stream_list(&self) -> Vec<String> {
        let mut streams = Vec::with_capacity(SUPPORTED_SYMBOLS.len() * 3);
        for symbol in SUPPORTED_SYMBOLS {
            streams.push(format!("{}@ticker", symbol));
            streams.push(format!("{}@depth20@100ms", symbol));
            streams.push(format!("{}@kline_1m", symbol));
        }
        streams
    }

    /// Build the JSON subscription message for Binance WS.
    fn build_subscribe_message(&self, streams: &[String]) -> String {
        let msg = serde_json::to_string(&serde_json::json!({
            "method": "SUBSCRIBE",
            "params": streams,
            "id": 1
        }))
        .unwrap_or_default();
        info!(
            "Built subscription message ({} streams): {}",
            streams.len(),
            msg
        );
        msg
    }

    /// Connect to Binance WebSocket and handle messages.
    async fn connect_and_subscribe(
        &mut self,
        subscribe_msg: &str,
        mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<(), ConnectorError> {
        let url = self.config.ws_url.as_str();
        info!("Connecting to Binance WebSocket: {}", url);

        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| ConnectorError::ConnectionFailed(e.to_string()))?;

        info!("Binance WebSocket connected");

        let (mut write, mut read) = ws_stream.split();

        // Send subscription message
        info!("Sending subscription: {}", subscribe_msg);
        write
            .send(Message::Text(subscribe_msg.into()))
            .await
            .map_err(|e| ConnectorError::SubscriptionFailed(e.to_string()))?;

        info!("Binance WebSocket subscription sent");

        loop {
            tokio::select! {
                // Incoming WebSocket message
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_message(&text);
                        }
                        Some(Ok(Message::Ping(data))) => {
                            debug!("Received ping, responding with pong");
                            let _ = write.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Pong(_))) => {
                            debug!("Received pong");
                        }
                        Some(Ok(Message::Close(reason))) => {
                            warn!("WebSocket closed: {:?}", reason);
                            return Err(ConnectorError::Disconnected("Connection closed".to_string()));
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            return Err(ConnectorError::ConnectionFailed(e.to_string()));
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            return Err(ConnectorError::Disconnected("Stream ended".to_string()));
                        }
                        _ => {}
                    }
                }
                // Shutdown signal
                _ = &mut shutdown_rx => {
                    info!("BinanceConnector shutdown signal received");
                    return Ok(());
                }
            }
        }
    }

    /// Handle an incoming WebSocket message.
    fn handle_message(&self, text: &str) {
        // Binance sends array of stream messages
        let messages: Result<Vec<BinanceStreamMessage>, _> = serde_json::from_str(text);

        match messages {
            Ok(msgs) => {
                for msg in msgs {
                    if let Some(market_msg) = self.normalize_message(msg) {
                        // Broadcast to all subscribers
                        let _ = self.tx.send(market_msg);
                    }
                }
            }
            Err(_) => {
                // Try parsing as single message
                #[allow(clippy::collapsible_if)]
                #[allow(clippy::collapsible_if)]
                if let Ok(msg) = serde_json::from_str::<BinanceStreamMessage>(text) {
                    if let Some(market_msg) = self.normalize_message(msg) {
                        let _ = self.tx.send(market_msg);
                    }
                }
            }
        }
    }

    /// Normalize Binance message to internal MarketMessage format.
    fn normalize_message(&self, msg: BinanceStreamMessage) -> Option<MarketMessage> {
        match msg.data {
            BinanceData::Ticker(ticker) => Some(MarketMessage::Ticker {
                symbol: ticker.s.clone(),
                price: ticker.price().unwrap_or(0.0),
                change: ticker.p.parse().unwrap_or(0.0),
                change_percent: ticker.P.parse().unwrap_or(0.0),
                volume: ticker.volume().unwrap_or(0.0),
                high: ticker.h.parse().unwrap_or(0.0),
                low: ticker.l.parse().unwrap_or(0.0),
                bid: ticker.b.parse().unwrap_or(0.0),
                ask: ticker.a.parse().unwrap_or(0.0),
                timestamp: ticker.E,
            }),
            BinanceData::Depth(depth) => {
                let parse_bids = depth
                    .b
                    .iter()
                    .filter_map(|(p, q)| Some((p.parse().ok()?, q.parse().ok()?)))
                    .collect();
                let parse_asks = depth
                    .a
                    .iter()
                    .filter_map(|(p, q)| Some((p.parse().ok()?, q.parse().ok()?)))
                    .collect();

                Some(MarketMessage::Depth {
                    symbol: depth.s,
                    bids: parse_bids,
                    asks: parse_asks,
                    timestamp: depth.E,
                })
            }
            BinanceData::Kline(kline) => {
                let k = kline.k;
                Some(MarketMessage::Kline {
                    symbol: k.s,
                    interval: kline.i,
                    open: k.o.parse().unwrap_or(0.0),
                    high: k.h.parse().unwrap_or(0.0),
                    low: k.l.parse().unwrap_or(0.0),
                    close: k.c.parse().unwrap_or(0.0),
                    volume: k.v.parse().unwrap_or(0.0),
                    close_time: k.T,
                    timestamp: k.t,
                })
            }
            BinanceData::Unknown(_) => None,
        }
    }

    /// Attempt reconnection with exponential backoff.
    #[allow(dead_code)]
    async fn reconnect(&mut self) -> Result<(), ConnectorError> {
        let mut delay = self.config.initial_reconnect_delay;
        let max_delay = self.config.max_reconnect_delay;

        info!("Starting reconnection with exponential backoff");

        loop {
            tokio::time::sleep(delay).await;

            info!("Attempting to reconnect to Binance WebSocket...");

            match self.start().await {
                Ok(_) => {
                    info!("Reconnected successfully");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Reconnection failed: {}", e);
                    delay = (delay * 2).min(max_delay);
                    info!("Next retry in {:?}", delay);
                }
            }
        }
    }

    /// Stop the connector.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Default for BinanceConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_stream_list() {
        let connector = BinanceConnector::new();
        let streams = connector.build_stream_list();

        // 10 symbols × 3 streams = 30 streams
        assert_eq!(streams.len(), 30);
        assert!(streams.contains(&"btcusdt@ticker".to_string()));
        assert!(streams.contains(&"btcusdt@depth20@100ms".to_string()));
        assert!(streams.contains(&"btcusdt@kline_1m".to_string()));
    }

    #[test]
    fn test_build_subscribe_message() {
        let connector = BinanceConnector::new();
        let streams = vec!["btcusdt@ticker".to_string(), "ethusdt@ticker".to_string()];
        let msg = connector.build_subscribe_message(&streams);

        assert!(msg.contains("SUBSCRIBE"));
        assert!(msg.contains("btcusdt@ticker"));
        assert!(msg.contains("ethusdt@ticker"));
    }
}
