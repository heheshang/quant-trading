# Release Note v0.9.2 — P1-F6 告警推送

## 功能
- **AlertNotificationService**: 多渠道告警通知服务
  - Trait-based channel architecture (NotificationChannel trait)
  - 告警收敛：5分钟窗口去重 (key = "alert_type:symbol")
  - 支持动态添加/移除渠道
- **WeChatChannel**: 企业微信 Webhook
  - Markdown 格式消息
  - 环境变量 `WECHAT_WEBHOOK_URL` 配置
  - 未配置时静默跳过
- **EmailChannel**: SMTP 邮件
  - 环境变量 `SMTP_HOST`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM`
  - 未配置时静默跳过

## 技术
- Trait: `NotificationChannel` (async send, name, clone)
- `Arc<dyn NotificationChannel>` 多渠道管理
- `RwLock` 线程安全并发访问

## 测试
- 10 tests passed (notification module)
- 267 total tests passed