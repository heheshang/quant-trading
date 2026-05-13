# 策略回测引擎 — API 设计文档

> 版本: v1.0 | 基于: ADR-007, PRD-backtest-engine.md
> 说明：本文档是对 PRD 第 6 章 API 设计的补充细化，不是重写。

---

## 1. API 总览

| 方法 | 路径 | 说明 | 优先级 | 对应 User Story |
|------|------|------|--------|----------------|
| POST | `/api/v1/backtest/run` | 启动回测 | P0 | US-BT-01 |
| GET | `/api/v1/backtest/{id}/result` | 获取回测结果 | P0 | US-BT-02/03/04 |
| GET | `/api/v1/backtest/{id}/progress` | 获取进度（轮询） | P1 | US-BT-06 |
| POST | `/api/v1/backtest/{id}/cancel` | 取消回测 | P1 | US-BT-06 |
| GET | `/api/v1/backtest/history` | 策略回测历史列表 | P1 | US-BT-05 |
| DELETE | `/api/v1/backtest/{id}` | 删除回测记录 | P2 | US-BT-05 |

---

## 2. 端点详情

### 2.1 POST /api/v1/backtest/run

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

**验证规则（补充 PRD）：**
| 字段 | 规则 | 错误码 |
|------|------|--------|
| start_date | < end_date, 间隔 ≥ 7 天 | 42301 |
| initial_capital | ≥ 100 | 42301 |
| fee_rate | [0, 0.01] | 42301 |
| slippage_rate | [0, 0.01] | 42301 |
| strategy_id | 必须存在且属于当前用户 | 40401 |
| K 线数据 | symbol+interval+时间范围必须有数据 | 42302 |
| 并发数 | 当前运行中回测 < 5 | 42303 |

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

**关键设计决策：** 回测是异步的，API 立即返回 202，前端通过 WS 或轮询获取进度。

---

### 2.2 GET /api/v1/backtest/{id}/result

**响应 (200)：**

基础字段参考 PRD。此处补充大字段控制：

**查询参数：**
| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| include_trades | bool | false | 是否包含交易明细数组 |
| include_curve | bool | false | 是否包含权益曲线数组 |
| max_curve_points | int | 5000 | 权益曲线采样上限 |

**理由：** trades 和 equity_curve 可能很大，默认不返回。前端先加载 KPI 指标，用户点击标签时再拉取明细。

**示例：** `GET /api/v1/backtest/{id}/result?include_trades=true&include_curve=true&max_curve_points=2000`

---

### 2.3 GET /api/v1/backtest/{id}/progress

**响应：**
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

**前端决策：** 优先使用 WebSocket 推送（每 10%），此 API 作为兜底轮询（每 2s）。WS 断开时前端自动切到轮询。

---

### 2.4 POST /api/v1/backtest/{id}/cancel

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
- 404：回测不存在
- 42304：回测状态不是 running（已完成/已取消的不能再次取消）

---

### 2.5 GET /api/v1/backtest/history

**查询参数：**
| 参数 | 类型 | 必填 | 默认 | 说明 |
|------|------|------|------|------|
| strategy_id | UUID | 是 | - | 策略 ID |
| page | int | 否 | 1 | 页码 |
| size | int | 否 | 20 | 每页条数 (max 100) |

**响应：**
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

**设计说明：** 历史列表只返回简化的指标预览（4 个核心指标），完整指标需要点击进入详情页获取。

---

### 2.6 DELETE /api/v1/backtest/{id}

**响应 (200)：** 空 body

**规则：** 只能删除已完成的或已失败的记录。运行中的记录不能删除（必须先取消）。

---

## 3. WebSocket 事件（增量补充）

参考 PRD 6.3 节，补充以下事件类型：

### backtest_progress
```json
{
  "type": "backtest_progress",
  "data": {
    "backtest_id": "660e8400-...",
    "status": "running",
    "progress": 45,
    "current_bar": 3942,
    "total_bars": 8760,
    "elapsed_ms": 2100
  }
}
```

### backtest_completed
```json
{
  "type": "backtest_completed",
  "data": {
    "backtest_id": "660e8400-...",
    "status": "completed",
    "duration_ms": 4230
  }
}
```

前端收到 `backtest_completed` 后应：
1. 停止进度动画
2. 拉取 `GET /api/v1/backtest/{id}/result` 获取完整结果
3. 切换视图到结果模式

---

## 4. 错误码（补充）

| 码 | 名称 | HTTP | 说明 |
|----|------|------|------|
| 42301 | ERR_BACKTEST_INVALID_RANGE | 400 | 日期范围/金额/费率无效 |
| 42302 | ERR_BACKTEST_NO_DATA | 400 | 所选时间范围内无 K 线数据 |
| 42303 | ERR_BACKTEST_CONCURRENT_LIMIT | 429 | 并发回测超限（上限5） |
| 40401 | ERR_BACKTEST_NOT_FOUND | 404 | 回测记录不存在 |
| 42304 | ERR_BACKTEST_CANCEL_FAILED | 400 | 取消失败（非 running 状态） |
| 40100 | ERR_UNAUTHORIZED | 401 | 未登录/Token 过期 |
| 40300 | ERR_FORBIDDEN | 403 | 无权操作（操作他人策略/回测） |

---

## 5. 前端 Pinia Store

```typescript
// stores/backtest.ts
export const useBacktestStore = defineStore('backtest', () => {
  const selectedStrategyId = ref<string | null>(null)
  const config = ref<BacktestParams | null>(null)
  const isLoading = ref(false)

  // Running state
  const currentRunId = ref<string | null>(null)
  const currentStatus = ref<'idle' | 'pending' | 'running' | 'completed' | 'failed'>('idle')
  const progress = ref(0)
  const errorMessage = ref<string | null>(null)

  // Result state
  const currentResult = ref<BacktestResult | null>(null)
  const history = ref<BacktestSummary[]>([])
  const historyTotal = ref(0)

  // Actions
  async function runBacktest(strategyId: string, cfg: BacktestParams) { /*...*/ }
  async function fetchResult() { /*...*/ }
  async function cancelRun() { /*...*/ }
  async function fetchHistory(strategyId: string, page: number) { /*...*/ }
  function reset() { /*...*/ }
})
```

---

## 6. 前端类型补充

```typescript
// Result API 请求参数控制
interface ResultQuery {
  include_trades?: boolean   // 默认 false
  include_curve?: boolean    // 默认 false
  max_curve_points?: number  // 默认 5000
}

// equity_curve 点
interface EquityPoint {
  time: number        // ms timestamp
  equity: number      // 账户权益（USDT）
  drawdown_pct: number // 回撤百分比（负值）
}

// 交易记录
interface TradeRecord {
  entry_time: number
  exit_time: number
  direction: 'long' | 'short'
  entry_price: number
  exit_price: number
  quantity: number
  pnl_usdt: number
  pnl_pct: number
  holding_period_ms: number
  exit_reason: 'signal' | 'stop_loss' | 'take_profit' | 'liquidation'
  fee: number
  slippage: number
}
```

---

## 7. 路由设计

| 路径 | 组件 | 模式 | 说明 |
|------|------|------|------|
| `/backtest` | `BacktestView.vue` | 配置/结果双模式 | MVP 阶段不拆子路由 |

前端视图切换逻辑：
1. 初始加载 → 配置模式（策略选择 + 参数配置）
2. 点击"运行回测" → 进度模式（进度条 + 取消按钮）
3. 回测完成 → 结果模式（KPI + 图表 + 标签页）
4. 点击"返回配置"或"重新回测" → 回到配置模式
