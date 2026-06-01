---
status: WIP
date: 20260601
---

# PRD-Trigger-Order-20260601 — 条件单引擎

## 1. 功能概述

Trigger Order（条件触发单）引擎为用户提供高级委托管理能力，包括止损单、止盈单、OCO（One-Cancels-Other）组合单和 TWAP（时间加权平均）分批单。与 Position Alert 不同，Trigger Order 在触发后直接生成真实委托并发送到交易所。

**目标用户**：量化交易员（精细化订单管理）

---

## 2. 用户故事

- **US-TO1**：作为交易员，我希望设置止损单，当价格跌破阈值时自动触发市价/限价平仓，以控制最大亏损
- **US-TO2**：作为交易员，我希望设置止盈单，当价格达到目标时自动止盈离场
- **US-TO3**：作为交易员，我希望设置 OCO 单（同时挂止损和止盈），触发任意一个则取消另一个，避免两者同时成交
- **US-TO4**：作为交易员，我希望使用 TWAP 分批下单，将大额订单拆分为多个小单在时间区间内均匀执行，以减少市场冲击

---

## 3. API 端点

### 3.1 POST /api/v1/trigger-orders/stop-loss — 创建止损单

**认证**：JWT

**请求体**：
```json
{
  "position_id": "uuid",
  "symbol": "BTCUSDT",
  "trigger_price": 58000.00,
  "base_price": 60000.00,
  "quantity": 0.5
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| position_id | UUID | 是 | 关联持仓ID |
| symbol | string | 是 | 交易对 |
| trigger_price | float | 是 | 触发价格 |
| base_price | float | 否 | 基准价（用于计算偏移） |
| quantity | float | 是 | 触发后下单数量 |

**响应** `201 Created`：
```json
{
  "id": "uuid",
  "user_id": "uuid",
  "position_id": "uuid",
  "symbol": "BTCUSDT",
  "trigger_type": "stop_loss",
  "status": "pending",
  "trigger_direction": "below",
  "trigger_price": 58000.00,
  "base_price": 60000.00,
  "side": "SELL",
  "quantity": 0.5,
  "filled_quantity": 0.0,
  "created_at": "2026-06-01T12:00:00Z"
}
```

---

### 3.2 POST /api/v1/trigger-orders/take-profit — 创建止盈单

**请求体**：同止损单，`trigger_direction` 自动为 `above`

**响应**：同止损单结构

---

### 3.3 POST /api/v1/trigger-orders/oco — 创建 OCO 单

**描述**：同时创建一对止损单和止盈单，触发任意一个则取消另一个

**认证**：JWT

**请求体**：
```json
{
  "position_id": "uuid",
  "symbol": "BTCUSDT",
  "stop_loss_price": 58000.00,
  "take_profit_price": 65000.00,
  "base_price": 62000.00,
  "quantity": 0.5
}
```

**响应** `201 Created`：
```json
{
  "stop_loss": { "...trigger_order_fields" },
  "take_profit": { "...trigger_order_fields" }
}
```

> OCO 单创建后，`stop_loss.oco_pair_id = take_profit.id`，反之亦然

---

### 3.4 POST /api/v1/trigger-orders/twap — 创建 TWAP 单

**描述**：时间加权平均单，将大单拆分为多个子单在时间区间内均匀执行

**认证**：JWT

**请求体**：
```json
{
  "symbol": "BTCUSDT",
  "side": "BUY",
  "quantity": 10.0,
  "slice_quantity": 1.0,
  "interval_secs": 60,
  "duration_secs": 3600
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| symbol | string | 是 | 交易对 |
| side | string | 是 | `BUY` / `SELL` |
| quantity | float | 是 | 总数量 |
| slice_quantity | float | 是 | 每份数量 |
| interval_secs | int | 是 | 间隔秒数 |
| duration_secs | int | 是 | 总执行时长 |

**响应** `201 Created`：返回 TWAP 订单对象（含 `twap_slice_quantity`、`twap_interval_secs`、`twap_executed_slices`、`twap_max_slices` 字段）

---

### 3.5 GET /api/v1/trigger-orders — 查询条件单列表

**认证**：JWT

**Query 参数**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| status | string | 否 | `pending` / `triggered` / `cancelled` / `expired` |
| symbol | string | 否 | 按交易对筛选 |

**响应** `200 OK`：返回 `TriggerOrderResponse[]` 数组

---

### 3.6 GET /api/v1/trigger-orders/:id — 查询单个条件单

**认证**：JWT

---

### 3.7 DELETE /api/v1/trigger-orders/:id — 取消条件单

**认证**：JWT

**请求体**：
```json
{
  "reason": "用户主动取消"
}
```

**响应** `204 No Content`

---

## 4. 数据库模型

### 4.1 表：`trigger_orders`

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| user_id | UUID | 用户ID |
| position_id | UUID | 关联持仓ID（可为 null，如 TWAP） |
| symbol | VARCHAR(20) | 交易对 |
| trigger_type | ENUM | `stop_loss` / `take_profit` / `oco` / `twap` |
| status | ENUM | `pending` / `triggered` / `cancelled` / `expired` |
| trigger_direction | ENUM | `above` / `below` |
| trigger_price | DOUBLE | 触发价格 |
| trigger_price_upper | DOUBLE | OCO 止盈触发价（可选） |
| trigger_price_lower | DOUBLE | OCO 止损触发价（可选） |
| base_price | DOUBLE | 基准价格（可选） |
| side | ENUM | `BUY` / `SELL` |
| quantity | DOUBLE | 订单数量 |
| filled_quantity | DOUBLE | 已成交数量 |
| avg_fill_price | DOUBLE | 平均成交价（可选） |
| oco_pair_id | UUID | OCO 配对单ID（可选） |
| triggered_order_id | UUID | 触发后生成的交易所订单ID（可选） |
| trigger_reason | VARCHAR(100) | 触发原因（可选） |
| triggered_at | TIMESTAMP | 触发时间（可选） |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |
| twap_slice_quantity | DOUBLE | TWAP 每份数量 |
| twap_interval_secs | INT | TWAP 间隔秒数 |
| twap_executed_slices | INT | TWAP 已执行份数 |
| twap_max_slices | INT | TWAP 总份数 |

---

## 5. 业务流程

### 5.1 止损/止盈触发流程

```
行情心跳 → 检查 pending 条件单
    │
    ├─→ stop_loss: price <= trigger_price → 触发
    ├─→ take_profit: price >= trigger_price → 触发
    │
    触发后：
    ├─→ 更新 status = triggered，triggered_at
    ├─→ 调用 exchange/order 下市价/限价单
    ├─→ 更新 triggered_order_id
    └─→ 推送 WebSocket trade_executed 通知
```

### 5.2 OCO 触发流程

```
OCO 任意一个触发：
    │
    ├─→ 触发方：status=triggered，下单
    ├─→ 配对方：status=cancelled（不触发）
    └─→ oco_pair_id 关联两方
```

### 5.3 TWAP 执行流程

```
TWAP 创建：
    ├─→ 计算总份数 = duration_secs / interval_secs
    └─→ status = pending

定时器（每 interval_secs）：
    ├─→ 检查剩余数量
    ├─→ 下单 slice_quantity
    ├─→ twap_executed_slices++
    │
    全部成交 或 超时：
    └─→ status = triggered（完成）或 cancelled（取消）
```

---

## 6. 边界条件

- 止损单 trigger_price 必须低于当前市场价（做多），止盈必须高于市场价
- OCO 中 stop_loss_price < take_profit_price（做多场景）
- TWAP interval_secs 最小 10 秒，duration_secs 最大 24 小时
- 触发时若持仓已平（position_id 无效），自动取消条件单
- 交易所下单失败时，status 保持 pending，可重试

---

## 7. 验收标准（Gherkin 格式）

```gherkin
Feature: 条件单引擎

  Scenario: 创建止损单
    Given 用户有一笔 BTCUSDT 多头持仓
    When 用户创建止损单（trigger_price=58000）
    Then 止损单状态为 pending
    And trigger_direction 为 below

  Scenario: 止损单触发
    Given 用户有一笔 pending 止损单
    When 价格跌破触发价
    Then 条件单状态变为 triggered
    And 系统在交易所下市价平仓单
    And 用户收到 trade_executed WebSocket 通知

  Scenario: OCO 单止损触发
    Given 用户有一笔 OCO 单（止损 58000，止盈 65000）
    When 价格跌至 58000
    Then 止损单触发并下单
    And 止盈单状态变为 cancelled

  Scenario: TWAP 分批执行
    Given 用户创建 TWAP 单（总量 10 BTC，每份 1 BTC，间隔 60 秒）
    When 60 秒后
    Then 系统自动下单 1 BTC
    And executed_slices = 1
    When 再次 60 秒后
    Then 再下 1 BTC
    And executed_slices = 2

  Scenario: OCO 止盈触发
    Given 用户有一笔 OCO 单
    When 价格涨至止盈价
    Then 止盈单触发并下单
    And 止损单状态变为 cancelled

  Scenario: 取消 TWAP 单
    Given 用户有一笔执行中的 TWAP 单（已执行 3/10 份）
    When 用户取消 TWAP 单
    Then 状态变为 cancelled
    And 不再执行剩余份数

  Scenario: 条件单列表过滤
    Given 用户有多笔 pending 和 triggered 条件单
    When 用户查询 status=pending
    Then 只返回 pending 的条件单
```
