# ADR-010: 风控规则引擎架构

> ID: ADR-010
> 日期: 2026-05-21
> 状态: Accepted
> 关联: PRD-P0-F2, PRD-Phase4-Risk-Management, ADR-015

---

## 背景

P0-F2 要求实现三项核心资金风控规则：**单日最大亏损限制**、**单笔最大亏损限制**、**总回撤限制**。这些规则需在用户下单前进行前置检查，触发时拒绝开仓或自动平仓。同时 Phase 4 要求风控规则可通过 API 动态配置，并记录完整风控日志供审计。

已有实现文件：
- `backend/src/services/risk_manager.rs` — 风控引擎核心逻辑
- `backend/src/handlers/risk.rs` — REST API Handler
- `backend/src/db/risk_rules.rs` — 风控规则数据模型
- `backend/src/db/risk_logs.rs` — 风控日志数据模型

---

## 技术方案

### 1. 数据模型

#### risk_rules 表

```sql
CREATE TABLE IF NOT EXISTS risk_rules (
    user_id          UUID        PRIMARY KEY REFERENCES users(id),
    daily_loss_limit        DECIMAL(20, 8) NOT NULL DEFAULT 0,
    daily_loss_auto_close   BOOLEAN        NOT NULL DEFAULT FALSE,
    single_trade_loss_ratio DECIMAL(20, 8) NOT NULL DEFAULT 0,
    max_drawdown_ratio      DECIMAL(20, 8) NOT NULL DEFAULT 0.1,
    drawdown_auto_close     BOOLEAN        NOT NULL DEFAULT FALSE,
    stop_loss_type          VARCHAR(20)    NOT NULL DEFAULT 'fixed',
    atr_period              INT,
    atr_multiplier          DECIMAL(10, 4),
    is_active               BOOLEAN        NOT NULL DEFAULT FALSE,
    created_at              TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ    NOT NULL DEFAULT NOW()
);
```

对应 Rust 模型（`risk_rules.rs`）：

```rust
pub struct Model {
    pub user_id: Uuid,
    pub daily_loss_limit: Decimal,
    pub daily_loss_auto_close: bool,
    pub single_trade_loss_ratio: Decimal,
    pub max_drawdown_ratio: Decimal,
    pub drawdown_auto_close: bool,
    pub stop_loss_type: String,      // "fixed" | "atr_multiplier"
    pub atr_period: Option<i32>,
    pub atr_multiplier: Option<Decimal>,
    pub is_active: bool,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}
```

#### risk_logs 表

```sql
CREATE TABLE IF NOT EXISTS risk_logs (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID        NOT NULL REFERENCES users(id),
    rule_type         VARCHAR(20) NOT NULL,   -- "daily_loss" | "single_trade" | "drawdown" | "paused" | "resumed"
    triggered_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    position_value    DECIMAL(20, 8),
    account_equity    DECIMAL(20, 8) NOT NULL,
    threshold         DECIMAL(20, 8) NOT NULL,
    actual_value      DECIMAL(20, 8) NOT NULL,
    action_taken      VARCHAR(20) NOT NULL,   -- "rejected" | "auto_closed" | "paused" | "resumed"
    order_id          UUID,
    notification_sent BOOLEAN      NOT NULL DEFAULT FALSE
);
```

### 2. 风控检查流程

每次下单前调用 `RiskManager::check_order()`，执行以下检查：

```
下单请求
    ↓
1. 加载风控规则（risk_rules 表）
    ↓
2. 获取账户快照（equity / peak_equity / daily_pnl）
    ↓
3. 单日亏损检查
    ├─ 计算当日累计浮动亏损（positions.unrealized_pnl 总和）
    ├─ 若 daily_loss <= -daily_loss_limit：
    │   ├─ daily_loss_auto_close == true → emergency_close() + 告警
    │   └─ 返回 Error(RF-001)
    ↓
4. 单笔亏损检查（仅开仓订单）
    ├─ 估算最大潜在亏损 = |order_price - stop_loss_price| * quantity
    ├─ loss_ratio = estimated_loss / equity
    ├─ 若 loss_ratio > single_trade_loss_ratio → 拒绝 + 记录日志
    ↓
5. 总回撤检查
    ├─ drawdown = (peak_equity - equity) / peak_equity
    ├─ 若 drawdown >= max_drawdown_ratio：
    │   ├─ drawdown_auto_close == true → emergency_close() + 告警
    │   └─ 返回 Error(RF-003)
    ↓
通过 → 允许下单
```

