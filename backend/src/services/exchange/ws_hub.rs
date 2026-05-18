//! WebSocket Hub - Manages WS client connections and Binance WS data flow
//!
//! Single-source-of-truth for real-time market data distribution.
//! Forwards BinanceConnector broadcast messages to all connected WS clients.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, broadcast, mpsc};
use tracing::{info, warn};

use crate::services::exchange::{BinanceConnector, MarketMessage};
use crate::services::kline_writer::KlineRecord;

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
    /// Trade execution notification — routed to specific user only
    TradeExecuted {
        user_id: uuid::Uuid,
        order_id: uuid::Uuid,
        symbol: String,
        side: String,
        filled_quantity: f64,
        avg_fill_price: f64,
        is_fully_filled: bool,
        realized_pnl: Option<f64>,
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
    /// Shutdown flag
    shutdown: Arc<AtomicBool>,
    /// Broadcast channel for WS clients to receive market data
    tx: broadcast::Sender<HubMessage>,
    /// Server-level event broadcast
    hub_tx: broadcast::Sender<HubEvent>,
    /// Channel to KlineWriter for DB persistence (Send+Sync safe)
    kline_writer_tx: Arc<Mutex<Option<mpsc::Sender<KlineRecord>>>>,
    /// Redis cache for ticker/depth data (updated on HubMessage::Ticker/Depth)
    redis_cache: Arc<Mutex<Option<crate::services::redis_cache::RedisCache>>>,
}

/// Builder for WsHub to set kline_writer_tx before start
pub struct WsHubBuilder {
    shutdown: Arc<AtomicBool>,
    tx: broadcast::Sender<HubMessage>,
    hub_tx: broadcast::Sender<HubEvent>,
    kline_writer_tx: Option<mpsc::Sender<KlineRecord>>,
    redis_cache: Option<crate::services::redis_cache::RedisCache>,
}

impl WsHubBuilder {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(2048);
        let (hub_tx, _) = broadcast::channel(100);
        Self {
            shutdown: Arc::new(AtomicBool::new(false)),
            tx,
            hub_tx,
            kline_writer_tx: None,
            redis_cache: None,
        }
    }

    pub fn with_kline_writer_tx(mut self, tx: mpsc::Sender<KlineRecord>) -> Self {
        self.kline_writer_tx = Some(tx);
        self
    }

    pub fn with_redis_cache(mut self, redis: crate::services::redis_cache::RedisCache) -> Self {
        self.redis_cache = Some(redis);
        self
    }

    pub fn build(self) -> WsHub {
        WsHub {
            shutdown: self.shutdown,
            tx: self.tx,
            hub_tx: self.hub_tx,
            kline_writer_tx: Arc::new(Mutex::new(self.kline_writer_tx)),
            redis_cache: Arc::new(Mutex::new(self.redis_cache)),
        }
    }
}

impl Default for WsHubBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}

