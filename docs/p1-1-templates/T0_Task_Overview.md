# T0 — P1-1 Strategy Templates 任务总览

> **任务 ID**：P1-1
> **前置**：P0 完成 + 现有 10 templates
> **日期**：2026-06-01
> **状态**：✅ Done

## 范围

业务审计报告（`docs/design/2026-06-01_Business_Audit_Report.md`）指出：现有 strategy 模板集中在 **trend-following**（MA/MACD/Ichimoku）和 **mean-reversion**（RSI/Bollinger/Keltner/Double-Bollinger）两类。P1-1 系统化补齐 **3 个业界经典模板**：

| Template | 类别 | 解决什么 |
|---|---|---|
| **Grid Trading** | mean_reversion | 震荡行情区间挂单 |
| **Martingale** | recovery | 亏损加倍加仓 |
| **Breakout** | momentum | 价格突破入场（带 ATR filter）|

## 交付

| 资产 | 路径 | 改动 |
|---|---|---|
| 模板实现 | `backend/src/services/strategy/templates_impls.rs` | +400 行（3 struct + 3 impl + 11 unit tests）|
| Registry | `backend/src/services/strategy/registry.rs` | +6 行（3 use + 3 Box::new）|
| 现有 test 升级 | `backend/src/services/strategy/crud.rs` | +2 行（assert 10→13 + valid category 集合扩 2 项）|

## 11 个新 unit test

| # | 名称 | 验证 |
|---|---|---|
| 1 | `test_p1_1_ids_unique` | 3 新 id 全部在 registry 且 13 个 id 无重复 |
| 2 | `test_grid_validate_rejects_inverted` | upper < lower 拒绝 / 合法 accept |
| 3 | `test_grid_signal_buy_below_lower` | close < lower_band → Buy(0.2) |
| 4 | `test_grid_signal_sell_above_upper` | close > upper_band → Sell(0.2) |
| 5 | `test_martingale_validate_doubling_cap` | max_doubling=10 拒绝 |
| 6 | `test_martingale_signal_doubles_on_down_bar` | down bar → Buy(0.1 × 2³ = 0.8) |
| 7 | `test_martingale_signal_hold_on_up_bar` | up bar → Hold |
| 8 | `test_breakout_validate_lookback_range` | lookback=3 拒绝 |
| 9 | `test_breakout_signal_buy_above_window_high` | close > window_high + 0 → Buy |
| 10 | `test_breakout_signal_sell_below_window_low` | close < window_low - 0 → Sell |
| 11 | `test_breakout_signal_hold_during_warmup` | idx < lookback → Hold |

## 质量门禁

| 阶段 | 结果 |
|---|---|
| `cargo build` | ✅ clean |
| `cargo test --lib` | ✅ **390/390 pass** (was 379, +11) |
| `cargo clippy --lib --all-targets -- -D warnings` | ✅ **0 warnings** |
| `cargo test --no-run` | ✅ clean compile |
| 实跑 `/api/v1/strategies/templates` | ✅ 返回 13 个 templates 含 3 新 ID |
| default_parameters 与设计一致 | ✅ grid 4 / martingale 3 / breakout 4 |
| parameter_schema 字段完整 | ✅ 11 个参数全部正确 |

## 业务价值

| 之前 | 现在 |
|---|---|
| 10 templates × 4 类别 | 13 templates × **6 类别**（+2: momentum/recovery）|
| 缺震荡市挂单工具 | Grid 模板 + 比例区间 |
| 缺加倍加仓系统 | Martingale + max_doubling cap |
| 突破入场滞后 | Breakout + ATR 防假突破 |

**用户**：前端 strategy builder 拉模板列表自动看到 3 个新模板，**零前端改动**。

**风险**：
- 0 个新 metric 埋点（业务指标 orders_* 等已覆盖）
- 0 个新依赖
- 0 个 schema 变更

## 文档

```
docs/p1-1-templates/
├── T0_Task_Overview.md                                       (本文件)
├── requirements/T1-PRD_templates_p1-1.md
├── architecture/T2_Design.md
├── qa/T4_Unit_Test_Report.md
└── t9/T9_Final_Review.md
```

## 后续任务

| ID | 任务 | 来源 |
|---|---|---|
| **P1-1.1** | Martingale 真实 loss tracking | backtest 引擎对接 |
| **P1-1.2** | Grid 绝对价 + 实际挂单 | 业务审计 |
| **P1-1.3** | Breakout volume filter | 业界标准 |
| **P1-1.4** | Martingale safety net | 风控必要 |
| **P1-2** | 订单类型扩展 | 业务审计 |
| **P1-3** | 风控粒度细化 | 业务审计 |
