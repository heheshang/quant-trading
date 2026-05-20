# ADR-013: 资金风控规则引擎

| 字段 | 值 |
|------|-----|
| **ID** | ADR-013 |
| **状态** | **待评审** |
| **日期** | 2026-05-19 |
| **决策者** | Tech Lead |
| **影响范围** | 订单创建/修改/撤单、持仓管理、风控日志、告警推送 |
| **关联** | ADR-010 (订单管理), P0-F2 PRD |

---

## 1. 背景

P0-F2 资金风控是量化交易系统的「保命机制」。当前 `risk_manager.rs` 服务不存在（ADR-010 D8 规划未落地）。所有订单在创建前必须经过风控前置检查，防止单日亏损、单笔交易亏损、总回撤超限导致的账户爆仓。

**已有基础设施：**
- `services/matching_engine.rs` — 撮合引擎（可获取持仓/权益）
- `services/portfolio.rs` — 账户权益查询
- `db/order.rs` — orders / trades / positions 表（可查询历史盈亏）
- `services/exchange/rate_limiter.rs` — 频率限制（可复用架构）

---

## 2. 决策

### D1: RiskManager 服务架构

**位置：** `services/risk_manager.rs`

```rust
pub struct RiskManager {
    db: Arc<DatabaseConnection>,
}

impl RiskManager {
    /// 前置风控检查 — 每次下单前必须调用
    pub async fn check_order(&self, user_id: Uuid, order: &NewOrder, position_value: f64) -> Result<(), AppError>;

    /// 查询当前风控规则
    pub async fn get_rules(&self, user_id: Uuid) -> Result<RiskRules, AppError>;

    /// 更新风控规则（实时生效，无需重启）
    pub async fn update_rules(&self, user_id: Uuid, rules: RiskRulesUpdate) -> Result<RiskRules, AppError>;

    /// 查询风控日志（分页）
    pub async fn list_logs(&self, user_id: Uuid, params: RiskLogQuery) -> Result<PaginatedResponse<RiskLog>, AppError>;

    /// 手动触发风控检查
    pub async fn manual_check(&self, user_id: Uuid) -> Result<RiskCheckResult, AppError>;

    /// 紧急全平（不受任何风控条件限制）
    pub async fn emergency_close(&self, user_id: Uuid, admin: bool) -> Result<EmergencyCloseResult, AppError>;

    /// 暂停/恢复交易
    pub async fn pause(&self, user_id: Uuid, reason: &str) -> Result<(), AppError>;
    pub async fn resume(&self, user_id: Uuid) -> Result<(), AppError>;
}
```

**前置检查流程（check_order）：**

```
1. 查询用户当前风控规则（risk_rules 表）
2. 计算当日累计亏损（从 trades 表 SUM(realized_pnl) WHERE date = today）
3. 计算当前持仓总额（positions 表 SUM(value)）
4. 校验各项阈值
5. 如触发，记录 risk_logs + 推送告警
6. 返回 Ok(()）或 AppError::RiskViolation(code, message)
```

**校验顺序：**

| # | 检查项 | 错误码 | 说明 |
|---|--------|--------|------|
| 1 | 风控规则已启用 | — | 未启用时跳过所有检查 |
| 2 | 当日亏损 ≤ daily_loss_limit | RF-001 | 超限且 auto_close=true 则全平 |
| 3 | 单笔预估亏损 ≤ single_trade_loss_ratio × 权益 | RF-002 | 仅对开仓订单生效 |
| 4 | 总回撤 ≤ max_drawdown_ratio | RF-003 | 从历史峰值权益计算 |
| 5 | 交易已暂停 | RF-004 | pause 接口触发 |

### D2: 数据模型

**risk_rules 表：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| user_id | UUID | ✅ | 所属用户 |
| daily_loss_limit | Decimal | ✅ | 单日亏损限制（USDT） |
| daily_loss_auto_close | Bool | ✅ | 超限是否自动平仓 |
| single_trade_loss_ratio | Decimal | ✅ | 单笔最大亏损比例（0.0~1.0） |
| max_drawdown_ratio | Decimal | ✅ | 最大回撤比例（0.0~1.0） |
| drawdown_auto_close | Bool | ✅ | 回撤超限是否自动平仓 |
| stop_loss_type | String | ✅ | "fixed" / "atr_multiplier" |
| atr_period | i32 | ❌ | ATR 周期（默认14） |
| atr_multiplier | Decimal | ❌ | ATR 系数（默认1.5） |
| is_active | Bool | ✅ | 是否启用 |
| created_at | DateTimeUtc | ✅ | 创建时间 |
| updated_at | DateTimeUtc | ✅ | 更新时间 |

