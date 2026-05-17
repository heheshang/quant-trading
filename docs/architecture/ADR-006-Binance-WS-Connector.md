# ADR-006: Binance WebSocket Connector

## Status
Proposed 2026-05-17

## Context
Phase 1 已实现 Redis 缓存层 + Binance REST 降级，但 REST 轮询存在 1-5 秒延迟，无法满足实时行情推送需求。WebSocket 是必需方案。

## Decision

### 模块结构

```
backend/src/services/exchange/
├── mod.rs                  # ExchangeConnector trait + 工厂函数
├── binance_connector.rs    # Binance WS 连接器实现
├── types.rs                # Binance 原始消息类型
└── errors.rs               # Connector 错误类型
```

### ExchangeConnector Trait

```rust
pub trait ExchangeConnector: Send + Sync {
    async fn connect(&self) -> Result<(), ConnectorError>;
    async fn subscribe(&self, channel: &str) -> Result<(), ConnectorError>;
    async fn unsubscribe(&self, channel: &str) -> Result<(), ConnectorError>;
    fn on_message(&self, callback: impl Fn(MarketMessage) + Send + 'static);
    async fn disconnect(&self) -> Result<(), ConnectorError>;
}
```

### BinanceStreamMessage Enum

```rust
pub enum BinanceStreamMessage {
    Ticker24hr(Ticker24hr),      // <symbol>@ticker
    Depth20(DepthUpdate),        // <symbol>@depth20@100ms
    Kline1m(KlineUpdate),        // <symbol>@kline_1m
    Raw(json::Value),            // 未解析的原始消息
}
```

### 重连策略

- 指数退避：1s → 2s → 4s → 8s → max 30s
- 60s Ping/Pong 心跳保活
- 最大重连次数：无限制（永久尝试）
- 断连期间：Phase 1 REST API 继续兜底

### Redis Pub/Sub 分发

- Channel `market:ticker`：TickerUpdate 消息
- Channel `market:depth`：DepthUpdate 消息
- Channel `market:kline`：KlineUpdate 消息

### 消息格式

```json
// Redis Pub/Sub 消息
{
  "type": "ticker",
  "symbol": "BTCUSDT",
  "data": { "price": 77950.0, "bid": 77949.0, "ask": 77951.0, ... },
  "ts": 1747452000000
}
```

## Consequences

### Positive
- 实时行情延迟从 1-5s 降低到 <100ms
- 自动重连 + 心跳保活提高可靠性
- Redis Pub/Sub 解耦数据采集和分发

### Negative
- WS 连接复杂度高于 REST
- 需要处理 Binance WS 协议细节（封装/心跳）

## Implementation Notes

1. **Phase 2**：BinanceConnector 实现，仅数据采集
2. **Phase 3**：WebSocket Hub，实现前端推送
3. 不能跳过 Phase 2 直接做 Phase 3（需要基础数据通道）