# T1-PRD — Strategy Templates: Grid / Martingale / Breakout (P1-1)

> **任务 ID**：P1-1
> **前置**：P0 完成 + 现有 10 个 strategy template (`templates_impls.rs`)
> **日期**：2026-06-01

## 1. 背景

业务审计报告（`docs/design/2026-06-01_Business_Audit_Report.md`）指出：现有 strategy 模板集中在 **trend-following (MA / MACD / Ichimoku / Breakout-style)** 和 **mean-reversion (RSI / Bollinger / Keltner / Double-Bollinger)** 两类。**缺失**：

| 缺失类型 | 业务场景 | 用户痛点 |
|---|---|---|
| **Grid Trading** | 震荡行情（横盘 consolidation） | 用户需手写脚本挂 N 笔限价单，运维/状态机复杂 |
| **Martingale** | 抗套牢加仓 / recovery | 用户没系统化工具放大胜率但接受可控回撤 |
| **Breakout** | 趋势启动瞬间入场 | 现有模板用 "MA cross" 滞后；用户要"突破即上" |

P1-1 把这 3 个业界经典模板系统化，**降低用户写代码门槛**。

## 2. Gherkin 验收

### Scenario 1: Grid 模板注册并可创建策略
```
Given user calls GET /api/v1/strategy/templates
When response returns
Then templates list contains 13 entries (was 10)
And one of them has id="grid", name="Grid Trading", category="mean_reversion"
And default_parameters = {upper: 1.05, lower: 0.95, grid_levels: 10, size_per_grid: 0.1}
And parameter_schema has 4 params (upper/lower numeric, grid_levels int, size_per_grid float)
```

### Scenario 2: Grid 模板在 backtest 触发
```
Given user creates strategy with template_type="grid", parameters={upper:1.05, lower:0.95, grid_levels:5, size_per_grid:0.2}
And runs backtest on BTCUSDT 1h klines over 90 days with price range [95000, 105000]
When backtest processes 2160 bars
Then at least 3 grid orders are filled (price enters and exits the [0.95*ref, 1.05*ref] band)
And signal log shows "grid" buy when close < lower * reference and sell when close > upper * reference
```

### Scenario 3: Martingale 加倍逻辑
```
Given user creates martingale strategy with base_size=0.1, max_doubling=3
And previous round had a losing trade
When next signal generates a Buy
Then Signal.Buy.quantity_pct = 0.2 (doubled from 0.1 base_size)
When third consecutive loss occurs
Then Signal.Buy.quantity_pct = 0.4 (doubled again)
And when max_doubling=3 reached, signal stays at 0.4 (no further doubling)
```

### Scenario 4: Breakout 信号
```
Given user creates breakout strategy with lookback=20, atr_mult=1.5
When current close > max(high[1..20]) + 1.5 * ATR(14)
Then Signal::Buy { quantity_pct: 1.0 }
When current close < min(low[1..20]) - 1.5 * ATR(14)
Then Signal::Sell { quantity_pct: 1.0 }
Otherwise
Then Signal::Hold
```

### Scenario 5: Parameter validation
```
When user creates grid strategy with upper < lower
Then POST /api/v1/strategy returns 400 with "upper must be > lower"
When user creates martingale with max_doubling > 5
Then POST returns 400 with "max_doubling must be <= 5"
When user creates breakout with lookback < 5
Then POST returns 400 with "lookback must be >= 5"
```

## 3. 非功能需求

- **NFR-1 模板数量**: 从 10 → 13 (+3)
- **NFR-2 无 regression**: 现有 379 unit test 全部继续 pass
- **NFR-3 trait 一致**: 3 个新模板严格遵守 `StrategyTemplate` trait 的 8 个方法签名
- **NFR-4 ID 唯一**: 3 个新 id（grid/martingale/breakout）不与现有 10 个重复（registry 自动校验）
- **NFR-5 类别分布合理**: grid→mean_reversion, martingale→recovery, breakout→momentum（不与现有冲突）
- **NFR-6 默认参数保守**: Grid 默认 5% 区间，Martingale 默认 max_doubling=2，Breakout 默认 lookback=20
- **NFR-7 validate() 严格**: 拒绝越界参数，返回中文/英文混合可读错误
- **NFR-8 generate_signal 防御性**: current_idx < lookback → Signal::Hold（不 panic）

## 4. 不在范围

- ❌ Martingale 的"亏损记忆"持久化（**P2+ 任务**，本次实现 backtest 内连续 loss 计数）
- ❌ Grid 的"挂单"（实际订单）执行（**P1-2 任务**，本次只发 Signal）
- ❌ Breakout 的 volume filter（**P1+ 任务**）
- ❌ 模板级 P&L 拆分（**P1+ 任务**）
- ❌ 前端 strategy builder UI（**P2+ 任务**）
