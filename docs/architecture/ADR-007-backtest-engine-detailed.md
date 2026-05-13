# ADR-007: 回测引擎详细设计 — 信号生成、撮合逻辑与绩效计算

| 字段 | 值 |
|------|------|
| **ID** | ADR-007 |
| **状态** | 已批准 |
| **日期** | 2026-05-13 |
| **决策者** | Tech Lead |
| **影响范围** | 回测引擎、StrategyTemplate trait、K线数据模型、绩效计算、API设计 |
| **基于** | ADR-004（全内存向量化计算）、PRD-backtest-engine.md |
| **替代** | 无 |

## 背景

ADR-004 已完成顶层架构决策——采用"全内存向量化计算"方案。本 ADR 进一步细化回测引擎的详细设计，涵盖：

- K线数据加载与内存模型
- 策略信号生成接口（`generate_signal` 方法定义）
- 回测引擎内部状态机（账户、持仓、撮合、风控）
- 绩效指标向量化计算
- 线程模型与进度/取消机制
- 分页采样策略
- 数据量估算与性能保障

## 决策

### 1. K线数据内存模型

**决策：** 使用 `Vec<Kline>`  一次性加载，Kline 结构使用 f64 避免多次类型转换。

```rust
#[derive(Debug, Clone, Copy)]
pub struct Kline {
    pub open_time: i64,            // 毫秒时间戳
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}
```

**理由：**
- 回测引擎内部全部使用 f64 计算，避免 Decimal ↔ f64 来回转换
- `Kline` 实现 `Copy` 减少所有权开销
- 1年1h数据（8760 bars）仅 ~0.4MB，5年1h（43800 bars）~2MB，全内存无压力

**查询约束：**
- `WHERE symbol = $1 AND interval = $2 AND open_time >= $3 AND open_time <= $4 ORDER BY open_time ASC`
- 使用 `idx_kline_lookup` 索引（symbol, interval, open_time DESC）
- 查询结果直接映射到 Vec<Kline>，通过 SeaORM 的 `stream` 或 `all` 方法

### 2. Signal 枚举定义

**决策：** 增强 PRD 的简单 Buy/Sell/Hold，支持带数量的方向信号。

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal {
    /// 买入（做多），quantity_pct 为资金使用比例 [0, 1]
    Buy { quantity_pct: f64 },
    /// 卖出（做空）
    Sell { quantity_pct: f64 },
    /// 平掉全部持仓
    CloseAll,
    /// 不操作
    Hold,
}
```

**理由：**
- PRD 的纯 Buy/Sell 无法表达仓位管理（如只用 50% 资金开仓）
- `quantity_pct` 允许策略表达"金字塔加仓"、"分批减仓"等复杂逻辑
- 默认按全部资金/仓位执行（`quantity_pct = 1.0`）时行为等价于 PRD 的 Buy/Sell

### 3. StrategyTemplate trait 扩展

**决策：** 在现有 trait 上新增 `generate_signal` 方法，新增 `generate_signals` 批量计算默认实现。

```rust
pub trait StrategyTemplate: Send + Sync {
    // === 现有方法 ===
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> &str;
    fn default_parameters(&self) -> Value;
    fn parameter_schema(&self) -> Vec<ParameterDef>;
    fn validate(&self, params: &Value) -> Result<(), String>;

    // === 新增方法 ===
    /// 对指定 K 线索引生成交易信号
    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal;
}
```

**设计原则：**
- `generate_signal` 每次调用只传入**当前视野**的 K 线切片（`&[Kline]`）和当前索引
- 从 `current_idx` 往前看 `params` 中定义的周期长度
- 不要求实现者维护内部状态——模板是无状态的，所有状态来自 `klines[..=current_idx]`
- 这保证了回测的**确定性**：相同参数 + 相同 K 线 → 相同结果

**对 10 个内置模板的影响：**
- 每个模板需实现 `generate_signal`，复用现有的指标计算逻辑
- 策略参数（如 fast_period/slow_period）从 `params` 读取
- 由于模板目前是无状态 struct，新增方法无需修改实例生命周期

**P0 内置模板的信号逻辑：**

| 模板 | 信号逻辑 | 
|------|---------|
| MA Crossover | fast_ma > slow_ma → Buy, < → Sell |
| Triple MA | short > medium > long → Buy, reverse → Sell |
| MACD | MACD 线上穿 Signal → Buy, 下穿 → Sell |
| Bollinger Bands | close < lower → Buy, close > upper → Sell |
| RSI | RSI < oversold → Buy, RSI > overbought → Sell |
| Keltner | 突破上轨 → Buy, 跌破下轨 → Sell |
| ATR Stop | ATR 扩/缩结合价格行为（P1） |
| Mean Reversion | close < mean - entry_std * stddev → Buy |
| Ichimoku | price > cloud + TK cross → Buy |
| Double Bollinger | 内外带配合信号 |

### 4. BacktestEngine 内部结构

**决策：** 将回测引擎封装为单一结构体，暴露 `run()` 方法。

```rust
pub struct BacktestEngine {
    config: BacktestConfig,
    klines: Vec<Kline>,
    strategy: Box<dyn StrategyTemplate>,
    strategy_params: Value,
    // 运行时状态
    account: Account,
    open_position: Option<Position>,
    trades: Vec<Trade>,
    equity_points: Vec<EquityPoint>,
    // 控制
    progress: Arc<AtomicU32>,
    cancel_token: CancellationToken,
}
```

**Account 状态机：**

```
                 +---> 已开多仓 --+
                 |               |
  空闲 ------> 下单 ----------> 信号平仓/止损/止盈
                 |               |
                 +---> 已开空仓 --+
