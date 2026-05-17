//! WebSocket Hub - Manages WS client connections and Binance WS data flow
//!
//! Single-source-of-truth for real-time market data distribution.
//! Forwards BinanceConnector broadcast messages to all connected WS clients.

use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::{error, info, warn};

use crate::services::exchange::{BinanceConnector, MarketMessage};

/// HubMessage - standardized message format for WS clients
#[derive(Clone, Debug)]
pub enum HubMessage {
    Ticker {
        symbol: String,
        price: f64,
        change: f64,
        change_pct: f64,
        volume: f64,
        high: f64,
        low: f64,
        bid: f64,
        ask: f64,
    },
    Depth {
        symbol: String,
        bids: Vec<(f64, f64)>,
        asks: Vec<(f64, f64)>,
    },
    Kline {
        symbol: String,
        interval: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    },
}

/// HubEvent - server lifecycle events
#[derive(Clone, Debug)]
pub enum HubEvent {
    Connected,
    Disconnected,
    Reconnecting { attempt: u32 },
    Error { message: String },
}

/// WebSocket Hub - singleton manager for WS connections and Binance data
pub struct WsHub {
    /// Shutdown signal sender (sent to connector task on drop)
    shutdown_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    /// Broadcast channel for WS clients to receive market data
    tx: broadcast::Sender<HubMessage>,
    /// Server-level event broadcast
    hub_tx: broadcast::Sender<HubEvent>,
    /// Active subscriber count for monitoring
    subscriber_count: std::sync::atomic::AtomicUsize,
}

impl WsHub {
    /// Create a new WsHub with broadcast channels.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(2048);
        let (hub_tx, _) = broadcast::channel(100);
        Self {
            shutdown_tx: Arc::new(Mutex::new(None)),
            tx,
            hub_tx,
            subscriber_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Subscribe to market data (call from WS client handler).
    pub fn subscribe(&self) -> broadcast::Receiver<HubMessage> {
        self.tx.subscribe()
    }

    /// Subscribe to hub events (for monitoring/admin).
    pub fn subscribe_events(&self) -> broadcast::Receiver<HubEvent> {
        self.hub_tx.subscribe()
    }

    /// Start Binance connector and spawn message forwarder.
    /// Call once at server startup.
    pub fn start(&self) {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let tx = self.tx.clone();
        let hub_tx = self.hub_tx.clone();

        // Spawn connector + forwarder task
        tokio::spawn(async move {
            let connector = BinanceConnector::new();
            Self::run_connector(connector, tx, hub_tx, shutdown_rx).await;
        });

        info!("WebSocket Hub started");
    }

    /// Run loop: connect to Binance, forward messages to hub broadcast.
    async fn run_connector(
        mut connector: BinanceConnector,
        hub_tx: broadcast::Sender<HubMessage>,
        _event_tx: broadcast::Sender<HubEvent>,
        shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    ) {
        // Subscribe to BinanceConnector's broadcast
        let mut binance_rx = connector.subscribe();

        loop {
            tokio::select! {
                // Forward Binance messages → Hub
                msg = binance_rx.recv() => {
                    match msg {
                        Ok(market_msg) => {
                            let hub_msg = Self::convert_message(market_msg);
                            if hub_tx.send(hub_msg).is_err() {
                                // No subscribers, but that's ok
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("WS Hub lagged {} messages, catching up", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("BinanceConnector channel closed, reconnecting...");
                            // Connector will auto-reconnect, re-subscribe
                            let new_rx = connector.subscribe();
                            binance_rx = new_rx;
                        }
                    }
                }
                // Shutdown signal
                _ = shutdown_rx => {
                    info!("WS Hub shutdown received");
                    break;
                }
            }
        }
    }

    /// Convert Binance MarketMessage → HubMessage
    fn convert_message(msg: MarketMessage) -> HubMessage {
        match msg {
            MarketMessage::Ticker {
                symbol,
                price,
                change,
                change_percent,
                volume,
                high,
                low,
                bid,
                ask,
                ..
            } => HubMessage::Ticker {
                symbol,
                price,
                change,
                change_pct: change_percent,
                volume,
                high,
                low,
                bid,
                ask,
            },
            MarketMessage::Depth {
                symbol,
                bids,
                asks,
                ..
            } => HubMessage::Depth { symbol, bids, asks },
            MarketMessage::Kline {
                symbol,
                interval,
                open,
                high,
                low,
                close,
                volume,
                ..
            } => HubMessage::Kline {
                symbol,
                interval,
                open,
                high,
                low,
                close,
                volume,
            },
        }
    }

    /// Get current subscriber count.
    pub fn subscriber_count(&self) -> usize {
        self.subscriber_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get hub's broadcast sender (for admin/monitoring).
    pub fn hub_sender(&self) -> broadcast::Sender<HubMessage> {
        self.tx.clone()
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for WsHub {
    fn clone(&self) -> Self {
        Self {
            shutdown_tx: self.shutdown_tx.clone(),
            tx: self.tx.clone(),
            hub_tx: self.hub_tx.clone(),
            subscriber_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}