# Contract Review: Order Management Module (B1-B4)

> ADR: ADR-010
> 审查日期: 2026-05-14
> 审查者: Tech Lead
> 对照: PRD-trade-execution.md vs 现有代码

## B1: snake_case 字段映射

### 规则
TECH_CHARTER 规定 JSON 字段使用 snake_case。后端 Rust 枚举 serde 序列化必须与前端 TypeScript 类型完全对齐。

### 审查结果

| 后端字段 | JSON 序列化 | 前端类型字段 | 对齐 | 备注 |
|---------|-----------|-----------|------|------|
| `order_id` | `"order_id"` | `order_id: string` | ✅ | |
| `order_type` | `"order_type"` | `order_type: OrderType` | ✅ | |
| `filled_quantity` | `"filled_quantity"` | `filled_quantity: string` | ✅ | |
| `avg_fill_price` | `"avg_fill_price"` | `avg_fill_price: string\|null` | ✅ | |
| `reject_reason` | `"reject_reason"` | `reject_reason: string\|null` | ✅ | 前端已有 |
| `time_in_force` | `"time_in_force"` | `time_in_force: TimeInForce` | ✅ | |
| `cancelled_at` | `"cancelled_at"` | `cancelled_at: string\|null` | ✅ | |
| `filled_at` | `"filled_at"` | `filled_at: string\|null` | ✅ | |
| `frozen_balance` | `"frozen_balance"` | `frozen_balance: string` | ✅ | |
| `initial_balance` | `"initial_balance"` | `initial_balance: string` | ✅ | |
| `avg_entry_price` | `"avg_entry_price"` | `avg_entry_price: string` | ✅ | |
| `unrealized_pnl` | `"unrealized_pnl"` | `unrealized_pnl: string` | ✅ | |
| `realized_pnl` | `"realized_pnl"` | `realized_pnl: string` | ✅ | |
| `available_quantity` | `"available_quantity"` | `available_quantity: string` | ✅ | |
| `base_currency` | `"base_currency"` | `base_currency: string` | ✅ | |
| `quote_currency` | `"quote_currency"` | `quote_currency: string` | ✅ | |
| `price_precision` | `"price_precision"` | `price_precision: number` | ✅ | |
| `quantity_precision` | `"quantity_precision"` | `quantity_precision: number` | ✅ | |
| `min_quantity` | `"min_quantity"` | `min_quantity: string` | ✅ | |
| `max_quantity` | `"max_quantity"` | `max_quantity: string` | ✅ | |
| `min_notional` | `"min_notional"` | `min_notional: string` | ✅ | |
| `fee_rate` | `"fee_rate"` | `fee_rate: string` | ✅ | |

### CRITICAL 缺口

| 问题 | 严重性 | 说明 |
|------|--------|------|
| PRD 定义 `stop_price` 字段 | **MEDIUM** | PRD CreateOrderRequest 含 stop_price，但后端 CreateOrderRequest 和前端 CreateOrderRequest 均未包含。P1 止损单时需补充 |
| PRD 定义 `strategy_id` 字段 | **MEDIUM** | PRD CreateOrderRequest 含 strategy_id，后端 DB model 有但 CreateOrderRequest 缺少。策略信号下单时需补充 |
| PRD 定义 `mode` 字段 | **LOW** | PRD CreateOrderRequest 含 mode，后端 CreateOrderRequest 缺少（硬编码 Paper），MVP 可接受 |
| PRD 定义 `source` 筛选 | **LOW** | PRD ListOrdersQuery 含 source=manual/strategy，后端未实现 |

## B2: UUID 序列化

### 规则
所有 ID 字段使用 UUID v4，JSON 中序列化为小写连字符格式字符串 (`"550e8400-e29b-41d4-a716-446655440000"`)。

### 审查结果

| 字段 | 后端类型 | 序列化方式 | 前端类型 | 对齐 |
|------|---------|-----------|---------|------|
| `order_id` | `Uuid` | `.to_string()` | `string` | ✅ |
| `strategy_id` | `Option<Uuid>` | `.map(\|u\| u.to_string())` | (缺失) | ⚠️ |
| `user_id` (AccountResponse) | `Uuid` | `.to_string()` | `number` | **CRITICAL** |
| `id` (PositionResponse) | `Uuid` | `.to_string()` | `number` | **CRITICAL** |
| `id` (Position type) | `Uuid` | `.to_string()` | `number` | **CRITICAL** |

### CRITICAL 缺口

| 问题 | 严重性 | 说明 |
|------|--------|------|
| 前端 `PaperAccount.user_id` 类型为 `number` | **CRITICAL** | 后端返回 UUID 字符串，前端定义为 number 类型不匹配，运行时必然报错 |
| 前端 `Position.id` 类型为 `number` | **CRITICAL** | 同上，UUID 无法解析为 number |
| 前端 `Position.user_id` 类型为 `number` | **CRITICAL** | 同上 |
| 后端 `ClosePositionRequest` 使用 Path\<String\> (symbol) | **OK** | PRD 用 symbol 路径参数，但前端 `closePosition(id: number)` 传的是数字 ID | 

**修复方案：** 前端 `types/order.ts` 中 `PaperAccount.user_id`, `Position.id`, `Position.user_id` 全部改为 `string` 类型；`closePosition` 参数改为 `symbol: string`。

## B3: Option\<T\> → undefined | null 映射

### 规则
Rust `Option<T>` 在 JSON 中序列化为 `null`（serde 默认行为），前端对应 `T | null`。不可遗漏 nullable 标注。

### 审查结果

