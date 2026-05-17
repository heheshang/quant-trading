# 行情数据管道实现计划

> 目标：实现 ADR-005 设计的统一数据网关架构，接入 Binance 实时行情，替换全量 Mock 数据

---

## 架构总览

```
[Binance WS] → [binance_connector] → [Redis Cache + PubSub]
                                              ↓
[Axum WS Handler] ← [ws_hub] ← [Redis PubSub subscriber]
      ↓
[Vue Client]
      ↓
[REST API] ← [market_data service] ← [Redis Cache]
```

---

## Phase 1: Redis 缓存层

### 目标
实现 `market_data.rs` 的 Redis 读写逻辑，保留 Mock 兜底

### 新增依赖
```toml
# backend/Cargo.toml
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }
```

### 实现文件
`backend/src/services/redis_cache.rs` — 新建

### 核心功能
| 函数 | Redis Key | 类型 |
|------|-----------|------|
| `get_ticker(symbol)` | `ticker:{symbol}` | HASH |
| `set_ticker(ticker)` | `ticker:{symbol}` | HASH + PubSub `market:ticker:{symbol}` |
| `get_all_tickers()` | `tickers:all` | JSON String |
| `set_tickers_cache(tickers)` | `tickers:all` | JSON String, TTL=5s |
| `get_depth(symbol)` | `depth:{symbol}` | JSON String |
| `set_depth(symbol, depth)` | `depth:{symbol}` | JSON String + PubSub `market:depth:{symbol}` |

### 降级策略
- Redis 连接失败 → 回退到 `build_mock_ticker()` / `build_mock_depth()`
- 日志 WARNING，计数器 +1

### 修改文件
- `backend/src/services/mod.rs` — 导出 `redis_cache`
- `backend/src/services/market_data.rs` — 注入 `redis::Client`，改造 `get_all_tickers` / `get_ticker_by_symbol` / `get_depth`

---

## Phase 2: Binance Exchange Connector

### 目标
建立与 Binance WebSocket 的持久连接，实现数据规范化

### 新增文件
```
backend/src/services/exchange/
├── mod.rs
├── binance_connector.rs
├── types.rs
└── errors.rs
```

### Binance WebSocket URL
```
wss://stream.binance.com:9443/stream?streams=
  btcusdt@ticker/
  btcusdt@depth20@100ms/
  btcusdt@kline_1m/
  ethusdt@ticker/
  ...
```

### 数据流
```
Binance WS Message
  → parse_raw_message() → BinanceStreamMsg
  → normalize() → TickerResponse / DepthResponse / KlineRaw
  → redis_cache::set_xxx()
  → ws_hub::broadcast()
```

### 订阅管理
- `MarketCollector` 管理所有 symbol 的订阅
- 支持动态添加/移除 symbol
- 自动重连（指数退避：1s→2s→4s→8s→max 30s）
- Ping/Pong 心跳保活（每 60s）

### 修改 main.rs
- 启动时 spawn `MarketCollector::start()` 作为背景 task
- `AppState` 新增 `market_collector: Arc<MarketCollector>`

---

## Phase 3: WebSocket Hub 实时推送

### 目标
改造 `ws.rs`，实现多主题订阅 + Redis PubSub 消费 → 推送到前端

### 消息格式
```json
// 客户端订阅
{ "action": "subscribe", "channels": ["ticker:BTCUSDT", "depth:ETHUSDT"] }

// 服务端推送
{ "type": "ticker", "symbol": "BTCUSDT", "data": {...}, "ts": 1715500000000 }
```

### WsManager 改造
```rust
pub struct WsManager {
    tx: broadcast::Sender<String>,           // 全局广播（ticker/depth）
    subscriptions: Arc<RwLock<HashMap<String, HashSet<String>>>>, // channel → sessions
}
// 每条消息按订阅关系过滤后发送
```

### 订阅流程
1. 客户端发送 `{ "action": "subscribe", "channels": [...] }`
2. 服务端解析 → 更新 `subscriptions` → 发送 `{ "type": "subscribed", "channel": "..." }`
3. `RedisPubSubListener` 线程消费 `market:ticker:*` / `market:depth:*` → 全量广播
4. 客户端收到后按 `type` + `symbol` 分发到对应 Vue 组件

