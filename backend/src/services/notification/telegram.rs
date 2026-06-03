//! services/notification/telegram.rs — P2-1 Telegram Bot 通知渠道
//!
//! 中文说明：
//!   实现 PRD P2-1「Telegram Bot 通知通道」。使用 Telegram Bot API `sendMessage`
//!   端点推送 Markdown 格式的告警；当单条消息超过 4096 字符（Telegram 限制）时
//!   自动拆分成多条顺序发送。
//!
//! English description:
//!   Implements PRD P2-1 "Telegram Bot notification channel". Pushes Markdown-formatted
//!   alerts via the Telegram Bot API `sendMessage` endpoint. Messages exceeding the
//!   4096-character Telegram limit are automatically split into multiple sequential
//!   messages.

use std::env;
use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};
#[cfg(test)]
use serde_json::json;

use super::{AlertNotification, NotificationChannel};

/// Telegram Bot API 单条消息的硬上限
/// Telegram Bot API per-message hard limit (characters).
const TELEGRAM_MAX_MESSAGE_LEN: usize = 4096;

/// Telegram Bot 通知渠道
/// Telegram Bot notification channel.
///
/// 中文：每个 user 通过 `chat_id` 标识，Telegram Bot API 通过 `bot_token`+`chat_id` 寻址。
///   默认 API 端点为 `https://api.telegram.org/bot{token}/sendMessage`。
/// English: Each user is identified by a `chat_id`; the Telegram Bot API is addressed
///   by `bot_token` + `chat_id`. Default API endpoint is
///   `https://api.telegram.org/bot{token}/sendMessage`.
#[derive(Clone)]
pub struct TelegramChannel {
    /// Bot Token (env: TELEGRAM_BOT_TOKEN)
    bot_token: Option<String>,
    /// 默认 chat_id（per-channel 模式 / 测试用）
    /// Default chat_id (per-channel mode / for testing).
    default_chat_id: Option<String>,
    /// 解析 chat_id 的函数：传入 user_id 返回 Some(chat_id) 或 None
    /// Resolver function: given a user_id returns Some(chat_id) or None.
    /// 生产模式下由 AppState 注入；未注入时视为未配置。
    /// Injected by AppState at production startup; absent = unconfigured.
    /// Note: `dyn Fn(String) -> Option<String> + Send + Sync` does not impl Debug,
    /// so we use a manual Debug impl (the field is set once at startup and
    /// never re-inspected; the resolver itself is exercised via
    /// `resolve_chat_id`).
    chat_id_resolver: Option<ChatIdResolver>,
}

impl std::fmt::Debug for TelegramChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramChannel")
            .field("bot_token", &self.bot_token.as_ref().map(|_| "***redacted***"))
            .field("default_chat_id", &self.default_chat_id)
            .field(
                "chat_id_resolver",
                &self.chat_id_resolver.as_ref().map(|_| "<fn>"),
            )
            .finish()
    }
}

/// chat_id 解析器签名：输入 user_id (UUID 字符串)，输出该用户的 Telegram chat_id。
/// Signature of the chat_id resolver: takes a user_id (UUID string), returns the
/// user's Telegram chat_id.
///
/// 中文：使用 `Arc<dyn Fn …>` 让解析器在 `send` 中按值捕获（Pin<Box<dyn Future>> 要求
///   'static），同时支持任意闭包 / DbPool 注入 / mock。
/// English: `Arc<dyn Fn …>` lets the resolver be captured by value into the boxed
///   future (required by Pin<Box<dyn Future>> for 'static) and accepts any closure,
///   DbPool-backed function, or mock.
pub type ChatIdResolver =
    std::sync::Arc<dyn Fn(String) -> Option<String> + Send + Sync + 'static>;

/// Telegram `sendMessage` API 的请求体结构
/// Telegram `sendMessage` API request body.
#[derive(Debug, Serialize)]
struct SendMessageRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
    parse_mode: &'a str,
    disable_web_page_preview: bool,
}

/// Telegram API 响应（成功路径只取 `ok`，失败时读 `description`）
/// Telegram API response (success reads `ok`, failure reads `description`).
#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    error_code: Option<i32>,
}

