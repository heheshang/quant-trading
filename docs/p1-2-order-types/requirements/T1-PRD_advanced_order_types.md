# T1-PRD — Advanced Order Types: Iceberg / Bracket / Trailing (P1-2)

> **任务 ID**：P1-2
> **前置**：P0 完成 + P1-1 strategy templates
> **日期**：2026-06-01

## 1. 背景

业务审计报告（`docs/design/2026-06-01_Business_Audit_Report.md`）指出：当前 `OrderType` 仅支持 `limit` / `market` 两种基础类型。**业界三大高级订单类型**缺失：

| 类型 | 业务场景 | 用户痛点 |
|---|---|---|
| **Iceberg** | 大单分批 | 用户需手写脚本循环下小单；订单簿暴露意图；滑点高 |
| **Bracket** | 自动止盈止损 | 用户需手挂 3 单（母 + SL + TP），状态机复杂 |
| **Trailing Stop** | 移动止损 | 用户需实时监控 + 手动改 SL 单 |

P1-2 系统化支持 3 个高级订单类型，**对接现有 trigger_order 系统**做联动。

## 2. Gherkin 验收

### Scenario 1: Iceberg 订单注册并被拆为子单
```
Given user POSTs /api/v1/orders with order_type="iceberg",
  quantity=10.0, visible_quantity=1.0, limit_price=100
When backend processes
Then 1 parent order is created with status="pending", advanced_type="iceberg", advanced_params={visible_qty:1.0, total_qty:10.0}
And 10 child limit orders are created with quantity=1.0 each
And parent.order_id is referenced by children.parent_order_id (or via parent_id field)
And first child is submitted to matching engine immediately
```

### Scenario 2: Bracket 母单成交 → 自动挂 SL + TP
```
Given user POSTs /api/v1/orders with order_type="bracket",
  side="buy", quantity=1.0, entry_price=100, stop_loss=95, take_profit=110
When matching engine fills the bracket order
Then 1 trigger_order of type=StopLoss is auto-created at price=95
And 1 trigger_order of type=TakeProfit is auto-created at price=110
And 1 trigger_order of the other side is created as OCO (cancels the other when triggered)
```

### Scenario 3: Trailing Stop 后台服务
```
Given user POSTs /api/v1/orders with order_type="trailing_stop",
  side="sell" (closing long), quantity=1.0, trail_amount=2.0, trail_type="absolute"
And market price is 100, current stop is 98 (entry_price 100 - 2.0)
When price moves up to 102
Then trailing service adjusts stop to 100 (102 - 2.0 = 100)
And when price drops to 100
Then trailing stop triggers and 1 sell order fills at market
```

### Scenario 4: 参数校验
```
When user POSTs iceberg with visible_quantity >= quantity
Then 400 with "visible_quantity must be < total quantity"
When user POSTs bracket with stop_loss >= entry_price (for buy)
Then 400 with "stop_loss must be < entry_price for buy"
When user POSTs trailing_stop with trail_amount <= 0
Then 400 with "trail_amount must be > 0"
```

### Scenario 5: OrderType enum 扩 3 变体
```
Given GET /api/v1/orders/schema/order_types
When response returns
Then order_type enum has 5 values: limit, market, iceberg, bracket, trailing_stop
And postgres order_type enum type also has 5 values
```

## 3. 非功能需求

- **NFR-1 不破坏现有 limit/market 流程**：所有现有 limit/market order 测试 100% 继续 pass
- **NFR-2 JSONB 参数存储**：3 种新 type 用同一 `advanced_params JSONB` 列，**避免 3 个 column 浪费**
- **NFR-3 严格 validate**：每种新 type 的参数都有范围检查 + 交叉约束
- **NFR-4 不引入新表**：bracket/trailing_stop 复用 `trigger_orders` 表
- **NFR-5 iceberg 拆单可配置**：默认 visible_qty=min(quantity * 0.1, symbol.min_quantity * 10)
- **NFR-6 trailing service 后台线程**：5s 轮询，可配置 interval
- **NFR-7 metric 埋点**：`iceberg_child_orders_total` / `bracket_sl_triggered_total` / `bracket_tp_triggered_total` / `trailing_stop_adjusted_total`
- **NFR-8 schema migration 兼容**：新 enum values 用 `ALTER TYPE ... ADD VALUE`（PostgreSQL 12+）

## 4. 不在范围

- ❌ Iceberg 子单的并行匹配优化（**P2+ 任务**）
- ❌ Bracket OCO 状态机双向 cancel（**P2+ 任务**，本期仅单向）
- ❌ Trailing Stop 的百分比模式（**P2+ 任务**，本期仅 absolute）
- ❌ Frontend 高级订单 UI（**P2+ 任务**）
- ❌ Backtest engine 完整支持（**P1+ 任务**）
- ❌ Live exchange 实际下单（**P1+ 任务**，仅 paper mode）

## 5. 与现有 trigger_order 集成

```
OrderType::Bracket
  ├── 母单成交
  │   ├── INSERT trigger_orders (type=StopLoss, price=stop_loss, parent=母单)
  │   └── INSERT trigger_orders (type=TakeProfit, price=take_profit, parent=母单)
  └── 任一 trigger 触发
      └── 取消另一个 trigger（OCO 语义）

OrderType::TrailingStop
  ├── 母单成交
  │   └── INSERT trigger_orders (type=StopLoss, price=entry-trail, parent=母单, is_trailing=true)
  └── 5s 轮询服务
      └── price 上涨时 UPDATE trigger_orders SET trigger_price = high - trail_amount
```
