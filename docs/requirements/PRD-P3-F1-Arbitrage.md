# PRD-P3-F1: 套利模块 (Arbitrage Module)

## 1. 功能概述

**功能描述：** 实现跨期套利、跨品种套利、期现套利三种策略的自动识别与执行，支持价差监控、信号生成、自动下单。

**用户故事：** 作为量化交易者，我希望系统能自动识别不同交易所/合约间的价差机会并执行套利，无需人工盯盘。

## 2. 套利类型

### 2.1 跨期套利 (Calendar Spread Arbitrage)
- **标的：** 同一交易对不同到期日期货（如 BTCUSDT 合约当月/次月）
- **信号逻辑：** 当近月-远月价差偏离历史均值超过阈值时，开仓价差套利；价差回归均值时平仓
- **参数：** `spread_entry_threshold`（开仓阈值）、`spread_exit_threshold`（平仓阈值）、`max_spread_ratio`

### 2.2 跨品种套利 (Cross-Margin Arbitrage)
- **标的：** 相关性强资产对（如 BTC/ETH 相关性 > 0.9）
- **信号逻辑：** 当两个资产价比偏离历史均值时，做空相对强势、做多相对弱势
- **参数：** `correlation_threshold`（相关性阈值，默认 0.9）、`z_score_entry`（开仓 Z-score）、`z_score_exit`（平仓 Z-score）

### 2.3 期现套利 (Spot-Futures Arbitrage)
- **标的：** 现货与对应期货合约
- **信号逻辑：** 当期货溢价超过持有成本（融资利率 + 存储成本），做空期货+买现货；反向套利当期货折价
- **参数：** `futures_spot_premium_threshold`（溢价阈值）、`funding_rate_threshold`

## 3. 核心功能

### 3.1 套利对管理
- 添加/编辑/删除套利交易对（symbol_pair, exchange, spread_config）
- 支持手动和自动两种模式

### 3.2 价差计算引擎
- 实时计算不同交易对/合约间的价差（spread）
- 支持价格比率、百分比差、Z-score 多种计算方式
- 维护历史价差数据窗口（用于均值/标准差计算）

### 3.3 信号生成
- 基于价差偏离度生成套利信号（入场/出场/止损）
- 支持双向开仓（近空远多 或 近多远空）
- 信号过滤（最小价差、最小成交量）

### 3.4 执行引擎
- 支持市价单和限价单执行
- 双向仓位管理（side_a/side_b）
- 执行延迟补偿
- 滑点估算

### 3.5 监控面板
- 实时价差图表
- 开仓套利对状态（持仓、盈亏、离平仓点距离）
- 历史套利统计（成功率、平均持仓时间）

## 4. API 设计

### 4.1 套利对管理
```
POST   /api/v1/arbitrage/pairs          创建套利对
GET    /api/v1/arbitrage/pairs          列表
GET    /api/v1/arbitrage/pairs/{id}      详情
PUT    /api/v1/arbitrage/pairs/{id}     更新
DELETE /api/v1/arbitrage/pairs/{id}     删除
```

### 4.2 监控数据
```
GET    /api/v1/arbitrage/spread/{pair_id}     当前价差
GET    /api/v1/arbitrage/positions            当前套利持仓
GET    /api/v1/arbitrage/signals              最近的套利信号
```

### 4.3 请求/响应格式
```json
// POST /api/v1/arbitrage/pairs
{
  "pair_type": "calendar_spread",  // calendar_spread | cross_pair | spot_futures
  "symbol_a": "BTCUSDT_240625",
  "symbol_b": "BTCUSDT_240926",
  "exchange": "binance",
  "spread_config": {
    "spread_entry_threshold": 0.02,
    "spread_exit_threshold": 0.005,
    "max_position_size": 1000,
    "calculation_mode": "percentage"  // ratio | percentage | zscore
  }
}

// GET /api/v1/arbitrage/spread/{pair_id}
{
  "pair_id": 1,
  "spread": 0.015,
  "spread_pct": 1.5,
  "z_score": 2.3,
  "historical_mean": 0.01,
  "historical_std": 0.002,
  "signal": "long_spread",  // long_spread | short_spread | neutral
  "timestamp": "2026-05-21T12:00:00Z"
}
```

