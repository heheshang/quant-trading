# ADR: 行情模块（深度数据与实时Ticker）架构决策

| 字段 | 值 |
|------|------|
| **ID** | ADR-MARKET-DEPTH-TICKER |
| **状态** | 已批准 |
| **日期** | 2026-05-13 |
| **决策者** | Tech Lead |
| **基于** | ADR-002（WebSocket实时推送）、ADR-005（市场数据管道）、PRD-market-depth-ticker v1.0 |
| **影响范围** | 行情REST API、WebSocket推送、Redis缓存层、前端行情展示 |

---

## 背景

PRD-market-depth-ticker 定义了行情模块的核心实时展示功能：Ticker REST/WS 推送（US-MD-01/03）、深度数据 REST/WS 推送（US-MD-02/04）、前端展示（US-MD-05/06）、心跳连接管理（US-MD-07）和 Ticker 快照持久化（US-MD-08）。

当前状态：
- 前端 `Ticker`/`Depth` 类型已定义但缺少 `bid`/`ask`/`timestamp` 字段
- 前端 `api/market.ts` 有 `getTickers()`/`getDepth(symbol)` 占位
- 后端 `handlers/market.rs` 不存在
- 后端 `services/market_data.rs` 不存在
- `handlers/ws.rs` 仅实现 echo 模式，无订阅/推送逻辑
- Redis 缓存层未部署

需要做出以下架构决策：API字段命名统一、深度数据格式（结构化 vs 二维数组）、WS频道路径选择、缓存降级策略、深度档位权限控制、推送合并窗口参数、Ticker快照持久化方案。

---

## 决策

### D1: Ticker API 响应字段命名 — 以 PRD-market-depth-ticker 为准（数值类型）

**上下文：** 两份PRD对 Ticker 字段命名有分歧。PRD-market-module 使用 `last`/`change_24h`/`volume_24h`/`high_24h`/`low_24h`，PRD-market-depth-ticker 使用 `price`/`change`/`change_percent`/`volume`/`high`/`low`。两者字段值也不同：前者用字符串 `"50500.00"`，后者用数值 `50500.0`。

**决策：** 采用 PRD-market-depth-ticker 的字段命名和数值类型。

| 字段 | 类型 | 说明 |
|------|------|------|
| symbol | string | 交易对 |
| price | f64 | 最新价 |
| change | f64 | 24h涨跌额 |
| change_percent | f64 | 24h涨跌幅 |
| volume | f64 | 24h成交量 |
| high | f64 | 24h最高 |
| low | f64 | 24h最低 |
| bid | f64 | 买一价 |
| ask | f64 | 卖一价 |
| timestamp | i64 | 毫秒时间戳 |

**理由：**
1. PRD-market-depth-ticker 是本模块的详细PRD，字段命名更简洁（无 `_24h` 后缀冗余——Ticker 天然就是24h数据）
2. 数值类型（f64）比字符串序列化更适合前端直接运算（涨跌幅计算、排序、闪烁判断），避免 `parseFloat()` 开销
3. `price` 比 `last` 语义更通用（WebSocket 推送也是最新价，不限于 "last traded price" 概念）
4. `change` + `change_percent` 拆分比单一 `change_24h` 更灵活（前端可独立展示金额涨跌和百分比涨跌）

**影响：** 前端 `api/market.ts` 需从 `last`/`change_24h` 改为 `price`/`change`/`change_percent`，类型定义 `Ticker` 需同步更新。PRD-market-module 中的 Ticker 响应示例需标记为已废弃。

---

### D2: 深度数据响应格式 — 结构化对象（非二维数组）

**上下文：** PRD-market-depth-ticker 的 REST API 示例返回结构化对象 `{price, quantity, total}`，但 WS 推送示例使用二维数组 `[["50499.0", 1.5]]`。PRD-market-module 统一使用二维数组 `[["50499.00", "1.500"]]`。

**决策：** REST API 和 WS 推送**统一使用结构化对象**。

