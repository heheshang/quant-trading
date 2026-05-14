# Portfolio 组合权益 API 文档

> 版本: v0.7.0 | 基础路径: `/api/v1` | 认证: Bearer Token (JWT)

---

## 目录

- [API 总览](#api-总览)
- [1. GET /portfolio/summary — 组合权益汇总](#1-get-portfoliosummary--组合权益汇总)
- [2. GET /portfolio/positions — 持仓汇总](#2-get-portfoliopositions--持仓汇总)
- [3. GET /portfolio/performance — 多策略绩效对比](#3-get-portfolioperformance--多策略绩效对比)
- [4. GET /portfolio/equity_curve — 权益曲线数据](#4-get-portfolioequity_curve--权益曲线数据)
- [数据模型](#数据模型)
- [错误码](#错误码)

---

## API 总览

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| GET | `/api/v1/portfolio/summary` | 获取组合权益汇总（总资产/当日盈亏/累计盈亏/持仓数） | 必需 |
| GET | `/api/v1/portfolio/positions` | 获取持仓列表（分页/按交易对筛选） | 必需 |
| GET | `/api/v1/portfolio/performance` | 获取多策略绩效对比数据 | 必需 |
| GET | `/api/v1/portfolio/equity_curve` | 获取权益曲线时序数据（支持粒度/日期范围） | 必需 |

---

## 统一响应格式

### 成功响应

```json
{
  "code": 0,
  "data": { ... },
  "message": "success"
}
```

### 分页响应

```json
{
  "code": 0,
  "data": {
    "items": [...],
    "page": 1,
    "size": 20,
    "total": 42
  },
  "message": "success"
}
```

### 错误响应

```json
{
  "code": 40301,
  "message": "无权限查看该组合"
}
```

---

## 权限模型

所有 Portfolio 端点使用统一的访问控制规则：

- **普通用户**：仅能查看自己的组合数据（不传 `user_id` 或传入自己的 ID）
- **管理员**（`role=admin`）：可查看任意用户的组合数据（通过 `user_id` 参数指定）
- 越权访问返回 `40301` 错误

---

## 1. GET /portfolio/summary — 组合权益汇总

获取当前用户（或指定用户）的组合权益汇总信息，包括总资产、当日盈亏、累计盈亏及持仓数量。

### 请求

```
GET /api/v1/portfolio/summary
Authorization: Bearer <access_token>
```

### 查询参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `user_id` | string (UUID) | 否 | 目标用户 ID；省略时默认为当前用户。仅 admin 角色可查看他人 |

### curl 示例

```bash
# 查看自己的组合汇总
curl -s 'http://localhost:8080/api/v1/portfolio/summary' \
  -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9...'

# 管理员查看指定用户的组合汇总
curl -s 'http://localhost:8080/api/v1/portfolio/summary?user_id=550e8400-e29b-41d4-a716-446655440000' \
  -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9...'
```

### 成功响应 (200)

```json
{
  "code": 0,
  "data": {
    "total_equity": "100000.00",
    "daily_pnl": "1234.56",
    "daily_pnl_rate": "1.25",
    "cumulative_pnl": "15000.00",
    "cumulative_pnl_rate": "17.65",
    "total_positions": 5,
    "updated_at": "2026-05-14T12:00:00+00:00"
  },
  "message": "success"
}
```

### 响应字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `total_equity` | string | 总资产（含现金 + 持仓市值），保留 2 位小数 |
| `daily_pnl` | string | 当日盈亏金额，正数表示盈利，保留 2 位小数 |
| `daily_pnl_rate` | string | 当日盈亏百分比（如 `"1.25"` 表示 +1.25%），保留 2 位小数 |
| `cumulative_pnl` | string | 累计盈亏金额，保留 2 位小数 |
| `cumulative_pnl_rate` | string | 累计盈亏百分比，保留 2 位小数 |
| `total_positions` | integer | 当前持仓数量 |
| `updated_at` | string (ISO 8601) | 数据更新时间 |

> **计算逻辑**：`total_equity` 从 `paper_accounts` 聚合；`daily_pnl` 取当日已成交订单的盈亏合计；`cumulative_pnl` 为 `paper_accounts.total_pnl` 合计；百分比 = 盈亏 / 初始资金 × 100。

---

## 2. GET /portfolio/positions — 持仓汇总

获取当前用户（或指定用户）的持仓列表，支持分页和按交易对筛选。

### 请求

```
GET /api/v1/portfolio/positions
Authorization: Bearer <access_token>
```

### 查询参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `user_id` | string (UUID) | 否 | 当前用户 | 目标用户 ID，仅 admin 可用 |
| `symbol` | string | 否 | — | 按交易对筛选（精确匹配，如 `BTCUSDT`） |
| `page` | integer | 否 | `1` | 页码，从 1 开始 |
| `size` | integer | 否 | `20` | 每页条数，范围 1–100 |

> **已知限制**（来自 UI-Checklist BA-05）：当前版本不支持 `side`（方向）筛选参数。前端传入 `side` 参数将被忽略。计划在后续版本中补充。

### curl 示例

```bash
# 获取持仓列表（默认第 1 页，每页 20 条）
curl -s 'http://localhost:8080/api/v1/portfolio/positions' \
  -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9...'

# 按交易对筛选 + 分页
curl -s 'http://localhost:8080/api/v1/portfolio/positions?symbol=BTCUSDT&page=1&size=10' \
  -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9...'

# 管理员查看指定用户的持仓
curl -s 'http://localhost:8080/api/v1/portfolio/positions?user_id=550e8400-e29b-41d4-a716-446655440000' \
  -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9...'
```

### 成功响应 (200)

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "symbol": "BTCUSDT",
        "side": "long",
        "quantity": "0.50000000",
        "avg_price": "65000.00000000",
        "current_price": "66000.00000000",
        "unrealized_pnl": "500.00",
        "unrealized_pnl_rate": "1.54"
      },
      {
        "symbol": "ETHUSDT",
        "side": "short",
        "quantity": "2.00000000",
        "avg_price": "3500.00000000",
        "current_price": "3450.00000000",
        "unrealized_pnl": "100.00",
        "unrealized_pnl_rate": "1.43"
      }
    ],
    "total": 5,
    "page": 1,
    "size": 20
  },
  "message": "success"
}
```

### 响应字段（items 内每条记录）

| 字段 | 类型 | 说明 |
|------|------|------|
| `symbol` | string | 交易对名称（如 `BTCUSDT`） |
| `side` | string | 持仓方向：`long`（多头）/ `short`（空头） |
| `quantity` | string | 持仓数量，保留 8 位小数 |
| `avg_price` | string | 平均开仓价格，保留 8 位小数 |
| `current_price` | string | 当前价格，保留 8 位小数 |
| `unrealized_pnl` | string | 未实现盈亏金额，正数为盈利，保留 2 位小数 |
| `unrealized_pnl_rate` | string | 未实现盈亏百分比，保留 2 位小数 |

> **已知限制**（来自 UI-Checklist BA-08）：`current_price` 当前使用 `avg_entry_price` 作为占位值，真实系统应从行情服务获取实时价格。这导致 `unrealized_pnl` 的计算精度受限，浮动盈亏约等于持仓记录中的 `unrealized_pnl` 字段值。

### 空结果

当用户无持仓时，返回空数组：

```json
{
  "code": 0,
  "data": {
    "items": [],
    "total": 0,
    "page": 1,
    "size": 20
  },
  "message": "success"
}
```

---

## 3. GET /portfolio/performance — 多策略绩效对比

获取当前用户（或指定用户）所有策略的绩效对比数据，用于策略表现横向评估。

### 请求

```
GET /api/v1/portfolio/performance
Authorization: Bearer <access_token>
```

### 查询参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `user_id` | string (UUID) | 否 | 目标用户 ID；省略时默认为当前用户。仅 admin 可用 |

### curl 示例

```bash
# 获取策略绩效对比
curl -s 'http://localhost:8080/api/v1/portfolio/performance' \
  -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9...'
```

### 成功响应 (200)

```json
{
  "code": 0,
  "data": {
    "strategies": [
      {
        "strategy_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "strategy_name": "双均线交叉",
        "total_pnl": "5000.00",
        "total_pnl_rate": "12.50",
        "max_drawdown": "3.45",
        "trade_count": 42,
        "win_rate": "65.00"
      },
      {
        "strategy_id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
        "strategy_name": "MACD 策略",
        "total_pnl": "-1200.00",
        "total_pnl_rate": "-3.00",
        "max_drawdown": "8.20",
        "trade_count": 28,
        "win_rate": "42.86"
      }
    ]
  },
  "message": "success"
}
```

### 响应字段（strategies 内每条记录）

| 字段 | 类型 | 说明 |
|------|------|------|
| `strategy_id` | string (UUID) | 策略 ID |
| `strategy_name` | string | 策略名称 |
| `total_pnl` | string | 策略总盈亏金额，保留 2 位小数 |
| `total_pnl_rate` | string | 策略总盈亏百分比，保留 2 位小数 |
| `max_drawdown` | string | 最大回撤百分比，保留 2 位小数（始终为正值，如 `"3.45"` 表示 3.45% 回撤） |
| `trade_count` | integer | 成交笔数（已成交订单数） |
| `win_rate` | string | 胜率百分比，保留 2 位小数（如 `"65.00"` 表示 65%） |

> **已知限制**（来自 UI-Checklist BA-03）：当前版本仅返回各策略独立绩效，**不包含** 组合整体指标（`max_drawdown`、`sharpe_ratio`、`win_rate`）。前端类型定义中预留了这 3 个字段，但后端暂未返回，导致绩效卡片区域显示 `--`。计划在后续版本补充。

> **数据来源**：`total_pnl` 和 `total_pnl_rate` 取自 `backtest_results.metrics.total_return`；`max_drawdown` 和 `win_rate` 取自 `backtest_results.metrics` 中对应字段；`trade_count` 统计该策略下状态为 `filled` 的订单数。

---

## 4. GET /portfolio/equity_curve — 权益曲线数据

获取当前用户（或指定用户）的权益曲线时序数据，支持按粒度和日期范围筛选。

### 请求

```
GET /api/v1/portfolio/equity_curve
Authorization: Bearer <access_token>
```

### 查询参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `user_id` | string (UUID) | 否 | 当前用户 | 目标用户 ID，仅 admin 可用 |
| `start_date` | string | 否 | — | 起始日期，格式 `YYYY-MM-DD`（如 `2026-04-14`） |
| `end_date` | string | 否 | — | 截止日期，格式 `YYYY-MM-DD`（如 `2026-05-14`），截止时间取当天 23:59:59 |
| `granularity` | string | 否 | `day` | 数据粒度：`hour`（小时）/ `day`（日）/ `week`（周） |

> **已知限制**（来自 UI-Checklist BA-04）：`week` 粒度当前实现与 `day` 相同（仅 `hour` 和 `day` 两个聚合分支），周级别聚合逻辑计划在后续版本补充。

### curl 示例

```bash
# 获取默认（日线）权益曲线
curl -s 'http://localhost:8080/api/v1/portfolio/equity_curve' \
  -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9...'

# 指定日期范围 + 小时粒度
curl -s 'http://localhost:8080/api/v1/portfolio/equity_curve?start_date=2026-05-01&end_date=2026-05-14&granularity=hour' \
  -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9...'

# 最近 30 天日线
curl -s 'http://localhost:8080/api/v1/portfolio/equity_curve?start_date=2026-04-14&end_date=2026-05-14&granularity=day' \
  -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9...'
```

### 成功响应 (200)

```json
{
  "code": 0,
  "data": {
    "points": [
      {
        "timestamp": "2026-05-01T00:00:00+00:00",
        "equity": "85000.00"
      },
      {
        "timestamp": "2026-05-02T00:00:00+00:00",
        "equity": "86200.00"
      },
      {
        "timestamp": "2026-05-14T00:00:00+00:00",
        "equity": "100000.00"
      }
    ]
  },
  "message": "success"
}
```

### 响应字段（points 内每条记录）

| 字段 | 类型 | 说明 |
|------|------|------|
| `timestamp` | string (ISO 8601) | 数据点时间戳 |
| `equity` | string | 该时刻的权益值，保留 2 位小数 |

### 粒度行为

| 粒度 | 聚合方式 | 时间键格式 | 说明 |
|------|---------|-----------|------|
| `hour` | 按小时取最后一条 | `YYYY-MM-DDThh` | 每小时取最后一个快照 |
| `day` | 按日取最后一条 | `YYYY-MM-DD` | 每日取最后一个快照（默认） |
| `week` | 当前同 `day` | `YYYY-MM-DD` | 周级别聚合待实现 |

### 空结果

当无权益历史数据时：

```json
{
  "code": 0,
  "data": {
    "points": []
  },
  "message": "success"
}
```

---

## 数据模型

### PortfolioSummary

| 字段 | 类型 | 说明 |
|------|------|------|
| `total_equity` | string | 总资产（2 位小数） |
| `daily_pnl` | string | 当日盈亏（2 位小数） |
| `daily_pnl_rate` | string | 当日盈亏率（2 位小数，百分比） |
| `cumulative_pnl` | string | 累计盈亏（2 位小数） |
| `cumulative_pnl_rate` | string | 累计盈亏率（2 位小数，百分比） |
| `total_positions` | integer | 持仓数量 |
| `updated_at` | string (ISO 8601) | 更新时间 |

### PortfolioPosition

| 字段 | 类型 | 说明 |
|------|------|------|
| `symbol` | string | 交易对 |
| `side` | string | 方向：`long` / `short` |
| `quantity` | string | 数量（8 位小数） |
| `avg_price` | string | 均价（8 位小数） |
| `current_price` | string | 现价（8 位小数） |
| `unrealized_pnl` | string | 浮动盈亏（2 位小数） |
| `unrealized_pnl_rate` | string | 浮动盈亏率（2 位小数，百分比） |

### StrategyPerformance

| 字段 | 类型 | 说明 |
|------|------|------|
| `strategy_id` | string (UUID) | 策略 ID |
| `strategy_name` | string | 策略名称 |
| `total_pnl` | string | 总盈亏（2 位小数） |
| `total_pnl_rate` | string | 总盈亏率（2 位小数，百分比） |
| `max_drawdown` | string | 最大回撤（2 位小数，百分比） |
| `trade_count` | integer | 交易次数 |
| `win_rate` | string | 胜率（2 位小数，百分比） |

### EquityCurvePoint

| 字段 | 类型 | 说明 |
|------|------|------|
| `timestamp` | string (ISO 8601) | 时间戳 |
| `equity` | string | 权益值（2 位小数） |

### Paginated\<T\>

| 字段 | 类型 | 说明 |
|------|------|------|
| `items` | T[] | 数据列表 |
| `total` | integer | 总记录数 |
| `page` | integer | 当前页码 |
| `size` | integer | 每页条数 |

### 数据库表

| 表名 | 说明 |
|------|------|
| `portfolio_equity_history` | 权益历史快照（`user_id` + `equity` + `timestamp`），由 `record_equity_snapshot` 写入 |
| `paper_accounts` | 模拟账户（`balance` + `total_pnl` + `initial_balance`），聚合计算总资产和累计盈亏 |
| `positions` | 持仓记录（`symbol` + `side` + `quantity` + `avg_entry_price` + `unrealized_pnl`） |
| `orders` | 订单记录，用于统计当日盈亏和策略交易次数 |
| `user_strategies` | 用户策略，用于绩效对比 |
| `backtest_results` | 回测结果，提供策略绩效指标（`metrics` JSON 字段） |

---

## 错误码

### 通用错误码

| code | HTTP 状态码 | 含义 |
|------|-----------|------|
| 0 | 200 | 成功 |
| 40101 | 401 | 认证失败（Token 缺失/无效） |
| 40102 | 401 | Token 已过期 |
| 40301 | 403 | 权限不足（非 admin 尝试查看他人组合） |
| 50001 | 500 | 内部错误 |
| 50002 | 500 | 数据库错误 |

### 错误响应示例

**未认证：**

```json
{
  "code": 40101,
  "message": "认证失败"
}
```

**越权访问：**

```json
{
  "code": 40301,
  "message": "无权限查看该组合"
}
```

**数据库错误：**

```json
{
  "code": 50002,
  "message": "Database error: connection refused"
}
```

---

## 前端集成速查

### TypeScript 类型定义

```typescript
// 与后端 B1-B4 契约对齐
// B1: snake_case 字段名  B2: UUID → string  B3: Option<T> → T | null  B4: Decimal → string

interface PortfolioSummary {
  total_equity: string
  daily_pnl: string
  daily_pnl_rate: string
  cumulative_pnl: string
  cumulative_pnl_rate: string
  total_positions: number
  updated_at: string
}

interface PortfolioPosition {
  symbol: string
  side: 'long' | 'short'
  quantity: string
  avg_price: string
  current_price: string
  unrealized_pnl: string
  unrealized_pnl_rate: string
}

interface StrategyPerformance {
  strategy_id: string
  strategy_name: string
  total_pnl: string
  total_pnl_rate: string
  max_drawdown: string
  trade_count: number
  win_rate: string
}

interface EquityCurvePoint {
  timestamp: string
  equity: string
}
```

### API 调用示例（Axios）

```typescript
import client from '@/api/client'

// 组合汇总
const summary = await client.get('/portfolio/summary')

// 持仓列表
const positions = await client.get('/portfolio/positions', {
  params: { symbol: 'BTCUSDT', page: 1, size: 10 }
})

// 策略绩效
const performance = await client.get('/portfolio/performance')

// 权益曲线
const equityCurve = await client.get('/portfolio/equity_curve', {
  params: {
    start_date: '2026-04-14',
    end_date: '2026-05-14',
    granularity: 'day'
  }
})
```

---

## 已知限制与后续计划

| 编号 | 限制 | 影响 | 计划 |
|------|------|------|------|
| BA-03 | `/performance` 缺少整体 `max_drawdown`/`sharpe_ratio`/`win_rate` | 绩效卡片显示 `--` | v0.8.0 补充 |
| BA-05 | `/positions` 不支持 `side` 筛选参数 | 前端方向筛选失效 | v0.8.0 补充 |
| BA-08 | `current_price` 使用 `avg_entry_price` 占位 | 浮动盈亏精度不足 | 接入行情服务后修复 |
| BA-04 | `week` 粒度实现与 `day` 相同 | 周线权益曲线数据不准 | v0.8.0 补充周级聚合 |
