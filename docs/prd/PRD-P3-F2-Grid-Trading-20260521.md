# PRD: P3-F2 网格交易 + 马丁格尔加仓

> 版本: v1.0
> 状态: Draft
> Author: PM
> Date: 2026-05-21

---

## 1. 背景

网格交易是震荡行情中的经典量化策略，适合 BTC/ETH 等波动性资产。用户设置价格区间和网格数量，系统自动在每个网格点位挂单买卖。

马丁格尔（Martingale）是资金管理策略，亏损后将仓位翻倍以摊薄成本，配合网格使用可提升收益但风险极高。

---

## 2. 功能描述

### 2.1 静态等距网格

- 用户设置：价格上限、价格下限、网格数量（N）
- 系统将区间等分为 N 档
- 当价格触及网格线时：
  - 价格下跌触及网格 → 买入 1 单位
  - 价格上升触及网格 → 卖出 1 单位
- 初始资金均分到每个网格的买入单
- 支持双向网格（同时做多和做空）

### 2.2 动态网格

- 根据ATR（Average True Range）实时调整网格密度
- 波动率升高 → 网格间距变大，减少挂单频率
- 波动率降低 → 网格间距变小，增加交易机会
- 参数：atr_period（默认14）, atr_multiplier（默认2.0）

### 2.3 马丁格尔加仓

- 亏损后下一网格仓位 = 当前仓位 × 系数（默认2.0）
- 最大连续加仓次数（默认5次）
- 到达上限后停止加仓，等待价格回归
- 高风险模式，需要额外的止损保护

### 2.4 震荡自适应

- 检测市场状态：震荡 vs 趋势
- 指标：布林带带宽、ADX
- 震荡市场：使用静态网格
- 趋势市场：自动缩小网格密度或暂停新开仓

---

## 3. 核心接口

### 3.1 创建网格策略

```rust
GridStrategy::new(symbol: &str, lower: f64, upper: f64, grids: u32)
```

### 3.2 添加马丁格尔

```rust
GridStrategy::with_martingale(multiplier: f64, max_consecutive: u32)
```

### 3.3 添加动态网格

```rust
GridStrategy::with_dynamic_grid(atr_period: u32, atr_multiplier: f64)
```

### 3.4 主要方法

```rust
/// 当价格变化时调用，返回需要执行的订单列表
fn on_price_update(&mut self, current_price: f64) -> Vec<GridOrder>;

/// 计算当前持仓和盈亏
fn get_status(&self) -> GridStatus;
```

### 3.5 GridOrder 结构

```rust
pub struct GridOrder {
    pub order_id: String,
    pub side: OrderSide,       // BUY or SELL
    pub price: f64,            // 网格价格
    pub quantity: f64,        // 数量（马丁格尔翻倍）
    pub grid_level: u32,      // 触发的网格编号
    pub is_martingale: bool,  // 是否为马丁格尔加仓单
}
```

### 3.6 GridStatus 结构

```rust
pub struct GridStatus {
    pub total_pnl: f64,               // 总盈亏（已实现+未实现）
    pub realized_pnl: f64,           // 已实现盈亏
    pub unrealized_pnl: f64,          // 未实现盈亏
    pub open_orders_count: u32,      // 活跃挂单数
    pub martingale_level: u32,       // 当前马丁格尔层级
    pub market_mode: MarketMode,     // TRENDING or RANGING
    pub grid_filled: Vec<u32>,       // 已触发的网格编号
}
```

---

## 4. 风险管理

- 最大同时持仓数量限制
- 单笔亏损超过阈值时触发止损
- 马丁格尔连续加仓达到上限后强制止损
- 趋势行情检测到后暂停马丁格尔模式

---

## 5. 适用场景

- 震荡行情（最有效）
- 高流动性交易对（BTC, ETH）
- 适合现货或低杠杆合约

---

## 6. 非适用场景

- 强烈单边趋势（会持续亏损）
- 低流动性交易对（滑点大）
- 马丁格尔模式在熊市中风险极高