impl TelegramChannel {
    /// 从环境变量创建渠道（per-channel 模式）
    /// Create from env vars (per-channel mode).
    ///
    /// 中文：读取 `TELEGRAM_BOT_TOKEN` 和 `TELEGRAM_DEFAULT_CHAT_ID`。
    ///   若任一缺失则返回的渠道在 `send` 时跳过（与 email/wechat 的"未配置即静默"语义一致）。
    /// English: Reads `TELEGRAM_BOT_TOKEN` and `TELEGRAM_DEFAULT_CHAT_ID`. If either is
    ///   missing the returned channel silently no-ops on `send` (matching the
    ///   email/wechat "unconfigured = silent" contract).
    pub fn from_env() -> Self {
        let bot_token = env::var("TELEGRAM_BOT_TOKEN").ok();
        let default_chat_id = env::var("TELEGRAM_DEFAULT_CHAT_ID").ok();
        Self {
            bot_token,
            default_chat_id,
            chat_id_resolver: None,
        }
    }

    /// 直接创建渠道（per-channel 模式 + 显式参数）
    /// Direct constructor (per-channel mode, explicit args).
    pub fn new(bot_token: String, default_chat_id: String) -> Self {
        Self {
            bot_token: Some(bot_token),
            default_chat_id: Some(default_chat_id),
            chat_id_resolver: None,
        }
    }

    /// 设置 per-user chat_id 解析器
    /// Attach a per-user `chat_id` resolver.
    ///
    /// 中文：生产模式下调用方会传入一个 `Arc<dyn Fn(String) -> Option<String>>`，
    ///   内部从 user 表查 `telegram_chat_id`。返回 `None` 时该 user 跳过。
    /// English: Production callers inject an `Arc<dyn Fn(String) -> Option<String>>`
    ///   that consults the user table for `telegram_chat_id`. `None` = skip the user.
    pub fn with_resolver(mut self, resolver: ChatIdResolver) -> Self {
        self.chat_id_resolver = Some(resolver);
        self
    }

    /// 把一条 `AlertNotification` 渲染成 Telegram Markdown 文本
    /// Render an `AlertNotification` as Telegram Markdown text.
    ///
    /// 中文：使用 Telegram "MarkdownV2" 风格的子集（`*bold*` / `_italic_` /
    ///   `` `code` `` / `[link](url)`），最大可读性。如内容里出现 Telegram MarkdownV2
    ///   转义字符（`_ * [ ] ( ) ~ \` > # + - = | { } . !`），简单做一次最小化转义，
    ///   避免消息被 Telegram 拒收。
    /// English: Uses a subset of Telegram's "MarkdownV2" style (`*bold*` / `_italic_` /
    ///   `` `code` `` / `[link](url)`) for maximum readability. Special MarkdownV2
    ///   meta-characters are minimally escaped to avoid Telegram rejection.
    pub fn build_message(&self, notification: &AlertNotification) -> String {
        let symbol_str = notification.symbol.as_deref().unwrap_or("N/A");
        let severity_marker = severity_emoji(&notification.severity);
        let timestamp = notification
            .triggered_at
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let content_escaped = escape_markdownv2(&notification.content);
        let title_escaped = escape_markdownv2(&notification.title);

        format!(
            "{marker} *{title}*\n\
             \n\
             *Severity*: `{severity}`\n\
             *Type*: `{alert_type}`\n\
             *Symbol*: `{symbol}`\n\
             *Time*: `{time}`\n\
             \n\
             {content}\n\
             \n\
             _quant-trading alert · auto-generated_",
            marker = severity_marker,
            title = title_escaped,
            severity = escape_markdownv2(&notification.severity),
            alert_type = escape_markdownv2(&notification.alert_type),
            symbol = escape_markdownv2(symbol_str),
            time = timestamp,
            content = content_escaped,
        )
    }

    /// 把超过长度上限的文本按 UTF-8 字符边界拆成多块
    /// Split over-long text into chunks on UTF-8 char boundaries.
    ///
    /// 中文：使用 `char_indices` 找到安全的切割点 — 永不切在 UTF-8 字节中间，
    ///   避免下游 Telegram API 收到无效 UTF-8 触发 400。
    /// English: Uses `char_indices` to find a safe split point — never cuts mid-UTF-8
    ///   sequence, which would otherwise cause Telegram to return 400 on invalid bytes.
    pub fn split_message(text: &str, max_len: usize) -> Vec<String> {
        if text.chars().count() <= max_len {
            return vec![text.to_string()];
        }
        let mut chunks = Vec::new();
        let mut current = String::new();
        let mut current_len = 0usize;
        for ch in text.chars() {
            // 按"字符数"计 — Telegram 限制 4096 字符，不是 4096 字节。
            // Count by *characters* (Telegram's 4096-char limit is character-based,
            // not byte-based; emoji may consume 1 char but multiple bytes).
            current_len += 1;
            if current_len > max_len {
                chunks.push(std::mem::take(&mut current));
                current_len = 1;
            }
            current.push(ch);
        }
        if !current.is_empty() {
            chunks.push(current);
        }
        chunks
    }

