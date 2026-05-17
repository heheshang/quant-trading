# PRD-Binance-WS-Connector-20260517

## 1. 功能背景

### 问题描述
Phase 1 已实现 Redis 缓存层 + Binance REST 降级，但 REST 轮询存在 1-5 秒延迟，无法满足实时行情推送需求。量化交易策略需要毫秒级行情更新，WebSocket 是必需方案。

### 解决目标
实现 Binance WebSocket 连接器，从 `wss://stream.binance.com:9443/stream` 订阅实时行情数据，通过 Redis Pub/Sub 分发给 WebSocket Hub，为 Phase 3 前端实时推送奠定基础。

---

## 2. 用户角色

| 角色 | 权限 | 场景 |
|------|------|------|
| 量化交易者 | 订阅实时行情 | WebSocket 连接 → 订阅 ticker/depth → 接收实时推送 |
| 系统 | 采集+分发行情 | Binance WS → Redis PubSub → WebSocket Hub → 前端 |

---

## 3. 功能说明

### 3.1 数据流架构

```
[Binance WS] → [binance_connector] → [Redis PubSub]
                                           ↓
                   [ws_hub] ← [Redis PubSub subscriber]
                      ↓
              [Axum WS Handler]
                      ↓
              [Vue Client]
```

### 3.2 Binance Exchange Connector（Phase 2 核心）

**功能：**
- 连接 `wss://stream.binance.com:9443/stream`
- 订阅：`<symbol>@ticker`、`<symbol>@depth20@100ms`、`<symbol>@kline_1m`
- 消息规范化（BINANCE 原始格式 → 内部 `MarketMessage` 枚举）
- 自动重连（指数退避：1s→2s→4s→8s→max 30s）
- 60s Ping/Pong 心跳保活

**支持的交易对：**
BTCUSDT, ETHUSDT, SOLUSDT, BNBUSDT, XRPUSDT, DOGEUSDT, ADAUSDT, AVAXUSDT, DOTUSDT, LINKUSDT

### 3.3 Redis Pub/Sub 分发

**Channel 规范：**
| Channel | 消息类型 | 说明 |
|---------|----------|------|
| `market:ticker` | TickerUpdate | 24hr ticker 推送 |
| `market:depth` | DepthUpdate | 订单簿 20 档推送 |
| `market:kline` | KlineUpdate | 1min K线推送 |

**消息格式（JSON）：**
```json
{
  "type": "ticker",
  "symbol": "BTCUSDT",
  "data": { "price": 77950.0, "bid": 77949.0, "ask": 77951.0, ... },
  "ts": 1747452000000
}
```

### 3.4 WebSocket Hub（Phase 3 前置）

**功能：**
- `GET /api/v1/ws?token=xxx` 升级为 WebSocket
- 客户端订阅：`{ "action": "subscribe", "channels": ["ticker:BTCUSDT"] }`
- 服务端推送：`{ "type": "ticker", "symbol": "BTCUSDT", "data": {...}, "ts": ... }`
- 心跳：30s 无消息自动发送 `{"type":"heartbeat","ts":...}`

---

## 4. 边界定义

### 做
- ✅ Binance 一个交易所 WS 连接
- ✅ BTC/USDT 永续（USDT 结算）
- ✅ 自动重连 + 心跳保活
- ✅ Redis Pub/Sub 分发
- ✅ Phase 2 + Phase 3 串联（避免重复返工）

### 不做
- ❌ Binance 认证交易（Phase P1）
- ❌ 多交易所（OKX/Bybit）
- ❌ 股票/期货/期权数据
- ❌ K线历史数据导入（已有 kline 模块）

---

## 5. 异常场景

| 场景 | 处理 |
|------|------|
| Binance WS 断连 | 指数退避重连，期间 REST 继续兜底（Phase 1 已实现） |
| Binance API 429 限流 | 降级到 REST，延迟 60s |
| Redis PubSub 不可用 | 回退到本地广播（降低优先级） |
| WS 客户端慢（队列满） | 断开慢客户端连接 |
| 非法订阅消息格式 | 返回 `{"type":"error","code":40001,"message":"Invalid format"}` |

---

## 6. 验收标准

| ID | 标准 | 验证方式 |
|----|------|----------|
| AC1 | WS 订阅 ticker 推送频率 ≥ 1条/秒 | WS 连接 10s 计数 |
| AC2 | 连续运行 5 分钟无断连 | 监控 WS 连接状态 |
| AC3 | 服务启动后 10s 内开始接收 Binance 数据 | 日志时间戳验证 |
| AC4 | Redis 故障时 WS 自动降级 REST | kill redis 重试 |
| AC5 | 不支持交易对返回 404 | 请求 INVALIDCOIN 验证 |

---

## 7. Gherkin 场景

```gherkin
Feature: Binance WebSocket 连接器
  场景: 连接 Binance WebSocket 并接收 ticker 数据
    假设 系统已启动，Binance WS 已连接
    当 Binance 推送 ticker 数据
    那么 Redis PubSub channel "market:ticker" 收到消息
    并且 消息格式为 {"type":"ticker","symbol":"BTCUSDT","data":{...},"ts":...}

  场景: 订阅实时 Ticker 推送
    假设 用户已通过 JWT 认证并连接 WebSocket
    当 用户发送 {"action":"subscribe","channels":["ticker:BTCUSDT"]}
    那么 服务端在 1 秒内返回 {"type":"subscribed","channel":"ticker:BTCUSDT"}
    并且 随后每秒收到 ticker 类型消息

  场景: Binance WS 断连后自动重连
    假设 用户已连接 Binance WS
    当 Binance WS 断连（网络问题）
    那么 服务端自动以指数退避重连（1s→2s→4s→8s→max 30s）
    并且 REST API 继续返回缓存数据

  场景: WS 客户端慢（广播队列满）
    假设 客户端网络慢，接收队列积压
    当 广播队列超过阈值（1000 条/秒）
    那么 服务端断开该客户端连接
    并且 记录日志 "Slow client disconnected"

  场景: 订阅不支持的交易对
    当 用户发送 {"action":"subscribe","channels":["ticker:INVALIDCOIN"]}
    那么 服务端返回 {"type":"error","code":40401,"message":"Symbol INVALIDCOIN not supported"}
```

---

## 8. 技术约束

- Rust 1.85 + Axum 0.8 + tokio 1.x
- `tokio-tungstenite` 用于 WS 连接
- Redis 7 Alpine（docker-compose 已部署）
- SeaORM 1.x ORM
- WebSocket 使用 `axum::extract::ws`
- 所有异步操作通过 `#[tokio::test]` 单元测试覆盖
- 编译通过 `cargo clippy -- -D warnings`