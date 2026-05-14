# ADR-010: 订单管理模块架构设计

| 字段 | 值 |
|------|------|
| **ID** | ADR-010 |
| **状态** | 已批准 |
| **日期** | 2026-05-14 |
| **决策者** | Tech Lead |
| **影响范围** | 交易执行模块、撮合引擎、风控、持仓、前端交易页面 |
| **关联** | ADR-006 (撮合引擎), ADR-001 (Rust+Axum), PRD-trade-execution.md |

## 背景

PRD-trade-execution 定义了 US-TE-01~10 十个用户故事，涵盖手动下单、模拟撮合、委托管理、成交记录、持仓管理、风控拦截、策略信号自动下单、交易页面布局、账户余额、交易通知。现有代码库状态：

- **backend/src/handlers/order.rs** — 已实现 create_order / list_orders / get_order / cancel_order / cancel_all_orders / list_trades / list_positions / get_account / list_symbols，共 9 个 handler
- **backend/src/services/matching_engine.rs** — 已实现 OrderBook (BTreeMap) + match_market + on_depth_update + rebuild_order_book
- **backend/src/db/order.rs** — 已实现 orders / trades / positions / paper_accounts / symbol_configs 五个 SeaORM Entity + 状态机 can_transition_to
- **frontend/src/api/order.ts** — 已实现 createOrder / getOrders / getOrder / cancelOrder / cancelAllOrders / getAccount / getSymbols / getPositions / closePosition
- **frontend/src/types/order.ts** — 已实现 Order / CreateOrderRequest / OrderQueryParams / Position / PaperAccount / SymbolConfig 等类型