```json
// REST: GET /api/v1/market/depth
{
  "code": 0,
  "data": {
    "bids": [
      {"price": 50499.0, "quantity": 1.5, "total": 1.5}
    ],
    "asks": [
      {"price": 50501.0, "quantity": 1.2, "total": 1.2}
    ],
    "timestamp": 1715500000000
  },
  "message": "success"
}

// WS: depth 推送（首次完整快照）
{
  "type": "depth",
  "symbol": "BTCUSDT",
  "data": {
    "bids": [
      {"price": 50499.0, "quantity": 1.5, "total": 1.5}
    ],
    "asks": [
      {"price": 50501.0, "quantity": 1.2, "total": 1.2}
    ],
    "timestamp": 1715500000000
  },
  "ts": 1715500000000
}

// WS: depth_update 增量推送（quantity=0 表示删除该价位）
{
  "type": "depth_update",
  "symbol": "BTCUSDT",
  "data": {
    "bids": [
      {"price": 50499.0, "quantity": 1.8, "total": 0}
    ],
    "asks": [
      {"price": 50503.0, "quantity": 0.5, "total": 0}
    ],
    "timestamp": 1715500000001
  },
  "ts": 1715500000001
}
```

**理由：**
1. 结构化对象可扩展（未来加 `order_count`、`is_snapshot` 等字段不需要破坏协议）
2. 前端不需要记住数组下标含义（`[0]=price, [1]=quantity`）
3. REST 和 WS 格式一致，前端可复用同一个 `DepthLevel` 类型做快照/增量合并
4. `total` 字段在 REST 响应中有值（累计量），在 WS 增量中为 0（由前端本地计算），语义清晰
5. 增量推送中 `quantity=0` 表示删除该价位的挂单，比省略该条目更明确

**影响：** 前端 `Depth` 类型需用 `DepthLevel[]` 替代 `[number, number][]`。WS 增量合并逻辑需处理 `quantity=0` 删除和 `total` 本地重算。

---

### D3: WebSocket 频道路径 — 统一前缀 + 无分隔符 symbol

**上下文：** PRD-market-module 使用 `ticker:BTC/USDT`（含斜杠），PRD-market-depth-ticker 使用 `ticker:BTCUSDT`（无斜杠）。

**决策：** 采用无分隔符格式 `ticker:{symbol}` / `depth:{symbol}` / `kline:{symbol}:{interval}`，symbol 统一为 `BTCUSDT` 格式（无斜杠）。

**频道命名规范：**

| 频道 | 格式 | 示例 |
|------|------|------|
| Ticker | `ticker:{symbol}` | `ticker:BTCUSDT` |
| Depth | `depth:{symbol}` | `depth:BTCUSDT` |
| K线 | `kline:{symbol}:{interval}` | `kline:BTCUSDT:1m` |

**理由：**
1. Redis Pub/Sub 频道名不支持 `/`（会触发层级分隔），`ticker:BTCUSDT` 直接映射为 `market:ticker:BTCUSDT` Pub/Sub 频道
2. WebSocket 消息的 JSON channel 字段不含特殊字符，避免前端 URL encode 问题
3. 内部数据处理层统一使用 `BTCUSDT` 格式（Binance API 原生格式），仅在展示层转换为 `BTC/USDT`

**映射规则：** 内部 `BTCUSDT` ↔ 展示 `BTC/USDT`，前端 `symbol` 参数传 `BTCUSDT`，展示时根据 `symbol_metadata` 表或前端本地规则加斜杠。

**影响：** 前端 API 调用传 `symbol=BTCUSDT`。WS 订阅频道使用 `ticker:BTCUSDT`。前端展示层做 `BTCUSDT` → `BTC/USDT` 转换。

---

### D4: 深度档位权限控制 — 中间件层 RBAC 检查

**上下文：** PRD 要求 trader 最多查看 20 档深度，pro-trader 可查看 50 档。需决定权限检查放在哪一层。

**决策：** 在 handler 层通过 `AuthenticatedUser.role` 做 RBAC 检查，service 层不做权限判断。

```
handler 层:
  1. 从 AuthenticatedUser 获取 role
  2. 校验 levels 参数合法性 (5/10/20/50)
  3. 若 levels > 20 且 role != "pro-trader" && role != "admin" → 403
  4. 调用 service 层获取数据

service 层:
  1. 接收 symbol + levels，不做权限判断
  2. 从 Redis 获取深度数据
  3. 截取 levels 档返回
```

**理由：**
1. 权限检查属于 HTTP 层关注点，与业务逻辑解耦
2. Service 层可被内部调用（如回测引擎需要50档深度），不需要角色限制
3. Handler 层已有 `AuthenticatedUser` 中间件注入的角色信息，零额外查询

**合法 levels 值：** `[5, 10, 20, 50]`。默认 10。非法值（如 0, 3, 15）返回 400。

---

### D5: Redis 缓存降级策略 — 内存 HashMap 兜底