```

```rust
struct Account {
    initial_capital: f64,
    cash: f64,            // 可用资金
    equity: f64,          // 总权益 = cash + position_value
}

struct Position {
    direction: Direction,  // Long | Short
    quantity: f64,
    entry_price: f64,
    entry_time: i64,
    stop_loss: Option<f64>,
    take_profit: Option<f64>,
    fee_paid: f64,
    slippage_paid: f64,
}
```

### 5. 撮合逻辑

**决策：** 市价单按 close 价成交 + 滑点 + 手续费。

**每 bar 处理流程：**
```
1. 策略调用 generate_signal(klines, i, params)
2. 如果 signal == Hold → 跳过
3. 如果 signal == CloseAll → 以当前 bar 的 close 价平仓
4. 如果 signal == Buy/Sell:
   a. 检查是否有反向持仓 → 先平仓再开仓
   b. 计算成交价 = close * (1 ± slippage_rate)
   c. 计算手续费 = 成交金额 * fee_rate
   d. 判断资金是否足够 → 不足则按剩余资金开仓
   e. 更新 account 和 position
5. 如果已有持仓 → 检查止损/止盈条件
6. 记录 equity_point
```

**止损/止盈实现（P1 可选）：**
- 止损：持仓期间，如果某个 bar 的 low（多头）低于止损价，触发止损平仓
- 止盈：类似逻辑
- 通过 `Position.stop_loss` / `Position.take_profit` 字段配置

**关键精度约束：**
- 所有价格计算使用 f64
- 最终结果归一化保留 8 位（使用 format/decimal round）
- 夏普比率计算使用无风险利率 r_f = 0（简化模型）

### 6. 绩效指标向量化计算

**决策：** 回测完成后一次性批量计算所有指标。

```
输入: trades: Vec<Trade>, equity_points: Vec<EquityPoint>
输出: BacktestMetrics
```

| 指标 | 计算公式 | 前提条件 |
|------|---------|---------|
| 总收益率 | `(final_equity - initial_capital) / initial_capital * 100` | 始终存在 |
| 年化收益率 | `(1 + total_return)^(365/days) - 1` | days > 0 |
| 最大回撤 | `min(equity_i / peak_before_i - 1) * 100` | equity_points 不为空 |
| 夏普比率 | `(mean_daily_return - r_f) / std_daily_return * sqrt(365)` | trade_days > 0 |
| 索提诺比率 | `(mean_daily_return - r_f) / downside_std * sqrt(365)` | trade_days > 0 |
| 卡尔玛比率 | `annualized_return / |max_drawdown|` | max_drawdown != 0 |
| 胜率 | `win_count / total_trades * 100` | total_trades > 0 |
| 盈亏比 | `avg_win / |avg_loss|` 或 `profit_factor` | 有亏损交易 |
| 净利润因子 | `total_profit / total_loss` | total_loss > 0 |
| 平均盈利 | `sum(win_pnls) / win_count` | win_count > 0 |
| 平均亏损 | `sum(loss_pnls) / loss_count` | loss_count > 0 |
| 交易次数 | `trades.len()` | 始终存在 |

**算法复杂度：** O(n) 一次遍历 equity_points 和 trades 即可计算所有指标，n 为 equity_points 长度（等于 bars 数量）。

### 7. 线程模型与并发控制

**决策：** 采纳 ADR-004 的 tokio blocking task + Semaphore + CancellationToken 方案。

```
┌─────────────┐     ┌──────────────┐     ┌──────────────┐
│  POST /run   │────→│  Semaphore    │────→│  spawn_block │
│  handler     │     │  (max 5)      │     │  ing 引擎    │
└─────────────┘     └──────────────┘     └──────┬───────┘
                                                │
                  ┌──────────────────────────────┤
                  │                              │
                  ▼                              ▼
       ┌──────────────────┐        ┌──────────────────┐
       │  进度 AtomicU32   │        │  CancellationToken│
       │  (WS每10%推送)    │        │  (POST /cancel)   │
       └──────────────────┘        └──────────────────┘
