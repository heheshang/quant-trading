# Release Note v0.10.2 — P3-F1 套利模块

- Version: 0.10.2
- Date: 2026-05-22
- Author: ssk

## 升级说明

本次升级包含 **P3-F1 套利模块**，实现币币套利策略的核心功能：交易对管理、价差计算、信号生成和持仓管理。

## 新增功能

### 后端实现

**SpreadCalculator** — 价差计算引擎
```rust
// backend/src/services/arbitrage/spread_calculator.rs
pub fn calculate_spread(
    &self,
    pair_id: i32,
    price_a: Decimal,  // ask
    price_b: Decimal,  // bid
    mode: &SpreadCalculationMode,
) -> Result<LiveSpread, AppError>
```

支持三种计算模式：
- `Ratio`: price_a / price_b
- `Percentage`: (price_a - price_b) / price_b * 100
- `ZScore`: 基于历史窗口的 Z-Score 标准化

**SignalGenerator** — 套利信号生成器
```rust
// backend/src/services/arbitrage/signal_generator.rs
pub fn generate_signal(
    &self,
    spread: &LiveSpread,
    threshold: Decimal,
    zscore_threshold: Decimal,
) -> ArbitrageSignal
```

信号类型：`EntryLong`, `EntryShort`, `Exit`, `Hold`

**PairManager** — 交易对管理
- 支持币币交易对 CRUD
- 配置手续费率、滑点容忍度

### 前端实现

**ArbitrageView.vue** — 套利操作界面
- 交易对列表 / 添加交易对
- 实时价差监控面板
- 信号历史记录
- 持仓管理

**arbitrage.ts** — API 客户端
```typescript
GET /api/v1/arbitrage/pairs
GET /api/v1/arbitrage/signals
GET /api/v1/arbitrage/positions
POST /api/v1/arbitrage/pairs
```

## 数据库

新增表：
- `arbitrage_pairs`: 交易对配置
- `arbitrage_positions`: 套利持仓记录
- `arbitrage_signals`: 信号历史

## 质量

- cargo test: ✅ 309 passed, 0 failed
- clippy: ✅ warnings only, 0 errors
- P0 Bug: 0
