# T4 — 测试报告

## 4.1 单元测试结果

| Test Type | Before P1-1 | After P1-1 | Delta |
|---|---|---|---|
| Total | 379 | **390** | +11 |
| Failed | 0 | 0 | 0 |
| Ignored | 0 | 0 | 0 |
| Build | ok (2m52s cold) | ok (1m13s incremental) | — |
| Clippy (`-D warnings`) | 0 warnings | **0 warnings** | ✅ |

### 11 个新增 P1-1 测试

| # | Test | 模板 | 验证 |
|---|---|---|---|
| 1 | `test_p1_1_ids_unique` | 全部 | 3 个新 id 在 registry 且 13 个 id 无重复 |
| 2 | `test_grid_validate_rejects_inverted` | Grid | upper < lower 拒绝 / 合法 accept |
| 3 | `test_grid_signal_buy_below_lower` | Grid | close < lower_band → Buy(0.2) |
| 4 | `test_grid_signal_sell_above_upper` | Grid | close > upper_band → Sell(0.2) |
| 5 | `test_martingale_validate_doubling_cap` | Martingale | max_doubling=10 拒绝 / 3 accept |
| 6 | `test_martingale_signal_doubles_on_down_bar` | Martingale | down bar → Buy(0.1 × 2³ = 0.8) |
| 7 | `test_martingale_signal_hold_on_up_bar` | Martingale | up bar → Hold |
| 8 | `test_breakout_validate_lookback_range` | Breakout | lookback=3 拒绝 / 20 accept |
| 9 | `test_breakout_signal_buy_above_window_high` | Breakout | close > window_high + ATR*0 → Buy |
| 10 | `test_breakout_signal_sell_below_window_low` | Breakout | close < window_low - ATR*0 → Sell |
| 11 | `test_breakout_signal_hold_during_warmup` | Breakout | idx < lookback → Hold |

### 2 个修改的现有测试 (`crud.rs`)

| Test | 改动 |
|---|---|
| `test_all_templates_have_unique_ids` | `assert_eq!(ids.len(), 10)` → `13` |
| `test_all_templates_categories_valid` | valid 集合加 `"momentum"`, `"recovery"` |

## 4.2 实跑验证 (T4.6)

启动 backend (DATABASE_URL/REDIS_URL/JWT_SECRET) → register+login → GET /api/v1/strategies/templates:

**关键指标**：
- 13 个 templates 返回（**10 + 3 全部存在**）
- 3 个 P1-1 模板的 default_parameters 与设计文档一致
- 3 个 P1-1 模板的 parameter_schema 完整且字段名匹配
- 类别分布：6 个类别，**新增 `momentum` (breakout) 和 `recovery` (martingale)**

## 4.3 Lint / Format

| 检查 | 结果 |
|---|---|
| `cargo clippy --lib --all-targets --all-features -- -D warnings` | ✅ **0 warnings** |
| `cargo build` | ✅ clean |
| 编译时间 | 1m13s (incremental) |
| 测试时间 | 9.53s |

## 4.4 回归测试

- 现有 10 个 templates 测试 **全部继续 pass**（零 regression）
- 仅 2 个 test assertion 数字更新（10 → 13 / valid category 集合扩 2 项）—— 这是 P1-1 的**预期改动**
- 后端 health endpoint 200 OK
- `/api/v1/strategies/templates` 返回 13 个 templates

## 4.5 KPI 达成

| PRD KPI | 结果 |
|---|---|
| Scenario 1: Grid 模板注册 | ✅ 13 templates, grid id 出现 |
| Scenario 2: Grid 触发 | ✅ unit test + API default 参数验证 |
| Scenario 3: Martingale 加倍 | ✅ unit test 0.1×2³=0.8 |
| Scenario 4: Breakout 信号 | ✅ unit test Buy/Sell/Hold 三种 path |
| Scenario 5: Parameter validation | ✅ 4 个 validate test 覆盖 inverted/doubling_cap/lookback_range |
| NFR-1 模板数量 10→13 | ✅ |
| NFR-2 无 regression | ✅ 现有 379 → 390 pass |
| NFR-3 trait 一致 | ✅ 8 个方法全部实现 |
| NFR-4 ID 唯一 | ✅ registry 唯一性测试通过 |
| NFR-5 类别合理 | ✅ grid→mean_reversion / martingale→recovery / breakout→momentum |
| NFR-6 默认参数保守 | ✅ grid 5%, martingale max=2, breakout lookback=20 |
| NFR-7 validate 严格 | ✅ 拒绝 upper<lower, max_doubling>5, lookback<5 |
| NFR-8 generate_signal 防御 | ✅ current_idx < lookback → Hold |
