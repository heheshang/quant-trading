# Contract-Review: B1-B4 行情模块字段对齐

| 字段 | 值 |
|------|-----|
| **关联 ADR** | ADR-MARKET-DEPTH-TICKER |
| **审核范围** | frontend ↔ backend 契约字段对齐（行情模块） |
| **日期** | 2026-05-13 |
| **决策者** | Tech Lead |

---

## B1: Ticker 响应字段对齐

### Backend `TickerResponse`（新增，schemas.rs）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerResponse {
    pub symbol: String,            // ✅ 对齐
    pub price: f64,                // ✅ 对齐
    pub change: f64,               // ✅ 对齐
    pub change_percent: f64,       // ✅ 对齐
    pub volume: f64,               // ✅ 对齐
    pub high: f64,                 // ✅ 对齐
    pub low: f64,                  // ✅ 对齐
    pub bid: f64,                  // ✅ 对齐
    pub ask: f64,                  // ✅ 对齐
    pub timestamp: i64,            // ✅ 对齐
}
```

### Frontend `Ticker` (types/market.ts:10-18)

```typescript
export interface Ticker {
  symbol: string          // ✅ 对齐
  price: number           // ✅ 对齐
  change: number          // ✅ 对齐
  change_percent: number  // ✅ 对齐
  volume: number          // ✅ 对齐
  high: number            // ✅ 对齐
  low: number             // ✅ 对齐
  // MISSING: bid           ❌
  // MISSING: ask           ❌
  // MISSING: timestamp     ❌
}
```

### 对齐矩阵

| 字段 | Backend (新增) | Frontend (现有) | PRD 要求 | 状态 | 修复方案 |
|------|---------------|----------------|---------|------|----------|
| symbol | `String` | `string` | 必填 | ✅ OK | — |
| price | `f64` | `number` | 必填 | ✅ OK | — |
| change | `f64` | `number` | 必填 | ✅ OK | — |
| change_percent | `f64` | `number` | 必填 | ✅ OK | — |
| volume | `f64` | `number` | 必填 | ✅ OK | — |
| high | `f64` | `number` | 必填 | ✅ OK | — |
| low | `f64` | `number` | 必填 | ✅ OK | — |
| **bid** | `f64` | **MISSING** | 必填 | ❌ | Frontend 需增加 `bid: number` |
| **ask** | `f64` | **MISSING** | 必填 | ❌ | Frontend 需增加 `ask: number` |
| **timestamp** | `i64` | **MISSING** | 必填 | ❌ | Frontend 需增加 `timestamp: number` |

---

## B2: Depth 响应字段对齐

### Backend `DepthResponse`（新增，schemas.rs）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthLevel {
    pub price: f64,           // ✅ 结构化对象
    pub quantity: f64,        // ✅
    pub total: f64,           // ✅ 累计量
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthResponse {
    pub bids: Vec<DepthLevel>,  // ✅ 结构化对象数组
    pub asks: Vec<DepthLevel>,  // ✅ 结构化对象数组
    pub timestamp: i64,         // ✅
}
```

### Frontend `Depth` (types/market.ts:20-24)

```typescript
export interface Depth {
  bids: [number, number][]   // ❌ 二维数组，需改为 DepthLevel[]
  asks: [number, number][]   // ❌ 二维数组，需改为 DepthLevel[]
  timestamp: number           // ✅
}
```

### 对齐矩阵

| 字段 | Backend (新增) | Frontend (现有) | PRD 要求 | 状态 | 修复方案 |
|------|---------------|----------------|---------|------|----------|
| bids | `Vec<DepthLevel>` | `[number, number][]` | 结构化对象 | ❌ | Frontend 新增 `DepthLevel` 接口，`Depth.bids` 改为 `DepthLevel[]` |
| asks | `Vec<DepthLevel>` | `[number, number][]` | 结构化对象 | ❌ | 同上 |
| timestamp | `i64` | `number` | 必填 | ✅ | — |

### 新增 Frontend `DepthLevel` 接口

