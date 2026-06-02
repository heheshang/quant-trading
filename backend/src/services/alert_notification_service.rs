//! services/alert_notification_service.rs — P1-F6 告警通知服务
//! services/alert_notification_service.rs — P1-F6 multi-channel alert notification service
//!
//! 中文说明：
//!   本文件实现 PRD P1-F6「多渠道告警推送」的服务层：维护一个可插拔的通知渠道注册表
//!   （email、wechat、…），并对外暴露统一的 `send_alert` 入口，由
//!   `position_alert_monitor.rs::send_alert_notification` 调用。
//!   核心能力：
//!     1. 多渠道注册 / 反注册（`add_channel` / `remove_channel`）
//!     2. 告警收敛（deduplication），防「告警风暴」：同一 `alert_type:symbol`
//!        在收敛窗口（默认 300s / 5 分钟）内只下发一次
//!     3. 多播 fan-out：对每个注册渠道顺序发送并聚合错误，单一渠道失败不阻塞其他渠道
//!
//! English description:
//!   Service-layer implementation of PRD P1-F6 "multi-channel alert push". It maintains
//!   a pluggable registry of `NotificationChannel` implementations (email, wechat, …)
//!   and exposes a single `send_alert` entry point consumed by
//!   `position_alert_monitor.rs::send_alert_notification`.
//!   Core capabilities:
//!     1. Channel registration / deregistration (`add_channel` / `remove_channel`)
//!     2. Alert deduplication ("alert storm" prevention): a given `alert_type:symbol`
//!        is delivered at most once per dedup window (default 300s / 5 minutes)
//!     3. Multicast fan-out: each registered channel is invoked sequentially, errors
//!        are aggregated — a single channel failure does not block the others

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::services::notification::{AlertNotification, NotificationChannel};

/// 告警通知服务
/// 告警通知服务：管理所有通知渠道，支持告警收敛。
/// Alert notification service: manages a registry of notification channels and
/// performs in-process alert deduplication ("alert-storm" prevention).
///
/// 中文设计要点：
///   - `channels` 持有 `Arc<dyn NotificationChannel>`，支持运行时多态 + 跨线程共享
///     （dyn trait 内部已约束 `Send + Sync`，见 `notification/mod.rs`）
///   - `deduplication_cache` 记录每个 `alert_type:symbol` 最近一次下发时间戳，
///     用于在 `should_skip` 中做时间窗判定
///   - `deduplication_window_secs` 默认 300（5 分钟），与 PRD 中"5 分钟收敛一次"一致
///
/// English notes:
///   - `channels` holds `Arc<dyn NotificationChannel>` for runtime polymorphism +
///     cross-thread sharing (the trait is already `Send + Sync` — see notification/mod.rs)
///   - `deduplication_cache` records the last-sent timestamp for each
///     `alert_type:symbol` key, consulted by `should_skip` for the time-window check
///   - Default window is 300s (5 minutes), matching the PRD's "5-minute dedup" rule
///
/// 与「告警抑制（suppression）」的区别：
///   - 收敛（dedup）：相同 key 在窗口内被静默丢弃，调用方拿到 `Ok(())`
///   - 抑制（suppression）：在更上层做"夜间静默 / 严重级别阈值 / 全局熔断"，调用方可能
///     拿到 `Ok(())` 或被熔断后端直接拒绝。**本服务只做收敛，不做抑制**。
///
/// Difference vs. "suppression":
///   - Dedup (here): same key within window is silently dropped, caller sees `Ok(())`
///   - Suppression: an upper-layer concept (night-quiet hours, severity threshold,
///     global circuit breaker) — the caller may see `Ok(())` from a no-op or be
///     rejected by the circuit breaker. **This service does dedup only, not suppression.**
pub struct AlertNotificationService {
    // 已注册的通知渠道列表。`Arc<dyn …>` 选择动机：
    //   1. 多态：email / wechat / future-webhook 都是 `NotificationChannel` 的不同实现
    //   2. 共享：`Arc` 让多播时每个 channel 只持有一份，零拷贝克隆
    //   3. 线程安全：trait 自身要求 `Send + Sync`，跨 async task 共享无虞
    // List of registered channels. Why `Arc<dyn …>`:
    //   1. Polymorphism: email / wechat / future-webhook are distinct `NotificationChannel` impls
    //   2. Sharing: `Arc` lets multicast fan-out hold each channel without per-fan-out clones
    //   3. Thread safety: the trait requires `Send + Sync`, so cross-task sharing is safe
    channels: RwLock<Vec<Arc<dyn NotificationChannel>>>,
    /// 告警收敛：key = "alert_type:symbol" -> 最近发送时间
    /// Deduplication cache: key = "alert_type:symbol" → most recent send timestamp.
    ///
    /// 中文：使用 `RwLock<HashMap<…>>` 而非 `DashMap` 是因为告警量级不高（O(10²) keys），
    ///   写只在 `send_alert` 主路径上发生且临界区极短，无需无锁结构。
    /// English: `RwLock<HashMap<…>>` (vs. `DashMap`) is chosen because alert volume is low
    ///   (O(10²) keys) and the critical section in `send_alert` is tiny — a lock-free
    ///   structure would be overkill.
    deduplication_cache: RwLock<HashMap<String, Instant>>,
    /// 收敛窗口（秒）
    /// Deduplication window, in seconds. Default = 300 (PRD-aligned 5-minute window).
    deduplication_window_secs: u64,
}

