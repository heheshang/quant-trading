# T2+T3 — 技术选型 & 设计文档

## T2 选型

### 2.1 OrderType 扩展方式

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **PostgreSQL ALTER TYPE ADD VALUE** | 业界标准 / 兼容性好 | 不能在事务中运行（migration 需独立运行）| ✅ |
| 新表 `order_types` | 灵活 | 改 join 多，SeaORM derive 失效 | ❌ |
| JSON 字段代替 enum | 无需改 schema | 失去 enum 类型约束 | ❌ |

**结论**：用 `ALTER TYPE ... ADD VALUE` + `IF NOT EXISTS` 守卫。

### 2.2 参数存储

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **单一 `advanced_params JSONB` 列** | schema 简单，应用层 validate | 无 db-side 约束 | ✅ |
| 3 个 JSONB 列 (iceberg/bracket/trailing) | 应用层 join 简单 | 99% 行为 NULL，浪费 | ❌ |
| 关联子表 `order_advanced_params` | 强 schema | 复杂 join | ❌ |

**结论**：单一 `advanced_params JSONB` + 应用层 validate。

### 2.3 3 种订单的内部执行策略

| 类型 | 内部执行 | 关键 |
|---|---|---|
| **Iceberg** | 拆为 N 个 limit 子单 | 子单入 book，metadata 记录 parent |
| **Bracket** | 母单成交后挂 SL+TP trigger | 复用 `trigger_orders` 表 |
| **TrailingStop** | 后台服务轮询调整 SL | 新增 `trailing_stop_service` |

### 2.4 与现有 trigger_order 集成

**P1-2 不引入新 TriggerType**——复用现有 StopLoss / TakeProfit / Oco：
- Bracket = Oco (1 SL + 1 TP)
- TrailingStop = 动态调 StopLoss trigger

**好处**：不扩 trigger_order enum，trailing 复用 StopLoss 现有路径。

## T3 设计

### 3.1 OrderType 枚举扩展

```rust
pub enum OrderType {
    Limit,        // existing
    Market,       // existing
    // P1-2: 3 new variants
    Iceberg,      // 大单拆解
    Bracket,      // 自动 SL + TP
    TrailingStop, // 移动止损
}
```

### 3.2 3 种新订单的 API 请求/响应

#### Iceberg
```json
POST /api/v1/orders
{
  "symbol": "BTCUSDT",
  "side": "buy",
  "order_type": "iceberg",
  "price": 100000.0,
  "quantity": 10.0,
  "advanced_params": { "visible_quantity": 1.0 }
}
```

**行为**：拆为 10 个 1.0 BTC 子单，每次成交后**自动补 1 个子单**直到总成交量 = 10.0。

**关键设计**：
- 子单用 `parent_order_id` 关联（用 `advanced_params.parent_id`）
- 子单是普通 limit，可独立 cancel
- 母单 status 反映子单综合状态

#### Bracket
```json
POST /api/v1/orders
{
  "symbol": "BTCUSDT",
  "side": "buy",
  "order_type": "bracket",
  "price": 100000.0,
  "quantity": 1.0,
  "advanced_params": {
    "entry_price": 100000.0,
    "stop_loss": 95000.0,
    "take_profit": 110000.0
  }
}
```

**行为**：
- 母单先以 limit 价成交
- 成交后**自动创建 1 个 OCO trigger** = StopLoss(95k) + TakeProfit(110k)
- 任一触发 → 取消另一个（标准 OCO 行为）

**关键设计**：
- 复用 `trigger_orders` 表的 OCO entry
- 母单通过 `advanced_params.bracket_parent_id` 关联 OCO

#### TrailingStop
```json
POST /api/v1/orders
{
  "symbol": "BTCUSDT",
  "side": "sell",
  "order_type": "trailing_stop",
  "price": null,         // 移动止损单，无固定 entry
  "quantity": 1.0,
  "advanced_params": {
    "entry_price": 100000.0,    // 用户认为的"激活价"
    "trail_amount": 2000.0,     // 跟踪距离
    "trail_type": "absolute"    // absolute | percent (P2+)
  }
}
```

**行为**：
- 母单**直接成交**（作为 market/limit 关闭现有仓位的保护）
- 成交后**自动挂 1 个 StopLoss trigger** 价格 = current_price - trail
- 后台服务每 5s 检查：
  - Buy trailing: 跟踪 **lowest_price**，stop = lowest - trail
  - Sell trailing: 跟踪 **highest_price**，stop = highest - trail
