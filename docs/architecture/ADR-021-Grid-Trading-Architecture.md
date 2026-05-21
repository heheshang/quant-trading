# ADR-021: 网格交易 + 马丁格尔架构

> 版本: v1.0
> 状态: Accepted
> Date: 2026-05-21

## 背景

网格交易是经典震荡市场策略。马丁格尔加仓可提升收益但风险极高。需要在收益和风控之间取得平衡。

## 决策

### 核心模块

```
backend/src/strategies/grid_trading/
├── mod.rs                    # 模块入口
├── grid_engine.rs           # 核心网格引擎
├── martingale.rs            # 马丁格尔加仓管理
├── market_mode.rs           # 趋势/震荡检测
├── types.rs                 # GridOrder, GridStatus, MarketMode
└── tests.rs                 # 单元测试
```

### 核心类型

```rust
// 网格策略配置
pub struct GridConfig {
    pub symbol: String,
    pub lower_price: f64,      // 价格下限
    pub upper_price: f64,      // 价格上限
    pub grid_count: u32,       // 网格数量
    pub quantity_per_grid: f64,// 每格基础数量
    pub martingale: Option<MartingaleConfig>,
    pub dynamic: Option<DynamicGridConfig>,
}

// 马丁格尔配置
pub struct MartingaleConfig {
    pub multiplier: f64,       // 加仓倍数（默认2.0）
    pub max_consecutive: u32,   // 最大连续加仓次数（默认5）
}

// 动态网格配置
pub struct DynamicGridConfig {
    pub atr_period: u32,        // ATR周期（默认14）
    pub atr_multiplier: f64,   // ATR倍数（默认2.0）
    pub min_price_step: f64,   // 最小网格间距
}

// 网格状态
pub struct GridStatus {
    pub total_pnl: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub open_orders: Vec<GridOrder>,
    pub martingale_level: u32,
    pub market_mode: MarketMode,
    pub filled_grids: Vec<u32>,
}
```

### 核心逻辑

#### 网格触发
1. 计算价格所属网格编号：`level = ((price - lower) / (upper - lower) * grids).floor()`
2. 如果新 level 与上次不同，触发网格交易
3. 买单：价格下跌触发；卖单：价格上升触发

#### 马丁格尔
- 追踪每个网格的连续亏损次数
- 亏损后下一笔数量 = 基础数量 × multiplier^consecutive
- 达到 max_consecutive 后停止加仓

#### 趋势检测
- ADX < 25 → 震荡市场（使用静态网格）
- ADX >= 25 → 趋势市场（自动缩小网格或暂停）

### 集成

- 实现为独立策略模块，可被 backtest_engine 调用
- 不直接绑定实盘 API
- MarketModeDetector 可复用（布林带+ADX）

## 替代方案

| 方案 | 优点 | 缺点 |
|------|------|------|
| 独立模块（选中） | 解耦、可测试、可复用 | 需要适配接口 |
| 扩展现有 strategy.rs | 改动小 | 职责混杂 |
| 前端实时计算 | 无后端负载 | 无法做回测 |

## 风险

- 马丁格尔在长趋势中会导致快速亏损 → 已设置 max_consecutive 上限
- 需要配合止损保护 → PRD 中明确要求