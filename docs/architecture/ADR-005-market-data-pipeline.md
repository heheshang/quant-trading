# ADR-005: 市场数据管道架构

| 字段 | 值 |
|------|------|
| **ID** | ADR-005 |
| **状态** | 已批准 |
| **日期** | 2026-05-12 |
| **决策者** | Tech Lead |
| **影响范围** | 行情模块、数据管道、实时性、多交易所支持 |

## 背景

行情数据是量化交易系统的生命线。PRD 要求：
- WebSocket 推送延迟 < 500ms
- 支持 K 线（多周期）、Ticker、深度数据
- 数据模型支持新增交易品种（股票、期货、期权）
- 可扩展至多个交易所（Binance, OKX, Bybit）
- 交易所 API 限流时自动降级到缓存

## 架构设计

### 选项 A: 统一数据网关 (选定)

设计一个中央市场数据管道，所有交易所的数据统一接入、规范化、分发。

```
┌──────────┐
│ Exchange1 │──┐
├──────────┤  │  ┌──────────────────────┐    ┌───────────┐
│ Exchange2 │──┤  │  Market Data         │    │ PostgreSQL│
├──────────┤  ├─→│  Collector Daemon     │───→│ (historical)│
│ Exchange3 │──┤  │  (Rust, per-exchange  │    └───────────┘
└──────────┘  │  │   connector tokio task)│
              │  └──────────┬─────────────┘
              │             │
              │      ┌──────▼────────┐
              │      │  Redis Cache   │
              │      │  - tickers     │
              │      │  - latest kline│
              │      │  - depth_snap  │
              │      └──────┬─────────┘
              │             │
              │      ┌──────▼─────────────┐
              │      │  WebSocket Hub      │
              │      │  (Axum, per-channel │
              └──────│   broadcast group)  │
                     └────────────────────┘
```

**优势：**
- 统一数据抽象层，屏蔽交易所差异
- 数据格式标准化（内部规范格式）
- Redis cache 层避免重复计算
- 单点维护，易于监控

**劣势：**
- 单一故障点（可通过高可用缓解）
- 管道复杂度较高

### 选项 B: 前端直连交易所

**优势：** 延迟最低，架构简单
**劣势：** CORS/安全限制，无法做回测数据采集，无法计算指标

### 选项 C: 对等模式（每个服务独立拉取）

**优势：** 解耦
**劣势：** 重复连接、API 限流冲突、数据不一致

## 详细设计

### 数据采集层 (Collector Daemon)

```
每个交易所对应一个 connector:
  BinanceConnector ─→ WS stream / REST fallback
  OKXConnector     ─→ WS stream / REST fallback
  BybitConnector   ─→ WS stream / REST fallback

每个 connector 内部:
  tokio::select! {
      ws_msg = stream.next() => 处理并规范化
      rest_tick = interval.tick() => REST 拉取补充
      heartbeat = interval.tick() => 发送 ping
  }
```

### 数据标准化格式

```json
// 内部统一数据格式（独立于交易所）
{
  "exchange": "binance",
  "symbol": "BTC/USDT",        // 统一格式
  "type": "kline",
  "data": {
    "open": 50000.0,
    "high": 51000.0,
    "low": 49000.0,
    "close": 50500.0,
    "volume": 1234.5,
    "timestamp": 1715500000000,
    "interval": "1m"
  }
}

// Ticker 格式
{
  "exchange": "binance",
  "symbol": "BTC/USDT",
  "type": "ticker",
  "data": {
    "last": 50500.0,
    "bid": 50499.0,
    "ask": 50501.0,
    "volume_24h": 123456.7,
    "change_24h": 2.35,         // 百分比
    "high_24h": 51000.0,
    "low_24h": 49000.0,
    "timestamp": 1715500000000
  }
}

// 深度格式
{
  "exchange": "binance",
  "symbol": "BTC/USDT",
  "type": "depth",
  "data": {
    "bids": [[50499.0, 1.5], [50498.0, 2.3], ...],
    "asks": [[50501.0, 1.2], [50502.0, 1.8], ...],
    "timestamp": 1715500000000
  }
}
```

### 数据流

```
[Exchange WebSocket]
    ↓ 原始数据
[Connector: 解析 + 规范化 + 去重]
    ↓ 统一格式
[Redis Publisher: 写入 cache + publish]
    ↓
    ├─→ Redis Hash (latest_kline:1m:BTC/USDT)
    ├─→ Redis Hash (ticker:BTC/USDT)
    ├─→ Redis Stream (raw_kline:BTC/USDT)
    └─→ Redis PubSub (channel: market:raw)
          ↓
    [WebSocket Hub: 订阅 Redis PubSub]
          ↓
    [Connected Clients: 通过 WS 推送]
```

### 历史数据持久化

- 实时数据同时写入 PostgreSQL (kline_data 表)
- 批量写入：每 5 秒 flush 一次，减少写放大
- 历史数据保留策略：
  - 1m K 线：保留 30 天
  - 15m/1h K 线：保留 1 年
  - 4h/1d K 线：永久保留

### 降级策略

| 故障场景 | 降级行为 |
|----------|----------|
| 交易所 WS 断开 | 退避重连 + 切换到 REST 拉取 |
| 交易所 API 限流 (429) | 退避重试 + 使用 Redis 缓存数据 |
| Redis 不可用 | 缓存降级为内存 HashMap（重启丢失） |
| PostgreSQL 不可用 | 实时行情正常推送（不受影响），回测暂停 |

## 决策

采用 **统一数据网关架构**，以 Redis 为中心的数据管道。

## 预期后果

**正面：**
- 数据格式统一，前端只需处理一种 schema
- 多交易所扩展只需添加新 connector
- Redis cache 层确保高并发读
- 降级策略保障系统可用性

**负面：**
- Redis 成为关键依赖（需要 Redis 哨兵/集群保障）
- 数据管道有两跳（Exchange→Redis→WS→Client）延迟
- connector 需要为每个交易所做适配

## 关联决策

- ADR-002（WebSocket 推送方案）
- PRD 数据字段标准化
- 多交易所架构扩展计划（v2.0）
