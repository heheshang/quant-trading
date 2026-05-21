# Release Note v0.10.0 — P3-F2 网格交易 + 马丁格尔

## 新增模块
- `strategies/grid_trading/`: 独立网格交易策略模块
  - `GridEngine`: 核心引擎，价格触发网格挂单
  - `MartingaleTracker`: 马丁格尔仓位管理器
  - `MarketModeDetector`: 市场状态检测（趋势/震荡）
  - `types.rs`: GridConfig, GridOrder, GridPosition, GridStatus

## 核心功能
- 静态等距网格（可配置价格区间+网格数量）
- 马丁格尔加仓（可配置倍数和上限）
- 动态网格（ATR自适应间距）
- 震荡自适应（ADX + 布林带）

## 测试
- 281 tests passed (+14 新增)

## Breaking Changes
- 新增 `strategies` 模块，需要 `cargo build`
