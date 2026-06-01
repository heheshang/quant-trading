# T0 — P0-3 Business Metrics Instrumentation 任务总览

> **任务 ID**：P0-3
> **前置**：[P0-1 /metrics 端点](../p0-metrics/T0_Task_Overview.md)
> **[P0-2 HTTP middleware](../p0-http-metrics-mw/T0_Task_Overview.md)**
> **日期**：2026-06-01
> **状态**：✅ Done

## 范围

P0-1 声明了 15 个 metric family，P0-2 让 HTTP 请求被自动计数，但 **15 个 metric 中只有 `BUILD_INFO` 真正有数据**。P0-3 的目标：

1. 在 **12 个 metric family** 的实际生产/业务流程中插入 `inc()` / `inc_by()` / `observe()` 调用
2. 保证 label cardinality 上限封闭（不引入 user_id / order_id 维度）
3. 走完完整质量门禁（cargo test + clippy + deny）
4. 实跑验证：手工 curl 触发后 `/metrics` 出现新行

## 12 个埋点 metric

| Metric | 埋点位置 | Label | 数量 |
|---|---|---|---|
| `orders_created_total` | `handlers/order.rs:create_order` (2 path) | side, type, exchange | 2 |
| `orders_cancelled_total` | `handlers/order.rs:cancel_order` | actor, exchange | 1 |
| `orders_filled_total` | `services/matching_engine.rs` (3 path) | side, exchange | 3 |
| `trigger_orders_created_total` | `services/trigger_order.rs` 4 个 create_* | type | 4 |
| `trigger_orders_fired_total` | `services/trigger_order.rs:trigger_order` | type | 1 |
| `risk_rules_tripped_total` | `services/risk_manager.rs:check_order` 3 trip 点 | rule | 3 |
| `rate_limit_denied_total` | `services/order_rate_limiter.rs` 3 scope | scope | 3 |
| `ws_messages_broadcast_total` | `services/exchange/ws_hub.rs:broadcast` | kind | 1 |
| `ws_connections_active` | `handlers/ws.rs:handle_socket` (inc+dec) | kind | 2 |
| `kline_persist_total` | `services/kline_writer.rs:flush` | exchange, symbol | 1 |
| `ai_predictions_total` | `handlers/ai.rs:get_prediction` | direction | 1 |

**Total: 22 inc/observe call sites**

## 质量门禁

| 阶段 | 命令 | 结果 |
|---|---|---|
| 类型检查 | `cargo check --all-targets` | ✅ 0 errors |
| 单元测试 | `cargo test --lib` | ✅ 379/379 pass |
| Lint | `cargo clippy --lib --all-targets --all-features -- -D warnings` | ✅ 0 warnings |
| 安全审计 | `cargo deny check` | ✅ 4/4 ok (advisories / bans / licenses / sources) |
| 实跑验证 | `cargo run` + curl 5 路径 | ✅ `/metrics` 输出新行 |

## 业务价值

**之前** `/metrics` 输出只有 0 行数据 (lazy family)
**现在** 真实业务流产生数据：
- 订单创建/取消/成交 → 业务活跃度可监控
- 风险规则触发 → 风控报警基础
- 限流拒绝 → 防滥用信号
- WS 广播 / 连接数 → 实时分发健康度
- K线持久化 → 数据管道吞吐
- AI 预测分布 → 模型偏差监控

下一步（**P0-4 Grafana Dashboard**）可以直接基于这 12 metric 设计 5+ 面板。
