# PRD-Market-Data-Pipeline-20260517

## 1. 功能背景

### 问题描述
当前 `market_data.rs` 全量使用硬编码 Mock 数据，无法反映真实市场价格。量化交易系统依赖实时行情进行策略执行和下单决策，Mock 数据导致：
- 策略回测结果不可信
- 模拟交易无法验证真实交易逻辑
- 前端展示无实际参考价值

### 解决目标
实现 ADR-005 统一数据网关架构，接入 Binance 实时行情 WebSocket，构建 Redis 缓存层，支持实时推送和历史查询。

---

## 2. 用户角色

| 角色 | 权限 | 场景 |
|------|------|------|
| 量化交易者 | 查看行情、下单、回测 | 盯盘、设置策略、查看账户 |
| 系统 | 采集行情、推送数据 | Binance WS → Redis → Client |

---

## 3. 功能说明

### 3.1 数据流架构

```
[Binance WS] → [binance_connector] → [Redis Cache + PubSub]
                                              ↓
[Axum WS Handler] ← [ws_hub] ← [Redis PubSub subscriber]
      ↓                                        ↓
[REST API] ← [market_data service] ← [Redis Cache]
      ↓                                        ↓
[Vue Client] ←─────────────────────────────
```

### 3.2 Phase 1 — Redis 缓存层

**功能：**
- `GET /api/v1/market/ticker?symbol=BTCUSDT` → 读 Redis `ticker:BTCUSDT`，未命中则调 Binance REST
- `GET /api/v1/market/tickers` → 读 Redis `tickers:all`，TTL=5s
- `GET /api/v1/market/depth?symbol=ETHUSDT&levels=20` → 读 Redis `depth:ETHUSDT`
- Redis 不可用时回退 Mock 数据（不报错）

**Redis Key 规范：**
| Key | 类型 | TTL | 说明 |
|-----|------|-----|------|
| `ticker:{symbol}` | HASH | 10s | 最新价格 |
| `tickers:all` | String(JSON) | 5s | 全币种快照 |
| `depth:{symbol}` | String(JSON) | 2s | 订单簿20档 |

### 3.3 Phase 2 — Binance Exchange Connector

**功能：**
- 连接 `wss://stream.binance.com:9443/stream`
- 订阅：`<symbol>@ticker`、`<symbol>@depth20@100ms`、`<symbol>@kline_1m`
- 消息规范化（BINANCE → 内部格式）
- 自动重连（指数退避：1s→2s→4s→8s→max 30s）
- 60s Ping/Pong 心跳保活

**支持的交易对：**
BTCUSDT, ETHUSDT, SOLUSDT, BNBUSDT, XRPUSDT, DOGEUSDT, ADAUSDT, AVAXUSDT, DOTUSDT, LINKUSDT

### 3.4 Phase 3 — WebSocket 实时推送

**功能：**
- `GET /api/v1/ws?token=xxx` 升级为 WebSocket
- 客户端订阅消息：`{ "action": "subscribe", "channels": ["ticker:BTCUSDT", "depth:ETHUSDT"] }`
- 服务端推送：`{ "type": "ticker", "symbol": "BTCUSDT", "data": {...}, "ts": 1715500000000 }`
- 支持心跳（30s 无消息自动发送 `{"type":"heartbeat","ts":...}`）

### 3.5 Phase 4 — 历史数据持久化

**功能：**
- `ticker_snapshots` 表：每 60s 批量写入 Binance 价格快照
- `GET /api/v1/market/ticker/history?symbol=BTCUSDT&start=...&end=...&page=1&page_size=100`
- 每天 03:00 UTC 清理 90 天前快照

**表结构：**
```sql
CREATE TABLE ticker_snapshots (
    id BIGSERIAL PRIMARY KEY,
    symbol VARCHAR(20) NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    change DOUBLE PRECISION NOT NULL,
    change_percent DOUBLE PRECISION NOT NULL,
    volume DOUBLE PRECISION NOT NULL,
    high DOUBLE PRECISION NOT NULL,
    low DOUBLE PRECISION NOT NULL,
    bid DOUBLE PRECISION NOT NULL,
    ask DOUBLE PRECISION NOT NULL,
    timestamp BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_ticker_snapshots_symbol_timestamp ON ticker_snapshots(symbol, timestamp DESC);
```