```typescript
export interface DepthLevel {
  price: number       // 价格
  quantity: number    // 数量
  total: number       // 累计量
}

export interface Depth {
  bids: DepthLevel[]  // 买盘（价格降序）
  asks: DepthLevel[]  // 卖盘（价格升序）
  timestamp: number
}
```

---

## B3: WS 消息类型对齐

### Backend WS 推送消息（新增 schemas）

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WsOutMessage {
    #[serde(rename = "ticker")]
    Ticker { symbol: String, data: TickerResponse, ts: i64 },
    #[serde(rename = "depth")]
    Depth { symbol: String, data: DepthResponse, ts: i64 },
    #[serde(rename = "depth_update")]
    DepthUpdate { symbol: String, data: DepthResponse, ts: i64 },
    #[serde(rename = "heartbeat")]
    Heartbeat { ts: i64 },
    #[serde(rename = "subscribed")]
    Subscribed { channel: String },
    #[serde(rename = "unsubscribed")]
    Unsubscribed { channel: String },
    #[serde(rename = "kick")]
    Kick { reason: String },
    #[serde(rename = "error")]
    Error { code: i32, message: String },
}

#[derive(Debug, Deserialize)]
pub struct WsInMessage {
    pub action: String,              // "subscribe" | "unsubscribe" | "pong"
    pub channels: Option<Vec<String>>, // subscribe/unsubscribe 时必填
}
```

### Frontend WS 消息类型（新增）

```typescript
// 现有：无 WS 消息类型定义
// 需新增：

export interface WsMessage {
  type: 'ticker' | 'depth' | 'depth_update' | 'heartbeat' | 'subscribed' | 'unsubscribed' | 'kick' | 'error'
  symbol?: string
  data?: Ticker & DepthData  // 根据 type 不同
  ts?: number
  channel?: string
  reason?: string
  code?: number
  message?: string
}

export interface WsSubscribe {
  action: 'subscribe' | 'unsubscribe'
  channels: string[]  // e.g. ['ticker:BTCUSDT', 'depth:ETHUSDT']
}

