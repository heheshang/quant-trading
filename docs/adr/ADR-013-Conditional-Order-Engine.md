# ADR-013: 条件单引擎架构

**ID**: ADR-013
**日期**: 2026-05-21
**状态**: Accepted
**关联**: PRD-P1-F2, PRD-P1-F3, ADR-015

---

## 状态

Accepted — P1 功能，Phase 3 条件触发单核心实现

---

## 背景

P1-F3 要求实现条件触发单引擎，支持止损单、止盈单、OCO（二选一）和 TWAP 时间加权平均单。依赖 P1-F2 实盘止盈止损的基础架构。

---

## 技术方案

### 条件单类型

| 类型 | 描述 | 触发逻辑 |
|------|------|----------|
| **止损单 (Stop-Loss)** | 价格跌破触发价激活市价/限价单 | 多头持仓：价格 ≤ 触发价；空头持仓：价格 ≥ 触发价 |
| **止盈单 (Take-Profit)** | 价格涨超触发价激活市价/限价单 | 多头持仓：价格 ≥ 触发价；空头持仓：价格 ≤ 触发价 |
| **OCO (One-Cancels-Other)** | 同时设置止盈和止损，触发一个取消另一个 | OCO 订单对：任一触发时另一单自动取消 |
| **TWAP** | 时间加权平均价格，在指定时间内分批成交 | 按时间间隔分片执行，不依赖价格触发 |

### 触发执行流程

```
行情更新 (WsHub/TickerEvent)
    ↓
TriggerOrderMonitor (background task)
    ↓ 订阅行情，调用 check_trigger(symbol, current_price)
    ↓
检查触发条件 (价格条件 ≥/≤ 触发价)
    ↓
触发订单 → 创建市场委托 (OrderType::Market)
    ↓
更新条件单状态为 Triggered
    ↓
发送执行指令至交易执行层
```

### 数据模型

**trigger_orders 表**

```sql
CREATE TABLE IF NOT EXISTS trigger_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    position_id UUID REFERENCES positions(id),  -- 止损止盈关联持仓
    symbol VARCHAR(20) NOT NULL,
    trigger_type VARCHAR(20) NOT NULL,  -- 'stop_loss' | 'take_profit' | 'oco' | 'twap'
    status VARCHAR(20) NOT NULL,         -- 'pending' | 'triggered' | 'cancelled' | 'expired' | 'failed'
    trigger_direction VARCHAR(10) NOT NULL,  -- 'up' | 'down'
    trigger_price DECIMAL(20, 8) NOT NULL,
    trigger_price_upper DECIMAL(20, 8),       -- OCO 止盈价
    trigger_price_lower DECIMAL(20, 8),       -- OCO 止损价
    base_price DECIMAL(20, 8),                -- 挂单基础价
    side VARCHAR(10) NOT NULL,               -- 'buy' | 'sell'
    quantity DECIMAL(20, 8) NOT NULL,
    filled_quantity DECIMAL(20, 8) NOT NULL DEFAULT 0,
    avg_fill_price DECIMAL(20, 8),
    -- TWAP 字段
    twap_slice_quantity DECIMAL(20, 8) NOT NULL DEFAULT 0,
    twap_interval_secs INT NOT NULL DEFAULT 60,
    twap_start_time TIMESTAMPTZ,
    twap_end_time TIMESTAMPTZ,
    twap_executed_slices INT NOT NULL DEFAULT 0,
    twap_max_slices INT NOT NULL DEFAULT 0,
    -- OCO 关联
    oco_pair_id UUID REFERENCES trigger_orders(id),
    triggered_order_id UUID,                 -- 触发后创建的市价单ID
    trigger_reason TEXT,
    triggered_at TIMESTAMPTZ,
    expire_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    cancelled_at TIMESTAMPTZ
);
CREATE INDEX idx_trigger_user ON trigger_orders(user_id);
CREATE INDEX idx_trigger_symbol ON trigger_orders(symbol);
CREATE INDEX idx_trigger_status ON trigger_orders(status);
CREATE INDEX idx_trigger_position ON trigger_orders(position_id);
```

**TriggerType 枚举**

```rust
pub enum TriggerType {
    StopLoss,   // 止损单
    TakeProfit, // 止盈单
    Oco,        // OCO 单
    Twap,       // TWAP 单
}
```

**TriggerStatus 枚举**

```rust
pub enum TriggerStatus {
    Pending,   // 待触发
    Triggered, // 已触发
    Cancelled, // 已取消
    Expired,   // 已过期
    Failed,    // 失败
}
```

**TriggerDirection 枚举**

