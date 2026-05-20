//! services/alert_notification_service.rs — P1-F6 告警通知服务
//!
//! PRD: P1-F6 多渠道告警推送
//! 管理所有通知渠道，提供告警收敛逻辑

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::services::notification::{AlertNotification, NotificationChannel};

/// 告警通知服务
/// 管理所有通知渠道，支持告警收敛
pub struct AlertNotificationService {
    channels: RwLock<Vec<Arc<dyn NotificationChannel>>>,
    /// 告警收敛：key = "alert_type:symbol" -> 最近发送时间
    deduplication_cache: RwLock<HashMap<String, Instant>>,
    /// 收敛窗口（秒）
    deduplication_window_secs: u64,
}

impl AlertNotificationService {
    /// 创建新服务
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(Vec::new()),
            deduplication_cache: RwLock::new(HashMap::new()),
            deduplication_window_secs: 300, // 5 分钟收敛窗口
        }
    }

    /// 创建带自定义收敛窗口的服务
    pub fn with_window(window_secs: u64) -> Self {
        Self {
            channels: RwLock::new(Vec::new()),
            deduplication_cache: RwLock::new(HashMap::new()),
            deduplication_window_secs: window_secs,
        }
    }

    /// 添加通知渠道
    pub async fn add_channel<C: NotificationChannel + 'static>(&self, channel: C) {
        let mut channels = self.channels.write().await;
        channels.push(Arc::new(channel));
    }

    /// 移除指定名称的渠道
    pub async fn remove_channel(&self, name: &str) {
        let mut channels = self.channels.write().await;
        channels.retain(|c| c.name() != name);
    }

    /// 获取所有渠道名称
    pub async fn channel_names(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels.iter().map(|c| c.name().to_string()).collect()
    }

    /// 生成去重 key
    fn deduplication_key(notification: &AlertNotification) -> String {
        let symbol = notification.symbol.as_deref().unwrap_or("_global_");
        format!("{}:{}", notification.alert_type, symbol)
    }

    /// 检查是否应该跳过（告警收敛）
    async fn should_skip(&self, notification: &AlertNotification) -> bool {
        let key = Self::deduplication_key(notification);
        let window = Duration::from_secs(self.deduplication_window_secs);

        let mut cache = self.deduplication_cache.write().await;

        if let Some(last_sent) = cache.get(&key) {
            if last_sent.elapsed() < window {
                tracing::debug!(
                    "Deduplicating alert: key={}, elapsed={:?}",
                    key,
                    last_sent.elapsed()
                );
                return true;
            }
        }

        // 更新发送时间
        cache.insert(key, Instant::now());

        // 清理过期条目
        cache.retain(|_, v| v.elapsed() < window * 2);

        false
    }

    /// 发送告警通知（已收敛）
    pub async fn send_alert(&self, notification: AlertNotification) -> Result<(), String> {
        if self.should_skip(&notification).await {
            tracing::info!(
                "Alert deduplicated: type={}, symbol={:?}",
                notification.alert_type,
                notification.symbol
            );
            return Ok(());
        }

        let channels = self.channels.read().await;

        if channels.is_empty() {
            tracing::warn!("No notification channels configured");
            return Ok(());
        }

        let mut errors = Vec::new();

        for channel in channels.iter() {
            let name = channel.name().to_string();
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
    pub async fn send_alert_no_dedup(&self, notification: &AlertNotification) -> Result<(), String> {
        let channels = self.channels.read().await;

        if channels.is_empty() {
            return Ok(());
        }

        let mut errors = Vec::new();

        for channel in channels.iter() {
            let name = channel.name().to_string();
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
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use super::{AlertNotification, AlertNotificationService, NotificationChannel};

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
        fn with_failure(name: &str) -> Self {
            Self {
                name: name.to_string(),
                call_count: std::sync::atomic::AtomicUsize::new(0),
                should_fail: true,
            }
        }
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
            self.call_count
                .fetch_add(1, Ordering::SeqCst);
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
