# 设计规格：交易前端 WebSocket 实时行情

> 基于 PRD-trade-execution.md + ADR-011 + Design_TradingExecution.md
> 设计师：Designer | 版本 v1.0 | 日期：2026-05-18

---

## 1. 现状分析

### 1.1 已实现组件
| 组件 | 状态 | 位置 |
|------|------|------|
| `api/order.ts` | ✅ 完整 | `frontend/src/api/order.ts` |
| `api/trades.ts` | ✅ 存在 | `frontend/src/api/trades.ts` |
| `types/order.ts` | ✅ 完整类型 | `frontend/src/types/order.ts` |
| `TradingView.vue` UI | ✅ 完整 | `views/trade/TradingView.vue` |
| OrderForm/OrderList | ✅ 完整 | `components/trade/` |
| PositionPanel/TradeRecordTab | ✅ 完整 | `components/order/` |
| KlineChart | ✅ 完整 | `components/kline/` |
| 后端 `/ws` 端点 | ✅ 实现 | `handlers/ws.rs` |
| HubMessage ticker/depth/kline | ✅ 支持 | `ws_hub.rs` |

### 1.2 缺口
| 缺口 | 说明 | 影响 |
|------|------|------|
| wsTimer 模拟 | `setInterval` 模拟价格，非真实行情 | bestBid/bestAsk 虚假 |
| 前端 WS 封装 | 无 `ws.ts` | 无法连接后端 WS |
| WS store | 无 Pinia store | 连接状态无法跨组件共享 |
| 后端 subscribe 解析 | `handlers/ws.rs` 未解析 subscribe 消息 | 无法按 channel 过滤 |

---

## 2. WebSocket 架构

### 2.1 连接流程
```
TradingView.vue onMounted
  → ws.connect(symbol, channels)
  → WebSocket.open → wsConnected = true
  → 发送 subscribe 消息 → 后端注册过滤器
  → 后端 HubMessage 广播 → ws.ts 接收
  → 解析 channel → 更新 reactive state
```

### 2.2 WS 消息格式

**订阅请求（客户端 → 服务端）**
```json
{"action": "subscribe", "channels": ["market:ticker", "market:depth"], "symbol": "BTCUSDT"}
```

**取消订阅**
```json
{"action": "unsubscribe", "channels": ["market:ticker"], "symbol": "BTCUSDT"}
```

**行情推送（服务端 → 客户端）**
```json
{"channel": "market:ticker:BTCUSDT", "symbol": "BTCUSDT", "data": {
  "price": "49500.00", "bid": "49499.00", "ask": "49501.00",
  "change": "+1.25", "changePercent": "+0.03",
  "volume": "12345.67", "high": "50000.00", "low": "49000.00"
}}
```

```json
{"channel": "market:depth:BTCUSDT", "symbol": "BTCUSDT", "data": {
  "bids": [["49499.00", "10.5"], ...],
  "asks": [["49501.00", "8.3"], ...]
}}
```

### 2.3 后端变更：handlers/ws.rs

`subscribe` 消息解析（新增）：
- 解析 `action: "subscribe"` / `action: "unsubscribe"`
- 解析 `channels` 数组 + `symbol`
- 注册 channel 过滤器到 session 级别
- 已有 `HubMessage` 类型无需修改

### 2.4 前端 ws.ts 封装

```typescript
// frontend/src/api/ws.ts
class MarketWebSocket {
  private ws: WebSocket | null = null
  private reconnectAttempts = 0
  private maxReconnectAttempts = 5

  connect(symbol: string, channels: string[]) { ... }
  disconnect() { ... }
  onMessage(handler: (channel: string, data: any) => void) { ... }
}
```

### 2.5 Pinia Store

```typescript
// frontend/src/stores/trading.ts
export const useTradingStore = defineStore('trading', () => {
  const wsConnected = ref(false)
  const bestBid = ref<string | null>(null)
  const bestAsk = ref<string | null>(null)
  const ws = new MarketWebSocket()
  return { wsConnected, bestBid, bestAsk, ws }
})
```

---

## 3. TradingView.vue 改造

### 3.1 替换 wsTimer

**Before：**
```typescript
let wsTimer: ReturnType<typeof setInterval> | null = null
function connectWebSocket() {
  wsConnected.value = true
  wsTimer = setInterval(() => { ... }, 5000)
}
```

**After：**
```typescript
import { useTradingStore } from '@/stores/trading'
const trading = useTradingStore()

onMounted(async () => {
  await Promise.all([loadSymbolConfigs(), loadAccount(), loadPositions(), loadTrades(), loadKlineData()])
  trading.ws.connect(selectedSymbol.value, ['ticker', 'depth'])
  trading.ws.onMessage((channel, data) => {
    if (channel === 'market:ticker') {
      bestBid.value = data.bid
      bestAsk.value = data.ask
    }
  })
})
onUnmounted(() => trading.ws.disconnect())
```

---

## 4. API 数据流（改造后）

```
┌─────────────────────────────────────────────────────────────┐
│  后端                                                         │
│  Binance WS → ws_hub.rs → HubMessage → handlers/ws.rs → 广播 │
└─────────────────────┬───────────────────────────────────────┘
                      │ WebSocket /ws
                      ▼
┌─────────────────────────────────────────────────────────────┐
│  前端                                                         │
│  ws.ts → Pinia store → TradingView reactive                  │
│                           ↓                                   │
│  bestBid/bestAsk 更新 → OrderForm 显示买卖价                  │
│  depth 更新     → KlineChart 深度图                          │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. 验收标准

| 验收项 | 判定标准 |
|--------|----------|
| WS 连接状态 | 连接成功 wsStatusClass = 'ws-status--connected' |
| bestBid/bestAsk | 来自真实 Binance WS 推送，非模拟值 |
| 重连机制 | WS 断开后 3s 内自动重连，最多 5 次 |
| 订阅解析 | 后端按 channel 过滤，不全量广播 |
| 性能 | WS 消息延迟 < 100ms |

---

## 6. 文件变更清单

| 文件 | 操作 |
|------|------|
| `frontend/src/api/ws.ts` | 新建 |
| `frontend/src/stores/trading.ts` | 新建 |
| `frontend/src/views/trade/TradingView.vue` | 修改：替换 wsTimer |
| `backend/src/handlers/ws.rs` | 修改：解析 subscribe 消息 |
| `docs/adr/ADR-011-TradingFront-WS.md` | 新建 |
| `docs/design/Design_TradingFront_WS.md` | 新建 |
| `docs/checklists/T1_TradingFront_WS_20260518.md` | 新建 |