```

**关键细节：**
- `Semaphore::new(5)` 作为全局实例，guard 在 handler 作用域获取
- 引擎函数需要周期性检查 `cancel_token.is_cancelled()`
  - 检查频率：每 100 bars（约 0.1s/check，远小于总计算时间）
  - 被取消后函数立即返回 `Err(Cancelled)`
- WS 推送：在 engine 外部的 tokio task 中轮询 `Arc<AtomicU32>`
  - 轮询间隔：100ms
  - 仅当值变化 ≥ 10 时推送
  - 推送通过 WS hub 的 channel 发送

### 8. equity_curve 采样策略

**决策：** 全量存储但前端通过采样渲染。

- DB 存储：全量 equity_curve 数组（8760 ~ 43800 点，JSONB 体积约 0.5~2MB）
- 前端加载：全量加载后，ECharts 内部处理大面积渲染
- 如果未来数据量过大（>100K 点），可在 API 层增加 `sample_factor` 参数
- 采样算法：最大三角形桶 (LTTB) 或 均匀采样每 N 点取 1

### 9. 数据量估算

| 维度 | 1年1h | 5年1h | 1年1m | 5年1m |
|------|-------|-------|-------|-------|
| K线条数 | 8,760 | 43,800 | 525,600 | 2,628,000 |
| K线内存 (f64) | ~0.4MB | ~2MB | ~25MB | ~125MB |
| 回测耗时估计 | < 0.5s | < 3s | < 30s | < 150s |
| equity_curve JSONB | ~0.5MB | ~2.5MB | ~28MB | 过大(建议采样) |
| trades 数组 | 1K~10K笔 | 1K~10K笔 | 1K~10K笔 | 1K~10K笔 |

**约束规则：**
- 1m 周期数据最大查询 30 天（数据保留策略限制）
- 超过 50,000 bars 时 equity_curve 自动启用采样（每 5 点取 1）

### 10. 错误处理策略

| 错误场景 | 处理方式 | HTTP/WS 响应 |
|---------|---------|-------------|
| K线数据为空 | 回测失败，不写结果 | 400 ERR_BACKTEST_NO_DATA |
| 持仓已爆仓 | 停止模拟，记录到爆仓点 | 正常完成，追加爆仓标记 |
| 策略 generate_signal panic | catch_unwind，标记失败 | 500 + error 字段 |
| 取消回测 | 不等引擎结束立即标记 cancelled | status = cancelled |
| 并发超限 | handler 直接返回 429 | 429 ERR_BACKTEST_CONCURRENT_LIMIT |

### 11. 回测结果存储优化

**决策：** 分离大字段写入，使用两步提交。

1. 先写入 `backtest_results` 的元数据行（status=running）
2. 引擎完成后 UPDATE 该行，填充 metrics/trades/equity_curve
3. 引擎失败则 UPDATE status=failed + error

**理由：** 避免部分写入的脏数据。如果引擎在计算过程中崩溃，数据库不会残留半写 JSONB。

### 12. 代码模块结构

```
backend/src/
├── handlers/
│   └── backtest.rs          # 新增：回测 API handler（run/result/progress/cancel/history/delete）
├── services/
│   ├── strategy.rs           # 现有：StrategyTemplate trait + 10 个模板
│   │                         # 新增：每个模板的 generate_signal 实现
│   └── backtest_engine.rs    # 新增：BacktestEngine + BacktestMetrics
├── db/
│   └── backtest.rs           # 新增：backtest_results 读写（SeaORM entity）
└── models/
    └── schemas.rs            # 新增：BacktestRequest/BacktestResponse/BacktestSummary 等
```

## 关联 ADR

- **ADR-004** — 顶层架构决策，本 ADR 是其详细设计
- **ADR-003** — 策略沙箱，回测共享策略模板与参数体系

## 预期后果

**正面：**
- generate_signal 无状态设计保证回测定定性
- 全量内存 + f64 计算，1年1h 回测 < 0.5s，远超 PRD 5s 要求
- 模块划分清晰：handler/service/db 三层，测试友好
- 10 个模板实现 generate_signal 后可直接复用现有指标代码

**负面：**
- generate_signal 需要访问 klines 切片，模板作者需理解索引计算
- 1m 粒度大数据集回测受限于数据保留策略（30天）
- f64 精度可能产生极小的浮点误差，但不影响夏普比率 < 5% 的精度目标

**风险缓解：**
- 为每个模板编写单元测试，用已知 K 线数据验证信号正确性
- 绩效指标实现对照参考库（如 `empyrical`）验证精度