**上下文：** Redis 可能不可用（部署前/故障时），REST API 需要降级方案。

**决策：** 三级数据获取策略：

```
1. Redis 命中 → 直接返回 (P99 < 100ms)
2. Redis 未命中/不可用 → MarketCollector 内存 HashMap 兜底
3. 两者均不可用 → 503 + ERR_MARKET_COLLECTOR_DOWN
```

**MarketCollector 内存缓存：**
- `ticker_cache: Arc<DashMap<String, TickerData>>` — Collector 写入 Redis 时同步写入
- `depth_cache: Arc<DashMap<String, DepthData>>` — 同上
- TTL 由 Collector 更新频率保证（WS 推送即更新），不做显式过期

**理由：**
1. Collector 是数据源头，天然拥有最新数据，内存缓存零延迟
2. 避免引入额外的 fallback 数据库查询（深度数据不在 PostgreSQL 中）
3. DashMap 并发安全，与 Redis 不可用的场景匹配

**影响：** `MarketDataService` 需注入 `Arc<MarketCollector>` 引用以访问内存缓存。Collector 需公开 `get_ticker_from_cache()` / `get_depth_from_cache()` 方法。

---

### D6: WebSocket 推送合并窗口参数

**上下文：** PRD 定义 Ticker 合并窗口 250ms、Depth 合并窗口 100ms。需确认具体实现方式。

**决策：** 采用 tokio time 批量合并方案：

| 频道类型 | 合并窗口 | 最大推送频率 | 实现方式 |
|---------|---------|-------------|---------|
| ticker:{symbol} | 250ms | 4次/s | `tokio::time::interval(250ms)` 窗口内取最新值 |
| depth:{symbol} | 100ms | 10次/s | `tokio::time::interval(100ms)` 窗口内合并增量 |
| heartbeat | 15s | — | `tokio::time::interval(15s)` |

**Ticker 合并逻辑：** 窗口内多次更新，仅保留最新状态（整体替换），窗口结束时推送一次。

**Depth 合并逻辑：** 窗口内多个增量按价位合并（同价位取最新 quantity），窗口结束时推送合并后的增量。

**理由：**
1. tokio interval 基于时间槽，比 debounce 更精确
2. Ticker 是全量替换语义（最新状态），Depth 是增量合并语义，分别处理合理
3. 合并窗口在 WS Hub 层实现，不侵入 Collector 逻辑

---

### D7: WsHub 扩展方案 — 从 echo 到频道订阅

**上下文：** 现有 `WsManager` 仅有 `broadcast::Sender<String>`，无频道概念。需要扩展为支持订阅/取消/频道路由的 Hub。

**决策：** 扩展 `WsManager` 为 `MarketWsHub`，保留现有广播通道作为内部消息总线，增加频道订阅管理。

```rust
pub struct MarketWsHub {
    // 现有广播通道（内部消息总线，Collector→Hub）
    pub tx: broadcast::Sender<String>,
    // 频道→订阅者集合
    pub channels: DashMap<String, HashSet<String>>,
    // 连接ID→发送端
    pub sessions: DashMap<String, mpsc::Sender<String>>,
    // 用户ID→连接ID列表（用于连接数限制）
    pub user_connections: DashMap<String, Vec<String>>,
}
```

**消息流：**
```
Collector → Redis PUBLISH market:ticker:BTCUSDT
    ↓
MarketWsHub (Redis SUBSCRIBE) → 收到消息
    ↓
合并窗口处理（250ms for ticker, 100ms for depth）
    ↓
查找 channels["ticker:BTCUSDT"] 的所有 session_id
    ↓
逐个 session sender.send() 推送
```

**连接数限制：** 每用户最多 5 个 WS 连接。新连接超过上限时，踢出最早连接并发送 `{"type":"kick","reason":"max_connections_exceeded"}`。

**理由：**
1. 保留现有 `broadcast::Sender` 兼容性，渐进式扩展
2. DashMap 无锁并发，适合高频订阅/取消操作
3. Redis Pub/Sub 解耦 Collector 和 WS Hub，Collector 不感知 WS 存在

---

### D8: Ticker 快照持久化方案 — 定时批量写入

**上下文：** US-MD-08 (P1) 要求每 1 分钟将所有交易对 Ticker 快照写入 `ticker_snapshots` 表，保留 90 天。

**决策：** Collector 内部 tokio 定时任务，每 60s 批量 INSERT。

