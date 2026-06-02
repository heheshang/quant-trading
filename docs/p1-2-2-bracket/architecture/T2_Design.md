# T2+T3 — P1-2.2 Bracket 技术选型 & 设计

## T2 选型

### 2.1 OCO 触发模式 (P1-2.2 范围)

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **A. 延后 OCO (本任务)**: 母单 fill → 写 bracket_link → 前端轮询 → 调 create_oco | 解耦, hot path 轻 | 前端需 2 步 | ✅ (用户选 A) |
| B. 自动 OCO: 母单 fill → 立即 create_oco (需 position_id) | 用户体验好 | 需 P0 修 position 自动建 | ❌ (P0 范围外) |
| C. 写 trigger_orders 表但 oco_pair_id=NULL | 一站式 | 失去 OCO 联动 | ❌ (非真实 OCO) |

**结论**: 路径 A — backend 写 bracket_link + emit event, OCO 由前端/客户端后续调 `/trigger-orders/oco`.

### 2.2 bracket_link 存储

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **`bracket_links` 独立表** | 强 schema, 查询简单, 不污染 advanced_params | 多 1 表 | ✅ |
| **复用 `advanced_params` JSONB** | 0 新表 | 不便 join/索引, 与 P1-2.1 Iceberg 范式不一致 | ❌ |
| **WS 事件存 Redis** | 实时推送 | 不持久化, 客户端断线丢失 | ❌ |

**schema (核心字段)**:
```sql
CREATE TABLE bracket_links (
    id              UUID PRIMARY KEY,
    parent_order_id UUID NOT NULL REFERENCES orders(id),
    user_id         UUID NOT NULL,
    symbol          TEXT NOT NULL,
    sl_price        NUMERIC(20,8) NOT NULL,
    tp_price        NUMERIC(20,8) NOT NULL,
    side            TEXT NOT NULL,  -- parent 方向 (buy/sell)
    filled_quantity NUMERIC(20,8) NOT NULL,  -- 母单成交数
    oco_status      TEXT NOT NULL DEFAULT 'pending',  -- pending|linked|cancelled|failed
    sl_trigger_id   UUID,  -- 创建 OCO 后回填
    tp_trigger_id   UUID,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_bracket_links_user_pending ON bracket_links(user_id, oco_status) WHERE oco_status = 'pending';
```

### 2.3 fill hook 时机

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **flush_trades 末尾检测** | 复用 P1-2.1 hook, 100ms 延迟 | 已是 P1-2.1 模式 | ✅ |
| 新加 fill listener channel | 实时 | 新增 task | ❌ (overkill) |

**与 P1-2.1 共存**: flush_trades 末尾已有 iceberg hook, 加 `else if adv_type == "bracket" && filled_qty == quantity { ... }` — 链式 `if/else if`.

### 2.4 校验时机

- **路径 A 不创建 OCO, 仅校验**: `stop_loss_price` 与 `take_profit_price` 相对 `price` 的方向
  - buy: `stop_loss_price < price < take_profit_price`
  - sell: `take_profit_price < price < stop_loss_price`

## T3 设计

### 3.1 数据结构

#### 母单 advanced_params
```json
{
  "stop_loss_price": 49000.0,
  "take_profit_price": 52000.0,
  "oco_status": "pending"
}
```
> 注意: `oco_status` 是 metadata 摘要, 权威状态在 `bracket_links.oco_status` 表. 同步更新.

#### bracket_link row
```sql
(id, parent_order_id, user_id, symbol, sl_price, tp_price, side,
 filled_quantity, oco_status, sl_trigger_id, tp_trigger_id, created_at, updated_at)
```

### 3.2 业务流程

#### Create bracket
```
POST /api/v1/orders {order_type:"bracket", price, quantity, side,
                     stop_loss_price, take_profit_price}
  ↓
handler.create_order
  ├─ validate BracketParams::validate_new (stop_loss/take_profit 方向)
  ├─ Set order_model.advanced_type="bracket"
  ├─ Set order_model.advanced_params = {sl, tp, oco_status:"pending"}
  ├─ insert (走普通 limit 流程, 入 book)
  └─ return 201
```

#### Fill hook (flush_trades 末尾)
```
for each trade:
  update child order status
  ↓
  if child.advanced_type == "iceberg_child" { iceberg replenish }
  else if parent.advanced_type == "bracket" && child.filled_quantity == child.quantity {
    bracket::record_filled(parent_id, sl, tp, filled_qty, symbol, side, user_id)
      ├─ INSERT INTO bracket_links
      ├─ UPDATE orders.advanced_params.oco_status = "pending"
      ├─ increment BRACKET_PARENT_FILLED_TOTAL
      └─ WS broadcast: bracket_filled
  }
```

#### Frontend polling + OCO creation (v2 不在 P1-2.2 范围)
```
GET /api/v1/bracket-links/pending
  ↓
前端根据 strategy 信号调:
POST /api/v1/trigger-orders/oco
  body: { position_id (前端构造, P0 修), sl_price, tp_price, quantity, symbol, side }
  ↓
(P1-2.2 不实现, P0 position 自动建后再做)
```

### 3.3 不做的事 (P1-2.2 范围外)
- ❌ 自动调 create_oco (需 position_id, P0 范围)
- ❌ bracket 修改 (改 SL/TP)
- ❌ 多 OCO 链 (多层挂单)
- ❌ 联动 cancel 母单 → 取消 pending OCO (P1-2.2 v1 不实现)
- ❌ WebSocket 事件订阅 (复用已有 ws hub, 仅在 P1-2.4 UI 集成时细化)

### 3.4 文件清单
- `backend/src/models/bracket_params.rs` (新): BracketParams + validate_new + MockBuilder + 8 unit tests
- `backend/src/services/bracket.rs` (新): record_filled + list_pending + 8 unit tests
- `backend/src/handlers/bracket.rs` (新): GET /api/v1/bracket-links/pending
- `backend/src/handlers/order.rs` (改): create_order Bracket 分支
- `backend/src/services/matching_engine.rs` (改): flush_trades hook 加 bracket 检测
- `backend/src/main.rs` (改): router() 加 .route("/api/v1/bracket-links/pending")
- `backend/src/metrics.rs` (改): BRACKET_PARENT_FILLED_TOTAL counter
- `backend/migrations/20260602000001_bracket_links.sql` (新)
