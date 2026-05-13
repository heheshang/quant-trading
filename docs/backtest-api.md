# 回测引擎 API 文档

> 版本: v0.3.0 | 基础路径: `/api/v1` | 认证: Bearer Token (JWT)

---

## 目录

- [API 总览](#api-总览)
- [1. POST /backtest — 运行回测](#1-post-backtest--运行回测)
- [2. GET /backtest/{id} — 获取回测结果](#2-get-backtestid--获取回测结果)
- [3. GET /backtest/{id}/trades — 获取交易明细](#3-get-backtestidtrades--获取交易明细)
- [4. GET /backtest/{id}/equity — 获取权益曲线](#4-get-backtestidequity--获取权益曲线)
- [5. GET /backtest/history — 回测历史列表](#5-get-backtesthistory--回测历史列表)
- [6. DELETE /backtest/{id} — 删除回测记录](#6-delete-backtestid--删除回测记录)
- [7. POST /backtest/{id}/cancel — 取消运行中回测](#7-post-backtestidcancel--取消运行中回测)
- [数据模型](#数据模型)
- [错误码](#错误码)

---

## API 总览

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| POST | `/api/v1/backtest` | 运行回测（异步） | 必需 |
| GET | `/api/v1/backtest/{id}` | 获取回测完整结果 | 必需 |
| GET | `/api/v1/backtest/{id}/trades` | 获取交易明细 | 必需 |
| GET | `/api/v1/backtest/{id}/equity` | 获取权益曲线（采样至 2000 点） | 必需 |
| GET | `/api/v1/backtest/history` | 回测历史列表（分页） | 必需 |
| DELETE | `/api/v1/backtest/{id}` | 删除回测记录 | 必需 |
| POST | `/api/v1/backtest/{id}/cancel` | 取消运行中的回测 | 必需 |

---

## 1. POST /backtest — 运行回测

提交回测任务，异步执行。系统立即返回回测 ID，前端通过轮询或 WebSocket 获取进度。

### 请求体

```json
{
  "strategy_id": "550e8400-e29b-41d4-a716-446655440000",
  "config": {
    "symbol": "BTC/USDT",
    "interval": "1h",
    "start_date": "2024-01-01",
    "end_date": "2024-12-31",
    "initial_capital": 100000.0,
    "fee_rate": 0.001,
    "slippage_rate": 0.0005
  }
}
```

### 请求参数说明

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `strategy_id` | UUID | 是 | — | 策略 ID，必须存在且属于当前用户 |
| `config.symbol` | string | 是 | — | 交易对，如 `BTC/USDT`、`ETH/USDT` |
| `config.interval` | string | 是 | — | K 线周期: `1m` / `5m` / `15m` / `30m` / `1h` / `2h` / `4h` / `1d` / `1w` |
| `config.start_date` | string | 是 | — | 开始日期，格式 `YYYY-MM-DD` |
| `config.end_date` | string | 是 | — | 结束日期，格式 `YYYY-MM-DD` |
| `config.initial_capital` | number | 是 | — | 初始资金（USDT），≥ 100 |
| `config.fee_rate` | number | 否 | `0.001` | 手续费率，范围 [0, 0.01] |
| `config.slippage_rate` | number | 否 | `0.0005` | 滑点率，范围 [0, 0.01] |

### 验证规则

| 规则 | 说明 |
|------|------|
| `start_date` < `end_date` | 开始日期必须早于结束日期 |
| 日期间隔 ≥ 7 天 | 至少 7 天的回测数据才有统计意义 |
| `initial_capital` ≥ 100 | 初始资金不得低于 100 USDT |
| `fee_rate` ∈ [0, 0.01] | 手续费率 0% ~ 1% |
| `slippage_rate` ∈ [0, 0.01] | 滑点率 0% ~ 1% |
| 并发限制 ≤ 5 | 同时运行的回测任务不超过 5 个 |

### 响应 200

```json
{
  "code": 0,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "status": "running",
    "progress": 0,
    "created_at": "2026-05-13T07:00:00Z"
  },
  "message": "success"
}
```

### 响应字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 回测结果 ID，用于后续查询 |
| `status` | string | 初始状态为 `running` |
| `progress` | integer | 进度百分比 (0 ~ 100) |
| `created_at` | datetime | 创建时间 (ISO 8601) |

### curl 示例

```bash
curl -X POST http://localhost:3000/api/v1/backtest \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "strategy_id": "550e8400-e29b-41d4-a716-446655440000",
    "config": {
      "symbol": "BTC/USDT",
      "interval": "1h",
      "start_date": "2024-01-01",
      "end_date": "2024-12-31",
      "initial_capital": 100000.0,
      "fee_rate": 0.001,
      "slippage_rate": 0.0005
    }
  }'
```

### 错误响应示例

**验证失败 (400):**
```json
{
  "code": 40002,
  "message": "Validation error: start_date must be before end_date"
}
```

**策略不存在 (404):**
```json
{
  "code": 40401,
  "message": "Not found: strategy not found"
}
```

**并发超限 (429):**
```json
{
  "code": 42902,
  "message": "Too many requests: backtest concurrency limit reached (5)"
}
```

---

## 2. GET /backtest/{id} — 获取回测结果

获取回测的完整结果，包括配置、绩效指标、交易明细和权益曲线。

### 路径参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 回测结果 ID |

### 响应 200

```json
{
  "code": 0,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "strategy_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "completed",
    "progress": 100,
    "config": {
      "symbol": "BTC/USDT",
      "interval": "1h",
      "start_date": "2024-01-01",
      "end_date": "2024-12-31",
      "initial_capital": 100000.0,
      "fee_rate": 0.001,
      "slippage_rate": 0.0005
    },
    "metrics": {
      "total_return_pct": 25.3,
      "annualized_return_pct": 25.3,
      "max_drawdown_pct": -12.5,
      "sharpe_ratio": 1.85,
      "sortino_ratio": 2.41,
      "calmar_ratio": 2.02,
      "win_rate": 58.82,
      "total_trades": 85,
      "profit_factor": 1.73,
      "avg_win_pct": 2.15,
      "avg_loss_pct": -1.42,
      "avg_trade_pct": 0.72,
      "total_fees": 850.0,
      "total_slippage": 425.0
    },
    "trades": [
      {
        "entry_time": 1704067200000,
        "exit_time": 1704153600000,
        "direction": "long",
        "entry_price": 42000.0,
        "exit_price": 43100.0,
        "quantity": 2.38,
        "pnl_usdt": 2498.8,
        "pnl_pct": 2.50,
        "holding_period_ms": 86400000,
        "exit_reason": "signal",
        "fee": 101.78,
        "slippage": 50.89
      }
    ],
    "equity_curve": [
      { "time": 1704067200000, "equity": 100000.0, "drawdown_pct": 0.0 },
      { "time": 1704153600000, "equity": 102380.2, "drawdown_pct": 0.0 }
    ],
    "start_time": "2026-05-13T07:00:01Z",
    "end_time": "2026-05-13T07:00:05Z",
    "duration_ms": 4230,
    "error": null
  },
  "message": "success"
}
```

### 响应字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 回测结果 ID |
| `strategy_id` | UUID | 关联策略 ID |
| `status` | string | 状态: `running` / `completed` / `failed` |
| `progress` | integer | 进度 (0 ~ 100)，完成时为 100 |
| `config` | object | 回测配置（同请求参数） |
| `metrics` | object \| null | 绩效指标，运行中或失败时为 null |
| `trades` | array \| null | 交易记录列表，运行中时为 null |
| `equity_curve` | array \| null | 权益曲线，运行中时为 null |
| `start_time` | datetime \| null | 开始执行时间 |
| `end_time` | datetime \| null | 执行完成时间 |
| `duration_ms` | integer \| null | 执行耗时（毫秒） |
| `error` | string \| null | 失败原因，成功时为 null |

### curl 示例

```bash
curl http://localhost:3000/api/v1/backtest/660e8400-e29b-41d4-a716-446655440001 \
  -H "Authorization: Bearer <access_token>"
```

---

## 3. GET /backtest/{id}/trades — 获取交易明细

仅返回回测的交易记录列表，适用于先加载指标后按需加载明细的场景。

### 路径参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 回测结果 ID |

### 响应 200

```json
{
  "code": 0,
  "data": [
    {
      "entry_time": 1704067200000,
      "exit_time": 1704153600000,
      "direction": "long",
      "entry_price": 42000.0,
      "exit_price": 43100.0,
      "quantity": 2.38,
      "pnl_usdt": 2498.8,
      "pnl_pct": 2.50,
      "holding_period_ms": 86400000,
      "exit_reason": "signal",
      "fee": 101.78,
      "slippage": 50.89
    }
  ],
  "message": "success"
}
```

### TradeRecord 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `entry_time` | integer | 入场时间（毫秒时间戳） |
| `exit_time` | integer | 出场时间（毫秒时间戳） |
| `direction` | string | 方向: `long` / `short` |
| `entry_price` | number | 入场价格（含滑点） |
| `exit_price` | number | 出场价格（含滑点） |
| `quantity` | number | 交易数量 |
| `pnl_usdt` | number | 盈亏金额（USDT） |
| `pnl_pct` | number | 盈亏百分比 |
| `holding_period_ms` | integer | 持仓时长（毫秒） |
| `exit_reason` | string | 平仓原因: `signal` / `stop_loss` / `take_profit` / `liquidation` |
| `fee` | number | 手续费总额（开仓 + 平仓） |
| `slippage` | number | 滑点成本总额（开仓 + 平仓） |

### curl 示例

```bash
curl http://localhost:3000/api/v1/backtest/660e8400-e29b-41d4-a716-446655440001/trades \
  -H "Authorization: Bearer <access_token>"
```

---

## 4. GET /backtest/{id}/equity — 获取权益曲线

返回回测的权益曲线数据。当数据点超过 2000 个时，系统自动均匀采样至 2000 点（保留首尾数据点）。

### 路径参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 回测结果 ID |

### 响应 200

```json
{
  "code": 0,
  "data": [
    { "time": 1704067200000, "equity": 100000.0, "drawdown_pct": 0.0 },
    { "time": 1704153600000, "equity": 102380.2, "drawdown_pct": 0.0 },
    { "time": 1704240000000, "equity": 98700.5, "drawdown_pct": -3.5 }
  ],
  "message": "success"
}
```

### EquityPoint 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `time` | integer | 时间戳（毫秒） |
| `equity` | number | 账户权益（USDT），持仓时按标记价格计算 |
| `drawdown_pct` | number | 当前回撤百分比（负值或 0） |

### curl 示例

```bash
curl http://localhost:3000/api/v1/backtest/660e8400-e29b-41d4-a716-446655440001/equity \
  -H "Authorization: Bearer <access_token>"
```

---

## 5. GET /backtest/history — 回测历史列表

分页查询回测历史记录，支持按策略 ID 筛选。

### 查询参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `strategy_id` | UUID | 否 | — | 按策略 ID 筛选 |
| `page` | integer | 否 | `1` | 页码，≥ 1 |
| `size` | integer | 否 | `20` | 每页条数，1 ~ 100 |

### 响应 200

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": "660e8400-e29b-41d4-a716-446655440001",
        "strategy_id": "550e8400-e29b-41d4-a716-446655440000",
        "status": "completed",
        "progress": 100,
        "config": {
          "symbol": "BTC/USDT",
          "interval": "1h",
          "start_date": "2024-01-01",
          "end_date": "2024-12-31",
          "initial_capital": 100000.0,
          "fee_rate": 0.001,
          "slippage_rate": 0.0005
        },
        "metrics": {
          "total_return_pct": 25.3,
          "sharpe_ratio": 1.85,
          "max_drawdown_pct": -12.5,
          "total_trades": 85
        },
        "trades": null,
        "equity_curve": null,
        "start_time": "2026-05-13T07:00:01Z",
        "end_time": "2026-05-13T07:00:05Z",
        "duration_ms": 4230,
        "error": null
      }
    ],
    "total": 15,
    "page": 1,
    "size": 20
  },
  "message": "success"
}
```

> 历史列表中的 `metrics` 仅包含 4 个核心指标预览（`total_return_pct`、`sharpe_ratio`、`max_drawdown_pct`、`total_trades`），完整指标需通过 `GET /backtest/{id}` 获取。`trades` 和 `equity_curve` 在列表中始终为 null。

### curl 示例

```bash
# 查询所有回测历史
curl "http://localhost:3000/api/v1/backtest/history?page=1&size=20" \
  -H "Authorization: Bearer <access_token>"

# 按策略筛选
curl "http://localhost:3000/api/v1/backtest/history?strategy_id=550e8400-e29b-41d4-a716-446655440000&page=1&size=10" \
  -H "Authorization: Bearer <access_token>"
```

---

## 6. DELETE /backtest/{id} — 删除回测记录

删除回测记录。如果回测仍在运行中，会先发送取消信号再删除。

### 路径参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 回测结果 ID |

### 响应 200

```json
{
  "code": 0,
  "data": null,
  "message": "success"
}
```

### curl 示例

```bash
curl -X DELETE http://localhost:3000/api/v1/backtest/660e8400-e29b-41d4-a716-446655440001 \
  -H "Authorization: Bearer <access_token>"
```

---

## 7. POST /backtest/{id}/cancel — 取消运行中回测

取消正在运行的回测任务。仅对 `status: running` 的回测有效。

### 路径参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 回测结果 ID |

### 请求体

无

### 响应 200

```json
{
  "code": 0,
  "data": {},
  "message": "success"
}
```

### 错误响应

**回测未运行 (404):**
```json
{
  "code": 40401,
  "message": "Not found: backtest not running or not found"
}
```

### curl 示例

```bash
curl -X POST http://localhost:3000/api/v1/backtest/660e8400-e29b-41d4-a716-446655440001/cancel \
  -H "Authorization: Bearer <access_token>"
```

---

## 数据模型

### 回测状态流转

```
running → completed  (成功完成)
       → failed      (执行失败/被取消)
```

| 状态 | 说明 |
|------|------|
| `running` | 回测正在执行中，可通过进度接口查询进度 |
| `completed` | 回测成功完成，`metrics`/`trades`/`equity_curve` 已填充 |
| `failed` | 回测失败，`error` 字段包含失败原因 |

### 交易方向

| 值 | 说明 |
|------|------|
| `long` | 做多：买入开仓，价格上涨盈利 |
| `short` | 做空：卖出开仓，价格下跌盈利 |

### 平仓原因

| 值 | 说明 |
|------|------|
| `signal` | 策略信号触发平仓 |
| `stop_loss` | 触及止损价 |
| `take_profit` | 触及止盈价 |
| `liquidation` | 爆仓（权益归零） |

---

## 错误码

### 通用错误码

| 码 | HTTP | 说明 |
|----|------|------|
| 40001 | 400 | 请求参数错误 |
| 40002 | 400 | 数据验证失败 |
| 40101 | 401 | 凭证无效 |
| 40102 | 401 | Token 已过期 |
| 40103 | 401 | Token 无效 |
| 40301 | 403 | 无权限访问 |
| 40401 | 404 | 资源不存在 |
| 40901 | 409 | 资源冲突 |
| 42901 | 429 | 请求频率超限 |
| 42902 | 429 | 并发回测超限（上限 5） |
| 50001 | 500 | 服务器内部错误 |
| 50002 | 500 | 数据库错误 |

### 回测特有验证错误

| 触发条件 | 错误码 | 错误信息示例 |
|----------|--------|-------------|
| `start_date` 格式错误 | 40002 | `start_date must be YYYY-MM-DD` |
| `start_date` ≥ `end_date` | 40002 | `start_date must be before end_date` |
| 日期间隔 < 7 天 | 40002 | `date range must be at least 7 days` |
| `initial_capital` < 100 | 40002 | `initial_capital must be >= 100` |
| `fee_rate` 超出范围 | 40002 | `fee_rate must be 0..0.01` |
| `slippage_rate` 超出范围 | 40002 | `slippage_rate must be 0..0.01` |
| 策略不存在 | 40401 | `strategy not found` |
| 模板类型无效 | 40002 | `invalid template type` |
| 无 K 线数据 | 40002 | `no kline data for the specified range` |
| 并发超限 | 42902 | `backtest concurrency limit reached (5)` |
| 回测不存在 | 40401 | `backtest result not found` |
| 回测未运行（取消时） | 40401 | `backtest not running or not found` |
