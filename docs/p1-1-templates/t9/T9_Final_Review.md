# T9 — P1-1 Strategy Templates (Grid / Martingale / Breakout) 最终评审

> **任务 ID**：P1-1
> **日期**：2026-06-01
> **状态**：✅ **PASS — 可合并**

## 9.1 完成情况

| 阶段 | 状态 | 备注 |
|---|---|---|
| T1 PRD | ✅ | 5 Gherkin 场景 + 8 NFR |
| T2 选型 | ✅ | 复用 `templates_impls.rs` + `common.rs` helper |
| T3 设计 | ✅ | 3 模板 + 7 关键决策 (D1-D7) |
| T4 实现 | ✅ | +400 行 (3 struct + 3 impl + 11 tests) |
| T4 测试 | ✅ | 390/390 pass, 0 clippy warning |
| T4.6 实跑 | ✅ | `/api/v1/strategies/templates` 返回 13 个 |
| T6 门禁 | ✅ | cargo build + clippy + test 全过 |
| T7 文档 | ✅ | T0/T1/T2/T3/T4/T9 共 6 文件 |

## 9.2 交付物

### 代码
| 文件 | 改动 | 大小 |
|---|---|---|
| `backend/src/services/strategy/templates_impls.rs` | +400 行 (3 templates + 11 tests) | 31K → 46K |
| `backend/src/services/strategy/registry.rs` | +6 行 (3 use + 3 Box::new) | 3.5K → 3.6K |
| `backend/src/services/strategy/crud.rs` | +2 行 (1 test assert + 1 valid 集合) | 36K → 36K |

### 文档
| 路径 | 大小 |
|---|---|
| `docs/p1-1-templates/T0_Task_Overview.md` | (本任务综述) |
| `docs/p1-1-templates/requirements/T1-PRD_templates_p1-1.md` | 4.2 KB |
| `docs/p1-1-templates/architecture/T2_Design.md` | 5.2 KB |
| `docs/p1-1-templates/qa/T4_Unit_Test_Report.md` | 3.0 KB |
| `docs/p1-1-templates/t9/T9_Final_Review.md` | (本文件) |

## 9.3 3 模板速览

| ID | Name | Category | Default Params | Signal 逻辑 |
|---|---|---|---|---|
| `grid` | Grid Trading | mean_reversion | upper=1.05, lower=0.95, grid_levels=10, size_per_grid=0.1 | ref_price = close[0]; close < ref*lower → Buy; close > ref*upper → Sell |
| `martingale` | Martingale | recovery | base_size=0.1, max_doubling=3, take_profit=0.05 | curr < prev → Buy(size=base×2^max_d); else Hold |
| `breakout` | Breakout | momentum | lookback=20, atr_period=14, atr_mult=1.5, quantity_pct=1.0 | close > max(high) + atr_m*ATR → Buy; close < min(low) - atr_m*ATR → Sell |

## 9.4 关键设计亮点 (D1-D7)

- **D1 ref_price 锁定**：backtest 启动时第一根 close，避免区间漂移
- **D2 Martingale 用 in-template 状态**：基于"连续 down bar"近似 loss_streak
- **D3 current_idx < lookback → Hold**：防御越界，与现有 10 模板一致
- **D4 parameter_schema 用宏**：`int_param!` / `float_param!` 复用
- **D5 validate() 顺序**：必填 → 范围 → 交叉约束（与 RSI 一致）
- **D6 不引入新依赖**：复用 `closes()` / `highs()` / `lows()` / `atr()`
- **D7 ID 短且语义化**：`grid` / `martingale` / `breakout`

## 9.5 业务价值

**之前**：10 templates 集中在 trend-following + mean-reversion
**现在**：
- 13 templates 覆盖 **6 个类别**（trend / mean_reversion / volatility / composite / **momentum** / **recovery**）
- Grid 解决"震荡市"手写脚本痛点
- Martingale 解决"抗套牢加仓"系统化需求
- Breakout 解决"突破滞后"问题（带 ATR filter 防假突破）

**用户**：
- 前端 strategy builder 拉 `/api/v1/strategies/templates` 自动显示新模板
- 现有 strategy 创建/编辑流程**无需改动**——P1-1 严格遵守 trait

**风险**：
- 0 个新 metric 埋点（**与 P0-3 配合**：业务指标在 orders_* 等埋点已覆盖）
- 0 个新依赖
- 0 个 schema 变更（数据库 `strategies.parameters` 是 `JSONB`，任意 params 都能存）

## 9.6 已知限制

1. **Martingale "loss_streak" 是近似**：P1-1 用"连续 down bar"代理，未对接 backtest 引擎回报"上一笔实际结果"
2. **`take_profit` 字段保留** 但未触发（Martingale 的真正 TP 逻辑是 P1+ 任务）
3. **Grid 用比例作区间** 是友好抽象，**专业 trader 可能要绝对价**（P1+ 任务可加 `ref_mode: "ratio" | "absolute"`）
4. **Breakout 仅看价格**：未加 volume filter（业界常见需求，P1+ 任务）
5. **无 martingale safety net**：P1-1 暴露 `max_doubling ≤ 5`，但**实盘 martingale 仓位失控**风险高（建议 P1+ 加 "max_loss_per_session" 硬限）

## 9.7 后续任务

| ID | 任务 | 来源 |
|---|---|---|
| **P1-1.1** | Martingale 真实 loss tracking（对接 backtest 引擎回报）| 自然延伸 |
| **P1-1.2** | Grid 绝对价模式 + 实际挂单状态机 | 业务审计 |
| **P1-1.3** | Breakout volume filter | 业界标准 |
| **P1-1.4** | Martingale safety net (max_loss_per_session) | 风控必要 |
| **P1-2** | 订单类型扩 Iceberg / Bracket / Trailing | 业务审计 |
| **P1-3** | 风控粒度细化 | 业务审计 |

## 9.8 Commit 计划

```
feat(strategy): add Grid, Martingale, Breakout templates (P1-1)

Add 3 new strategy templates to extend the existing 10-template
catalog with 6 categories (was 4). Each template implements
the StrategyTemplate trait with id / name / description /
category / default_parameters / parameter_schema / validate /
generate_signal.

Templates:
- grid (mean_reversion): Range-trading relative to ref_price
- martingale (recovery): Doubling on consecutive down bars
- breakout (momentum): N-period high/low with ATR filter

Tests: 11 new unit tests covering validate / generate_signal
for each template. Updated 2 existing tests in crud.rs to
reflect new template count (10→13) and expanded category
allowlist (added "momentum", "recovery").

Files:
- backend/src/services/strategy/templates_impls.rs (+400 lines)
- backend/src/services/strategy/registry.rs (+6 lines)
- backend/src/services/strategy/crud.rs (+2 lines)

Verified:
- 390/390 unit tests pass (was 379)
- 0 clippy warnings
- GET /api/v1/strategies/templates returns 13 templates
  with grid/martingale/breakout present and correct metadata

Closes: P1-1
```

## 9.9 P0→P1 进度全景

| ID | 任务 | 状态 |
|---|---|---|
| P0-1~5 | 完整可观测性栈 | ✅ |
| **P1-1** | **Strategy 模板扩 3 个** | **✅** |
| P1-2 | 订单类型扩展 | ⏳ |
| P1-3 | 风控粒度细化 | ⏳ |