### 改动文件
- `backend/src/handlers/ws.rs` — 改造 `handle_socket`
- `backend/src/services/ws_hub.rs` — 新建，集中管理广播逻辑
- `backend/src/services/mod.rs` — 导出 ws_hub

---

## Phase 4: 历史数据持久化

### 目标
实现 `ticker_snapshots` 表写入与查询

### 新增表
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

### 实现
- `ticker_snapshots` 表 → SeaORM Entity（`backend/src/db/ticker_snapshot.rs`）
- `snapshot_tickers()` — 每 60s 批量 INSERT（`INSERT ... ON CONFLICT DO NOTHING`）
- `cleanup_expired_snapshots()` — 每天 03:00 UTC DELETE，保留 90 天
- `get_ticker_history()` — 分页查询，索引支持

### 改动文件
- `backend/src/db/mod.rs` — 注册 `TickerSnapshot` Entity
- `backend/src/db/ticker_snapshot.rs` — 新建
- `backend/src/services/market_data.rs` — 实现 `snapshot_tickers` / `cleanup_expired_snapshots` / `get_ticker_history`

---

## Phase 依赖关系

```
Phase 1 ──┬── Phase 2（Binance Connector 发布到 Redis）
          │
          └── Phase 3（WS Hub 消费 Redis PubSub）

Phase 4 独立（基于 Phase 1 的实体定义）
```

---

## 文件变更清单

### 新建
| 文件 | 用途 |
|------|------|
| `backend/src/services/redis_cache.rs` | Redis 读写抽象 |
| `backend/src/services/exchange/mod.rs` | Exchange 模块入口 |
| `backend/src/services/exchange/binance_connector.rs` | Binance WS 连接器 |
| `backend/src/services/exchange/types.rs` | Binance 原始消息类型 |
| `backend/src/services/exchange/errors.rs` | Connector 错误类型 |
| `backend/src/services/ws_hub.rs` | WS 广播 Hub |
| `backend/src/db/ticker_snapshot.rs` | Ticker 快照表实体 |

### 修改
| 文件 | 改动 |
|------|------|
| `backend/Cargo.toml` | 新增 `redis` crate |
| `backend/src/services/mod.rs` | 导出新模块 |
| `backend/src/services/market_data.rs` | 重构为 Redis-first + Mock 降级 |
| `backend/src/handlers/ws.rs` | 改造为订阅模式 |
| `backend/src/handlers/mod.rs` | 导出 ws 模块 |
| `backend/src/db/mod.rs` | 注册 TickerSnapshot Entity |
| `backend/src/main.rs` | 启动 MarketCollector，注入 AppState |
| `docker-compose.yml` | 可选：新增 `market_collector` 服务（独立进程） |

### 数据库迁移
| 文件 | 内容 |
|------|------|
| `docker-init/postgres/02_ticker_snapshots.sql` | 新建 ticker_snapshots 表 + 索引 |

---

## 风险点

| 风险 | 缓解 |
|------|------|
| Binance WS 限流 | 指数退避 + REST 降级 |
| Redis 不可用 | Mock 数据兜底 |
| 多 symbol 订阅消息量巨大 | 按需订阅（客户端指定 channels） |
| WebSocket 连接数过多 | `broadcast::channel(100)` 溢出时断开慢客户端 |

---

## 验收标准

- [ ] `GET /api/v1/market/ticker?symbol=BTCUSDT` 返回真实 Binance 价格（误差 < 1s）
- [ ] `GET /api/v1/market/depth?symbol=ETHUSDT&levels=20` 返回真实深度数据
- [ ] WebSocket 连接后订阅 `ticker:BTCUSDT` 能收到实时推送
- [ ] 连续运行 5 分钟无断连，ticker 推送频率 ≥ 1条/秒
- [ ] Redis 故障时 API 仍返回（Mock 降级），恢复后自动切回
- [ ] `GET /api/v1/market/ticker/history?symbol=BTCUSDT&start=...&end=...` 返回历史快照