impl WsHub {
    /// Create a new WsHub with broadcast channels.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(2048);
        let (hub_tx, _) = broadcast::channel(100);
        Self {
            shutdown: Arc::new(AtomicBool::new(false)),
            tx,
            hub_tx,
            kline_writer_tx: Arc::new(Mutex::new(None)),
            redis_cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the KlineWriter sender (called from main.rs after KlineWriter is spawned)
    pub fn set_kline_writer_tx(&self, tx: mpsc::Sender<KlineRecord>) {
        let mut guard = self.kline_writer_tx.blocking_lock();
        *guard = Some(tx);
    }

    /// Subscribe to market data (call from WS client handler).
    pub fn subscribe(&self) -> broadcast::Receiver<HubMessage> {
        self.tx.subscribe()
    }

    /// Broadcast a HubMessage to all subscribers
    pub fn broadcast(
        &self,
        msg: HubMessage,
    ) -> Result<usize, broadcast::error::SendError<HubMessage>> {
        self.tx.send(msg)
    }

    /// Subscribe to hub events (for monitoring/admin).
    pub fn subscribe_events(&self) -> broadcast::Receiver<HubEvent> {
        self.hub_tx.subscribe()
    }

    /// Start Binance connector and spawn message forwarder.
    /// Call once at server startup.
    pub fn start(&self) {
        let shutdown = self.shutdown.clone();
        let tx = self.tx.clone();
        let hub_tx = self.hub_tx.clone();
        let kline_writer_tx = self.kline_writer_tx.clone();
        let redis_cache = self.redis_cache.clone();

        // Spawn connector + forwarder task
        tokio::spawn(async move {
            Self::run_connector(shutdown, tx, hub_tx, kline_writer_tx, redis_cache).await;
        });

        info!("WebSocket Hub started");
    }

    /// Stop the hub (called on server shutdown)
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Run loop: connect to Binance, forward messages to hub broadcast.
    async fn run_connector(
        shutdown: Arc<AtomicBool>,
        hub_tx: broadcast::Sender<HubMessage>,
        _event_tx: broadcast::Sender<HubEvent>,
        kline_writer_tx: Arc<Mutex<Option<mpsc::Sender<KlineRecord>>>>,
        redis_cache: Arc<Mutex<Option<crate::services::redis_cache::RedisCache>>>,
    ) {
        let mut connector = BinanceConnector::new();
        let mut backoff_secs = 1u64;

        loop {
            if shutdown.load(Ordering::SeqCst) {
                info!("WS Hub shutdown detected");
                break;
            }

            // Connect with exponential backoff
            match connector.start().await {
                Ok(_) => {
                    info!("Binance WebSocket connected");
                    backoff_secs = 1; // reset backoff on success
                }
                Err(e) => {
                    warn!(
                        "BinanceConnector start failed: {}, retrying in {}s",
                        e, backoff_secs
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).clamp(1, 30);
                    // Create new connector for retry to avoid stale state
                    connector = BinanceConnector::new();
                    continue;
                }
            }

            let mut binance_rx = connector.subscribe();
            let mut msg_count: u64 = 0;
            info!("WS Hub subscribed to BinanceConnector, waiting for messages...");

            loop {
                tokio::select! {
                    // Poll shutdown flag periodically
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                        if shutdown.load(Ordering::SeqCst) {
                            break;
                        }
                        // Debug: if no messages after 10s, warn
                        if msg_count == 0 && std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs().is_multiple_of(10) {
                            info!("WS Hub: no messages yet, still waiting...");
                        }
                    }
                    // Forward Binance messages → Hub
                    msg = binance_rx.recv() => {
                        match msg {
                            Ok(market_msg) => {
                                msg_count += 1;
                                if msg_count.is_multiple_of(50) {
                                    info!("WS Hub processed {} messages (Kline={})",
                                        msg_count,
                                        std::matches!(market_msg, MarketMessage::Kline {..}));
                                }
                                let hub_msg = Self::convert_message(market_msg.clone());
                                if hub_tx.send(hub_msg).is_err() {
                                    // No subscribers, but that's ok
                                }
                                // Also send Kline data to KlineWriter for DB persistence
                                if let MarketMessage::Kline { symbol, interval, open, high, low, close, volume, close_time, timestamp } = &market_msg {
                                    let tx_guard = kline_writer_tx.lock().await;
                                    if let Some(ref tx) = *tx_guard {
                                        let record = KlineRecord {
                                            symbol: symbol.clone(),
                                            interval: interval.clone(),
                                            open_time: *timestamp as i64,
                                            close_time: *close_time as i64,
                                            open: rust_decimal::Decimal::try_from(*open).unwrap_or_default(),
                                            high: rust_decimal::Decimal::try_from(*high).unwrap_or_default(),
                                            low: rust_decimal::Decimal::try_from(*low).unwrap_or_default(),
                                            close: rust_decimal::Decimal::try_from(*close).unwrap_or_default(),
                                            volume: rust_decimal::Decimal::try_from(*volume).unwrap_or_default(),
                                            quote_volume: rust_decimal::Decimal::ZERO,
                                            trades: 0,
                                            source: "binance_ws".to_string(),
                                        };
                                        let _ = tx.try_send(record);
                                    }
                                }
                                // Update Redis cache for Ticker/Depth (best effort, non-blocking)
                                Self::update_redis_cache(&redis_cache, &market_msg).await;
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                warn!("WS Hub lagged {} messages, catching up", n);
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                info!("BinanceConnector channel closed, reconnecting...");
                                break; // exit inner loop, reconnect with new connector
                            }
                        }
                    }
                    // Poll shutdown flag periodically
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                        if shutdown.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                }
            }
            // Inner loop broke (channel closed) — back to outer loop for reconnect
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
                symbol, bids, asks, ..
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

    /// Update Redis cache with latest ticker/depth data (best effort, non-blocking).
    async fn update_redis_cache(
        redis_cache: &Arc<Mutex<Option<crate::services::redis_cache::RedisCache>>>,
        market_msg: &MarketMessage,
    ) {
        let redis_guard = redis_cache.lock().await;
        let Some(ref redis) = *redis_guard else {
            return;
        };

        match market_msg {
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
            } => {
                let ticker = crate::models::schemas::TickerResponse {
                    symbol: symbol.clone(),
                    price: *price,
                    change: *change,
                    change_percent: *change_percent,
                    volume: *volume,
                    high: *high,
                    low: *low,
                    bid: *bid,
                    ask: *ask,
                    timestamp: chrono::Utc::now().timestamp_millis(),
                };
                if let Err(e) = redis.set_ticker(&ticker).await {
                    warn!(error = %e, symbol = %symbol, "Failed to update ticker in Redis");
                }
            }
            MarketMessage::Depth {
                symbol, bids, asks, ..
            } => {
                use crate::models::market_schemas::DepthLevel;
                let depth = crate::models::market_schemas::DepthResponse {
                    bids: bids
                        .iter()
                        .map(|(p, q)| DepthLevel {
                            price: *p,
                            quantity: *q,
                            total: 0.0,
                        })
                        .collect(),
                    asks: asks
                        .iter()
                        .map(|(p, q)| DepthLevel {
                            price: *p,
                            quantity: *q,
                            total: 0.0,
                        })
                        .collect(),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                };
                if let Err(e) = redis.set_depth(symbol, &depth).await {
                    warn!(error = %e, symbol = %symbol, "Failed to update depth in Redis");
                }
            }
            _ => {}
        }
    }
}

impl Clone for WsHub {
    fn clone(&self) -> Self {
        Self {
            shutdown: self.shutdown.clone(),
            tx: self.tx.clone(),
            hub_tx: self.hub_tx.clone(),
            kline_writer_tx: self.kline_writer_tx.clone(),
            redis_cache: self.redis_cache.clone(),
        }
    }
}
