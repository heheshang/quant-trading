# 回测引擎 API 设计文档（修正版 v1.1）

> 版本: v1.1 | 修正: ADR-008 | 基于: ADR-007, ADR-008

## 1. 变更记录

| 版本 | 日期 | 变更 | 原因 |
|------|------|------|------|
| v1.0 | 2026-05-13 | 初始版本 | ADR-007 发布 |
| v1.1 | 2026-05-13 | 修正前端适配、路由澄清 | ADR-008 评审发现 |

**关键变更：**
- 前后端路由已对齐（后端为标准，前端适配）
- `POST /backtest/{id}/cancel` 已补充
- GET /backtest/{id}/result 增加 `include_trades` / `include_curve` / `max_curve_points` 参数
- response 示例与实际后端类型一致（非简化版）

---

## 2. API 总览

| 方法 | 路径 | 说明 | 优先级 | 实现状态 |
|------|------|------|--------|---------|
| POST | `/api/v1/backtest/run` | 启动回测 | P0 | ✅ handler ✅ engine ❌ DB |
| GET | `/api/v1/backtest/{id}/result` | 获取回测结果 | P0 | ✅ handler ❌ DB |
| GET | `/api/v1/backtest/{id}/progress` | 获取回测进度（轮询） | P1 | ✅ handler ❌ DB |
| POST | `/api/v1/backtest/{id}/cancel` | 取消运行中的回测 | P1 | ✅ handler ⚠️ 需 CancellationToken registry |
| GET | `/api/v1/backtest/history` | 获取策略回测历史列表 | P1 | ✅ handler ❌ DB |
| DELETE | `/api/v1/backtest/{id}` | 删除回测记录 | P2 | ✅ handler ❌ DB |

---

## 3. 端点详情

### 3.1 POST /api/v1/backtest/run

**请求体：**
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

**验证规则：**
| 字段 | 规则 | 后端状态 |
|------|------|---------|
| start_date | < end_date, 间隔 ≥ 7 天 | ✅ validate() |
| initial_capital | ≥ 100 | ✅ validate() |
| fee_rate | [0, 0.01] | ✅ validate() |
| slippage_rate | [0, 0.01] | ✅ validate() |
| strategy_id | 必须存在且属于当前用户 | ❌ DB 占位 |
| K 线数据 | 时间范围内必须有数据 | ❌ DB 占位 |
| 并发 | 同时回测 ≤ 5 | ✅ Semaphore |

**响应 (202 Accepted)：**
```json
{
  "code": 0,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "status": "pending",
    "progress": 0,
    "created_at": "2026-05-13T07:00:00Z"
  },
  "message": "success"
}
```

**错误响应：**
```json
// 429 并发超限
{ "code": 42303, "message": "backtest concurrency limit reached (5)" }

// 400 无K线数据
{ "code": 42302, "message": "no kline data for the specified range" }

// 404 策略不存在
{ "code": 40401, "message": "strategy not found" }
```

---

### 3.2 GET /api/v1/backtest/{id}/result

**查询参数：**
| 参数 | 类型 | 默认 | 说明 |
|------|------|------|------|
| include_trades | bool | false | 包含交易明细 |
| include_curve | bool | false | 包含权益曲线 |
| max_curve_points | int | 5000 | 曲线采样上限 |

**响应 (200)：**
```json
{
  "code": 0,
  "data": {
    "id": "660e8400-...",
    "strategy_id": "550e8400-...",
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
      "annualized_return_pct": 22.1,
      "max_drawdown_pct": -12.5,
      "sharpe_ratio": 1.85,
      "sortino_ratio": 2.1,
      "calmar_ratio": 1.77,
      "win_rate": 62.0,
      "total_trades": 85,
      "profit_factor": 2.3,
      "avg_win_pct": 3.2,
      "avg_loss_pct": -1.8,
      "avg_trade_pct": 0.3,
      "total_fees": 1250.0,
      "total_slippage": 620.0
    },
    "trades": [
      {
        "entry_time": 1704067200000,
        "exit_time": 1704153600000,
        "direction": "long",
        "entry_price": 42350.0,
        "exit_price": 43120.0,
        "quantity": 2.0,
        "pnl_usdt": 1540.0,
        "pnl_pct": 3.64,
        "holding_period_ms": 86400000,
        "exit_reason": "signal",
        "fee": 42.35,
        "slippage": 21.18
      }
    ],
    "equity_curve": [
      {
        "time": 1704067200000,
        "equity": 100000.0,
        "drawdown_pct": 0.0
      }
    ],
    "start_time": "2026-05-13T07:00:01Z",
    "end_time": "2026-05-13T07:00:04Z",
    "duration_ms": 3120,
    "error": null,
    "created_at": "2026-05-13T07:00:00Z"
  }
}
```