- 价格触及 stop → 触发 SL → 卖/买 平仓

**关键设计**：
- 后台 service 需 `Arc<TrailingStopService>` 注入 axum state
- 轮询可配 `interval_seconds`（默认 5）
- P1-2 范围：仅 absolute mode

### 3.3 关键设计决策 (D1-D8)

**D1: 3 种新 type 入 matching engine 方式**
- 都视作 `OrderType::Limit` 处理（入 book）
- 用 `advanced_type` 字段区分（冗余存储，查询用）
- matching engine 不感知（**P1-2 简化**）

**D2: advanced_params 在 order handler 层 validate**
- 每种新 type 一个 validate 函数
- 严格模式：缺字段/越界立即 400

**D3: Iceberg 子单存储**
- 用 `orders.parent_order_id` 还是 `advanced_params.parent_id`？
- **P1-2 简化**：用 `advanced_params.parent_id` (UUID string)，**不**加新 column
- 子单 cancel 通过 parent_id 查所有 → 全部 cancel

**D4: Bracket 母单成交后异步挂 trigger**
- matching engine 调 `on_order_filled` callback（**已有**）
- callback 检测 `advanced_type == "bracket"` → 创建 OCO trigger

**D5: TrailingStop 后台服务**
- 新文件 `services/trailing_stop_service.rs`
- `tokio::spawn` 启动一个 loop
- 每 5s 查 `orders WHERE advanced_type='trailing_stop' AND status='pending'`
- 调 market data service 拿 current_price
- 计算新 stop → UPDATE trigger_orders

**D6: metric 埋点**
- `iceberg_child_orders_total{action="created"|"filled"}` — 拆解次数
- `bracket_oco_triggered_total{triggered="sl"|"tp"}` — 触发次数
- `trailing_stop_adjustments_total{direction="up"|"down"}` — 调整次数

**D7: 3 种新 type 的 time_in_force**
- 沿用 default GTC
- Iceberg/Bracket 可用 IOC（不推荐）
- TrailingStop 用 GTC

**D8: validate() 在 service 层（不在 SeaORM）**
- handler 解析 → service 校验 → DB 写入
- 业务逻辑集中在 service

### 3.4 单元测试策略

| 测试文件 | 数量 | 内容 |
|---|---|---|
| `db/order.rs` (新增) | 5 | parse_order_type 5 个变体 + 错误 |
| `handlers/order.rs` (扩) | 8 | create_order iceberg/bracket/trailing + validate 失败 |
| `services/iceberg.rs` (新) | 3 | split_into_children / parent_id 查询 / 全部 cancel |
| `services/bracket.rs` (新) | 3 | on_fill 创建 OCO / 任一触发 cancel 另一个 |
| `services/trailing_stop_service.rs` (新) | 3 | adjust_stop / interval 轮询 / state 更新 |
| **合计** | **22** | |

### 3.5 影响面分析

| 现有模块 | 影响 | 处理 |
|---|---|---|
| `db/order.rs` OrderType | enum 扩 3 变体 | 直接改 |
| `handlers/order.rs` parse_order_type | 字符串→enum 扩 3 | 改 match |
| `handlers/order.rs` create_order | 多 match 分支 | 加 Iceberg/B/Trailing 分支 |
| `services/matching_engine.rs` | 无需改 | D1 |
| `services/trigger_order.rs` | 复用 create_oco | 无需改 |
| `services/risk_manager.rs` | 母单走风险检查 | 同 Limit |
| `services/order_rate_limiter.rs` | 母单占配额 | 同 Limit |
| `services/market_data.rs` | TrailingStop service 订阅 | 新增 |
| `db/orders` 表 | 加 2 column + enum 扩 | migration |
| `models/schemas/order.rs` | Request/Response 字段 | 扩 |

**新文件**：
- `migrations/20260602000000_advanced_order_types.sql`
- `services/iceberg.rs`
- `services/bracket.rs`
- `services/trailing_stop_service.rs`
- `services/advanced_order_validator.rs`（validate 函数集合）
- `models/schemas/advanced_order.rs`（3 种 params 的 typed struct）
- `db/advanced_order_type.rs`（enum + match helpers）
