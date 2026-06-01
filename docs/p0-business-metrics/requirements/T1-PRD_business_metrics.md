# T1-PRD — Business Metrics Instrumentation (P0-3)

> **任务 ID**：P0-3
> **前置**：P0-1 端点 + P0-2 middleware
> **日期**：2026-06-01

## 1. 背景

P0-1 实现了 `/metrics` 端点 + 15 metric family 声明。P0-2 让 HTTP middleware 自动记录请求数。**但这两个加起来**只覆盖了 **HTTP 维度**。量化交易业务的真正信号在：

- 订单生命周期（创建 / 取消 / 撮合成交）
- 触发单（条件单创建 / 触发激活）
- 风控（规则 trip 次数）
- 限流（user / symbol / global 拒绝）
- WS 实时分发（广播 / 活跃连接）
- 数据管道（K线持久化）
- AI 模型（预测方向分布）

## 2. Gherkin 验收

### Scenario 1: 订单创建被计数
```
Given backend running, authenticated user with valid JWT
When POST /api/v1/orders with a market order
Then /metrics output contains:
  orders_created_total{side="buy",order_type="market",exchange="local"} 1
```

### Scenario 2: market 单成交被计数
```
Given market order submitted, depth cache has matching orders
When matching engine fills the order
Then /metrics output contains:
  orders_filled_total{side="buy",exchange="local"} <filled_qty>
```

### Scenario 3: 触发单创建 + 激活被分别计数
```
Given user has open position
When POST /api/v1/triggers/stop_loss
Then trigger_orders_created_total{type="stop_loss"} += 1
When market price crosses stop price
Then trigger_orders_fired_total{type="stop_loss"} += 1
```

### Scenario 4: 风控规则 trip
```
Given user daily loss > daily_loss_limit
When user submits new order
Then risk_rules_tripped_total{rule="daily_loss"} += 1
And order is rejected with RiskViolation
```

### Scenario 5: WS 广播按类型计数
```
Given kline writer flushes a batch
When ws_hub.broadcast(HubMessage::Kline{..}) is called
Then ws_messages_broadcast_total{kind="kline"} += 1
```

## 3. 非功能需求

- **NFR-1 Cardinality**: 所有 label 值必须是封闭枚举（`buy`/`sell` / `up`/`down`/`neutral` / `user`/`symbol`/`global` 等）。禁止 user_id / order_id / symbol 作为 label（会爆炸）
- **NFR-2 零开销失败**: 埋点调用不能 panic / 不能阻塞业务。如果 metric registry 异常，业务路径继续（看 [`prometheus` crate](https://docs.rs/prometheus) 设计，Counter::inc() never panics）
- **NFR-3 不破坏现有测试**: 379 个 unit test 全部继续 pass，无 regression
- **NFR-4 lint 干净**: cargo clippy 0 warning
- **NFR-5 安全审计**: cargo deny 4/4 ok

## 4. 不在范围

- ❌ 业务 metric 报警（**P1+ 任务**）
- ❌ Grafana Dashboard JSON（**P0-4 任务**）
- ❌ 持久化历史（用 Prometheus 自己做，**运维侧**）
- ❌ 跨实例聚合（当前单实例，**P2+ 任务**）