## 5. 数据模型

### 5.1 arbitrage_pairs 表
```sql
CREATE TABLE arbitrage_pairs (
  id SERIAL PRIMARY KEY,
  pair_type VARCHAR(32) NOT NULL,        -- calendar_spread, cross_pair, spot_futures
  symbol_a VARCHAR(32) NOT NULL,
  symbol_b VARCHAR(32) NOT NULL,
  exchange VARCHAR(32) NOT NULL,
  status VARCHAR(16) DEFAULT 'active',
  spread_entry_threshold DECIMAL(10, 4) NOT NULL,
  spread_exit_threshold DECIMAL(10, 4) NOT NULL,
  max_position_size DECIMAL(10, 2) NOT NULL,
  calculation_mode VARCHAR(16) DEFAULT 'percentage',
  correlation_threshold DECIMAL(5, 4),    -- for cross_pair
  z_score_entry DECIMAL(5, 2),           -- for cross_pair
  z_score_exit DECIMAL(5, 2),            -- for cross_pair
  created_at TIMESTAMP DEFAULT NOW(),
  updated_at TIMESTAMP DEFAULT NOW()
);
```

### 5.2 arbitrage_positions 表
```sql
CREATE TABLE arbitrage_positions (
  id SERIAL PRIMARY KEY,
  pair_id INTEGER REFERENCES arbitrage_pairs(id),
  direction VARCHAR(16) NOT NULL,         -- long_spread (A多B空) | short_spread (A空B多)
  size_a DECIMAL(10, 4) NOT NULL,
  size_b DECIMAL(10, 4) NOT NULL,
  entry_spread DECIMAL(10, 6) NOT NULL,
  current_spread DECIMAL(10, 6),
  unrealized_pnl DECIMAL(12, 4),
  status VARCHAR(16) DEFAULT 'open',      -- open | closed | liquidated
  opened_at TIMESTAMP DEFAULT NOW(),
  closed_at TIMESTAMP
);
```

### 5.3 arbitrage_signals 表
```sql
CREATE TABLE arbitrage_signals (
  id SERIAL PRIMARY KEY,
  pair_id INTEGER REFERENCES arbitrage_pairs(id),
  signal_type VARCHAR(16) NOT NULL,       -- entry_long | entry_short | exit | stop_loss
  spread DECIMAL(10, 6),
  z_score DECIMAL(8, 4),
  confidence DECIMAL(5, 4),
  executed BOOLEAN DEFAULT FALSE,
  created_at TIMESTAMP DEFAULT NOW()
);
```

## 6. Edge Cases

1. **价差计算数据源不可用** — 同时获取 A/B 数据，任一失败等待重试
2. **套利对流动性不足** — 下单失败自动重试 3 次，间隔 100ms
3. **双向持仓部分成交** — 支持部分成交，更新仓位
4. **价差剧烈波动触发止损** — 止损信号优先于平仓信号
5. **相关交易对下线** — 自动关闭对应套利对，发送告警

## 7. Acceptance Criteria

- [ ] 套利对 CRUD API 完整可用
- [ ] 三种套利类型（跨期/跨品种/期现）信号计算正确
- [ ] Z-score 计算基于滑动窗口（默认 20 期）
- [ ] 双向挂单成功执行（side_a + side_b）
- [ ] 价差数据实时更新（≤ 1s 延迟）
- [ ] 单元测试覆盖率 ≥ 70%（核心逻辑 80%）
- [ ] 集成测试：模拟价差触发入场/出场场景

## 8. 工时估算

- **工期：** 7 天
- **复杂度：** 高（涉及多个数据源同步 + 双向订单协调）