---

## 4. 边界定义

### 做
- ✅ Binance 一个交易所
- ✅ BTC/USDT 永续合约（USDT 结算）
- ✅ Redis-first，Mock 降级
- ✅ REST 和 WebSocket 两种接口

### 不做
- ❌ 币安合约/现货 API 认证交易（Phase P1）
- ❌ 多交易所（OKX/Bybit）（Phase 2+）
- ❌ 股票/期货/期权数据
- ❌ K线历史数据导入（已有 kline 模块）

---

## 5. 异常场景

| 场景 | 处理 |
|------|------|
| Binance WS 断连 | 指数退避重连，期间 REST 拉取兜底 |
| Binance API 429 限流 | 退避 60s，继续用缓存数据 |
| Redis 连接失败 | Mock 数据兜底，日志 WARNING |
| 未知交易对 | 404 `{"code": 40401, "message": "交易对不存在"}` |
| WS 客户端慢（broadcast 队列满） | 断开慢客户端连接 |
| 非法订阅消息格式 | 返回 `{"type":"error","code":40001,"message":"Invalid format"}` |

---

## 6. 验收标准

| ID | 标准 | 验证方式 |
|----|------|----------|
| AC1 | `GET /api/v1/market/ticker?symbol=BTCUSDT` 延迟 < 1s，数据来自 Binance | curl 计时 + 对比 Binance API |
| AC2 | `GET /api/v1/market/depth?symbol=ETHUSDT&levels=20` 返回真实深度 | 对比 Binance depth API |
| AC3 | WebSocket 订阅 ticker 推送频率 ≥ 1条/秒 | WS 连接 10s 计数 |
| AC4 | Redis 故障时 API 返回 Mock 数据，HTTP 200 | kill redis，重试 API |
| AC5 | `GET /api/v1/market/ticker/history` 分页返回快照数据 | DB 查询验证 |
| AC6 | 服务启动后 10s 内开始接收 Binance 数据 | 日志时间戳验证 |
| AC7 | 连续运行 5 分钟无断连 | 监控 WS 连接状态 |

---

## 7. Gherkin 场景

```gherkin
Feature: 市场数据管道
  场景: 获取单个交易对行情
    假设 系统已启动且 Binance WS 已连接
    当 用户请求 GET /api/v1/market/ticker?symbol=BTCUSDT
    那么 返回 200 且 price 字段为真实 Binance 价格
    并且 bid < ask（买卖价差正常）

  场景: 订阅实时 Ticker 推送
    假设 用户已通过 JWT 认证并连接 WebSocket
    当 用户发送 {"action":"subscribe","channels":["ticker:BTCUSDT"]}
    那么 服务端在 1 秒内返回 {"type":"subscribed","channel":"ticker:BTCUSDT"}
    并且 随后每秒收到 ticker 类型消息

  场景: Redis 不可用时降级
    假设 Redis 服务已停止
    当 用户请求 GET /api/v1/market/ticker?symbol=BTCUSDT
    那么 返回 200 且 data.source="mock"
   并且 错误日志包含 "Redis unavailable, using mock data"

  场景: 订阅不支持的交易对
    当 用户请求 GET /api/v1/market/ticker?symbol=INVALIDCOIN
    那么 返回 404 且 message 包含 "INVALIDCOIN"

  场景: 查询历史行情快照
    假设 ticker_snapshots 表有历史数据
    当 用户请求 GET /api/v1/market/ticker/history?symbol=BTCUSDT&page=1&page_size=100
    那么 返回 200 且 items 为数组且 length ≤ 100
    并且 meta.total > 0
```

---

## 8. 技术约束

- Rust 1.85 + Axum 0.8 + tokio 1.x
- Redis 7 Alpine（docker-compose 已部署）
- SeaORM 1.x ORM
- WebSocket 使用 `tokio-tungstenite` + `axum::extract::ws`
- 所有异步操作通过 `#[tokio::test]` 单元测试覆盖
- 编译通过 `cargo clippy -- -D warnings`