**risk_logs 表：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | UUID | ✅ | 主键 |
| user_id | UUID | ✅ | 触发用户 |
| rule_type | String | ✅ | "daily_loss" / "single_trade" / "drawdown" / "emergency" / "paused" |
| triggered_at | DateTimeUtc | ✅ | 触发时间 |
| position_value | Decimal | ❌ | 触发时持仓金额 |
| account_equity | Decimal | ✅ | 账户权益 |
| threshold | Decimal | ✅ | 触发的阈值 |
| actual_value | Decimal | ✅ | 实际值 |
| action_taken | String | ✅ | "rejected" / "paused" / "closed_all" / "warning" |
| order_id | UUID | ❌ | 关联订单（如有） |
| notification_sent | Bool | ✅ | 是否已发送通知 |

### D3: API 端点设计

**路由前缀：** `/api/v1/risk`

| 端点 | 方法 | 认证 | 说明 |
|------|------|------|------|
| `/rules` | GET | JWT | 获取当前用户风控规则 |
| `/rules` | PUT | JWT+Admin | 更新风控规则 |
| `/logs` | GET | JWT | 分页查询风控日志 |
| `/check` | POST | JWT+Admin | 手动触发风控检查 |
| `/emergency-close` | POST | JWT+Admin | 紧急全平 |
| `/pause` | POST | JWT+Admin | 暂停交易 |
| `/resume` | POST | JWT+Admin | 恢复交易 |
| `/connection-status` | GET | JWT | 连接状态（占位，P0-F3） |

### D4: 与订单创建的集成

**订单创建前调用风控（ADR-010 D8）：**

```rust
// handlers/order.rs — create_order handler
pub async fn create_order(...) {
    // 1. 风控前置检查
    risk_manager.check_order(user.user_id, &new_order, position_value).await?;

    // 2. 写 DB
    let order = Order::create(...).await?;

    // 3. 撮合
    let result = matching_engine.submit(order).await?;

    // 4. 更新持仓/记录成交
    ...
}
```

**集成点（修改现有 handlers/order.rs）：**
- `create_order` — 新订单必须经过 `RiskManager::check_order()`
- `modify_order` — 修改后重新风控检查
- 持仓变动时更新风控记录

### D5: ATR 止损（P0-F2 MVP 不实现，P1-F2 实现）

P0-F2 MVP 阶段，`stop_loss_type` / `atr_period` / `atr_multiplier` 字段在 `risk_rules` 表预留，但 **P0 不实现 ATR 止损计算逻辑**。

原因：ATR 止损需要实时行情数据（P0-F3 WebSocket 连接状态）才能计算触发价。P0 MVP 聚焦规则引擎。

**P0 阶段实现：** 固定止损（用户传绝对价格，P1-F2 止盈止损单实现时对接）

---

## 3. 实现范围（MVP P0-F2）

**做：**
- ✅ RiskManager 服务 + 全部 7 个 API 端点
- ✅ risk_rules + risk_logs SeaORM Entity
- ✅ 当日亏损 / 单笔亏损 / 总回撤 三项检查
- ✅ 告警推送（微信 Webhook，占位）
- ✅ 紧急全平（调用 exchange API 或撮合引擎市价全平）
- ✅ 暂停/恢复交易
- ✅ 与 order handler 集成（风控前置）

**不做（MVP P0）：**
- ❌ ATR 动态止损计算（P1-F2）
- ❌ 止损触发市价平仓联动（P1-F2）
- ❌ WebSocket 断连检测（P0-F3）
- ❌ 涨跌停限制（P2-F3）
- ❌ 多交易对独立风控（MVP 单账户汇总）

---

## 4. 依赖关系

```
P0-F2 资金风控
  ├── 已完成：matching_engine.rs（持仓查询）
  ├── 已完成：portfolio.rs（权益查询）
  ├── 已完成：db/order.rs（orders/trades/positions）
  ├── 依赖中：P0-F1 signed_client（紧急全平时需要实盘 API）
  └── 依赖中：P0-F3 断线检测（与 pause/resume 联动）
```

---

## 5. 风险与缓解

| 风险 | 级别 | 缓解 |
|------|------|------|
| 风控检查增加下单延迟（每次 DB 查询） | Medium | 使用 Redis 缓存当日亏损值，增量更新而非每次重算 |
| 循环依赖（风控依赖持仓，持仓来自成交，成交需要风控） | Low | check_order 读持仓快照，不写 DB，无循环 |
| 紧急全平需要 P0-F1 signed_client | Low | MVP 阶段紧急全平走撮合引擎（模拟模式），P0-F1 后切换实盘 |
