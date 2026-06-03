//! services/notification/mod.rs — P1-F6 多渠道告警通知
//!
//! PRD: P1-F6 多渠道告警推送
//! 定义告警通知消息结构和通知渠道 trait

pub mod email;
pub mod telegram;
pub mod wechat;

pub use email::EmailChannel;
pub use telegram::TelegramChannel;
pub use wechat::WeChatChannel;

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// 告警通知消息
#[derive(Debug, Clone)]
pub struct AlertNotification {
    /// 通知标题
    pub title: String,
    /// 通知内容
    pub content: String,
    /// 告警类型: "position_alert", "risk_alert", "system_alert"
    pub alert_type: String,
    /// 严重级别: "info", "warning", "critical"
    pub severity: String,
    /// 交易对
    pub symbol: Option<String>,
    /// 触发时间
    pub triggered_at: DateTime<Utc>,
    /// 额外元数据
    pub metadata: Value,
}

impl AlertNotification {
    /// 创建新通知
    pub fn new(
        title: String,
        content: String,
        alert_type: String,
        severity: String,
        symbol: Option<String>,
    ) -> Self {
        Self {
            title,
            content,
            alert_type,
            severity,
            symbol,
            triggered_at: Utc::now(),
            metadata: Value::Object(serde_json::Map::new()),
        }
    }

    /// 创建带元数据的新通知
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl serde::Serialize) -> Self {
        if let (Ok(v), Some(obj)) = (serde_json::to_value(value), self.metadata.as_object_mut()) {
            obj.insert(key.into(), v);
        }
        self
    }
}

/// 通知渠道 trait（异步版本，使用 Pin<Box<dyn Future>> 以支持动态分发）
pub trait NotificationChannel: Send + Sync {
    /// 渠道名称
    fn name(&self) -> &str;
    /// 发送通知（返回 pinned boxed future，生命周期与 notification 绑定）
    fn send<'a>(
        &'a self,
        notification: &'a AlertNotification,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}
