# T2+T3 — 技术选型 & 设计文档

## T2 选型

### 2.1 子单执行机制

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **matching engine 现有 trade_sink channel → flush_trades hook** | 0 新组件，复用现有 batch flush | hook 时机 100ms 延迟（可接受）| ✅ |
| 新增独立 `iceberg_supervisor` 监听 channel | 时机更准（实时） | 新增后台服务 + 状态 | ❌（overkill） |
| 子单挂 book → matching engine 自动 match | 子单自己处理 | **不符合 Iceberg 语义**（子单不该被市场看到多次）| ❌ |

**结论**：复用 `flush_trades` 的 L755 update 后 hook 点。

### 2.2 子单存储

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **复用 `orders` 表，order_type=Limit + advanced_type="iceberg_child"** | 不加表 | advanced_params 字段含义随 type 变 | ✅ |
| 独立 `iceberg_children` 表 | 强 schema | 跨表 join 复杂，**违背 P1-2 MVP 决策** | ❌ |

### 2.3 拆单时机

| 方案 | 优 | 劣 | 选 |
|---|---|---|---|
| **创建母单时立即拆全部 N 个子单 + 只入第一个 book** | 简单 | 子单记录冗余（未入 book 的子单状态为 pending）| ✅ |
| 懒拆（每成交一片拆下一片）| DB 干净 | flush_trades 写状态复杂 | ❌ |
| 母单+全部子单一次性全部入 book | 错（违反 iceberg 语义）| 暴露总量 | ❌ |

**关键决策**：**只入第一个子单到 book**——子单成交 → flush_trades 检测到子单 filled → 创建并入 book 下一子单。

### 2.4 拆单数量与最后一片处理

**算法**：
```
total_slices = floor(total_quantity / visible_quantity)
last_slice_qty = total_quantity - visible_quantity * total_slices
if last_slice_qty > min_quantity:
    slices = total_slices + 1  (last slice has remainder)
else:
    slices = total_slices        (last slice == visible_quantity)
```

**示例**：
- total=10.0, visible=1.0 → 10 slices, each 1.0
- total=10.5, visible=1.0 → 11 slices, last=0.5
- total=10.0001, visible=1.0 → 10 slices (last .0001 absorbed by last)

## T3 设计

### 3.1 数据结构

#### 母单 advanced_params
```json
{
  "visible_quantity": 1.0,
  "total_quantity": 10.0,
  "filled_children": 0,
  "total_slices": 10,
  "slice_index_next": 0,
  "children_ids": ["uuid1", "uuid2", ...]
}
```

#### 子单 advanced_params
```json
{
  "parent_id": "uuid-parent",
  "slice_index": 0
}
```

### 3.2 业务流程

#### Create iceberg
```
POST /api/v1/orders {order_type:"iceberg", visible_quantity:1.0, ...}
  ↓
handler.create_order (existing)
  ├─ validate advanced_params (NEW)
  ├─ balance check (母单 total * price) (existing, but quant = total)
  ├─ risk check (existing)
  ├─ insert 母单 (OrderType::Iceberg, advanced_type="iceberg", advanced_params=...)
  │  (L567-569 SET advanced_type/params)
  ├─ IF OrderType::Iceberg:
  │   ├─ split: create N children (OrderType::Limit, advanced_type="iceberg_child")
  │   ├─ insert first child to matching engine book
  │   └─ return 母单 response (advanced_params 包含 children_ids)
  └─ ELSE: existing limit path
```

#### Child filled → replenish
```
matching engine sends TradeRecord { order_id: child_id }
  ↓
spawn_trade_writer receives in buffer
  ↓
flush_trades updates child.filled_quantity (existing L737-755)
  ↓
NEW: if child is iceberg_child:
  ├─ parent = SELECT * FROM orders WHERE id = advanced_params.parent_id
  ├─ parent.filled_quantity += child.filled_quantity
  ├─ if parent.filled_quantity >= parent.total_quantity:
  │   ├─ parent.status = Filled
  │   └─ DO NOT create next child
  ├─ else:
  │   ├─ next_slice_index = filled_children + 1
  │   ├─ if next_slice_index < total_slices:
  │   │   ├─ create new child (slice_index = next_slice_index)
  │   │   ├─ insert to matching engine book
  │   │   └─ metric: iceberg_child_orders_total{action="created"}.inc()
  │   └─ else (last slice filled):
  │       └─ parent.status = Filled
  └─ metric: iceberg_child_orders_total{action="filled"}.inc()
```