**缺失项：**
- close_position handler (后端只有 list_positions，无 close_position 路由)
- risk_manager.rs 服务（PRD P1，handler 未集成风控前置）
- TradeWsHub（交易 WebSocket 推送未实现）
- 策略信号接收（Redis PubSub 订阅未实现）
- 止损单 (stop / stop_limit) OrderType 及触发逻辑
- PRD 定义的 /api/v1/trade/* 路由前缀 vs 现有 /api/v1/orders 路由

## 决策

### D1: MatchingEngine 架构（沿用 ADR-006，补充实现细节）

**选型：** 内存撮合 + 异步 DB 写入

```
CreateOrderRequest
  → RiskManager.check() (D8 前置)
  → DB.insert(order) 先写 PG 保证不丢
  → MatchingEngine.submit():
      市价单 → match_market() 逐档撮合
      限价单 → insert_limit_order() 挂入 OrderBook
  → MatchResult → DB.update(order) + DB.insert(trades)
  → PubSub("trade:order:{uid}") → WS Hub → 客户端
```

**关键实现规则：**

| 规则 | 说明 |
|------|------|
| 先写 DB 再撮合 | 委托必须先持久化，即使撮合引擎宕机也可恢复 |
| 成交异步批量写入 | TradeRecord 通过 mpsc channel 批量写入 trades 表 |
| OrderBook 重启恢复 | rebuild_order_book() 从 PG 加载 status IN (pending, partial_filled) 的限价单 |
| 深度驱动限价撮合 | on_depth_update() 由行情推送触发，价格优先时间优先 |

**与现有代码的对齐：** matching_engine.rs 已实现上述逻辑，无需修改架构。

### D2: 订单状态机 + PG 行锁

**状态机（已有实现，确认对齐 PRD）：**

```
pending ──→ partial_filled ──→ filled
  │              │
  │              └──→ cancelled (保留已成交)
  ├──→ cancelled
  ├──→ expired
  └──→ rejected (风控拒绝)
```

**PG 行锁策略：**

撤单和部分成交更新必须使用 `SELECT ... FOR UPDATE` 行锁，防止并发状态冲突：

```rust
// cancel_order 和 on_depth_update 撮合后更新 order 时
let order = Entity::find_by_id(order_id)
    .lock(LockType::Update)         // SELECT FOR UPDATE
    .one(&*db)
    .await?;

// 状态转换校验
if !order.status.can_transition_to(&new_status) {
    return Err(AppError::Conflict("状态转换不合法"));
}
```

**现有代码缺口：** cancel_order() 未使用行锁，直接 find_by_id + update，高并发下可能丢失状态更新。需补充 `LockType::Update`。

### D3: 保证金三阶段冻结

限价买入和市价买入时，需冻结对应金额（保证金），三阶段生命周期：

| 阶段 | 操作 | 说明 |
|------|------|------|
| 1. 下单时 | balance -= notional, frozen += notional | 冻结等额保证金 |
| 2. 成交时 | frozen -= notional, positions_value += filled_value | 解冻转入持仓 |
| 3. 撤单时 | frozen -= unfrozen_amount, balance += unfrozen_amount | 解冻归还余额 |

**实现位置：** 在 TradeService 或 handler 中调用 `paper_accounts::ActiveModel` 更新。现有 create_order 缺少 D3 逻辑。

**卖出冻结：** 卖出时冻结对应持仓的 available_quantity（非金额），防止超额卖出。

```rust
// 下单时冻结
if side == Buy {
    let notional = price * quantity;
    account.balance -= notional;
    account.frozen_balance += notional;
}
if side == Sell {
    position.available_quantity -= quantity;  // 冻结可用持仓
}
```

### D4: 深度变更事件驱动撮合

沿用 ADR-006 的 on_depth_update() 设计，行情推送触发限价单撮合：

```
MarketDataCollector → Redis PubSub("depth:{symbol}")
  → MatchingEngine.on_depth_update(symbol, depth)
    → 遍历 OrderBook 买盘 (price >= best_ask)
    → 遍历 OrderBook 卖盘 (price <= best_bid)
    → 生成 TradeRecord → trade_sink → 批量写入 trades
    → 更新 order status → PubSub → WS Hub
```

**现有实现问题：** on_depth_update() 只通过 trade_sink 发送 TradeRecord，但**未同步更新 orders 表中的 status / filled_quantity**。需补充撮合后的 order 状态回写逻辑。

### D5: TradeWsHub 独立设计

交易 WebSocket 与行情 WebSocket 分离，独立 Hub 实例：

```rust
pub struct TradeWsHub {
    channels: DashMap<String, HashSet<String>>,     // channel → session_ids
    sessions: DashMap<String, mpsc::Sender<String>>, // session_id → sender
    redis_sub: redis::aio::PubSub,                   // 订阅 trade:order, trade:fill, trade:position
}
```

**频道设计（PRD §5.4）：**

| PubSub 频道 | 触发时机 | WS 推送消息类型 |
|-------------|---------|----------------|
| `trade:order:{user_id}` | 委托状态变更 | `order_update` |
| `trade:fill:{user_id}` | 成交通知 | `fill` |
| `trade:position:{user_id}` | 持仓变更 | `position_update` |
| `trade:risk:{user_id}` | 风控预警 | `risk_alert` |

**MVP 范围：** 复用现有 ws.rs 的 WebSocket 升级逻辑，新增 TradeWsHub 作为独立广播器。

### D6: API 路由设计

**PRD 定义 vs 现有实现的偏移：**

| PRD 路径 | 现有路径 | 差异 | 决策 |
|---------|---------|------|------|
| `/api/v1/trade/orders` | `/api/v1/orders` | 缺少 `trade/` 前缀 | **MVP 保持 `/api/v1/orders`**，Phase 2 迁移到 `/trade/` 前缀 |
| `/api/v1/trade/orders/{id}` | `/api/v1/orders/{id}` | 同上 | 同上 |
| `DELETE /api/v1/trade/orders/{id}` | `POST /api/v1/orders/{id}/cancel` | HTTP 方法+路径不同 | **保持 POST cancel**，REST 语义更清晰（撤单是动作不是删除） |
| `/api/v1/trade/fills` | `/api/v1/trades` | fills vs trades | **保持 trades**，与 DB 表名一致 |
| `/api/v1/trade/positions/{symbol}/close` | `/api/v1/positions/{id}/close` | symbol vs id | **改用 symbol** 路径参数，与 PRD 对齐 |
| `/api/v1/trade/account` | `/api/v1/account` | 缺少 `trade/` 前缀 | MVP 保持 `/api/v1/account` |
| `/api/v1/trade/ws` | (未实现) | — | 新增 `/api/v1/trade/ws` |
| `/api/v1/trade/risk/rules` | (未实现) | P1 | Phase 2 |

**新增 handler（现有 order.rs 缺失）：**

| Handler | 路由 | 优先级 |
|---------|------|--------|
| `close_position` | `POST /api/v1/positions/:symbol/close` | P0 |
| `init_account` | `POST /api/v1/account/init` | P1 |
| `trade_ws_handler` | `GET /api/v1/trade/ws` | P0 |

### D7: 加权平均均价

持仓均价计算公式（已实现于 matching_engine.rs）：

- **同向加仓：** `new_avg = (old_avg * old_qty + fill_price * fill_qty) / (old_qty + fill_qty)`
- **反向平仓：** 均价不变，盈亏计入 realized_pnl
- **反向翻转：** 新方向，新均价 = 成交价

### D8: 风控前置

下单前必须经过 RiskManager 检查，拦截链：

```
CreateOrderRequest
  → 1. 参数校验 (symbol, side, type, price, quantity)
  → 2. 交易对校验 (symbol_configs.enabled = true)
  → 3. 余额检查 (balance >= notional for buy)
  → 4. 最大持仓 (current_position + new_qty <= max_position)
  → 5. 每日亏损 (today_realized_loss < max_daily_loss)
  → 6. 单笔亏损预警 (estimated_loss > max_single_loss → Warn)
  → Pass → 进入撮合
  → Warn → 返回警告，前端二次确认
  → Block → 返回 40003 错误
```

**MVP 实现（P0 基础风控）：**
- 参数校验 ✅ 已有
- 余额检查 — 需新增（create_order 中缺少）
- 最大持仓 — P1 阶段

**P1 风控服务：**
- `services/risk_manager.rs` — 独立风控服务
- `risk_rules` 表 — 可配置规则
- admin 管理页面

### D9: 止损单扩展

现有 OrderType 仅支持 `limit` / `market`，PRD 要求支持 `stop` / `stop_limit`：

```rust
#[derive(...)]
pub enum OrderType {
    #[sea_orm(string_value = "limit")]
    Limit,
    #[sea_orm(string_value = "market")]
    Market,
    #[sea_orm(string_value = "stop")]
    Stop,           // 市价止损：触发价 → 市价单
    #[sea_orm(string_value = "stop_limit")]
    StopLimit,      // 限价止损：触发价 → 限价单
}
```

**新增字段：**
- `stop_price: Option<f64>` — 触发价
- orders 表需 ALTER ADD stop_price 列

**触发逻辑：**
1. 止损单创建时 status=pending，不进入 OrderBook
2. 行情推送 on_depth_update() 中检查止损触发条件
3. 触发后转换为市价单/限价单，进入撮合

**MVP 决策：** 止损单为 P1，MVP 阶段 OrderType 保持 limit+market，handler 中对 stop 类型返回 40001 参数错误。

### D10: 策略信号自动下单

```
Strategy Engine → Redis PubSub("strategy:signal:{strategy_id}")
  → TradeService.handle_strategy_signal(signal)
    → RiskManager.check()
    → create_order(strategy_id = signal.strategy_id)
    → WS 推送通知用户
```

**MVP 决策：** P1，MVP 不实现 Redis PubSub 订阅，但 orders.strategy_id 字段已预留。

### D11: 错误码体系

沿用 PRD §6.4 定义的 40xxx 错误码，现有 AppError 需补充：

| 错误码 | 含义 | 现有映射 |
|--------|------|---------|
| 40001 | 参数错误 | AppError::BadRequest |
| 40002 | 余额不足 | 需新增 AppError::InsufficientBalance |
| 40003 | 风控拒绝 | 需新增 AppError::RiskRejected |
| 40004 | 交易对不可交易 | 需新增 |
| 40401 | 委托不存在 | AppError::NotFound |
| 40402 | 委托不可撤销 | AppError::Conflict |
| 40403 | 持仓不存在 | AppError::NotFound |
| 40005 | 持仓不足 | 需新增 |
| 42901 | 下单频率限制 | 需新增 AppError::RateLimited |
| 50301 | 撮合引擎不可用 | 需新增 AppError::ServiceUnavailable |

## 预期后果

**正面：**
- 现有 handler + matching_engine + db model 覆盖了 P0 功能的 ~70%
- 状态机 can_transition_to 已实现，PG 行锁只需补充 LockType::Update
- 风控前置作为独立服务，与交易逻辑解耦，P1 迭代无侵入

**负面 / 风险：**

| 风险 | 影响 | 缓解 |
|------|------|------|
| on_depth_update 未回写 order 状态 | 限价单撮合后前端看不到状态变更 | 补充撮合后 DB 同步更新 |
| create_order 缺 D3 保证金冻结 | 余额可能被超额使用 | MVP 阶段补充冻结逻辑 |
| cancel_order 无行锁 | 并发撤单+成交可能状态冲突 | 补充 SELECT FOR UPDATE |
| API 路径与 PRD 不一致 (缺少 trade/ 前缀) | 前端契约对齐偏差 | MVP 保持现状，文档标注偏移 |
| 止损单 MVP 不支持 | PRD US-TE-01 要求止损 | P1 迭代 |

## 实施优先级

| 优先级 | 决策项 | 工作量 |
|--------|--------|--------|
| P0-Must | D2 行锁补充 | 0.5d |
| P0-Must | D3 保证金冻结 | 1d |
| P0-Must | D4 撮合后 order 状态回写 | 1d |
| P0-Must | D6 close_position handler | 0.5d |
| P0-Must | D5 TradeWsHub 基础实现 | 2d |
| P1 | D8 风控服务 | 2d |
| P1 | D9 止损单 | 1d |
| P1 | D10 策略信号 | 1d |
| P1 | D6 API 路径迁移 (trade/ 前缀) | 0.5d |
| P1 | D11 错误码体系完善 | 0.5d |

## 关联决策

- ADR-006 (撮合引擎三模式设计)
- ADR-001 (Rust + Axum 技术栈)
- ADR-002 (WebSocket 实时推送)
- ADR-003 (策略引擎沙箱)
