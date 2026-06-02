# T2+T3 — P1-2.3 Trailing Stop 技术选型 & 设计

## T2 选型

### 2.1 触发机制 (3 路径)

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **A. 后台轮询 task (本次)**: spawn task 2s 轮询 → exchange API → check_trigger | 简单、可靠、不依赖 WS | 资源开销 5 req/s | **✅** |
| B. Binance WS kline_1m → check_trigger | 资源轻 | 1min 延迟,接复杂 | 延后 |
| C. 复用 flush_trades | 无新 task | fill 间歇,trailing 不能跑 | 不适用 |

### 2.2 价格源 (Exchange API)

| 方案 | 选 |
|---|---|
| Binance REST `GET /api/v3/ticker/price` | **✅** (单 symbol,毫秒级) |
| 缓存 Redis | 延后 (v2) |

### 2.3 peak_price 状态存储

| 方案 | 选 |
|---|---|
| DB `orders.advanced_params.peak_price` (JSONB) | **✅** (P1-2.x 模式一致) |
| 独立 `trailing_stop_state` 表 | 过度设计 |

### 2.4 轮询 task 集成点

- **`main.rs` `spawn`**: 启动时 spawn 1 个 `tokio::spawn` task
- **生命周期**: 与 backend 进程同寿 (随 main shutdown 退出)
- **错误恢复**: 任意 1 次失败不 kill task,只 log error

## T3 设计

### 3.1 架构

```
┌─────────────────┐  every 2s   ┌────────────────────┐
│ trailing_poll   │ ──────────► │ Binance REST API   │
│ task (spawn)    │             │ /ticker/price      │
└────────┬────────┘             └────────────────────┘
         │
         │ symbol → price
         ▼
┌─────────────────────────────────────────────────────┐
│ 1. SELECT active trailing_stop orders from DB      │
│ 2. Group by symbol (避免 N+1 query)                │
│ 3. For each symbol, fetch price (1 req)            │
│ 4. For each order: update peak_price if beneficial │
│ 5. If price triggered, create market close order   │
└─────────────────────────────────────────────────────┘
```

### 3.2 新增/修改文件

| 类型 | 路径 |
|---|---|
| 新 | `models/trailing_stop_params.rs` (params schema + validation) |
| 新 | `services/trailing_stop.rs` (peak_price 逻辑 + poll loop) |
| 新 | `handlers/trailing_stop.rs` (admin endpoint - 可选) |
| 改 | `handlers/order.rs` (trailing_stop 分支) |
| 改 | `main.rs` (spawn poll task) |
| 改 | `metrics.rs` (新 counter) |

### 3.3 peak_price 算法

```rust
// Buy trailing stop: peak_price 只升不降
if side == "buy" {
    if current_price > peak_price {
        peak_price = current_price;  // 上移
    }
    // 触发价 = peak_price * (1 - trailing_distance)
    if current_price <= peak_price * (1 - trailing_distance) {
        trigger_market_close();
    }
}

// Sell trailing stop: peak_price 只降不升
if side == "sell" {
    if current_price < peak_price {
        peak_price = current_price;  // 下移
    }
    if current_price >= peak_price * (1 + trailing_distance) {
        trigger_market_close();
    }
}
```

### 3.4 关键决策

- **D1**: `orders.advanced_type = "trailing_stop"`（沿用 P1-2.x 模式）
- **D2**: peak_price 存 `advanced_params.peak_price` (JSONB) — 不新建表
- **D3**: 轮询频率 2s,可在 AppState 配 `TRAILING_POLL_INTERVAL_SECS` env var
- **D4**: 轮询 task 用 `tokio::time::interval` 简单实现,无复杂状态机
- **D5**: 触发时创建 close order 通过调用现有 `create_order` 路径 (内部 API)
- **D6**: 取消 trailing stop 不级联删除 (trailing 母单取消后, 不会有 close 订单)

### 3.5 数据流

```
POST /orders order_type=trailing_stop
  → handler validate
  → write orders.advanced_type=trailing_stop, advanced_params={peak_price: price, trailing_distance, side, ...}
  → 201 Created
  → (后台 poll task 每 2s)
  → SELECT * FROM orders WHERE advanced_type='trailing_stop' AND status='pending'
  → For each: GET /ticker/price → update peak_price if beneficial
  → If trigger: create market close + update order status
```

## 影响面

| 范围 | 影响 |
|---|---|
| 现有 limit/market 订单 | 无影响 (advanced_type 是新字段) |
| 匹配引擎 (matching_engine.rs) | 无需改 (trailing 用自己的轮询,不依赖 fill) |
| 现有 P1-2.1/2.2 业务 | 无影响 (独立代码路径) |
| 资源 | +1 background task + ~5 req/s exchange API |
| 数据库 | 无 migration 需求 (用 advanced_params JSONB) |