| 后端字段 | Rust 类型 | JSON 值 | 前端类型 | 对齐 |
|---------|---------|--------|---------|------|
| `price` | `Option<f64>` | `null` 或 `"50000.00"` | `string \| null` | ✅ |
| `avg_fill_price` | `Option<f64>` | `null` 或 `"50500.00"` | `string \| null` | ✅ |
| `reject_reason` | `Option<String>` | `null` 或 `"..."` | `string \| null` | ✅ |
| `cancelled_at` | `Option<DateTimeUtc>` | `null` 或 ISO8601 | `string \| null` | ✅ |
| `filled_at` | `Option<DateTimeUtc>` | `null` 或 ISO8601 | `string \| null` | ✅ |
| `strategy_id` | `Option<Uuid>` | `null` 或 UUID | (缺失) | ⚠️ |
| `expire_at` | `Option<DateTimeUtc>` | `null` 或 ISO8601 | (缺失) | ⚠️ |

### 缺口

| 问题 | 严重性 | 说明 |
|------|--------|------|
| 前端 Order 接口缺少 `strategy_id` | **MEDIUM** | 策略下单时需要显示来源，P1 补充 |
| 前端 Order 接口缺少 `expire_at` | **LOW** | 过期委托展示需要，MVP 可忽略 |
| `Option<f64>` 序列化为字符串而非 null | **INFO** | 后端 `order_to_response()` 将 `Option<f64>` 格式化为 `Option<String>` (如 `price: order.price.map(\|p\| format!("{:.8}", p))`)，这是正确做法 — 金额字段始终为字符串 |

## B4: 枚举值对齐

### 规则
后端 Rust 枚举的 serde 序列化值必须与前端 TypeScript 联合类型的字符串值完全一致。

### 审查结果

#### OrderSide

| 后端变体 | serde 值 | 前端值 | 对齐 |
|---------|---------|--------|------|
| `Buy` | `"buy"` | `'buy'` | ✅ |
| `Sell` | `"sell"` | `'sell'` | ✅ |

#### OrderType

| 后端变体 | serde 值 | 前端值 | 对齐 |
|---------|---------|--------|------|
| `Limit` | `"limit"` | `'limit'` | ✅ |
| `Market` | `"market"` | `'market'` | ✅ |
| `Stop` (PRD) | `"stop"` | (缺失) | ⚠️ P1 |
| `StopLimit` (PRD) | `"stop_limit"` | (缺失) | ⚠️ P1 |

#### OrderStatus

| 后端变体 | serde 值 | 前端值 | 对齐 |
|---------|---------|--------|------|
| `Pending` | `"pending"` | `'pending'` | ✅ |
| `PartialFilled` | `"partial_filled"` | `'partial_filled'` | ✅ |
| `Filled` | `"filled"` | `'filled'` | ✅ |
| `Cancelled` | `"cancelled"` | `'cancelled'` | ✅ |
| `Expired` | `"expired"` | `'expired'` | ✅ |
| `Rejected` | `"rejected"` | `'rejected'` | ✅ |

#### TradeMode

| 后端变体 | serde 值 | 前端值 | 对齐 |
|---------|---------|--------|------|
| `Paper` | `"paper"` | `'paper'` | ✅ |
| `Live` | `"live"` | `'live'` | ✅ |

**注意：** PRD 使用 `"simulation"` 但后端/前端使用 `"paper"`。这是一个命名偏移，需在文档中标注。

#### TimeInForce

| 后端变体 | serde 值 | 前端值 | 对齐 |
|---------|---------|--------|------|
| `GTC` | `"GTC"` | `'GTC'` | ✅ |
| `IOC` | `"IOC"` | `'IOC'` | ✅ |
| `FOK` | `"FOK"` | `'FOK'` | ✅ |

#### PositionSide

| 后端变体 | serde 值 | 前端值 | 对齐 |
|---------|---------|--------|------|
| `Long` | `"long"` | `'long'` | ✅ |
| `Short` | `"short"` | `'short'` | ✅ |

### 枚举缺口

| 问题 | 严重性 | 说明 |
|------|--------|------|
| PRD TradeMode = `"simulation"` vs 代码 `"paper"` | **MEDIUM** | 命名不一致，MVP 保持 `paper`，Phase 2 可考虑迁移 |
| PRD OrderType 缺 `stop` / `stop_limit` | **MEDIUM** | P1 止损单时需后端+前端同步扩展 |
| PRD WS 消息类型 vs 前端 TradeWsMessageType | **LOW** | 前端定义 `'trade'` 但 PRD 推送字段是 `'fill'`，需统一 |

---

## 汇总

### CRITICAL (3个 — 必须修复)

1. **前端 `PaperAccount.user_id` 类型为 `number`** → 应为 `string`（UUID）
2. **前端 `Position.id` 类型为 `number`** → 应为 `string`（UUID）
3. **前端 `Position.user_id` 类型为 `number`** → 应为 `string`（UUID）

### MEDIUM (4个 — P1 修复)

4. **PRD `stop_price` / `strategy_id` / `mode` 字段缺失** — CreateOrderRequest 需扩展
5. **PRD `simulation` vs 代码 `paper` 命名偏移** — 文档标注，Phase 2 统一
6. **OrderType 缺 `stop` / `stop_limit`** — P1 止损单时同步扩展
7. **前端 WS 消息类型 `trade` vs PRD `fill`** — 统一命名

### LOW (3个 — 可延后)

8. **PRD `source` 筛选未实现** — P1 策略信号功能时补充
9. **前端 Order 接口缺 `expire_at`** — MVP 可忽略
10. **closePosition 参数 id:number vs symbol:string** — 随 close_position handler 实现对齐
