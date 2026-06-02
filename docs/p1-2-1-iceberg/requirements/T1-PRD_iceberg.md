# T1-PRD — Iceberg Order Business Logic (P1-2.1)

> **任务 ID**：P1-2.1
> **前置**：P1-2 (MVP 基础架构) 完成
> **日期**：2026-06-01

## 1. 背景

P1-2 已完成基础架构：`OrderType::Iceberg` enum + DB column `advanced_type`/`advanced_params` + handler 解析 5 变体 + `parse_order_type` 5 变体。**但实际业务逻辑未实现**——**下 iceberg 单会被当普通 limit 写入**。

P1-2.1 实现 **3 个核心能力**：
1. **拆单**（split）：母单 1 → N 个子单（每个 `quantity = visible_quantity`）
2. **补单**（replenish）：子单成交后自动入下一子单
3. **取消级联**（cancel cascade）：母单 cancel → 全部子单 cancel + 从 book 撤单

## 2. Gherkin 验收

### Scenario 1: Iceberg 母单创建并被拆解
```
Given user POSTs /api/v1/orders with order_type="iceberg",
  price=100000.0, quantity=10.0, advanced_params={visible_quantity: 1.0}
When backend processes
Then 1 parent order is created with:
  - order_type="iceberg"
  - advanced_type="iceberg"
  - advanced_params={visible_quantity: 1.0, total_quantity: 10.0, filled_children: 0, children_ids: []}
  - status="pending"
And 1 child order is created with:
  - order_type="limit"  (internal)
  - advanced_type="iceberg_child"
  - advanced_params={parent_id: "<parent_uuid>", slice_index: 0}
  - quantity=1.0
  - status="pending"
And only the first child is inserted into matching engine order book
```

### Scenario 2: 子单成交后自动补单
```
Given parent iceberg order with 10 children (each 1.0)
And 3 children have already filled
When the 4th child fills at price=100000
Then parent.filled_quantity is updated to 4.0
And a 5th child is created and inserted into order book
And metric iceberg_child_orders_total{action="filled"} is incremented
```

### Scenario 3: 母单完成
```
Given parent iceberg with total_quantity=10.0
And the 10th child has just filled
When flush_trades hook runs
Then parent.filled_quantity == 10.0
And parent.status == "filled"
And parent.filled_at is set
And NO new child is created (children list exhausted)
```

### Scenario 4: 母单 cancel 级联到子单
```
Given parent iceberg with 3 children pending in order book
When user POSTs /api/v1/orders/{parent_id}/cancel
Then parent.status == "cancelled"
And all 3 children are also marked status="cancelled"
And all 3 children are removed from order book
And metric iceberg_child_orders_total{action="cancelled"} is incremented
```

### Scenario 5: 参数校验
```
When user POSTs iceberg with visible_quantity >= quantity
Then 400 with "visible_quantity must be < total quantity"
When user POSTs iceberg with visible_quantity <= 0
Then 400 with "visible_quantity must be > 0"
When user POSTs iceberg with advanced_params missing
Then 400 with "advanced_params required for iceberg orders"
When user POSTs iceberg with price missing
Then 400 with "Iceberg order requires price"
```

## 3. 非功能需求

- **NFR-1 拆单总数对齐**：sum(子单 quantity) = 母单 total_quantity
- **NFR-2 子单独立 cancel**：单独 cancel 一个子单不影响母单（**P1-2.1 不实现**，P2+ 任务）
- **NFR-3 子单成交时间窗口**：补单应在子单成交后 100ms 内入 book（实际由 flush_trades 100ms 周期决定）
- **NFR-4 不破坏现有 limit/market 流程**：所有现有测试 100% 继续 pass
- **NFR-5 失败回滚**：拆单失败 → 母单不回写入库（事务）
- **NFR-6 余额计算**：母单冻结 = total_quantity * price（不按 visible 算）
- **NFR-7 metric 埋点**：`iceberg_child_orders_total{action="created"|"filled"|"cancelled"}` Counter
- **NFR-8 子单 quantity 精度**：最后一片子单 = total - visible * floor(total/visible)，**避免浮点误差**

## 4. 不在范围（P2+ 任务）

- ❌ 子单独立 cancel（仅母单 cancel 级联）
- ❌ 子单 modify（修改价/量）
- ❌ 动态 visible_quantity（自适应市场深度）
- ❌ Iceberg 在 backtest engine 完整支持
- ❌ 前端 Iceberg UI（仅后端）

## 5. 与现有 trigger_order / 业务系统的关系

- **不**使用 trigger_order 系统
- **不**影响 matching engine 现有匹配逻辑（子单就是 limit）
- **不**改 OrderType 枚举（已 P1-2 MVP 完成）
- **不**改 DB schema（advanced_params JSONB 足够）
- **复用** matching engine 的 `trade_sink` channel（flush_trades hook 点）
- **复用** RiskManager（母单一次性 check，不分摊到子单）
- **复用** OrderRateLimiter（**关键决策**：母单算 1 次配额 vs 子单每次算 1 次 → **本 MVP 算 1 次**）
