# T1-PRD — P1-2.2 Bracket Order (延后 OCO 模式, 路径 A)

> **任务 ID**: P1-2.2
> **前置**: P1-2.1 (Iceberg) + P1-F3 (trigger_orders 表 + service)
> **路径 A**: 不在母单 fill 时自动建 OCO, 仅记录 metadata + emit "bracket_filled" 事件, 由前端轮询后调 `/trigger-orders/oco` 自行挂 SL+TP
> **日期**: 2026-06-02

## 1. 背景

### 1.1 问题
量化交易者开仓时希望"一个 API 调用同时设好止盈+止损", 避免 3 步手挂 (entry + SL + TP). 标准做法是 Bracket 母单: 一个 entry order + 2 个联动 OCO 子单 (SL + TP).

### 1.2 现状
- P1-F3 已建 `trigger_orders` 表 + `TriggerOrderService::create_oco()` + `/api/v1/trigger-orders/oco` endpoint
- P1-2 MVP 已加 `OrderType::Bracket` enum variant + handler 解析, 但无业务逻辑
- 母单成交后 **不写 position 表** — create_oco() 强求 `position_id` 不能 NULL

### 1.3 为什么延后 OCO (路径 A)
- **解耦**: OCO 创建涉及 user 持仓意图, 不同交易所 / 不同策略对 position_id 解析方式不同
- **前端灵活性**: 前端可结合策略信号 (RSI/MACD) 决定是否挂 OCO
- **后端原子性**: 母单 fill 状态 → 写 `bracket_links` 表, OCO 失败可重试不丢原子性
- **避免 hot path 阻塞**: fill hook 只写一行 bracket_link, 不调外部

## 2. Gherkin 验收

### Scenario 1: 创建 Bracket 母单
```
Given user POSTs /api/v1/orders with
  symbol="BTC/USDT", side="buy", order_type="bracket",
  price="50000", quantity="1.0",
  stop_loss_price="49000", take_profit_price="52000"
When handler validates and creates order
Then order created with status=pending
  and advanced_type="bracket"
  and advanced_params={stop_loss_price:49000, take_profit_price:52000, oco_status:"pending"}
```

### Scenario 2: Bracket 母单 fill → 写 bracket_link + emit event
```
Given bracket parent order is fully filled by matching engine
When flush_trades detects advanced_type="bracket" and order fully filled
Then 1 row is inserted into bracket_links (parent_order_id, sl_price, tp_price, oco_status="pending", oco_trigger_order_ids=null)
  and metrics BRACKET_PARENT_FILLED_TOTAL counter is incremented
  and WS event bracket_filled is broadcast to user
```

### Scenario 3: 前端轮询拉 pending bracket OCO
```
Given bracket parent is filled, oco_status=pending
When frontend GETs /api/v1/bracket-links/pending
Then response includes all pending bracket links for current user
  and each link exposes sl_price, tp_price, parent_filled_quantity
```

### Scenario 4: 校验失败 (buy 方向 stop_loss >= price)
```
Given user POSTs bracket with side="buy", price="100", stop_loss_price="105"
When handler validates
Then 400 with "stop_loss_price must be < price for buy bracket"
```

### Scenario 5: 校验失败 (trail quantity 非法)
```
Given user POSTs bracket with quantity="0" or stop_loss_price="0"
When handler validates
Then 400 with "quantity and prices must be > 0"
```

## 3. NFR

| # | 需求 | 阈值 |
|---|------|------|
| NFR-1 | Bracket 母单 fill 写 bracket_link 延迟 | < 100ms (复用 flush_trades batch) |
| NFR-2 | 校验失败响应延迟 | < 50ms (P95) |
| NFR-3 | GET bracket-links/pending 响应时间 | < 200ms (10k 行内) |
| NFR-4 | 并发: 1000 母单同时 fill 不丢失 bracket_link | 1k QPS |
| NFR-5 | bracket_links 表可追溯 90 天 | 不删 |
| NFR-6 | 公开 API 不暴露 advanced_params 内部字段 (只暴露 oco_status 摘要) | 一致性 |
| NFR-7 | fill hook 写 bracket_link 失败不阻塞 trade batch | 必填 |
| NFR-8 | DB migration 必须 IF NOT EXISTS, 不破坏已部署环境 | 必填 |

## 4. 验收标准 (Acceptance)

1. ✅ `cargo test --lib` 全 pass (含 8+ Bracket unit tests)
2. ✅ `cargo clippy --lib --all-targets --all-features -- -D warnings` 0 warning
3. ✅ `cargo build` 干净
4. ✅ E2E curl: `POST /api/v1/orders {order_type:"bracket", ...}` → 201
5. ✅ DB 验证: bracket 母单 row 存在 + advanced_type="bracket"
6. ✅ fill hook: 母单 fill 后 DB 出现 bracket_link row
7. ✅ `GET /api/v1/bracket-links/pending` 返回当前 user 的 pending links
8. ✅ 校验: buy 方向 stop_loss >= price → 400

## 5. 范围外 (Out of Scope)

- **OCO 自动创建** (P1-2.2 v2 计划): 母单 fill 时自动调 create_oco, 需先 P0 加 position 自动建
- **前端联动 UI** (P1-2.4 计划)
- **多交易所 SL/TP 同步**
- **Bracket 修改** (改 SL/TP 价格后 OCO 重挂)
