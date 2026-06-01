---
status: WIP
date: 20260601
---

# PRD-Position-Alert-20260601 — 仓位告警

## 1. 功能概述

Position Alert（仓位告警）模块允许用户为持仓设置止盈/止损/追踪止损规则，当市场价格触及触发条件时自动推送告警通知或触发平仓。告警规则独立于策略运行，支持手动创建、修改、暂停和删除。

**目标用户**：量化交易员（实盘持仓管理）

---

## 2. 用户故事

- **US-PA1**：作为交易员，我希望在开仓时附加止盈/止损规则，以便自动锁定利润和控制风险
- **US-PA2**：作为交易员，我希望随时修改已有告警的触发价格，以便应对行情变化
- **US-PA3**：作为交易员，我希望暂停不想执行的告警，而不是删除，以便需要时恢复
- **US-PA4**：作为交易员，我希望设置追踪止损（Trailing Stop），让止损线随价格上涨而上移

---

## 3. API 端点

### 3.1 POST /api/v1/alerts — 创建告警规则

**认证**：JWT

**请求体**：
```json
{
  "position_id": "uuid",
  "alert_type": "stop_loss",
  "trigger_price": "58000.00",
  "trigger_mode": "market",
  "limit_price": null,
  "trailing_distance": null,
  "note": "跌破 58000 止损"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| position_id | string (UUID) | 是 | 关联的持仓ID |
| alert_type | string | 是 | `take_profit` / `stop_loss` / `trailing_stop` |
| trigger_price | string | 是 | 触发价格（数值字符串） |
| trigger_mode | string | 否 | `market`（默认）/ `limit` |
| limit_price | string | 否 | 限价触发价格 |
| trailing_distance | string | 否 | 追踪止损距离（百分比 0~100） |
| note | string | 否 | 备注 |

**响应** `201 Created`：
```json
{
  "code": 0,
  "data": {
    "alert_id": "uuid",
    "position_id": "uuid",
    "alert_type": "stop_loss",
    "trigger_price": "58000.00000000",
    "trigger_mode": "market",
    "status": "active",
    "created_at": "2026-06-01T12:00:00Z"
  },
  "message": "success"
}
```

---

### 3.2 GET /api/v1/alerts — 列表查询

**认证**：JWT

**Query 参数**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| symbol | string | 否 | 按交易对筛选 |
| position_id | string | 否 | 按持仓筛选 |
| status | string | 否 | `active` / `paused` / `cancelled` / `triggered` |

**响应** `200 OK`：
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
      "trigger_price": "58000.00000000",
      "trigger_mode": "market",
      "created_at": "2026-06-01T10:00:00Z"
    }
  ],
  "message": "success"
}
```

---

### 3.3 GET /api/v1/alerts/:id — 告警详情

**认证**：JWT

**响应** `200 OK`：返回完整告警对象（包含 note、updated_at 等）

---

### 3.4 PUT /api/v1/alerts/:id — 修改告警

**认证**：JWT

**请求体**（所有字段可选）：
```json
{
  "trigger_price": "57000.00",
  "limit_price": null,
  "trigger_mode": "limit",
  "trailing_distance": null,
  "status": "paused"
}
```

**响应** `200 OK`：返回更新后的告警对象

---

### 3.5 DELETE /api/v1/alerts/:id — 删除告警

**认证**：JWT

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": { "message": "Alert cancelled successfully" },
  "message": "success"
}
```

> 注：DELETE 逻辑上等价于 `status` 设为 `cancelled`，物理删除可选

---

### 3.6 POST /api/v1/alerts/batch-check — 批量检查（内部/定时任务）

**描述**：由行情心跳调用，批量检查所有活跃告警是否触发

**触发时机**：每次行情更新时调用

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": {
    "checked": 15,
    "triggered": 2
  }
}
```

---

## 4. 数据库模型

### 4.1 表：`position_alerts`

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| user_id | UUID | 用户ID |
| position_id | UUID | 关联持仓ID |
| symbol | VARCHAR(20) | 交易对 |
| alert_type | ENUM | `take_profit` / `stop_loss` / `trailing_stop` |
| status | ENUM | `active` / `triggered` / `cancelled` / `paused` |
| trigger_price | DOUBLE | 触发价格 |
| limit_price | DOUBLE | 限价触发价格（可选） |
| trigger_mode | ENUM | `market` / `limit` |
| trailing_distance | DOUBLE | 追踪止损距离（0~1，可选） |
| note | TEXT | 备注（可选） |
| triggered_at | TIMESTAMP | 触发时间（可选） |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 5. 业务流程

```
用户创建告警
    │
    └─→ 存储到 position_alerts 表（status=active）
            │
行情心跳（batch-check）
    │
    ├─→ 查询所有 status=active 的告警
    ├─→ 对比 trigger_price 与当前市场价格
    ├─→ 触发 → 更新 status=triggered，触发_at
    │         └─→ 推送 WebSocket 通知（trade_executed）
    └─→ 未触发 → 继续等待
```

**追踪止损逻辑**：
- 初始化 `trailing_distance`（百分比）
- 当价格朝有利方向移动时，持续更新内部 `highest_price`（做多）或 `lowest_price`（做空）
- 止损线 = `highest_price * (1 - trailing_distance)` 或 `lowest_price * (1 + trailing_distance)`

---

## 6. 边界条件

- `trigger_price <= 0` 时拒绝创建
- `trailing_distance` 范围 0~100（百分比）
- 告警已触发（status=triggered）后不能修改
- 不能为他人持仓创建告警（user_id 校验）
- 行情数据不可用时跳过检查，不报错

---

## 7. 验收标准（Gherkin 格式）

```gherkin
Feature: 仓位告警管理

  Scenario: 为持仓创建止损告警
    Given 用户有一笔 BTCUSDT 多头持仓
    When 用户创建止损告警（trigger_price=58000）
    Then 告警状态为 active
    And 告警与持仓正确关联

  Scenario: 止盈止损触发后状态变更
    Given 用户有一笔活跃止损告警（trigger_price=58000）
    When 市场最新价跌至 58000
    Then 告警状态自动变为 triggered
    And triggered_at 时间被记录

  Scenario: 修改告警触发价格
    Given 用户有一笔活跃止损告警
    When 用户修改 trigger_price 为 57000
    Then 新价格生效
    And updated_at 时间更新

  Scenario: 暂停告警
    Given 用户有一笔活跃告警
    When 用户将状态设为 paused
    Then 行情心跳不再检查该告警
    And 用户可随时恢复为 active

  Scenario: 追踪止损动态调整
    Given 用户创建追踪止损告警（trailing_distance=1%）
    And 持仓成本 50000
    When 价格从 50000 涨到 55000
    Then 止损线随最高价上移（约 54450）
    When 价格从 55000 跌回 54450
    Then 告警触发

  Scenario: 批量检查性能
    Given 系统有 1000 个活跃告警
    When 定时任务执行 batch-check
    Then 所有告警检查在 1 秒内完成
```
