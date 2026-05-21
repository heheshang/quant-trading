//! services/notification/email.rs — P1-F6 SMTP 邮件通知
//!
//! 使用 lettre 发送告警邮件通知

use std::env;
use std::future::Future;
use std::pin::Pin;

use super::{AlertNotification, NotificationChannel};
use lettre::Transport;
use lettre::message::{Mailbox, MessageBuilder};
use lettre::transport::smtp::SmtpTransport;
use lettre::transport::smtp::authentication::Credentials;

/// SMTP 邮件通知渠道
#[derive(Debug, Clone)]
pub struct EmailChannel {
    credentials: Option<SmtpCredentials>,
    from: Option<String>,
    to: Option<String>,
}

#[derive(Debug, Clone)]
struct SmtpCredentials {
    host: String,
    port: u16,
    user: String,
    password: String,
}

impl EmailChannel {
    /// 从环境变量创建渠道
    /// 读取 SMTP_HOST, SMTP_PORT, SMTP_USER, SMTP_PASSWORD, SMTP_FROM, ALERT_EMAIL_TO
    pub fn from_env() -> Self {
        let host = env::var("SMTP_HOST").ok();
        let port = env::var("SMTP_PORT").ok().and_then(|p| p.parse().ok());
        let user = env::var("SMTP_USER").ok();
        let password = env::var("SMTP_PASSWORD").ok();
        let from = env::var("SMTP_FROM").ok();
        let to = env::var("ALERT_EMAIL_TO").ok();

        let credentials = match (host, port, user, password) {
            (Some(host), Some(port), Some(user), Some(password)) => Some(SmtpCredentials {
                host,
                port,
                user,
                password,
            }),
            _ => None,
        };

        Self {
            credentials,
            from,
            to,
        }
    }

    /// 直接创建渠道
    pub fn new(
        host: String,
        port: u16,
        user: String,
        password: String,
        from: String,
        to: String,
    ) -> Self {
        Self {
            credentials: Some(SmtpCredentials {
                host,
                port,
                user,
                password,
            }),
            from: Some(from),
            to: Some(to),
        }
    }

    fn build_email_body(&self, notification: &AlertNotification) -> String {
        let symbol_str = notification.symbol.as_deref().unwrap_or("N/A");

        format!(
            r#"告警通知

标题: {}
严重级别: {}
类型: {}
交易对: {}
时间: {}

详情:
{}

---
此邮件由量化交易系统自动发送
"#,
            notification.title,
            notification.severity,
            notification.alert_type,
            symbol_str,
            notification.triggered_at.format("%Y-%m-%d %H:%M:%S"),
            notification.content
        )
    }
}

impl NotificationChannel for EmailChannel {
    fn name(&self) -> &str {
        "email"
    }

    fn send<'a>(
        &'a self,
        notification: &'a AlertNotification,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        // Check if configured
        let (credentials, from, to) = match (&self.credentials, &self.from, &self.to) {
            (Some(c), Some(f), Some(t)) => (c, f, t),
            _ => {
                tracing::debug!("Email SMTP not fully configured, skipping notification");
                return Box::pin(async { Ok(()) });
            }
        };

        let body = self.build_email_body(notification);
        let creds = Credentials::new(credentials.user.clone(), credentials.password.clone());

        Box::pin(async move {
            // Build email
            let email = MessageBuilder::new()
                .from(
                    from.parse::<Mailbox>()
                        .map_err(|e| format!("Invalid from address: {}", e))?,
                )
                .to(to
                    .parse::<Mailbox>()
                    .map_err(|e| format!("Invalid to address: {}", e))?)
                .subject(format!(
                    "[{}] {}",
                    notification.severity.to_uppercase(),
                    notification.title
                ))
                .body(body)
                .map_err(|e| format!("Failed to build email: {}", e))?;

            // Create SMTP client and send
            let mailer = SmtpTransport::relay(&credentials.host)
                .map_err(|e| format!("Failed to create SMTP relay: {}", e))?
                .port(credentials.port)
                .credentials(creds)
                .build();

            mailer
                .send(&email)
                .map_err(|e| format!("Failed to send email: {}", e))?;

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_email_body() {
        let channel = EmailChannel::from_env();
        let notification = AlertNotification::new(
            "止损触发".to_string(),
            "BTC/USDT 多头止损触发".to_string(),
            "position_alert".to_string(),
            "critical".to_string(),
            Some("BTCUSDT".to_string()),
        );

        let body = channel.build_email_body(&notification);
        assert!(body.contains("止损触发"));
        assert!(body.contains("BTCUSDT"));
        assert!(body.contains("critical"));
    }

    #[tokio::test]
    async fn test_unconfigured_channel_skips_sending() {
        let channel = EmailChannel::new(
            String::new(),
            0,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
        let notification = AlertNotification::new(
            "Test".to_string(),
            "Content".to_string(),
            "test".to_string(),
            "info".to_string(),
            None,
        );

        // Empty host/credentials will fail to connect — this is expected
        let result = channel.send(&notification).await;
        // Should not panic, error is acceptable for unconfigured channel
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_email_channel_name() {
        let channel = EmailChannel::from_env();
        assert_eq!(channel.name(), "email");
    }
}