export interface WsPong {
  action: 'pong'
}
```

### 对齐矩阵

| 消息类型 | Backend (新增) | Frontend (需新增) | PRD 要求 | 状态 |
|---------|---------------|-----------------|---------|------|
| ticker | `WsOutMessage::Ticker` | `WsMessage type='ticker'` | P0 | ❌ 前端缺失 |
| depth | `WsOutMessage::Depth` | `WsMessage type='depth'` | P0 | ❌ 前端缺失 |
| depth_update | `WsOutMessage::DepthUpdate` | `WsMessage type='depth_update'` | P0 | ❌ 前端缺失 |
| heartbeat | `WsOutMessage::Heartbeat` | `WsMessage type='heartbeat'` | P1 | ❌ 前端缺失 |
| subscribed | `WsOutMessage::Subscribed` | `WsMessage type='subscribed'` | P0 | ❌ 前端缺失 |
| unsubscribed | `WsOutMessage::Unsubscribed` | `WsMessage type='unsubscribed'` | P0 | ❌ 前端缺失 |
| kick | `WsOutMessage::Kick` | `WsMessage type='kick'` | P1 | ❌ 前端缺失 |
| error | `WsOutMessage::Error` | `WsMessage type='error'` | P0 | ❌ 前端缺失 |

---

## B4: 查询参数与请求/响应对齐

### GET /api/v1/market/tickers

| 项目 | Backend | Frontend | PRD 要求 | 状态 |
|------|---------|----------|---------|------|
| 路由 | `GET /api/v1/market/tickers` | `client.get('/market/tickers')` | ✅ | ✅ |
| 参数 | 无 | 无 | 无 | ✅ |
| 响应 | `ApiResponse<Vec<TickerResponse>>` | `Promise<Ticker[]>` | Ticker数组 | ✅ (前端需补 bid/ask/timestamp) |

### GET /api/v1/market/ticker

| 项目 | Backend | Frontend | PRD 要求 | 状态 |
|------|---------|----------|---------|------|
| 路由 | `GET /api/v1/market/ticker` | **MISSING** | ✅ | ❌ 前端需新增 `getTicker(symbol)` |
| 参数 | `symbol: String (必填)` | — | — | ❌ |
| 响应 | `ApiResponse<TickerResponse>` | — | 单个Ticker | ❌ |

### GET /api/v1/market/depth

| 项目 | Backend | Frontend | PRD 要求 | 状态 |
|------|---------|----------|---------|------|
| 路由 | `GET /api/v1/market/depth` | `client.get('/market/depth')` | ✅ | ✅ |
| symbol | `String (必填)` | ✅ 传入 | 必填 | ✅ |
| levels | `Option<i32> (默认10)` | **MISSING** | 5/10/20/50 | ❌ 前端需新增 `levels` 参数 |
| 响应 | `ApiResponse<DepthResponse>` (结构化) | `Promise<Depth>` (二维数组) | 结构化对象 | ❌ 需改前端 Depth 类型 |

### GET /api/v1/market/ticker/history (P1)

| 项目 | Backend | Frontend | PRD 要求 | 状态 |
|------|---------|----------|---------|------|
| 路由 | `GET /api/v1/market/ticker/history` | **MISSING** | P1 | ❌ P1 阶段实现 |
| 参数 | `symbol, start, end, page, page_size` | — | — | ❌ |
| 响应 | `ApiResponse<Vec<TickerSnapshot>>` + 分页 | — | — | ❌ |

### PUT /api/v1/market/watchlist (P1)

| 项目 | Backend | Frontend | PRD 要求 | 状态 |
|------|---------|----------|---------|------|
| 路由 | `PUT /api/v1/market/watchlist` | **MISSING** | P1 | ❌ P1 阶段实现 |
| 请求体 | `{ symbols: Vec<String> }` | — | — | ❌ |
| 响应 | `{ symbols, count }` | — | — | ❌ |

---

## 修复优先级

| 优先级 | 问题 | 修复位置 | 说明 |
|--------|------|----------|------|
| **P0 CRITICAL** | Frontend `Ticker` 缺 `bid`/`ask`/`timestamp` | types/market.ts | PRD 必填字段，影响 US-MD-01/05 |
| **P0 CRITICAL** | Frontend `Depth` 用二维数组，需改结构化 `DepthLevel[]` | types/market.ts | ADR D2 决策，影响 US-MD-02/06 |
| **P0 CRITICAL** | Frontend `getDepth()` 缺 `levels` 参数 | api/market.ts | 影响深度档位切换 US-MD-06 |
| **P0 HIGH** | Frontend 缺 `getTicker(symbol)` 函数 | api/market.ts | 单个 Ticker 查询 US-MD-01 |
| **P0 HIGH** | Frontend 缺 WS 消息类型定义 | types/market.ts | WS 推送依赖这些类型 |
| **P1** | Frontend 缺 `getTickerHistory()` 函数 | api/market.ts | US-MD-08 P1 |
| **P1** | Frontend 缺 watchlist API 函数 | api/market.ts | US-MK-08 P1 |
| **P1** | Frontend 缺 WS composable (`useMarketWs`) | composables/ | 连接/订阅/重连管理 |

---

## 结论

**整体评估：❌ 需要重大修改（3 个 CRITICAL 缺口）**

- 核心 API 路由已对齐 ✅（`/market/tickers`, `/market/depth`）
- Ticker 基础字段已对齐 ✅（`symbol`/`price`/`change`/`change_percent`/`volume`/`high`/`low`）
- **关键缺口 1**：前端 `Ticker` 缺 `bid`/`ask`/`timestamp`（3 个必填字段）❌
- **关键缺口 2**：前端 `Depth` 用二维数组，与后端结构化对象不匹配 ❌
- **关键缺口 3**：前端 `getDepth()` 缺 `levels` 参数，无法切换深度档位 ❌
- WS 消息类型完全缺失，前端无法接收推送 ❌
- P1 功能（history/watchlist）前端全部缺失，但属于阶段二

修复 P0 缺口后需重新跑 Contract-Review 确认对齐。
