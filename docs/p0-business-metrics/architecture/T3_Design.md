# T3 — 设计文档

## 3.1 埋点矩阵

| # | Metric | 埋点函数 | 文件:行 | Label 值 | 数量 |
|---|---|---|---|---|---|
| 1 | `orders_created_total` | `create_order` (market) | `handlers/order.rs:603` | buy,market,local | +1 |
| 2 | `orders_created_total` | `create_order` (limit) | `handlers/order.rs:634` | buy/sell,limit,local | +1 |
| 3 | `orders_cancelled_total` | `cancel_order` | `handlers/order.rs:772` | user,local | +1 |
| 4 | `orders_filled_total` | `match_market` | `services/matching_engine.rs:336` | buy/sell,local | +filled |
| 5 | `orders_filled_total` | `match_limit` (ask 吃 bid) | `services/matching_engine.rs:449` | sell,local | +fill_qty |
| 6 | `orders_filled_total` | `match_limit` (bid 吃 ask) | `services/matching_engine.rs:504` | buy,local | +fill_qty |
| 7 | `trigger_orders_created_total` | `create_stop_loss` | `services/trigger_order.rs:269` | stop_loss | +1 |
| 8 | `trigger_orders_created_total` | `create_take_profit` | `services/trigger_order.rs:351` | take_profit | +1 |
| 9 | `trigger_orders_created_total` | `create_oco` | `services/trigger_order.rs:485` | oco | **+2** (sl+tp 一对) |
| 10 | `trigger_orders_created_total` | `create_twap` | `services/trigger_order.rs:553` | twap | +1 |
| 11 | `trigger_orders_fired_total` | `trigger_order` (激活) | `services/trigger_order.rs:681` | stop_loss/take_profit/oco | +1 |
| 12 | `risk_rules_tripped_total` | `check_order` (daily_loss) | `services/risk_manager.rs:208` | daily_loss | +1 |
| 13 | `risk_rules_tripped_total` | `check_order` (single_trade) | `services/risk_manager.rs:257` | single_trade | +1 |
| 14 | `risk_rules_tripped_total` | `check_order` (max_drawdown) | `services/risk_manager.rs:278` | max_drawdown | +1 |
| 15 | `rate_limit_denied_total` | `OrderRateLimiter::check` (user) | `services/order_rate_limiter.rs` | user | +1 |
| 16 | `rate_limit_denied_total` | ... (symbol) | 同上 | symbol | +1 |
| 17 | `rate_limit_denied_total` | ... (global) | 同上 | global | +1 |
| 18 | `ws_messages_broadcast_total` | `WsHub::broadcast` | `services/exchange/ws_hub.rs:294` | ticker/depth/kline/... | +1 |
| 19 | `ws_connections_active` | `handle_socket` (建连) | `handlers/ws.rs:257` | frontend | +1 |
| 20 | `ws_connections_active` | `handle_socket` (断连) | `handlers/ws.rs:359` | frontend | -1 |
| 21 | `kline_persist_total` | `KlineWriter::flush` | `services/kline_writer.rs:146` | binance,all | +inserted |
| 22 | `ai_predictions_total` | `get_prediction` | `handlers/ai.rs:282` | long/short/neutral | +1 |

**Total: 22 call sites** in 8 files

## 3.2 文件改动清单

| 文件 | 改动 | 新增行 |
|---|---|---|
| `backend/src/handlers/order.rs` | + 2 create_order inc + 1 cancel inc + 2 helper fn | ~25 |
| `backend/src/handlers/ws.rs` | + 1 inc + 1 dec | ~8 |
| `backend/src/handlers/ai.rs` | + 1 inc + match | ~14 |
| `backend/src/services/matching_engine.rs` | + 3 inc (inline match) | ~20 |
| `backend/src/services/risk_manager.rs` | + 3 inc | ~12 |
| `backend/src/services/order_rate_limiter.rs` | + 3 inc | ~12 |
| `backend/src/services/exchange/ws_hub.rs` | + 1 inc (match 6 variant) | ~17 |
| `backend/src/services/kline_writer.rs` | + 1 inc_by | ~7 |
| `backend/src/services/trigger_order.rs` | + 5 inc + `inc_by(2)` (OCO) | ~25 |

**Total**: ~140 lines of new code (含注释 + helper + match arms)

## 3.3 关键设计决策

### D1: OCO 双计
```rust
crate::metrics::TRIGGER_ORDERS_CREATED_TOTAL
    .with_label_values(&["oco"])
    .inc_by(2);  // OCO 创建 SL + TP 2 单
```
不拆 metric（避免 cardinality ×2），用 `inc_by(2)` 表达"一对 OCO"语义。

### D2: WS broadcast 用 match 而非 Display trait
```rust
let kind = match &msg {
    HubMessage::Ticker { .. } => "ticker",
    ...
    HubMessage::AIPredict { .. } => "ai_predict",
};
```
**理由**: Display impl 会污染 HubMessage 的语义（"展示给人看" vs "指标 label"）。在埋点处显式映射是**更纯净的关注点分离**。

### D3: trigger_type_str 已存在
发现 P0-1 测试代码已定义 `fn trigger_type_str(v: &TriggerType) -> &'static str` 在 `services/trigger_order.rs:34`。
**不要重复定义**（clippy::items_after_test_module 会 catch 末尾新定义）。
直接复用现有 helper，命名保持一致 (`trigger_type_str`)。

### D4: AI predictions 方向降级
`prediction.unwrap_or()` 默认返回 `direction = "neutral"`，所以即使模型调用失败，也能保证 metric label 集合封闭（**long / short / neutral**），不会引入 "unknown" / "error" 等开放值。

### D5: OrderSide label 在 services 层 inline
`matching_engine.rs` 在 `services/` 目录，但 `order_side_str` helper 在 `handlers/order.rs`。
**不跨 crate/模块复用**（避免 services 依赖 handlers 出现反向依赖）。
用 inline `match &entry.side { OrderSide::Buy => "buy", OrderSide::Sell => "sell" }`。
3 处复制 6 行 — **可接受**（DRY 不应过载 import 方向）。