```rust
pub enum TriggerDirection {
    Up,   // 价格 ≥ 触发价触发
    Down, // 价格 ≤ 触发价触发
}
```

### API 端点

| 方法 | 路径 | 描述 |
|------|------|------|
| `POST` | `/api/v1/trigger-orders/stop-loss` | 创建止损单 |
| `POST` | `/api/v1/trigger-orders/take-profit` | 创建止盈单 |
| `POST` | `/api/v1/trigger-orders/oco` | 创建 OCO 单（返回止损+止盈对） |
| `POST` | `/api/v1/trigger-orders/twap` | 创建 TWAP 单 |
| `GET` | `/api/v1/trigger-orders` | 查询条件单列表（支持 status/symbol 过滤） |
| `GET` | `/api/v1/trigger-orders/{id}` | 查询单个条件单详情 |
| `DELETE` | `/api/v1/trigger-orders/{id}` | 取消条件单 |

**请求示例 - 创建止损单**

```json
POST /api/v1/trigger-orders/stop-loss
{
    "position_id": "uuid",
    "symbol": "BTCUSDT",
    "trigger_price": 95000.0,
    "base_price": null,
    "quantity": 0.1
}
```

**请求示例 - 创建 OCO 单**

```json
POST /api/v1/trigger-orders/oco
{
    "position_id": "uuid",
    "symbol": "BTCUSDT",
    "stop_loss_price": 90000.0,
    "take_profit_price": 110000.0,
    "base_price": null,
    "quantity": 0.1
}
```

**请求示例 - 创建 TWAP 单**

```json
POST /api/v1/trigger-orders/twap
{
    "symbol": "BTCUSDT",
    "side": "buy",
    "quantity": 1.0,
    "slice_quantity": 0.1,
    "interval_secs": 60,
    "duration_secs": 3600
}
```

### 实现位置

| 模块 | 文件 | 职责 |
|------|------|------|
| Service | `backend/src/services/trigger_order.rs` | 条件单核心业务逻辑 (781行) |
| Handler | `backend/src/handlers/trigger_order.rs` | HTTP API 接口层 (312行) |
| DB Entity | `backend/src/db/trigger_order.rs` | SeaORM 数据模型和枚举定义 |

---

## 核心算法

### 止损/止盈触发检测

```rust
pub async fn check_trigger(&self, symbol: &str, current_price: f64) -> Result<(), AppError> {
    let pending_orders = TriggerOrderEntity::find()
        .filter(Column::Symbol.eq(symbol))
        .filter(Column::Status.eq(TriggerStatus::Pending))
        .all(self.db.as_ref())
        .await?;

    for order in pending_orders {
        let should_trigger = match order.trigger_direction {
            TriggerDirection::Up => current_price >= order.trigger_price,
            TriggerDirection::Down => current_price <= order.trigger_price,
        };
        if should_trigger {
            self.trigger_order(&order, current_price).await?;
        }
    }
    Ok(())
}
```

### OCO 互取消机制

触发 OCO 单时，自动取消其关联的另一单：

```rust
async fn trigger_order(&self, order: &TriggerOrder, current_price: f64) -> Result<(), AppError> {
    // 如果是 OCO 单，先取消关联的另一单
    if let Some(oco_pair_id) = order.oco_pair_id {
        self.cancel_trigger_order(oco_pair_id, "OCO pair triggered").await?;
    }
    // 创建市场委托...
}
```

### TWAP 分片执行

```rust
pub async fn process_twap_slice(&self, order_id: Uuid, current_price: f64) -> Result<bool, AppError> {
    // 每次执行一个 slice_quantity
    let slice_qty = remaining_qty.min(order.twap_slice_quantity);
    // 创建市价单
    // 更新 twap_executed_slices 和 filled_quantity
    // 检查是否完成（达到最大切片数或完成全部数量）
}
```

---

## 集成点

| 模块 | 集成方式 |
|------|----------|
| WsHub | 行情更新时触发 TriggerOrderMonitor 检查 |
| Order Service | 触发后创建市场委托 (OrderType::Market) |
| Position Service | 止损止盈关联持仓，验证持仓存在性 |
| OCO Pair | OCO 两个订单互相关联，触发一个取消另一个 |

---

## 交付物

1. `backend/src/services/trigger_order.rs` — 条件单核心服务 (781行)
2. `backend/src/handlers/trigger_order.rs` — HTTP API 处理器 (312行)
3. `backend/src/db/trigger_order.rs` — SeaORM 数据模型
4. 数据库迁移 `migrations/..._create_trigger_orders_table.sql`
5. 单元测试覆盖止损、止盈、OCO、TWAP 创建和触发逻辑