#### Cancel iceberg
```
POST /api/v1/orders/{parent_id}/cancel
  ↓
handler.cancel_order (existing)
  ├─ lock + check ownership (existing)
  ├─ NEW: if parent.advanced_type == "iceberg":
  │   ├─ SELECT all children WHERE advanced_params.parent_id = parent.id AND status=pending
  │   ├─ FOR each child:
  │   │   ├─ engine.remove_from_book(child.id)
  │   │   ├─ child.status = Cancelled
  │   │   └─ metric: iceberg_child_orders_total{action="cancelled"}.inc()
  │   └─ update children's DB records
  ├─ unfreeze_on_cancel (existing — 母单)
  ├─ engine.remove_from_book(parent.id) (existing — 母单不在 book，无 op)
  └─ update parent.status = Cancelled (existing)
```

### 3.3 关键设计决策 (D1-D8)

**D1 母单 unfreeze 一次**：子单 balance 不重复冻结/解冻（**关键避免双重解冻 bug**）

**D2 子单仅首片入 book**：其余片状态 pending 但不在 book——保证 market 看不到

**D3 flush_trades hook 加新职责**：检测子单 filled → 触发补单（**最小侵入式扩展**）

**D4 metric 埋点用 Counter**：iceberg_child_orders_total{action} Counter 3 个 label value

**D5 母单 status 由子单聚合驱动**：子单 filled → flush_trades update 母单 filled_quantity → 母单 status 由 `quantity` vs `filled_quantity` 决定

**D6 子单 cancel 通过 parent_id 关联**：不引入新外键，**复用 advanced_params**

**D7 子单 rate limit 算母单一次**：D2 决策（母单算 1 次配额，**不在子单每次算**）

**D8 拆单失败回滚**：拆单中任一 DB insert 失败 → 母单状态保留 pending，**手动 cancel**（**P1-2.1 不做事务回滚**，**P2+ 任务**）

### 3.4 单元测试策略

| 测试文件 | 数量 | 内容 |
|---|---|---|
| `services/iceberg.rs` (新) | 8 | validate_iceberg_params / split_into_children / append_next_child / cancel_iceberg_children / last_slice_replenishment / no_replenish_after_full / cancel_no_children / idempotent_cancel |
| `handlers/order.rs` (扩) | 4 | create_iceberg_success / create_iceberg_invalid_visible / create_iceberg_missing_params / cancel_iceberg_cascades_to_children |
| `db/order.rs` (扩) | 2 | OrderType::Iceberg serde roundtrip / parse_advanced_params helper |
| **合计** | **14** | |

### 3.5 影响面

| 模块 | 改动 |
|---|---|
| **新增** `services/iceberg.rs` | 4 个 pub fn + IcebergParams struct + 8 unit tests |
| `handlers/order.rs` create_order | +1 分支 (OrderType::Iceberg) 拦截 → 调 split_into_children |
| `handlers/order.rs` cancel_order | +1 分支 (advanced_type=="iceberg") 拦截 → 调 cancel_iceberg_children |
| `services/matching_engine.rs` flush_trades | +1 hook (子单 filled → 调 iceberg_replenish) |
| `services/metrics.rs` (P0-3 已建) | +1 Counter `ICEBERG_CHILD_ORDERS_TOTAL` |
| `models/iceberg_params.rs` (新) | IcebergParams struct (Serialize/Deserialize) |
| `lib.rs` (mod) | +`pub mod iceberg;` |

**新文件**：
- `services/iceberg.rs`
- `models/iceberg_params.rs`

**不修改 DB schema**（P1-2 MVP 已加 advanced_type + advanced_params）。
