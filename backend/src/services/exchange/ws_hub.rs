//! WebSocket Hub - Manages WS client connections and Binance WS data flow
//!
//! Single-source-of-truth for real-time market data distribution.
//! Forwards BinanceConnector broadcast messages to all connected WS clients.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::UNIX_EPOCH;
use tokio::sync::{Mutex, broadcast, mpsc};
use tracing::{debug, error, info, warn};

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
        timestamp: u64,
    },
    Kline {
        symbol: String,
        interval: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        timestamp: u64,
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
    /// Backtest progress update — broadcast to all subscribers of the specific channel
    BacktestProgress {
        backtest_id: uuid::Uuid,
        progress: u32,
        status: String,
    },
    /// AI prediction result — broadcast to all WS subscribers
    AIPredict {
        symbol: String,
        interval: String,
        direction: String,
        confidence: f64,
        signal: String,
        price_target: Option<f64>,
        analysis: String,
        indicators: serde_json::Value,
        generated_at: String,
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
    /// P0-F3: Timestamp of last received Binance message (心跳时间戳，0 = 从未收到)
    last_heartbeat: Arc<AtomicU64>,
    /// P0-F3: Disconnect threshold in seconds (默认 30s)
    disconnect_threshold_secs: Arc<AtomicU64>,
    /// P0-F3: Strategy pause flag — true = Binance 断连已触发自动暂停
    strategy_paused_by_disconnect: Arc<AtomicBool>,
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
            last_heartbeat: Arc::new(AtomicU64::new(0)),
            disconnect_threshold_secs: Arc::new(AtomicU64::new(30)),
            strategy_paused_by_disconnect: Arc::new(AtomicBool::new(false)),
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
            last_heartbeat: Arc::new(AtomicU64::new(0)),
            disconnect_threshold_secs: Arc::new(AtomicU64::new(30)),
            strategy_paused_by_disconnect: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set the KlineWriter sender (called from main.rs after KlineWriter is spawned)
    pub fn set_kline_writer_tx(&self, tx: mpsc::Sender<KlineRecord>) {
        let mut guard = self.kline_writer_tx.blocking_lock();
        *guard = Some(tx);
    }

    // ─── P0-F3: 断线检测与策略联动 ───────────────────────────────

    /// 返回当前连接状态（基于心跳是否活跃）
    pub fn is_binance_connected(&self) -> bool {
        let threshold = self.disconnect_threshold_secs.load(Ordering::SeqCst);
        let last = self.last_heartbeat.load(Ordering::SeqCst);
        if last == 0 {
            // 从未收到消息 → 认为断开
            false
        } else {
            let now = UNIX_EPOCH.elapsed().map(|d| d.as_secs()).unwrap_or(0);
            now.saturating_sub(last) < threshold
        }
    }

    /// F6: 返回策略是否因断线被暂停
    pub fn is_strategy_paused(&self) -> bool {
        self.strategy_paused_by_disconnect.load(Ordering::SeqCst)
    }

    /// 记录一次 Binance 心跳（收到消息时调用）
    pub fn record_heartbeat(&self) {
        if let Ok(now) = UNIX_EPOCH.elapsed() {
            self.last_heartbeat.store(now.as_secs(), Ordering::SeqCst);
        }
    }

    /// 检查断线并触发策略暂停（每秒调度一次）
    /// 返回 Some(断线秒数) 如果触发了暂停，否则 None
    pub fn check_disconnect_and_pause(&self) -> Option<u64> {
        if !self.is_binance_connected() {
            let threshold = self.disconnect_threshold_secs.load(Ordering::SeqCst);
            let last = self.last_heartbeat.load(Ordering::SeqCst);
            let now = UNIX_EPOCH.elapsed().map(|d| d.as_secs()).unwrap_or(0);
            let elapsed = if last == 0 {
                0 // 从未收到心跳，视为无断线（初始状态）
            } else {
                now.saturating_sub(last)
            };

            // 仅在首次超过阈值时触发
            if elapsed >= threshold
                && !self
                    .strategy_paused_by_disconnect
                    .swap(true, Ordering::SeqCst)
            {
                error!(
                    "Binance WebSocket disconnected for {}s (threshold: {}s), \
                     strategies paused",
                    elapsed, threshold
                );
                let _ = self.hub_tx.send(HubEvent::Disconnected);
                return Some(elapsed);
            }
        } else if self.strategy_paused_by_disconnect.load(Ordering::SeqCst) {
            // 重连了 → 检查是否需要恢复
            // 标记待恢复，下次 reconnect 事件触发真正恢复
        }
        None
    }

    /// P0-F3: Binance 重连成功后调用，恢复策略
    pub fn on_binance_reconnect(&self) {
        self.record_heartbeat();
        if self
            .strategy_paused_by_disconnect
            .swap(false, Ordering::SeqCst)
        {
            info!("Binance WebSocket reconnected, strategy auto-resumed");
            let _ = self.hub_tx.send(HubEvent::Connected);
        }
    }

    /// 获取断线状态（供 connection-status API 使用）
    pub fn get_connection_status(&self) -> ConnectionStatus {
        let threshold = self.disconnect_threshold_secs.load(Ordering::SeqCst);
        let last = self.last_heartbeat.load(Ordering::SeqCst);
        let now = UNIX_EPOCH.elapsed().map(|d| d.as_secs()).unwrap_or(0);
        let elapsed = if last == 0 {
            0 // 从未收到心跳，视为无断线（初始状态）
        } else {
            now.saturating_sub(last)
        };
        ConnectionStatus {
            exchange_connected: self.is_binance_connected(),
            disconnect_elapsed_secs: elapsed,
            disconnect_threshold_secs: threshold,
            strategy_paused: self.strategy_paused_by_disconnect.load(Ordering::SeqCst),
        }
    }

    /// 设置断线阈值（秒），供配置 API 调用
    pub fn set_disconnect_threshold(&self, secs: u64) {
        self.disconnect_threshold_secs.store(secs, Ordering::SeqCst);
    }

    /// Subscribe to market data (call from WS client handler).
    pub fn subscribe(&self) -> broadcast::Receiver<HubMessage> {
        self.tx.subscribe()
    }

    /// Broadcast a HubMessage to all subscribers.
    ///
    /// The error is boxed to keep the result type small — `HubMessage` is
    /// large (carries kline/ticker/trade payloads), so an unboxed error
    /// variant bloats every caller.
    pub fn broadcast(
        &self,
        msg: HubMessage,
    ) -> Result<usize, Box<broadcast::error::SendError<HubMessage>>> {
        // P0-3: count WS broadcast by HubMessage variant. The kind
        // label is the lowercase variant name so the same source of
        // truth (`HubMessage`) is used to mint the label, avoiding
        // a divergent string-table inside the dashboard.
        let kind = match &msg {
            HubMessage::Ticker { .. } => "ticker",
            HubMessage::Depth { .. } => "depth",
            HubMessage::Kline { .. } => "kline",
            HubMessage::TradeExecuted { .. } => "trade_executed",
            HubMessage::BacktestProgress { .. } => "backtest_progress",
            HubMessage::AIPredict { .. } => "ai_predict",
        };
        crate::metrics::WS_MESSAGES_BROADCAST_TOTAL
            .with_label_values(&[kind])
            .inc();
        self.tx.send(msg).map_err(Box::new)
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
        let _hub_tx = self.hub_tx.clone();
        let kline_writer_tx = self.kline_writer_tx.clone();
        let redis_cache = self.redis_cache.clone();
        let last_heartbeat = self.last_heartbeat.clone();
        let disconnect_threshold_secs = self.disconnect_threshold_secs.clone();
        let strategy_paused_by_disconnect = self.strategy_paused_by_disconnect.clone();
        let event_tx = self.hub_tx.clone();

        // Spawn connector + forwarder task
        tokio::spawn(async move {
            Self::run_connector(
                shutdown,
                tx,
                event_tx,
                kline_writer_tx,
                redis_cache,
                last_heartbeat,
                disconnect_threshold_secs,
                strategy_paused_by_disconnect,
            )
            .await;
        });

        info!("WebSocket Hub started");
    }

    /// Stop the hub (called on server shutdown)
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Run loop: connect to Binance, forward messages to hub broadcast.
    #[allow(clippy::too_many_arguments)]
    async fn run_connector(
        shutdown: Arc<AtomicBool>,
        hub_tx: broadcast::Sender<HubMessage>,
        event_tx: broadcast::Sender<HubEvent>,
        kline_writer_tx: Arc<Mutex<Option<mpsc::Sender<KlineRecord>>>>,
        redis_cache: Arc<Mutex<Option<crate::services::redis_cache::RedisCache>>>,
        last_heartbeat: Arc<AtomicU64>,
        disconnect_threshold_secs: Arc<AtomicU64>,
        strategy_paused_by_disconnect: Arc<AtomicBool>,
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
                    // P0-F3: 重置断线状态并记录心跳
                    strategy_paused_by_disconnect.store(false, Ordering::SeqCst);
                    if let Ok(now) = UNIX_EPOCH.elapsed() {
                        last_heartbeat.store(now.as_secs(), Ordering::SeqCst);
                    }
                    let _ = event_tx.send(HubEvent::Connected);
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
                    // Poll shutdown flag + P0-F3 disconnect detection every second
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                        if shutdown.load(Ordering::SeqCst) {
                            break;
                        }
                        // P0-F3: 检查断线是否触发策略暂停
                        let elapsed = {
                            let threshold = disconnect_threshold_secs.load(Ordering::SeqCst);
                            let last = last_heartbeat.load(Ordering::SeqCst);
                            if last == 0 {
                                // 从未收到消息 → 不视为断线（初始状态，等待首次心跳）
                                None
                            } else {
                                let now = UNIX_EPOCH.elapsed().map(|d| d.as_secs()).unwrap_or(0);
                                let gap = now.saturating_sub(last);
                                if gap >= threshold { Some(gap) } else { None }
                            }
                        };
                        if let Some(gap) = elapsed {
                            // 仅首次触发时暂停
                            if !strategy_paused_by_disconnect.swap(true, Ordering::SeqCst) {
                                error!(
                                    "Binance WS disconnected for {}s, strategies auto-paused",
                                    gap
                                );
                                let _ = event_tx.send(HubEvent::Disconnected);
                            }
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
                                // P0-F3: 记录心跳（每收到一条消息即刷新）
                                if let Ok(now) = UNIX_EPOCH.elapsed() {
                                    last_heartbeat.store(now.as_secs(), Ordering::SeqCst);
                                }
                                if msg_count.is_multiple_of(50) {
                                    info!("WS Hub processed {} messages (Kline={})",
                                        msg_count,
                                        std::matches!(market_msg, MarketMessage::Kline {..}));
                                }
                                // Debug: log first few Kline messages to confirm receipt
                                if matches!(market_msg, MarketMessage::Kline {..}) && msg_count <= 3
                                    && let MarketMessage::Kline { symbol, timestamp, .. } = &market_msg {
                                        debug!("Received Kline from Binance: {} at {}", symbol, timestamp);
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
                                        if tx.try_send(record).is_err() {
                                            debug!("KlineWriter channel full, dropping kline for {}", symbol);
                                        }
                                    } else {
                                        debug!("KlineWriter tx not set");
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
                symbol,
                bids,
                asks,
                timestamp,
            } => HubMessage::Depth {
                symbol,
                bids,
                asks,
                timestamp,
            },
            MarketMessage::Kline {
                symbol,
                interval,
                open,
                high,
                low,
                close,
                volume,
                timestamp,
                ..
            } => HubMessage::Kline {
                symbol,
                interval,
                open,
                high,
                low,
                close,
                volume,
                timestamp,
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
            last_heartbeat: self.last_heartbeat.clone(),
            disconnect_threshold_secs: self.disconnect_threshold_secs.clone(),
            strategy_paused_by_disconnect: self.strategy_paused_by_disconnect.clone(),
        }
    }
}

/// P0-F3: 连接状态数据结构（供 API 响应使用）
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    /// Binance 交易所连接是否活跃
    pub exchange_connected: bool,
    /// 距离上次收到消息的秒数
    pub disconnect_elapsed_secs: u64,
    /// 断线判定阈值（秒）
    pub disconnect_threshold_secs: u64,
    /// 策略是否因断线被自动暂停
    pub strategy_paused: bool,
}