#### 风控违规错误码

| 错误码 | 含义 |
|--------|------|
| RF-001 | 单日亏损超限 |
| RF-002 | 单笔亏损超限 |
| RF-003 | 总回撤超限 |
| RF-004 | 交易已暂停 |
| RF-005 | 风控规则未启用 |

### 3. API 端点

| 方法 | 路径 | 功能 |
|------|------|------|
| GET | `/api/v1/risk/rules` | 查询当前用户风控规则 |
| PUT | `/api/v1/risk/rules` | 更新风控规则 |
| GET | `/api/v1/risk/logs` | 分页查询风控日志 |
| POST | `/api/v1/risk/check` | 手动触发风控检查 |
| POST | `/api/v1/risk/emergency-close` | 紧急全平所有持仓 |
| POST | `/api/v1/risk/pause` | 暂停交易 |
| POST | `/api/v1/risk/resume` | 恢复交易 |
| GET | `/api/v1/risk/connection-status` | 查询行情连接状态 |

#### GET /api/v1/risk/rules Response

```json
{
  "code": 0,
  "data": {
    "user_id": "uuid",
    "daily_loss_limit": "1000.00",
    "daily_loss_auto_close": true,
    "single_trade_loss_ratio": "0.02",
    "max_drawdown_ratio": "0.10",
    "drawdown_auto_close": true,
    "stop_loss_type": "atr_multiplier",
    "atr_period": 14,
    "atr_multiplier": "1.5",
    "is_active": true
  }
}
```

#### PUT /api/v1/risk/rules Request

```json
{
  "daily_loss_limit": "2000.00",
  "daily_loss_auto_close": true,
  "single_trade_loss_ratio": "0.03",
  "max_drawdown_ratio": "0.15",
  "drawdown_auto_close": false,
  "stop_loss_type": "atr_multiplier",
  "atr_period": 14,
  "atr_multiplier": "2.0",
  "is_active": true
}
```

### 4. 实现位置

| 模块 | 文件 | 职责 |
|------|------|------|
| Service | `backend/src/services/risk_manager.rs` | 风控规则引擎核心逻辑、check_order、emergency_close |
| Handler | `backend/src/handlers/risk.rs` | REST API 路由和 Handler |
| Model | `backend/src/db/risk_rules.rs` | SeaORM Entity for risk_rules 表 |
| Model | `backend/src/db/risk_logs.rs` | SeaORM Entity for risk_logs 表 |

#### RiskManager 核心结构

```rust
pub struct RiskManager {
    db: Arc<DatabaseConnection>,
}

impl RiskManager {
    pub async fn check_order(...) -> Result<(), AppError>;   // 前置检查
    pub async fn manual_check(...) -> Result<RiskCheckResult, AppError>;
    pub async fn emergency_close(...) -> Result<EmergencyCloseResult, AppError>;
    pub async fn pause(...) -> Result<PauseResponse, AppError>;
    pub async fn resume(...) -> Result<(), AppError>;
}
```

---

## 决策

1. **风控规则按用户隔离**：每个用户独立一套风控规则，以 `user_id` 为主键。
2. **前置检查而非后置**：三项核心规则在 `check_order()` 中于下单前执行，拒绝违规订单。
3. **可选自动平仓**：单日亏损和总回撤均支持触发后自动平仓，由 `*_auto_close` 字段控制。
4. **风控日志持久化**：每次触发规则均写入 `risk_logs` 表，包含触发时间、阈值、实际值、操作动作。
5. **ATR 追踪止损预留**：`stop_loss_type` / `atr_period` / `atr_multiplier` 字段已预留，详见 ADR-015。
6. **权益快照简化**：`get_account_snapshot()` MVP 版本简化处理，峰值权益随当前权益更新（ADR-015 后续优化）。

---

## Consequences

**正面影响：**
- 用户可动态配置三项核心风控规则，无需重启服务
- 自动平仓机制在极端行情下保护账户不穿仓
- 完整风控日志支持事后审计和告警追溯

**潜在风险：**
- MVP 中当日盈亏使用持仓浮动盈亏估算，非真实已实现盈亏（需后续实现 `daily_pnl_summary` 表）
- MVP 中峰值权益未持久化，系统重启后峰值重置（ADR-015 优化项）
- ATR 追踪止损尚未与风控规则完全联动（ADR-015 实现中）
