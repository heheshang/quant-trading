# ADR-006: 撮合引擎设计

| 字段 | 值 |
|------|------|
| **ID** | ADR-006 |
| **状态** | 已批准 |
| **日期** | 2026-05-12 |
| **决策者** | 产品决策者 (ssk) |
| **影响范围** | 交易模块、回测引擎、委托系统、仿真交易 |

## 背景

PRD 要求系统支持模拟模式交易（不实际发往交易所）和回测撮合。目前系统没有本地撮合引擎处理订单簿匹配，导致：

1. **模拟模式**：委托创建后无法自动成交，用户无法体验完整的交易流程
2. **回测模式**：需要基于历史 K 线数据模拟成交，判断买卖点的触发

## 核心要求

- 模拟模式：使用本地 order book + 最新行情价格进行撮合
- 回测模式：使用 K 线 OHLC 进行撮合（开盘价成交/高低价突破判断）
- 实盘模式：直接透传至交易所 API（不经过本地撮合）
- 支持限价单（Limit）、市价单（Market）、止损单（Stop-Loss）
- 支持部分成交（Partial Fill）

## 技术方案

### 选项 A: 独立 matching_engine 模块 + 三模式调度 ✅ 选定

**架构设计：**

```
┌──────────────────────────────┐
│         Order Manager        │
│  (委托CRUD + 状态管理)        │
└──────────┬───────────────────┘
           │ 匹配指令
           ▼
┌──────────────────────────────┐
│      Matching Engine         │
│  ┌──────────────────────┐    │
│  │ Mode Dispatcher      │    │
│  │  ├─ SIMULATE → Local Order Book    │
│  │  ├─ BACKTEST → OHLC Matching       │
│  │  └─ LIVE    → Exchange API Pass-Thru │
│  └──────────────────────┘    │
└──────────────────────────────┘
```

### 模拟模式（SIMULATE）

```
Local Order Book（内存中的 Bids/Asks 红黑树）：
  Buy Side  (Bids): 价格从高到低排序
  Sell Side (Asks): 价格从低到高排序

撮合流程：
1. 新委托进入 → 检查价格是否匹配（限价单 vs order book 头寸）
2. 匹配规则：价格优先 > 时间优先
3. 成交：更新 order book 深度 + 生成 Fill 记录
4. 未成交部分：进入 order book 挂单
5. 新行情推送 → 触发挂单重检查（如止损单触发）
```

**数据结构：**

```rust
// 内存 order book
struct OrderBook {
    symbol: String,
    bids: BTreeMap<Reverse<Decimal>, Vec<Order>>,  // 买盘：价格降序
    asks: BTreeMap<Decimal, Vec<Order>>,             // 卖盘：价格升序
    last_price: Decimal,
    last_update: NaiveDateTime,
}

// 委托
struct Order {
    id: Uuid,
    user_id: Uuid,
    side: OrderSide,           // Buy | Sell
    order_type: OrderType,     // Limit | Market | StopLimit
    price: Option<Decimal>,    // None = 市价单
    quantity: Decimal,
    filled_quantity: Decimal,
    status: OrderStatus,       // Pending | PartialFilled | Filled | Cancelled
    created_at: NaiveDateTime,
}
```

### 回测模式（BACKTEST）

基于 K 线 OHLC 模拟成交，不维护完整 order book：

```rust
struct BacktestMatcher {
    // 逐 bar 模拟
    fn match_on_bar(&mut self, bar: &Kline) -> Vec<Fill> {
        let mut fills = Vec::new();
        
        for order in &mut self.open_orders {
            match order.order_type {
                // 市价单：按当前 bar 的开盘价（或 close 价）成交
                Market => {
                    fills.push( self.fill_order(order, bar.open) );
                },
                // 限价买单：price >= bar.low 则触发（有机会成交）
                Limit if order.side == Buy && order.price >= bar.low => {
                    let exec_price = min(bar.open, bar.close).clamp(order.price, bar.high);
                    fills.push( self.fill_order(order, exec_price) );
                },
                // 限价卖单：price <= bar.high 则触发
                Limit if order.side == Sell && order.price <= bar.high => {
                    let exec_price = max(bar.open, bar.close).clamp(bar.low, order.price);
                    fills.push( self.fill_order(order, exec_price) );
                },
                _ => {}
            }
        }
        fills
    }
}
```

### 实盘模式（LIVE）

简单透传至交易所 API，不做本地撮合：

```rust
Live mode flow:
1. 委托创建 → 调用交易所 REST API (POST /order)
2. 接收交易所 WS 成交推送 → 同步到本地 DB
3. 定时同步未成交订单状态
4. 不执行本地 order book 撮合
```

## 决策

采用 **独立 matching_engine 模块 + 三模式调度** 方案：

| 模式 | 撮合方式 | 数据源 | 用途 |
|------|---------|--------|------|
| SIMULATE | 内存 Order Book (BTreeMap) | 模拟行情/导入行情 | 仿真交易、策略验证 |
| BACKTEST | K 线 OHLC 匹配 | 历史 K 线 DB | 策略回测 |
| LIVE | 透传交易所 API | 实盘行情 | 真实交易 |

### MVP 范围

1. 仅实现 **SIMULATE** 模式的基本撮合（限价单 + 市价单）
2. 简单的 order book（仅维护 top 10 档位 depth，非全量 order book）
3. 回测模式直接使用 **ADR-004** 的设计，在引擎内嵌简易撮合
4. 托管模式使用 **IOC**（Immediate-or-Cancel）确保不产生未成交挂单

### 第二阶段

1. 全量 order book（BTreeMap 红黑树，支持 >100 档位）
2. 止损单 / Stop-Limit 触发
3. 冰山订单支持
4. 部分成交模拟

## 预期后果

**正面：**
- 模拟模式用户可体验完整交易流程（下单 → 撮合 → 成交 → 持仓）
- 回测模式可使用同一套撮合逻辑，结果一致
- 实盘模式无额外延迟（透传零开销）

**负面：**
- SIMULATE 模式需要额外的行情数据驱动才能撮合（需 Market Data Collector 提供模拟行情）
- 内存 order book 在多用户模拟模式下需要隔离（每个用户/策略独享 instance）

## 缓解措施

| 风险 | 缓解 |
|------|------|
| 模拟行情不可用 → 不能撮合 | 使用回放历史行情作为模拟行情源 |
| 多用户 order book 混乱 | 每个策略实例独立 matching_engine instance |
| 撮合性能瓶颈 | 单次撮合操作 < 1ms，tokio task 异步调度 |
