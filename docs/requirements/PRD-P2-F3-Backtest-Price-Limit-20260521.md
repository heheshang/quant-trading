# PRD-P2-F3-Backtest-Price-Limit

> **功能名称**：回测涨跌停限制
> **功能ID**：P2-F3
> **版本**：v1.0
> **日期**：2026-05-21
> **状态**：Draft

---

## 1. 功能概述

在回测撮合引擎中实现涨跌停板约束：
- **买入订单**（开多/平空）：当涨停价 < 买入请求价时，订单无法成交
- **卖出订单**（开空/平多）：当跌停价 > 卖出请求价时，订单无法成交
- **触及涨跌停**：实际成交价超出限价时，以涨跌停价替代，并在 TradeRecord 记录 hit_limit=true

---

## 2. 用户故事

### US-BL-01: 涨停日无法买入

**作为** 量化研究员
**我希望** 策略在回测中遇到涨停日时不再以更高价买入
**以便** 回测结果反映真实市场流动性约束

**验收条件**：
- GIVEN 前一根 K 线 close = 10000，limit_pct = 10%
- WHEN 策略在当前 K 线发出买入信号，且当前 K 线 high > 11000
- THEN 系统记录订单状态为 `hit_limit`，成交价 = 11000（涨停价）
- AND 持仓数量 = 0（未实际买入）

### US-BL-02: 跌停日无法卖出

**作为** 量化研究员
**我希望** 策略在回测中遇到跌停日时不再以更低价格卖出
**以便** 回测反映持仓在极端行情下的流动性风险

**验收条件**：
- GIVEN 前一根 K 线 close = 10000，limit_pct = 10%
- WHEN 策略在当前 K 线发出卖出信号，且当前 K 线 low < 9000
- THEN 系统记录订单状态为 `hit_limit`，成交价 = 9000（跌停价）
- AND 持仓数量不变（未实际卖出）

### US-BL-03: 触及涨跌停时记录标记

**作为** 量化研究员
**我希望** 回测报告中标记每笔触及涨跌停的交易
**以便** 分析策略在极端行情下的表现

---

## 3. 技术设计

### 3.1 配置字段（BacktestConfig）

```rust
pub struct BacktestConfig {
    // ... 已有字段 ...
    /// 涨跌停限制比例（默认 10%）
    pub price_limit_pct: Option<f64>,
}
```

默认值 `None` → 10%。前端可配置 5%/10%/20%。

### 3.2 涨跌停价计算

```rust
fn calc_price_limits(prev_close: f64, limit_pct: f64) -> (f64, f64) {
    let upper = prev_close * (1.0 + limit_pct);
    let lower = prev_close * (1.0 - limit_pct);
    (upper, lower)
}
```

### 3.3 撮合校验（open_long / open_short / close_position）

```rust
fn can_execute(exec_price: f64, direction: Direction, upper: f64, lower: f64) -> bool {
    match direction {
        Direction::Long  => exec_price <= upper,  // 买入：不能超过涨停价
        Direction::Short => exec_price >= lower,   // 卖出：不能低于跌停价
    }
}
```

### 3.4 TradeRecord 新增字段

```rust
pub struct TradeRecord {
    // ... 已有字段 ...
    /// 是否触及涨跌停
    pub hit_limit: bool,
    /// 触及的限价类型
    pub limit_type: Option<LimitType>, // LimitType::Upper | LimitType::Lower
}
```

### 3.5 文件变更清单

| 文件 | 变更 |
|------|------|
| `backend/src/services/backtest_engine.rs` | open_long/open_short/close_position 增加涨跌停校验 |
| `backend/src/services/backtest.rs` | BacktestConfig 增加 price_limit_pct 字段 |
| `backend/src/db/backtest_results.rs` | TradeRecord 增加 hit_limit + limit_type 字段 |
| `backend/src/handlers/backtest.rs` | POST /backtest/run 透传 price_limit_pct |
| `frontend/src/types/backtest.ts` | BacktestConfig 增加 price_limit_pct 类型 |
| `frontend/src/api/backtest.ts` | 前端请求透传 price_limit_pct |

---

## 4. 验收条件（Gherkin）

```gherkin
Feature: 回测涨跌停限制

  Scenario: 买入订单在涨停日无法成交（以涨停价替代）
    Given 前一根K线收盘价 = 10000，涨跌停比例 = 10%
    And 涨停价 = 11000，跌停价 = 9000
    And 当前K线数据：open=10500, high=11500, low=10400, close=11200
    When 策略发出买入信号（做多）
    Then 订单状态 = "filled"
    And 成交价 = 11000（涨停价，而非 10500 开仓价）
    And hit_limit = true
    And limit_type = "upper"

  Scenario: 卖出订单在跌停日无法成交（以跌停价替代）
    Given 前一根K线收盘价 = 10000，涨跌停比例 = 10%
    And 当前K线数据：open=9500, high=9600, low=8500, close=8800
    When 策略发出卖出信号（做空/平多）
    Then 订单状态 = "filled"
    And 成交价 = 9000（跌停价，而非 9500 开仓价）
    And hit_limit = true
    And limit_type = "lower"

  Scenario: 价格在涨跌停范围内正常成交
    Given 前一根K线收盘价 = 10000，涨跌停比例 = 10%
    And 当前K线数据：open=10200, high=10800, low=10100, close=10500
    When 策略发出买入信号（做多）
    Then 订单状态 = "filled"
    And 成交价 = 10200（正常滑点价）
    And hit_limit = false
    And limit_type = null

  Scenario: 涨跌停日无对手方流动性，订单完全不成交
    Given 涨跌停板封板（无成交量）
    When 策略发出对应方向订单
    Then 订单状态 = "rejected"
    And reason = "limit_hit"
    And 持仓不变
```

---

## 5. 边界条件

| 条件 | 预期行为 |
|------|----------|
| 前一 K 线 close = 0 或 null | 跳过涨跌停校验（数据异常保护） |
| limit_pct = 0 | 禁用涨跌停限制（向后兼容） |
| 新老回测结果混用 | 老结果 hit_limit=null（兼容），新结果必填 |
| ST/PT 股票（5%/20%涨跌停） | 前端传不同 limit_pct 参数，不硬编码 |