```
1. Collector 每 60s 触发 snapshot_tickers()
2. 从内存 ticker_cache 读取所有 symbol 的最新 Ticker
3. 批量 INSERT INTO ticker_snapshots (使用 INSERT ... ON CONFLICT DO NOTHING)
4. 单独 tokio task 每天清理 > 90天 的数据 (DELETE WHERE created_at < NOW() - INTERVAL '90 days')
5. 写入失败记录日志，不影响实时推送
```

**P1 优先级说明：** 快照持久化在 P0 功能（REST/WS 行情查询）之后实现。历史查询 API `GET /api/v1/market/ticker/history` 与快照持久化同步开发。

---

## 对齐问题汇总

### 与 PRD-market-module 的分歧

| 项目 | PRD-market-module | 本 ADR 决策 | 优先级 |
|------|-------------------|------------|--------|
| Ticker 字段名 | `last`, `change_24h`, `volume_24h` | `price`, `change`, `change_percent`, `volume` | D1 |
| Ticker 数值类型 | String `"50500.00"` | f64 `50500.0` | D1 |
| 深度 REST 格式 | 二维数组 `[["50499.00","1.500"]]` | 结构化对象 `{price, quantity, total}` | D2 |
| WS 路径 | `/api/v1/market/ws` | `/api/v1/ws`（现有端点） | D3 注 |
| symbol 格式 | `BTC/USDT` | `BTCUSDT`（内部/WS），展示层转换 | D3 |

**D3 注：** 现有 WS 端点为 `/api/v1/ws`，PRD-market-module 建议改为 `/api/v1/market/ws`。考虑到现有前端可能已依赖 `/api/v1/ws`，决策是**保留 `/api/v1/ws` 不变**，通过频道命名区分行情 vs 未来其他 WS 用途。若后续需支持非行情 WS 功能，再独立端点。

### 与前端现有类型的分歧

| 项目 | 现有前端 | 需要变更 | 优先级 |
|------|---------|---------|--------|
| Ticker.price | 不存在（现有为 `last`） | 新增 `price`, 删除或废弃 `last` | P0 |
| Ticker.bid/ask | 不存在 | 新增 | P0 |
| Ticker.timestamp | 不存在 | 新增 | P0 |
| DepthLevel | 不存在（现有 `Depth.bids: [number, number][]`） | 新增 `DepthLevel` 接口 | P0 |
| WsMessage types | 不存在 | 新增完整 WS 消息类型 | P0 |

---

## 新增错误码

```rust
pub const ERR_MARKET_SYMBOL_NOT_FOUND: i32 = 40401;     // 交易对不存在
pub const ERR_MARKET_DEPTH_LEVELS_INVALID: i32 = 40001;  // 无效档位参数
pub const ERR_MARKET_DEPTH_FORBIDDEN: i32 = 40301;       // 角色权限不足（深度档位）
pub const ERR_MARKET_WS_AUTH_FAILED: i32 = 40101;        // WS 鉴权失败
pub const ERR_MARKET_WS_CHANNEL_INVALID: i32 = 40002;    // 无效频道格式
pub const ERR_MARKET_COLLECTOR_DOWN: i32 = 50301;        // 采集器离线
pub const ERR_MARKET_WATCHLIST_FULL: i32 = 42901;        // 自选列表已满
pub const ERR_MARKET_WS_TOO_MANY_CHANNELS: i32 = 42902;  // 单连接订阅频道超限
```

---

## 风险评估

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| Redis 不可用导致 Ticker/Depth 数据丢失 | 中 | 高 | D5 内存 HashMap 兜底 |
| WS Hub 单点瓶颈（1000+ 连接） | 低 | 高 | DashMap + tokio broadcast，O(1) 推送 |
| 深度增量合并前端实现复杂 | 中 | 中 | 提供前端 `useDepth.ts` composable 封装 |
| symbol 格式混乱（BTC/USDT vs BTCUSDT） | 中 | 低 | D3 统一内部用 BTCUSDT，展示层转换 |
| Collector 延迟导致推送超 500ms | 低 | 中 | 合并窗口控制推送频率，避免洪泛 |

---

## 关联 ADR

- **ADR-001** — Rust + Axum 后端选型 → 继续有效
- **ADR-002** — WebSocket (tokio-tungstenite) → 本 ADR 扩展其 Hub 设计
- **ADR-005** — 市场数据管道 → 本 ADR 消费其 Collector 数据
- **ADR-KLINE-MGMT** — K线管理 → K线 REST API 由其覆盖，本 ADR 仅涉及 WS kline 推送频道命名
