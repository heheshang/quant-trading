# T2 — 技术选型

## 2.1 埋点 API 选型

| 方案 | 类型 | 性能 | 选 |
|---|---|---|---|
| `prometheus::IntCounterVec::with_label_values(&[..]).inc()` | 类型安全 + 编译期校验 | 高（lock-free atomic） | ✅ |
| `prometheus::register_int_counter_vec!` 宏 | 自动注册 | 高 | ❌ 与全局 Registry 冲突 |
| 直接构造 `Counter::new` | 灵活 | 需手动注册 | ❌ 重复劳动 |

**结论**：继续 P0-1 选型 — 用全局 `LazyLock<Registry>` + `*::with_label_values().inc()`。

## 2.2 Label Cardinality 控制

**关键原则**：所有 label 值必须是**编译期可枚举**的封闭集合。

| Metric | Label | 封闭集合 |
|---|---|---|
| `orders_created_total` | `side, type, exchange` | {buy, sell} × {limit, market, stop, stop_limit} × {local} |
| `trigger_orders_created_total` | `type` | {stop_loss, take_profit, oco, twap} |
| `risk_rules_tripped_total` | `rule` | {daily_loss, single_trade, max_drawdown} |
| `rate_limit_denied_total` | `scope` | {user, symbol, global} |
| `ws_messages_broadcast_total` | `kind` | {ticker, depth, kline, trade_executed, backtest_progress, ai_predict} |
| `ai_predictions_total` | `direction` | {long, short, neutral} |

**绝对禁止**：
- ❌ `user_id` / `order_id` / `symbol` 作为 label（cardinality 爆炸 → Prometheus OOM）
- ❌ `path` 原始 URL 作为 label（用 `MatchedPath` 路由模式替代，**P0-2 已用**）

## 2.3 Stringify enum 模式

```rust
// before: 写 helper 重复维护
fn trigger_type_str(v: &TriggerType) -> &'static str { ... }

// 在埋点处：
.with_label_values(&[trigger_type_str(&order.trigger_type)])
```

**已有 helper**（P0-1 测试代码遗留）：
- `handlers/order.rs::order_side_str()` / `order_type_str()`
- `services/trigger_order.rs::trigger_type_str()` (line 34)

**P0-3 新增 inline match**（避免新建 helper 文件）：
- `services/matching_engine.rs::OrderSide` — inline `match` (3 处)
- `handlers/ai.rs::PredictionResult::direction` — `match` 字符串

## 2.4 错误处理策略

```rust
// ✅ 业务路径用 inc() / inc_by() — 无 Result
crate::metrics::ORDERS_CREATED_TOTAL
    .with_label_values(&[...])
    .inc();

// ❌ 不要用带 Result 的 API（增加噪声）
// IntCounterVec::with_label_values 返回 Counter，不是 Result
```

如果 `with_label_values` 内部 panic（**不会**因为 prometheus crate 用 `OnceLock` + 首次 inc 时 lazy create），业务路径会一起 panic — 但这是 P0-1 已确认行为，整个 P0 系列保持一致。