---

### 3.3 GET /api/v1/backtest/{id}/progress

**响应 (200)：**
```json
{
  "code": 0,
  "data": {
    "id": "660e8400-...",
    "status": "running",
    "progress": 45,
    "current_bar": 3942,
    "total_bars": 8760,
    "elapsed_ms": 2100
  }
}
```

**状态说明：**
- `pending` → `running` → `completed` / `failed` / `cancelled`
- 前端每 2s 轮询此端点
- 在 WS 推送实现后，此 API 作为兜底

---

### 3.4 POST /api/v1/backtest/{id}/cancel

**请求体：** 无

**响应 (200)：**
```json
{
  "code": 0,
  "data": null,
  "message": "cancellation requested"
}
```

**错误场景：**
- 404 — 回测 ID 不存在或未运行中
- 400 — 回测状态不是 running

**实现依赖：**
- ✅ CancellationToken 创建 → handlers/backtest.rs 已实现
- ❌ 注册表 DashMap<Uuid, CancellationToken> → TODO (ADR-008 Decision 5)

---

### 3.5 GET /api/v1/backtest/history

**查询参数：**
| 参数 | 类型 | 必填 | 默认 | 说明 |
|------|------|------|------|------|
| strategy_id | UUID | 是 | - | 策略 ID |
| page | int | 否 | 1 | 页码 |
| size | int | 否 | 20 | 每页条数 (max 100) |

**响应 (200)：**
```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": "660e8400-...",
        "status": "completed",
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
        "start_time": "2026-05-13T07:00:01Z",
        "duration_ms": 4230,
        "created_at": "2026-05-13T07:00:01Z"
      }
    ],
    "total": 15,
    "page": 1,
    "size": 10
  }
}
```

---

### 3.6 DELETE /api/v1/backtest/{id}

**响应 (200)：** `{"code": 0, "data": null, "message": "success"}`

**规则：** 只能删除已完成的或已失败的记录。

---

## 4. WebSocket 事件

未来实现，当前通过轮询替代。

| 事件 | 说明 | 实现优先级 |
|------|------|-----------|
| `backtest_progress` | 每 10% 推送进度 | P1 |
| `backtest_completed` | 回测完成通知 | P1 |

---

## 5. 错误码

| 码 | 名称 | HTTP | 说明 |
|----|------|------|------|
| 42301 | ERR_BACKTEST_INVALID_RANGE | 400 | 日期范围 / 金额 / 费率无效 |
| 42302 | ERR_BACKTEST_NO_DATA | 400 | 所选时间范围内无 K 线数据 |
| 42303 | ERR_BACKTEST_CONCURRENT_LIMIT | 429 | 并发回测超限 (上限 5) |
| 40401 | ERR_BACKTEST_NOT_FOUND | 404 | 回测记录不存在 |
| 42304 | ERR_BACKTEST_CANCEL_FAILED | 400 | 取消失败（非 running 状态） |

---

## 6. 前端适配指引

前端 `api/backtest.ts` 需要对照 v1.0 → v1.1 做以下变更：

1. 路由：`/backtest` → `/backtest/run` (POST)
2. 请求体：扁平结构 → `{ strategy_id, config: {...} }` 嵌套
3. 移除 `/backtest/jobs/{jobId}` 端点 (不存在)
4. 增加 `/backtest/{id}/progress` 轮询端点
5. 增加 `/backtest/{id}/cancel` 取消端点
6. GET 结果：`/backtest/{id}` → `/backtest/{id}/result`
7. ID 类型：`number` → `string` (UUID)
8. 新增字段：`interval`, `fee_rate`, `slippage_rate`

详情见 `ADR-008-backtest-engine-frontend-alignment.md` 中的适配层代码。
