# T2+T3 — 技术选型 & 设计文档

## T2 选型

### 2.1 实现位置

| 选项 | 优 | 劣 | 选 |
|---|---|---|---|
| **在 `templates_impls.rs` 加 3 个 struct + impl** | 现有 10 个同文件，结构一致 | 文件变大（31K → ~42K） | ✅ |
| 新建 `templates_p1.rs` | 隔离清晰 | 需修改 `mod.rs` + `registry.rs` 多处 | ❌ 隔离没必要 |
| 拆为 `templates/grid.rs` 等多文件 | 模块化 | 现有 10 个都是单文件，破坏一致性 | ❌ |

**结论**：在 `templates_impls.rs` 文件末尾追加 3 个模板（保持现有 10 个不动）。`registry.rs` `get_all_templates()` 末尾追加 3 个 Box::new。

### 2.2 指标复用

**Grid / Martingale / Breakout 不引入新 metric**——它们复用 P0-3 已埋点：
- `trigger_orders_created_total`（条件单创建）
- `risk_rules_tripped_total`（风控 trip）
- HTTP middleware（`http_requests_total`）

**新增埋点**：0 个。

### 2.3 Signal 实现复杂度

| 模板 | 计算复杂度 | 关键 helper |
|---|---|---|
| Grid | O(N) per bar | closes() |
| Martingale | O(N) per bar, O(1) state | closes() + 内部 loss_streak |
| Breakout | O(N) per bar (max/min window) | highs() / lows() / atr() |

**不优化**（按 bar 算）—— 与现有 10 个模板一致。

## T3 设计

### 3.1 Grid 模板设计

**核心逻辑**：以"参考价"为中心，[lower, upper] 区间内放置 N 个网格。
- 参考价 = backtest 启动时第一根 kline 的 close
- 价格跌穿下轨 → 买入（信号）
- 价格涨穿上轨 → 卖出（信号）
- 价格在区间内 → Hold

**关键参数**：
```json
{
  "upper": 1.05,         // 上轨 (相对 ref_price 比例)
  "lower": 0.95,         // 下轨 (相对 ref_price 比例)
  "grid_levels": 10,     // 网格层数 (informational, 实际仍只用上下轨)
  "size_per_grid": 0.1   // 每次信号仓位比例
}
```

**为什么用比例而非绝对价**？**用户友好**——backtest 第一根 kline close 作 ref，不同 symbol 不需要重新算价。

### 3.2 Martingale 模板设计

**核心逻辑**：亏损后下一笔订单仓位加倍。
- 内部维护 `loss_streak: u32`
- 每次 generate_signal：维护 loss_streak（**需要 backtest 引擎配合回传上笔结果**——**当前限制**）
- **实际实现**：基于"信号连续产生"作为代理（更简单）。**这是 P1-1 的妥协方案**。

**简化版核心逻辑**：
- 用 close 价格 vs 上一根 close 比较
- 当前 close < 上一根 close → "down bar" → 累计 loss_streak += 1
- 当前 close >= 上一根 close → "up bar" → loss_streak = 0 (reset)
- Buy 信号的 size = `base_size * 2^min(loss_streak, max_doubling)`
- 买完 reset loss_streak（**业务逻辑**：进入新仓位视为"投资期"）

**关键参数**：
```json
{
  "base_size": 0.1,    // 基础仓位比例
  "max_doubling": 3,   // 最大加倍次数
  "take_profit": 0.05  // take_profit 比例 (未在 P1-1 触发，留扩展点)
}
```

**已知妥协**：`take_profit` 字段已暴露但 P1-1 不实现，**P1+ 任务**扩展。

### 3.3 Breakout 模板设计

**核心逻辑**：突破 N 期最高/最低（带 ATR filter 防假突破）。
- 当前 close > max(high[lookback_period-1..]) + atr_mult * ATR(atr_period) → Buy
- 当前 close < min(low[lookback_period-1..]) - atr_mult * ATR(atr_period) → Sell
- 其他 → Hold

**为什么用 ATR filter？** 直突破容易被噪声触发。**ATR × mult 给出"足够远"的缓冲**。

**关键参数**：
```json
{
  "lookback": 20,        // 突破回看期
  "atr_period": 14,      // ATR 计算周期
  "atr_mult": 1.5,       // ATR 乘数
  "quantity_pct": 1.0    // 仓位比例
}
```

### 3.4 关键设计决策

**D1: 参考价 (ref_price) 在 backtest 启动时锁定**
- 不动态更新 → 避免"区间漂移"导致买/卖逻辑不一致
- 业务上等价于"以 backtest 开始时 market price 为锚"

**D2: Martingale loss_streak 是 in-template 状态**
- Template 是 stateless struct（无字段）
- 状态用 generate_signal 调用间的"价格趋势"推断
- **不持久化**——backtest 重启归零（业务场景一致）

**D3: 3 个模板 generate_signal 都返回 Signal::Hold 当 current_idx < lookback**
- 防御性：避免越界访问
- 与现有 10 个模板行为一致

**D4: parameter_schema 用宏构造 (int_param! / float_param!)**
- 与现有 10 个模板一致
- 自动生成 min/max/default 提示

**D5: validate() 顺序：先 schema 必填，再范围，最后交叉约束**
- 与现有 RSI 模板一致（`oversold >= overbought` 类似的交叉约束）

**D6: 不引入新依赖**
- 复用 `common.rs` 已有 `closes()` / `highs()` / `lows()` / `atr()` / `sma()`

**D7: 模板 ID 短且语义化**
- `grid` / `martingale` / `breakout`（不是 `grid_trading_v1` 之类）
- 与现有 `ma_crossover` / `rsi` / `macd` 风格一致

### 3.5 单元测试策略

每个新模板 3-4 个 unit test：
1. `test_id_unique` — 3 个新 id 与现有 10 个 + 已有测试断言
2. `test_validate_*` — 边界值（min/max 越界、交叉约束）
3. `test_generate_signal_basic` — 简单 kline 序列 → 预期 Signal
4. `test_generate_signal_lookback` — current_idx < lookback → Hold
