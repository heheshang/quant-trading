# T2-Architecture-Binance-WS-Connector-20260517

## 检查时间
2026-05-17 10:30

## 功能名称
**Binance WebSocket Connector（市场数据管道 Phase 2）**

---

## ✅ 完成标准（DoD）

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 后端 TechDesign 已评审 | ✅ | 本文档 |
| 前端 TechDesign | N/A | Phase 3 才涉及 |
| 数据库表结构 | N/A | Phase 4 才涉及 |
| 状态机流转 | N/A | 不需要 |

---

## 📐 模块结构

```
backend/src/services/exchange/
├── mod.rs                  # 模块入口 + 导出
├── binance_connector.rs    # Binance WS 连接器（核心）
├── types.rs                # Binance 原始消息类型
└── errors.rs               # Connector 错误类型
```

---

## 📦 数据结构设计

### 1. BinanceStreamMessage（WS 原始消息）

```rust
pub struct BinanceStreamMessage {
    pub stream: String,       // e.g. "btcusdt@ticker"
    pub data: BinanceData,   // Ticker | Depth | Kline | Unknown
}
```

### 2. BinanceData（联合类型）

```rust
pub enum BinanceData {
    Ticker(TickerData),    // <symbol>@ticker
    Depth(DepthData),      // <symbol>@depth20@100ms
    Kline(KlineData),      // <symbol>@kline_1m
    Unknown(JsonValue),    // 未解析
}
```

### 3. TickerData（24hr ticker）

| 字段 | 类型 | 说明 |
|------|------|------|
| s | String | Symbol "BTCUSDT" |
| c | String | Last price |
| p | String | Price change (24h) |
| P | String | Price change percent (24h) |
| v | String | Volume (base) |
| h | String | High price (24h) |
| l | String | Low price (24h) |
| b | String | Bid price |
| a | String | Ask price |
| E | u64 | Event time (ms) |

### 4. MarketMessage（内部标准化消息）

```rust
pub enum MarketMessage {
    Ticker { symbol, price, change, change_percent, volume, high, low, bid, ask, timestamp },
    Depth { symbol, bids: Vec<(price, qty)>, asks: Vec<(price, qty)>, timestamp },
    Kline { symbol, interval, open, high, low, close, volume, close_time, timestamp },
}
```

---

## 🔌 API 设计

### ExchangeConnector Trait

```rust
pub trait ExchangeConnector: Send + Sync {
    async fn start(&self) -> Result<(), ConnectorError>;
    fn subscribe(&self) -> broadcast::Receiver<MarketMessage>;
    fn stop(&mut self);
}
```

### BinanceConnector 实现

```rust
pub struct BinanceConnector {
    config: BinanceConnectorConfig,
    tx: broadcast::Sender<MarketMessage>,  // 消息分发
    shutdown_tx: Option<oneshot::Sender<()>>,
}
```

---

## 🔄 重连策略

| 参数 | 值 |
|------|------|
| Initial delay | 1s |
| Max delay | 30s |
| Backoff | 2x |
| Ping interval | 60s |
| Max retries | 无限制（永久） |

---

## 📡 Redis Pub/Sub 分发

| Channel | 消息类型 | 频率 |
|---------|----------|------|
| `market:ticker` | MarketMessage::Ticker | ~1/sec |
| `market:depth` | MarketMessage::Depth | ~10/sec |
| `market:kline` | MarketMessage::Kline | ~1/min |

---

## ⚠️ 错误处理

| 错误 | 处理 |
|------|------|
| ConnectionFailed | 指数退避重连 |
| RateLimited(s) | 等待 s 秒后重连 |
| MessageParseError | 记录日志，跳过消息 |
| Disconnected | 立即重连（初始延迟） |

---

## ✅ 检查结果

**通过** - 架构设计清晰，模块边界明确，数据流顺畅。

---

## 📦 归档产出物

- `docs/architecture/ADR-006-Binance-WS-Connector.md`
- `backend/src/services/exchange/` 目录（含 types.rs, errors.rs, binance_connector.rs, mod.rs）