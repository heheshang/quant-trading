//! services/notification/wechat.rs — P1-F6 企业微信 Webhook 通知
//!
//! 使用企业微信 Webhook 发送 Markdown 格式通知

use std::env;
use std::future::Future;
use std::pin::Pin;

use super::{AlertNotification, NotificationChannel};

/// 企业微信 Webhook 通知渠道
#[derive(Debug, Clone)]
pub struct WeChatChannel {
    webhook_url: Option<String>,
}

impl WeChatChannel {
    /// 从环境变量创建渠道
    /// 读取 WECHAT_WEBHOOK_URL，如果未配置则返回不发送任何内容的渠道
    pub fn from_env() -> Self {
        let webhook_url = env::var("WECHAT_WEBHOOK_URL").ok();
        Self { webhook_url }
    }

    /// 直接创建渠道
    pub fn new(webhook_url: String) -> Self {
        Self {
            webhook_url: Some(webhook_url),
        }
    }

    fn build_message(&self, notification: &AlertNotification) -> serde_json::Value {
        let symbol_str = notification.symbol.as_deref().unwrap_or("N/A");

        serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "content": format!(
                    "### {} [{}\n> **类型**: {}\n> **交易对**: {}\n> **时间**: {}\n> **详情**: {}",
                    notification.title,
                    notification.severity.to_uppercase(),
                    notification.alert_type,
                    symbol_str,
                    notification.triggered_at.format("%Y-%m-%d %H:%M:%S"),
                    notification.content
                )
            }
        })
    }
}

impl NotificationChannel for WeChatChannel {
    fn name(&self) -> &str {
        "wechat"
    }

    fn send<'a>(
        &'a self,
        notification: &'a AlertNotification,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        // If not configured, skip sending
        let webhook_url = match &self.webhook_url {
            Some(url) if !url.is_empty() => url.clone(),
            _ => {
                tracing::debug!("WeChat webhook not configured, skipping notification");
                return Box::pin(async { Ok(()) });
            }
        };

        let body = self.build_message(notification);

        Box::pin(async move {
            let client = reqwest::Client::new();
            client
                .post(&webhook_url)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("WeChat request failed: {}", e))?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_message() {
        let channel = WeChatChannel::from_env();
        let notification = AlertNotification::new(
            "Test Alert".to_string(),
            "Test content".to_string(),
            "position_alert".to_string(),
            "warning".to_string(),
            Some("BTCUSDT".to_string()),
        );

        let msg = channel.build_message(&notification);
        let content = msg["markdown"]["content"].as_str().unwrap();

        assert!(content.contains("Test Alert"));
        assert!(content.contains("BTCUSDT"));
        assert!(content.contains("position_alert"));
    }

    #[test]
    fn test_unconfigured_channel_skips_sending() {
        let channel = WeChatChannel::new(String::new());
        let notification = AlertNotification::new(
            "Test".to_string(),
            "Content".to_string(),
            "test".to_string(),
            "info".to_string(),
            None,
        );

        // Should return Ok even though nothing was sent
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async { channel.send(&notification).await });
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wechat_channel_with_valid_url() {
        // This test verifies the channel structure works correctly
        // Actual HTTP calls would require a mock server
        let channel = WeChatChannel::from_env();
        assert_eq!(channel.name(), "wechat");
    }
}