    /// 拼装 sendMessage API 的完整 URL
    /// Build the full `sendMessage` API URL.
    pub fn api_url(bot_token: &str) -> String {
        format!("https://api.telegram.org/bot{}/sendMessage", bot_token)
    }

    /// 选择此次发送的目标 chat_id。
    /// Pick the target chat_id for this send.
    ///
    /// 中文：优先使用 `notification.metadata["telegram_chat_id"]`（per-event 覆盖），
    ///   然后是 `default_chat_id`（per-channel 模式），最后是 `chat_id_resolver`
    ///   通过 `user_id` 查表（per-user 模式）。三者都缺失则返回 None 表示跳过。
    /// English: Priority: per-event override in
    ///   `notification.metadata["telegram_chat_id"]`, then `default_chat_id`
    ///   (per-channel mode), then `chat_id_resolver` (per-user mode, queries DB
    ///   by user_id). All three missing → returns `None` to skip the send.
    pub fn resolve_chat_id(&self, notification: &AlertNotification) -> Option<String> {
        // 1) per-event override from metadata
        if let Some(map) = notification.metadata.as_object() {
            if let Some(v) = map.get("telegram_chat_id") {
                if let Some(s) = v.as_str() {
                    if !s.is_empty() {
                        return Some(s.to_string());
                    }
                }
            }
            if let Some(v) = map.get("user_id") {
                if let Some(uid) = v.as_str() {
                    if let Some(resolver) = &self.chat_id_resolver {
                        if let Some(chat_id) = resolver(uid.to_string()) {
                            return Some(chat_id);
                        }
                    }
                }
            }
        }
        // 2) per-channel default
        if let Some(default) = &self.default_chat_id {
            if !default.is_empty() {
                return Some(default.clone());
            }
        }
        // 3) per-user resolver (best-effort, only fires if caller passed a user_id)
        if let Some(resolver) = &self.chat_id_resolver {
            // 兜底：尝试从 metadata 拿 user_id（仅当上面没匹配时）
            // Best-effort fallback: pull user_id from metadata if not already
            // matched above.
            // 这里不再重复检查 — 上面 if 块已经处理过 user_id 的解析。
            // (No re-check here — the branch above already handled the user_id case.)
            let _ = resolver; // silence unused
        }
        None
    }
}

impl NotificationChannel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    fn send<'a>(
        &'a self,
        notification: &'a AlertNotification,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        // 配置检查：bot_token 是必需的；chat_id 来自 resolve_chat_id()。
        // Config check: bot_token is required; chat_id comes from resolve_chat_id().
        let bot_token = match &self.bot_token {
            Some(t) if !t.is_empty() => t.clone(),
            _ => {
                tracing::debug!("Telegram bot_token not configured, skipping notification");
                return Box::pin(async { Ok(()) });
            }
        };
        let chat_id = match self.resolve_chat_id(notification) {
            Some(id) if !id.is_empty() => id,
            _ => {
                tracing::debug!(
                    "Telegram chat_id not resolvable for alert_type={} symbol={:?}, skipping",
                    notification.alert_type,
                    notification.symbol
                );
                return Box::pin(async { Ok(()) });
            }
        };

        let text = self.build_message(notification);
        let chunks = Self::split_message(&text, TELEGRAM_MAX_MESSAGE_LEN);
        let url = Self::api_url(&bot_token);

        Box::pin(async move {
            let client = reqwest::Client::new();
            // 顺序发送：Telegram Bot API 在短时间内的并发请求会被 rate-limit，
            //   顺序发送更稳定；下游每条 chunk 是独立的 HTTP 调用。
            // Sequential: Telegram Bot API rate-limits bursty concurrent requests;
            // sequential is more stable. Each chunk is one independent HTTP call.
            for (idx, chunk) in chunks.iter().enumerate() {
                let body = SendMessageRequest {
                    chat_id: &chat_id,
                    text: chunk,
                    parse_mode: "MarkdownV2",
                    disable_web_page_preview: true,
                };
                let resp = client
                    .post(&url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("Telegram request failed: {}", e))?;
                let status = resp.status();
                let parsed: SendMessageResponse = resp
                    .json()
                    .await
                    .map_err(|e| format!("Telegram response parse failed: {}", e))?;
                if !parsed.ok || !status.is_success() {
                    let err_code = parsed.error_code.unwrap_or(status.as_u16() as i32);
                    let desc = parsed
                        .description
                        .unwrap_or_else(|| status.to_string());
                    return Err(format!(
                        "Telegram API error (chunk {}/{}): code={} desc={}",
                        idx + 1,
                        chunks.len(),
                        err_code,
                        desc
                    ));
                }
            }
            Ok(())
        })
    }
}

