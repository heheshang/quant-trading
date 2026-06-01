# Release Note v0.10.1 — P2-F3 回测引擎涨跌停限制

- Version: 0.10.1
- Date: 2026-05-22
- Author: ssk

## 升级说明

本次升级包含 **P2-F3 回测引擎涨跌停限制**模块，在回测撮合引擎中实现涨跌停板约束，使回测结果更贴近实际交易环境。

## 新增功能

### 回测配置字段

```rust
// backend/src/models/backtest.rs
pub struct BacktestConfig {
    // ... existing fields
    /// Price limit percentage for backtest (e.g. 0.10 = 10% limit).
    /// Set to None or 0.0 to disable price limit enforcement.
    pub price_limit_pct: Option<f64>,
}
```

### 涨跌停价格计算

```rust
// backend/src/services/backtest_engine.rs
fn price_limits(&self, prev_close: f64) -> (Option<f64>, Option<f64>) {
    // price_limit_pct=0.10, prev_close=100.0
    // -> upper=110.0, lower=90.0
}
```

### 涨跌停撮合规则

| 场景 | 规则 | 结果 |
|------|------|------|
| 买入订单 | exec_price > upper_limit | 无法成交（涨停拦截） |
| 卖出订单 | exec_price < lower_limit | 无法成交（跌停拦截） |
| 触及涨跌停 | 成交时 | hit_limit=true, limit_type=Upper/Lower |

### TradeRecord 扩展字段

```rust
// backend/src/models/backtest.rs
pub struct TradeRecord {
    // ... existing fields
    pub hit_limit: bool,           // 是否触及涨跌停
    pub limit_type: Option<LimitType>, // Upper=涨停, Lower=跌停
}
```

## API 兼容性

- `price_limit_pct` 字段在 BacktestConfig 中为可选（`Option<f64>`）
- 不设置或设为 `None`/`0.0` 表示禁用涨跌停限制（向后兼容）
- 前端回测配置表单可选择 5% / 10% / 20% 或不启用

## 测试状态

| 类别 | 数量 | 状态 |
|------|------|------|
| P2-F3 新增单元测试 | 7 | ✅ |

### 测试用例列表

| 测试用例 | 验证内容 |
|------|------|
| test_price_limits_calculation | 10% 涨跌停价格计算正确 |
| test_price_limits_disabled | price_limit_pct=None 返回 (None, None) |
| test_open_long_hits_limit | 涨停价 > 买入请求价时无法成交 |
| test_open_short_hits_limit | 跌停价 < 卖出请求价时无法成交 |
| test_close_long_hits_limit | 平多时触及跌停价 |
| test_close_short_hits_limit | 平空时触及涨停价 |
| test_hit_limit_flag_set | 触及限价时 hit_limit=true |

## 已知限制

- stop_loss/take_profit 与涨跌停的交互尚未测试（后续 T4.6）
- 前端配置表单下拉选项待实现（5%/10%/20%）
