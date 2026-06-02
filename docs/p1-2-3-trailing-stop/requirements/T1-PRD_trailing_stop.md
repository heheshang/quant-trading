# T1-PRD — P1-2.3 Trailing Stop Order

> **任务 ID**: P1-2.3
> **前置**: P1-2.1 (Iceberg) + P1-2.2 (Bracket) + P1-F2 (SL/TP) + P1-F3 (trigger_orders)
> **范围**: Trailing stop 作为 entry limit order + 动态 SL,价格朝有利方向移动时 SL 上移（buy）/下移（sell）,触发后变 market order 平仓

## 业务场景

### S1: 多头 Trailing Stop
- 交易员持有多头头寸（已开仓或同时开仓），设置 `trailing_distance = 1%`（`0.01`）
- 当前 BTC 价格 $50,000，trailing stop 触发价 = 50,000 × (1 - 0.01) = $49,500
- 价格涨到 $51,000 → trigger 价 = 51,000 × 0.99 = $50,490 (跟随上移)
- 价格跌回 $50,490 → 触发，市价卖出平仓

### S2: 空头 Trailing Stop
- 空头头寸 + `trailing_distance = 0.5%`
- 开仓价 $50,000，trigger 价 = 50,000 × (1 + 0.005) = $50,250
- 价格跌到 $49,000 → trigger 价 = 49,000 × 1.005 = $49,245 (跟随下移)
- 价格涨回 $49,245 → 触发，市价买入平仓

### S3: Entry + Trailing（一体化）
- 创建 `order_type=trailing_stop`,`side=buy`,`price=50000`,`quantity=0.5`,`trailing_distance=0.01`
- 系统创建 entry limit order @ 50000 + 跟踪的动态 SL
- 一旦 entry fill,trailing stop 立即开始跟随

## 功能需求 (Gherkin)

### G1: 创建 trailing stop
```gherkin
Given 用户已登录且账户有 USDT 余额
When POST /api/v1/orders 带 trailing_distance=0.01 price=50000 quantity=0.5
Then 返回 201 + 母单 ID
And DB orders.advanced_type = "trailing_stop"
And orders.advanced_params 包含 peak_price (initial = price) + trailing_distance
```

### G2: 价格朝有利方向移动 → SL 上移
```gherkin
Given trailing stop 母单已存在 peak_price=50000 trailing_distance=0.01
And 当前 mark_price=51000
When 后台轮询 task 检测到 51000 > 50000
Then 更新 DB peak_price=51000
And 当前 trigger_price = 51000 * 0.99 = 50490
```

### G3: 价格触发 trailing stop
```gherkin
Given trailing stop 母单 peak_price=51000 trailing_distance=0.01
And current_price 跌至 50400
When 轮询 task 比较 current_price vs trigger_price (50490)
Then 触发,创建 market close order
And 母单 status → "filled"
And trigger_orders.status → "triggered"
```

### G4: 价格朝不利方向移动 → SL 不变
```gherkin
Given trailing stop 母单 peak_price=50000 trailing_distance=0.01
And current_price 跌至 49800
When 轮询 task 比较 current_price vs trigger_price (49500)
Then 不触发,SL 仍为 49500 (peak_price 不变)
```

### G5: 取消 trailing stop
```gherkin
Given trailing stop 母单 status=pending
When DELETE /api/v1/orders/{id}
Then 200 OK + 母单 status=cancelled
And poll task 跳过已 cancelled 订单
```

## 非功能需求

| ID | 需求 |
|---|---|
| NFR1 | 轮询频率: 2 秒 (user-configurable,默认 2s) |
| NFR2 | 单次轮询所有活跃 trailing stop 订单,避免 N+1 查询 (用 symbol 分组) |
| NFR3 | 触发延迟容忍: ≤ 轮询间隔 + 100ms (默认 2.1s) |
| NFR4 | 资源占用: 10 symbols × 0.5 req/s = 5 req/s exchange API |
| NFR5 | 母单 cancel 后立即从轮询集移除 (无 1 个完整轮询周期延迟) |
| NFR6 | 多 trailing stop 同时存在不互相阻塞 (单 task 处理全部) |
| NFR7 | trailing_distance 范围 (0, 0.1) 即 0% < x < 10% |

## 验收标准

1. ✅ POST /orders order_type=trailing_stop → 201,advanced_type=trailing_stop
2. ✅ 后台轮询 task 启动 (main.rs) 并处理现有订单
3. ✅ peak_price 单调跟随: buy 时只升不降, sell 时只降不升
4. ✅ 触发时创建 market order (平仓方向) 并更新母单 status
5. ✅ 取消 trailing stop 后 1 个轮询周期内不再被检查
6. ✅ 单元测试覆盖 peak_price 跟随逻辑的所有边界
7. ✅ 集成测试模拟价格曲线 → 验证 trigger 时机
8. ✅ 0 clippy warnings + 全部测试 pass

## 范围外 (Out of Scope)

- 多 exchange price feed (只用 Binance, OKX 后续)
- Trailing take profit (只做 trailing stop)
- 用户级 trailing distance override (用订单级)
- 历史回放 / 纸上回测 (live 模式 only)
