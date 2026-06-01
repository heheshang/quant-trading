# ADR-014: 告警推送服务架构

**ID**: ADR-014
**日期**: 2026-05-21
**状态**: Accepted
**关联**: PRD-P1-F6, PRD-P0-F3, ADR-010

---

## 背景

PRD-P1-F6 要求多渠道告警推送（微信、邮件），PRD-P0-F3 要求开平仓/风控/系统异常时发送通知。当前已有完整实现：

- `backend/src/services/notification/` — 微信/邮件渠道抽象
- `backend/src/services/alert_notification_service.rs` — 告警收敛服务
- `backend/src/services/position_alert_service.rs` — 持仓止盈止损服务
- `backend/src/handlers/position_alert.rs` — HTTP API

本 ADR 记录已有实现的架构决策。

---

## 技术方案

### 告警触发场景

| 场景 | 告警类型 | 严重级别 |
|------|---------|---------|
| 开仓/平仓成交 | `position_alert` | info |
| 止损/止盈触发 | `position_alert` | critical |
| 风控规则触发 | `risk_alert` | critical |
| 策略异常/停止 | `strategy_alert` | warning |
| WebSocket 断连 | `system_alert` | warning |
| 宕机/进程异常 | `system_alert` | critical |

### 推送渠道

#### 微信企业微信 Webhook

**实现**: `backend/src/services/notification/wechat.rs`

```rust
pub struct WeChatChannel {
    webhook_url: Option<String>,
}

impl NotificationChannel for WeChatChannel {
    fn send<'a>(&'a self, notification: &'a AlertNotification)
        -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>
}
```

- 通过 `WECHAT_WEBHOOK_URL` 环境变量配置
- 发送 Markdown 格式消息，包含标题、严重级别、类型、交易对、时间、详情
- 未配置时跳过发送（不报错）

#### 邮件 SMTP

**实现**: `backend/src/services/notification/email.rs`

```rust
pub struct EmailChannel {
    credentials: Option<SmtpCredentials>,
    from: Option<String>,
    to: Option<String>,
}

impl NotificationChannel for EmailChannel {
    fn send<'a>(&'a self, notification: &'a AlertNotification)
        -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>
}
```

- 通过 `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASSWORD`, `SMTP_FROM`, `ALERT_EMAIL_TO` 环境变量配置
- 使用 `lettre` 库发送邮件
- 邮件主题格式: `[CRITICAL] 止损触发`

### 告警收敛机制

**实现**: `backend/src/services/alert_notification_service.rs`

```rust
pub struct AlertNotificationService {
    channels: RwLock<Vec<Arc<dyn NotificationChannel>>>,
    deduplication_cache: RwLock<HashMap<String, Instant>>,
    deduplication_window_secs: u64, // 默认 300 秒（5 分钟）
}
```

**收敛策略**:
- 去重 key = `alert_type:symbol`（如 `position_alert:BTCUSDT`）
- 同一 key 在收敛窗口内只发送一次
- 窗口大小可通过 `with_window()` 自定义
- 缓存自动清理超过 `window * 2` 的过期条目

### 数据模型

#### AlertNotification 结构

```rust
// backend/src/services/notification/mod.rs

pub struct AlertNotification {
    pub title: String,           // 通知标题
    pub content: String,         // 通知内容
    pub alert_type: String,      // "position_alert" | "risk_alert" | "system_alert" | "strategy_alert"
    pub severity: String,        // "info" | "warning" | "critical"
    pub symbol: Option<String>,  // 交易对
    pub triggered_at: DateTime<Utc>,
    pub metadata: Value,         // 额外元数据
}
```

#### AlertType 枚举

```rust
// backend/src/db/position_alerts.rs

pub enum AlertType {
    TakeProfit,
    StopLoss,
    TrailingStop,
}
```

#### AlertChannel（预留）

```rust
pub enum AlertChannel {
    Wechat,
    Email,
    Sms,  // 预留
}
```

#### position_alerts 表