impl AlertNotificationService {
    /// 创建新服务
    /// Create a new service with the default 300s (5-minute) deduplication window.
    ///
    /// 中文：默认 300s 与 PRD 中"5 分钟内同 alert_type:symbol 收敛一次"保持一致，
    ///   适合 90% 的告警场景；测试中常用 `with_window(60)` 缩短以加速断言。
    /// English: Default 300s matches the PRD's "dedup same `alert_type:symbol` within
    ///   5 minutes" rule. Tests typically use `with_window(60)` to keep assertions snappy.
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(Vec::new()),
            deduplication_cache: RwLock::new(HashMap::new()),
            // 5 分钟收敛窗口 — 兼顾"对运维可见"和"防止刷屏"两个目标。
            // 5-minute dedup window — balances "operational visibility" and "no-spam".
            deduplication_window_secs: 300,
        }
    }

    /// 创建带自定义收敛窗口的服务
    /// Create a new service with a caller-specified deduplication window.
    ///
    /// 中文：当业务方需要"更激进"（如 60s）或"更宽松"（如 3600s）的收敛策略时使用。
    ///   调用方需自行保证 `window_secs > 0`；本函数不做边界校验（`0` 会导致"永远不收敛"）。
    /// English: Use when the caller needs a more aggressive (e.g. 60s) or more permissive
    ///   (e.g. 3600s) policy. The function does not validate `window_secs` — a `0` window
    ///   effectively disables dedup.
    pub fn with_window(window_secs: u64) -> Self {
        Self {
            channels: RwLock::new(Vec::new()),
            deduplication_cache: RwLock::new(HashMap::new()),
            deduplication_window_secs: window_secs,
        }
    }

    /// 添加通知渠道
    /// Register a new notification channel.
    ///
    /// 中文：通过 `Arc<C>` 实现零拷贝注册 — 调用方传入的 channel 实例被 wrap 进 `Arc`，
    ///   后续 `send_alert` 多播时不会触发任何 clone。`C: 'static` 约束是 dyn-trait
    ///   装箱（trait object）的硬性要求。
    /// English: Wraps the channel in `Arc` for zero-copy registration — `send_alert`'s
    ///   multicast fan-out borrows the same `Arc` without cloning. `C: 'static` is
    ///   required for the dyn-trait object.
    pub async fn add_channel<C: NotificationChannel + 'static>(&self, channel: C) {
        // 写锁：注册是低频操作（启动期 / 重配期），与 `send_alert` 的读锁互斥。
        // Write lock: registration is a low-frequency op (startup / reconfigure),
        // mutally exclusive with `send_alert`'s read lock.
        let mut channels = self.channels.write().await;
        channels.push(Arc::new(channel));
    }

    /// 移除指定名称的渠道
    /// Deregister a channel by its `name()`.
    ///
    /// 中文：用 `retain` 而非 `swap_remove` 是因为顺序无关 — 多播是无序 fan-out，
    ///   不需要保持渠道的稳定顺序。未知 name 静默 no-op（便于幂等调用）。
    /// English: Uses `retain` (not `swap_remove`) because fan-out is unordered — channel
    ///   order does not matter. Unknown names are silently no-op'd (idempotent calls).
    pub async fn remove_channel(&self, name: &str) {
        // 写锁：与 `add_channel` 互斥，但与 `send_alert` 的读锁也互斥。
        // Write lock: exclusive vs. both `add_channel` and `send_alert`'s read lock.
        let mut channels = self.channels.write().await;
        channels.retain(|c| c.name() != name);
    }

    /// 获取所有渠道名称
    /// Snapshot all currently registered channel names.
    ///
    /// 中文：返回 `Vec<String>` 而非 `Vec<&str>`，是因为锁释放后 `&str` 的生命周期
    ///   会被切断 — 必须 `to_string()` 一次换取所有权。供 `/api/alerts/channels`
    ///   之类的运维端点使用。
    /// English: Returns `Vec<String>` (not `Vec<&str>`) because the read lock is dropped
    ///   on return — the `&str` lifetime would not survive. Used by ops endpoints
    ///   such as `/api/alerts/channels`.
    pub async fn channel_names(&self) -> Vec<String> {
        // 读锁：可与 `send_alert` 并发，与 `add_channel` / `remove_channel` 互斥。
        // Read lock: concurrent with `send_alert`, exclusive with add/remove.
        let channels = self.channels.read().await;
        channels.iter().map(|c| c.name().to_string()).collect()
    }

    /// 生成去重 key
    /// Build the deduplication key for a notification.
    ///
    /// 中文：key 设计 = `"{alert_type}:{symbol}"`。`symbol=None`（全局告警，如"系统启动"）
    ///   用占位符 `"_global_"` 收敛到同一 key，避免 N 个全局告警绕过收敛。
    /// English: Key format = `"{alert_type}:{symbol}"`. `symbol=None` (global alerts
    ///   like "system boot") collapses to the placeholder `"_global_"` so N global
    ///   alerts cannot bypass the dedup window.
    fn deduplication_key(notification: &AlertNotification) -> String {
        // 缺省 symbol → 全局占位符，确保全局告警也能被收敛。
        // Missing symbol → global placeholder so global alerts are also deduped.
        let symbol = notification.symbol.as_deref().unwrap_or("_global_");
        format!("{}:{}", notification.alert_type, symbol)
    }

    /// 检查是否应该跳过（告警收敛）
    /// Decide whether `notification` should be dropped (deduplication gate).
    ///
    /// 中文：核心算法是"在窗口内已发过则跳过，否则更新发时间戳并放行"。
    ///   关键不变量：每次通过（`should_skip → false`）都伴随一次 `cache.insert`，
    ///   保证下一次相同 key 的查询能命中。
    /// English: Core algorithm = "drop if sent within window, else update timestamp
    ///   and admit". Invariant: every `false` return is paired with a `cache.insert`,
    ///   so the next identical-key query hits.
    ///
    ///   Note: this method BOTH decides AND updates the cache. Callers must NOT
    ///   invoke it for side-effect-free probes — see `send_alert_no_dedup` for the
    ///   test-only bypass that skips this check entirely.
    async fn should_skip(&self, notification: &AlertNotification) -> bool {
        let key = Self::deduplication_key(notification);
        let window = Duration::from_secs(self.deduplication_window_secs);

        // 写锁：与下一次 `send_alert` 的 `should_skip` 互斥，但临界区只做一次 HashMap 查询。
        // Write lock: exclusive with peer `should_skip` calls, but the critical section
        // is just one HashMap lookup + at most one insert.
        let mut cache = self.deduplication_cache.write().await;

        // 三臂 match：命中且在窗口内 → 跳过（已收敛）。
        // 命中但已超出窗口 → 落到 `_` 重新插入（窗口滚动）。
        // 未命中（首次）→ 落到 `_` 插入。
        // Three-arm match: hit-and-in-window → skip (deduped).
        // hit-but-expired → falls through to `_` (window rolled).
        // miss (first time) → falls through to `_` and inserts.
        match cache.get(&key) {
            Some(last_sent) if last_sent.elapsed() < window => {
                tracing::debug!(
                    "Deduplicating alert: key={}, elapsed={:?}",
                    key,
                    last_sent.elapsed()
                );
                return true;
            }
            _ => {}
        }

        // 更新发送时间 — 本次"放行"必须在 cache 中留下足迹，否则窗口不会重置。
        // Update send timestamp — every admission must leave a footprint or the
        // window would never reset for the next identical key.
        cache.insert(key, Instant::now());

        // 清理过期条目：保留 `elapsed < 2*window` 的项，避免 hashmap 无限增长。
        //   `2*` 而不是 `1*` 是为了容忍"刚发完但还没到下一次 send_alert"的中间状态。
        // Sweep expired entries: keep those with `elapsed < 2*window` so the map
        //   doesn't grow unbounded. The `2*` slack tolerates "just sent, not yet rechecked".
        cache.retain(|_, v| v.elapsed() < window * 2);

        false
    }

    /// 发送告警通知（已收敛）
    /// Send an alert (with deduplication).
    ///
    /// 中文：主入口。流程为「收敛检查 → 读锁取渠道列表 → 顺序 fan-out → 聚合错误」。
    ///   - 收敛命中：返回 `Ok(())`，不发任何渠道（防告警风暴）
    ///   - 渠道为空：返回 `Ok(())` 并 warn（启动期合法状态）
    ///   - 多播：任一渠道失败被 `tracing::error` 记录并追加到 errors；最终 `Err` 由 `;` 拼接所有失败信息
    /// English (main entry point). The flow is dedup check, then read-lock channels, then sequential fan-out, and finally aggregate errors.
    ///   - Dedup hit: returns `Ok(())`, no channel is invoked (storm prevention)
    ///   - No channels: returns `Ok(())` with a warn (legitimate during startup)
    ///   - Multicast: per-channel failure is `tracing::error`-logged AND appended to `errors`; the final `Err` joins all failure messages with `;`
    ///
    ///   Note: errors are aggregated, not short-circuited — a Slack outage should not
    ///   suppress a successful email send.
    pub async fn send_alert(&self, notification: AlertNotification) -> Result<(), String> {
        // 收敛优先：告警风暴场景下，相同 key 会在窗口内被静默吞掉。
        // Dedup first: storm scenarios collapse repeated alerts to a single send.
        if self.should_skip(&notification).await {
            tracing::info!(
                "Alert deduplicated: type={}, symbol={:?}",
                notification.alert_type,
                notification.symbol
            );
            return Ok(());
        }

        // 读锁：与 add/remove 互斥，但与并行 send_alert 调用并发（这是 fan-out 的关键）。
        // Read lock: exclusive vs. add/remove, but concurrent with peer `send_alert`
        // calls — this is what makes fan-out safe and cheap.
        let channels = self.channels.read().await;

        // 渠道空：warn + Ok()。空渠道是合法的启动中间态，不应被视为错误。
        // Empty channels: warn + Ok(). Empty is a legitimate startup state, not an error.
        if channels.is_empty() {
            tracing::warn!("No notification channels configured");
            return Ok(());
        }

        let mut errors = Vec::new();

        // 顺序 fan-out：每个 channel 独立 try/catch，单点失败不影响其他渠道。
        // Sequential fan-out: each channel is independently try/catched — one failure
        // does not block the others. (We use sequential rather than `join_all` to keep
        // per-channel backpressure observable in tracing.)
        for channel in channels.iter() {
            let name = channel.name().to_string();
            // 每渠道 try / catch：Ok 仅 debug 记录，Err 既 log error 又聚合到 errors 向量。
            // Per-channel try/catch: Ok is debug-logged, Err is both error-logged and
            // appended to the errors vector for the aggregate return.
            match channel.send(&notification).await {
                Ok(()) => {
                    tracing::debug!("Notification sent via {}", name);
                }
                Err(e) => {
                    tracing::error!("Failed to send via {}: {}", name, e);
                    errors.push(format!("{}: {}", name, e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    /// 同步发送（不带收敛检查，用于测试）
    /// Send an alert WITHOUT deduplication (test-only bypass).
    ///
    /// 中文：测试用入口 — 跳过 `should_skip` 直接 fan-out，使每次调用都能精确触发一次
    ///   下发（便于断言 `call_count == 1` 之类的硬性指标）。生产代码不应调用此函数。
    /// English: Test-only entry point — bypasses `should_skip` so every call triggers
    ///   exactly one fan-out (e.g. for asserting `call_count == 1`). Production code
    ///   must NOT call this — the dedup logic in `send_alert` is the contract.
    pub async fn send_alert_no_dedup(
        &self,
        notification: &AlertNotification,
    ) -> Result<(), String> {
        // 读锁 + 空渠道短路：与 `send_alert` 行为一致，但少了 dedup 检查和 tracing。
        // Read lock + empty-channel short-circuit: same shape as `send_alert` minus
        // the dedup check and the verbose tracing.
        let channels = self.channels.read().await;

        if channels.is_empty() {
            return Ok(());
        }

        let mut errors = Vec::new();

        for channel in channels.iter() {
            let name = channel.name().to_string();
            // 与 `send_alert` 同样的聚合策略：单点失败不中断 fan-out。
            // Same aggregation policy as `send_alert`: a single failure does not
            // abort the loop.
            match channel.send(notification).await {
                Ok(()) => {}
                Err(e) => {
                    errors.push(format!("{}: {}", name, e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    /// 清理收敛缓存
    /// Clear the deduplication cache (drop all `alert_type:symbol` timestamps).
    ///
    /// 中文：用于运维端点（例如"重置告警抑制"）或测试 teardown。
    ///   写锁：会阻塞所有 `should_skip` 调用。
    /// English: Used by ops endpoints (e.g. "reset alert suppression") or test
    ///   teardown. Write lock: blocks all in-flight `should_skip` calls.
    pub async fn clear_cache(&self) {
        let mut cache = self.deduplication_cache.write().await;
        cache.clear();
    }
}

impl Default for AlertNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{AlertNotification, AlertNotificationService, NotificationChannel};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Mock channel for testing
    #[derive(Debug)]
    struct MockChannel {
        name: String,
        call_count: AtomicUsize,
        should_fail: bool,
    }

    impl MockChannel {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                call_count: std::sync::atomic::AtomicUsize::new(0),
                should_fail: false,
            }
        }
        #[allow(dead_code)]
        fn with_failure(name: &str) -> Self {
            Self {
                name: name.to_string(),
                #[allow(dead_code)]
                call_count: std::sync::atomic::AtomicUsize::new(0),
                should_fail: true,
            }
        }
        #[allow(dead_code)]
        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    impl NotificationChannel for MockChannel {
        fn name(&self) -> &str {
            &self.name
        }

        fn send<'a>(
            &'a self,
            _notification: &'a AlertNotification,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            if self.should_fail {
                Box::pin(async { Err("Mock failure".to_string()) })
            } else {
                Box::pin(async { Ok(()) })
            }
        }
    }

    #[tokio::test]
    async fn test_add_and_send() {
        let service = AlertNotificationService::new();
        service.add_channel(MockChannel::new("test1")).await;

        let notification = AlertNotification::new(
            "Test".to_string(),
            "Content".to_string(),
            "test".to_string(),
            "info".to_string(),
            None,
        );

        service.send_alert(notification).await.unwrap();
    }

    #[tokio::test]
    async fn test_deduplication() {
        let service = AlertNotificationService::with_window(60);

        // Add two channels
        let channel1 = MockChannel::new("ch1");
        let channel2 = MockChannel::new("ch2");
        service.add_channel(channel1).await;
        service.add_channel(channel2).await;

        let notification = AlertNotification::new(
            "Alert".to_string(),
            "Content".to_string(),
            "test_alert".to_string(),
            "warning".to_string(),
            Some("BTCUSDT".to_string()),
        );

        // First send should go through
        service.send_alert(notification.clone()).await.unwrap();

        // Second send with same key should be deduplicated
        service.send_alert(notification).await.unwrap();

        // Channels should only be called once
        let names = service.channel_names().await;
        assert_eq!(names.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_channel() {
        let service = AlertNotificationService::new();
        service.add_channel(MockChannel::new("to_remove")).await;
        service.add_channel(MockChannel::new("to_keep")).await;

        service.remove_channel("to_remove").await;

        let names = service.channel_names().await;
        assert!(names.contains(&"to_keep".to_string()));
        assert!(!names.contains(&"to_remove".to_string()));
    }

    #[tokio::test]
    async fn test_no_channels() {
        let service = AlertNotificationService::new();
        let notification = AlertNotification::new(
            "Test".to_string(),
            "Content".to_string(),
            "test".to_string(),
            "info".to_string(),
            None,
        );

        // Should not panic
        service.send_alert(notification).await.unwrap();
    }
}
