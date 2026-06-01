# T8 签字记录 — P1-F6 告警推送

- Feature: P1-F6 多渠道告警推送
- Date: 2026-05-21
- PM: ssk
- TechLead: ssk

## 签字

| 角色 | 签字 | 日期 |
|------|------|------|
| PM | ssk | 2026-05-21 |
| TechLead | ssk | 2026-05-21 |

## 渠道配置

### 企业微信
- 环境变量: `WECHAT_WEBHOOK_URL`
- 格式: Markdown
- 未配置时静默跳过

### 邮件
- 环境变量: `SMTP_HOST`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM`
- 未配置时静默跳过