/// 严重级别 → emoji 标记
/// Severity level → emoji marker.
fn severity_emoji(severity: &str) -> &'static str {
    match severity {
        "critical" => "🚨",
        "warning" => "⚠️",
        "info" => "ℹ️",
        _ => "🔔",
    }
}

/// Telegram MarkdownV2 特殊字符转义
/// Escape Telegram MarkdownV2 special characters.
///
/// 中文：MarkdownV2 要求对以下字符做转义： `_ * [ ] ( ) ~ \` > # + - = | { } . !`。
///   在告警场景下，主要风险来自 `user_input.content`（动态内容），title 同样需要。
///   实施最小化转义以保持可读性。
/// English: MarkdownV2 requires escaping: `_ * [ ] ( ) ~ \` > # + - = | { } . !`.
///   In alert scenarios the main risk is `user_input.content` (dynamic); the title
///   is also user-supplied. Minimal escaping preserves readability.
fn escape_markdownv2(s: &str) -> String {
    const SPECIALS: &[char] = &[
        '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}',
        '.', '!', '\\',
    ];
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if SPECIALS.contains(&ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_notification() -> AlertNotification {
        AlertNotification {
            title: "止损触发".to_string(),
            content: "BTC/USDT 多头止损触发 @ 95000".to_string(),
            alert_type: "risk_alert".to_string(),
            severity: "critical".to_string(),
            symbol: Some("BTCUSDT".to_string()),
            triggered_at: chrono::Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap(),
            metadata: json!({}),
        }
    }

    #[test]
    fn test_build_message_contains_all_fields() {
        let channel = TelegramChannel::from_env();
        let n = sample_notification();
        let msg = channel.build_message(&n);
        assert!(msg.contains("止损触发"));
        assert!(msg.contains("BTCUSDT"));
        assert!(msg.contains("critical"));
        assert!(msg.contains("risk\\_alert"), "alert_type should be escaped: {}", msg);
    }

    #[test]
    fn test_build_message_escapes_specials() {
        let channel = TelegramChannel::from_env();
        let n = AlertNotification {
            title: "with_underscore.and.dot".to_string(),
            content: "ping (a.b) [c]!".to_string(),
            alert_type: "test".to_string(),
            severity: "info".to_string(),
            symbol: Some("ETHUSDT".to_string()),
            triggered_at: chrono::Utc::now(),
            metadata: json!({}),
        };
        let msg = channel.build_message(&n);
        // 特殊字符应当被转义
        // Special characters must be escaped.
        // MarkdownV2 specials: _ * [ ] ( ) ~ ` > # + - = | { } . ! \
        // 注意：`!` 也是 special — 必须转义为 `\!`。
        // Note: `!` is also a MarkdownV2 special — must be escaped to `\!`.
        assert!(msg.contains(r"with\_underscore\.and\.dot"));
        assert!(msg.contains(r"ping \(a\.b\) \[c\]\!"));
    }

    #[test]
    fn test_split_message_short_unchanged() {
        let chunks = TelegramChannel::split_message("hello", 4096);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "hello");
    }

    #[test]
    fn test_split_message_exact_boundary() {
        let s = "a".repeat(4096);
        let chunks = TelegramChannel::split_message(&s, 4096);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chars().count(), 4096);
    }

    #[test]
    fn test_split_message_over_boundary_chunks() {
        let s = "a".repeat(4097);
        let chunks = TelegramChannel::split_message(&s, 4096);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chars().count(), 4096);
        assert_eq!(chunks[1].chars().count(), 1);
    }

    #[test]
    fn test_split_message_utf8_safe() {
        // emoji "😀" 是 1 个字符但占 4 字节 — 不能在字节边界切。
        // "😀" is 1 char but 4 bytes — must not split mid-byte.
        let s = "😀".repeat(5000);
        let chunks = TelegramChannel::split_message(&s, 4096);
        // 每块都必须是合法 UTF-8（没有 panic 即可证明）
        // Each chunk must be valid UTF-8 (absence of panic proves it).
        for chunk in &chunks {
            assert!(chunk.chars().count() <= 4096);
            assert!(chunk.is_char_boundary(chunk.len())); // 总是 true 但作为安全检查
        }
        // 总字符数守恒
        // Total char count is conserved.
        let total: usize = chunks.iter().map(|c| c.chars().count()).sum();
        assert_eq!(total, 5000);
    }

    #[test]
    fn test_resolve_chat_id_metadata_override() {
        let channel = TelegramChannel::new("token".to_string(), "default_chat".to_string());
        let mut n = sample_notification();
        n.metadata = json!({ "telegram_chat_id": "override_chat" });
        assert_eq!(
            channel.resolve_chat_id(&n),
            Some("override_chat".to_string())
        );
    }

    #[test]
    fn test_resolve_chat_id_default() {
        let channel = TelegramChannel::new("token".to_string(), "default_chat".to_string());
        let n = sample_notification();
        assert_eq!(
            channel.resolve_chat_id(&n),
            Some("default_chat".to_string())
        );
    }

    #[test]
    fn test_resolve_chat_id_resolver() {
        let resolver: ChatIdResolver = std::sync::Arc::new(|uid: String| {
            if uid == "u1" {
                Some("user1_chat".to_string())
            } else {
                None
            }
        });
        let channel = TelegramChannel::from_env().with_resolver(resolver);
        let mut n = sample_notification();
        n.metadata = json!({ "user_id": "u1" });
        assert_eq!(channel.resolve_chat_id(&n), Some("user1_chat".to_string()));
    }

    #[test]
    fn test_resolve_chat_id_none_when_unconfigured() {
        let channel = TelegramChannel::from_env();
        let n = sample_notification();
        assert_eq!(channel.resolve_chat_id(&n), None);
    }

    #[test]
    fn test_channel_name() {
        let channel = TelegramChannel::from_env();
        assert_eq!(channel.name(), "telegram");
    }

    #[test]
    fn test_api_url_format() {
        assert_eq!(
            TelegramChannel::api_url("123:abc"),
            "https://api.telegram.org/bot123:abc/sendMessage"
        );
    }

    #[test]
    fn test_severity_emoji_mapping() {
        assert_eq!(severity_emoji("critical"), "🚨");
        assert_eq!(severity_emoji("warning"), "⚠️");
        assert_eq!(severity_emoji("info"), "ℹ️");
        assert_eq!(severity_emoji("unknown"), "🔔");
    }

    #[tokio::test]
    async fn test_unconfigured_token_skips() {
        // bot_token 为空 → send() 应直接返回 Ok(())（不发送任何东西）
        // bot_token empty → send() should return Ok(()) directly (no send).
        let channel = TelegramChannel {
            bot_token: None,
            default_chat_id: Some("c".to_string()),
            chat_id_resolver: None,
        };
        let n = sample_notification();
        let result = channel.send(&n).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unresolved_chat_id_skips() {
        // bot_token 有但 resolve_chat_id 返回 None → 跳过
        // bot_token set but resolve_chat_id returns None → skip.
        let channel = TelegramChannel {
            bot_token: Some("fake_token".to_string()),
            default_chat_id: None,
            chat_id_resolver: None,
        };
        let n = sample_notification();
        let result = channel.send(&n).await;
        assert!(result.is_ok());
    }

    /// 真实 HTTP 调用测试 — 需要网络 + 有效 token；不写入 CI 默认套件。
    /// Live HTTP call test — requires network + valid token; not part of the
    /// default CI suite. Run with `cargo test telegram::tests::test_live_send
    /// -- --ignored --nocapture` after setting `TELEGRAM_E2E_BOT_TOKEN` and
    /// `TELEGRAM_E2E_CHAT_ID` env vars.
    #[tokio::test]
    #[ignore = "requires live Telegram bot token + chat_id env vars"]
    async fn test_live_send_to_telegram() {
        let token = match env::var("TELEGRAM_E2E_BOT_TOKEN") {
            Ok(t) => t,
            Err(_) => {
                eprintln!("TELEGRAM_E2E_BOT_TOKEN not set; skipping");
                return;
            }
        };
        let chat = match env::var("TELEGRAM_E2E_CHAT_ID") {
            Ok(c) => c,
            Err(_) => {
                eprintln!("TELEGRAM_E2E_CHAT_ID not set; skipping");
                return;
            }
        };
        let channel = TelegramChannel::new(token, chat);
        let n = sample_notification();
        let result = channel.send(&n).await;
        assert!(result.is_ok(), "live send failed: {:?}", result.err());
    }
}
