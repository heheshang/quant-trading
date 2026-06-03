//! Application shared state for all request handlers
//!
//! This module defines the `AppState` struct that holds all shared resources
//! accessible by route handlers via Axum's `State` extractor.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::db::DbPool;
use crate::services::binance_rest::BinanceRestClient;
use crate::services::exchange::ws_hub::WsHub;
use crate::services::redis_cache::RedisCache;
use crate::services::risk_manager::RiskManager;

/// P2-1: Process-wide cache of bound `user_id → telegram_chat_id`.
///
/// 中文：bind handler 写这个缓存；TelegramChannel 的 per-user 解析器从这里读。
///   同步读（`Fn(String) -> Option<String>`）要求解析器是 sync 的，所以走缓存而非 DB。
///   进程重启时缓存是空的 — 用户重新 bind 后即恢复。生产可换 DashMap 之类。
/// English: bind handler writes this cache; TelegramChannel's per-user resolver
///   reads from it. The `Fn(String) -> Option<String>` trait is sync, so we use
///   an in-memory cache rather than hitting the DB on every send. Cache is
///   empty on process start — users re-bind and it's populated. For production
///   consider DashMap or an LRU eviction policy.
#[derive(Default)]
pub struct TelegramChatIdCache {
    inner: RwLock<HashMap<Uuid, String>>,
}

impl TelegramChatIdCache {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// 中文：把 user → chat_id 写进缓存（bind handler 用）。
    /// English: Insert / replace a (user_id → chat_id) entry (called by bind).
    pub async fn set(&self, user_id: Uuid, chat_id: String) {
        let mut g = self.inner.write().await;
        g.insert(user_id, chat_id);
    }

    /// 中文：清掉某个 user 的绑定（unbind 流程用）。
    /// English: Remove a user's entry (called on unbind).
    #[allow(dead_code)]
    pub async fn remove(&self, user_id: Uuid) {
        let mut g = self.inner.write().await;
        g.remove(&user_id);
    }

    /// 中文：同步读，供 `Fn(String) -> Option<String>` 解析器用。
    /// English: Sync read for the `Fn(String) -> Option<String>` resolver trait.
    pub fn get_blocking(&self, user_id: &Uuid) -> Option<String> {
        // 优先尝试非阻塞读，失败时回退到 block_in_place。
        // First try a non-blocking read; fall back to block_in_place on contention.
        if let Ok(g) = self.inner.try_read() {
            return g.get(user_id).cloned();
        }
        tokio::task::block_in_place(|| {
            let h = tokio::runtime::Handle::current();
            h.block_on(async {
                let g = self.inner.read().await;
                g.get(user_id).cloned()
            })
        })
    }
}

/// Application shared state
///
/// Contains all resources needed by request handlers:
/// - `db`: PostgreSQL database connection pool
/// - `redis`: Redis cache for market data
/// - `binance`: Binance REST API client
/// - `risk_manager`: Risk management engine (shared across requests)
/// - `telegram_chat_ids`: P2-1 in-memory cache of bound user → chat_id mappings
#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub redis: Arc<RedisCache>,
    pub binance: Arc<BinanceRestClient>,
    pub ws_hub: Arc<WsHub>,
    pub risk_manager: Arc<RiskManager>,
    pub telegram_chat_ids: Arc<TelegramChatIdCache>,
}

impl AppState {
    /// Create a new AppState instance
    pub fn new(
        db: DbPool,
        redis: RedisCache,
        binance: BinanceRestClient,
        ws_hub: WsHub,
        risk_manager: RiskManager,
    ) -> Self {
        Self {
            db,
            redis: Arc::new(redis),
            binance: Arc::new(binance),
            ws_hub: Arc::new(ws_hub),
            risk_manager: Arc::new(risk_manager),
            telegram_chat_ids: Arc::new(TelegramChatIdCache::new()),
        }
    }
}