```sql
CREATE TABLE IF NOT EXISTS position_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    position_id UUID NOT NULL,
    symbol VARCHAR(20) NOT NULL,
    alert_type VARCHAR(20) NOT NULL, -- 'take_profit' | 'stop_loss' | 'trailing_stop'
    status VARCHAR(20) NOT NULL,      -- 'active' | 'paused' | 'cancelled' | 'triggered'
    trigger_price DECIMAL(20, 8) NOT NULL,
    limit_price DECIMAL(20, 8),
    trigger_mode VARCHAR(10) NOT NULL, -- 'market' | 'limit'
    trailing_distance DECIMAL(10, 4),
    trailing_activated BOOLEAN NOT NULL DEFAULT FALSE,
    activated_price DECIMAL(20, 8),
    triggered_at TIMESTAMPTZ,
    triggered_order_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    cancelled_at TIMESTAMPTZ,
    note TEXT
);
```

### API 端点

#### GET /api/v1/alerts — 查询告警历史

```rust
// backend/src/handlers/position_alert.rs

// Query params: symbol?, position_id?, status?
pub async fn list_alerts(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<ListAlertsQuery>,
) -> Result<Json<serde_json::Value>, AppError>
```

**响应**:
```json
{
  "code": 0,
  "data": [
    {
      "id": "uuid",
      "position_id": "uuid",
      "symbol": "BTCUSDT",
      "alert_type": "stop_loss",
      "status": "active",
      "trigger_price": "95000.00000000",
      "trigger_mode": "market",
      "created_at": "2026-05-21T10:00:00Z"
    }
  ]
}
```

#### PUT /api/v1/alerts/config — 更新告警配置

配置通知渠道开关和收敛窗口。实现于 `alert_notification_service.rs`，通过环境变量或配置中心管理。

#### POST /api/v1/alerts/test — 发送测试告警

```rust
// 发送测试通知验证渠道连通性
pub async fn send_test_alert(
    notification: AlertNotification,
) -> Result<(), String>
```

#### 其他端点

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/alerts` | 创建止盈/止损警戒 |
| GET | `/api/v1/alerts/:id` | 查询告警详情 |
| PUT | `/api/v1/alerts/:id` | 修改告警（价格/距离/状态） |
| DELETE | `/api/v1/alerts/:id` | 取消告警 |
| POST | `/api/v1/alerts/:id/trigger` | 手动触发（测试用） |

### 实现位置

```
backend/src/
├── services/
│   ├── notification/
│   │   ├── mod.rs          # AlertNotification, NotificationChannel trait
│   │   ├── wechat.rs       # WeChatChannel 企业微信 Webhook
│   │   └── email.rs        # EmailChannel SMTP
│   ├── alert_notification_service.rs  # 告警收敛、渠道管理
│   └── position_alert_service.rs     # 持仓止盈止损 CRUD + 触发检查
└── handlers/
    └── position_alert.rs   # HTTP API 路由
```

### NotificationChannel Trait

```rust
// backend/src/services/notification/mod.rs

pub trait NotificationChannel: Send + Sync {
    fn name(&self) -> &str;
    fn send<'a>(
        &'a self,
        notification: &'a AlertNotification,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}
```

新增渠道只需实现该 trait 并调用 `alert_notification_service.add_channel()` 注册。

---

## 决策

1. **微信 + 邮件双渠道**: 微信企业微信 Webhook 用于实时告警，邮件用于重要通知存档
2. **告警收敛默认 5 分钟**: 避免同一告警重复推送，减少用户干扰
3. **环境变量配置渠道**: 渠道凭据不硬编码，支持运行时启停
4. **trait-based 渠道抽象**: 新增 SMS 等渠道无需修改核心逻辑
5. **异步发送**: 所有渠道实现使用 `async fn`，非阻塞

---

## Consequences

**正面**:
- 已有完整实现，可直接使用
- 渠道可按需启用/禁用
- 收敛机制减少告警风暴

**负面**:
- SMS 渠道预留但未实现
- 告警历史未持久化（仅存储 position_alerts，system_alert 等内存推送）
- 告警配置 API（`PUT /api/v1/alerts/config`）尚未在 handler 中实现

**待办**:
- [ ] 实现告警配置管理 API
- [ ] 实现 SMS 渠道
- [ ] 告警历史持久化表（包含 system_alert、risk_alert 等非持仓告警）
