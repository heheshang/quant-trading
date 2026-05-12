# 策略管理用户指南

> 如何创建、配置和管理量化交易策略

---

## 目录

- [1. 快速入门](#1-快速入门)
- [2. 策略模板详解](#2-策略模板详解)
  - [2.1 趋势类 (trend)](#21-趋势类-trend)
  - [2.2 均值回归类 (mean_reversion)](#22-均值回归类-mean_reversion)
  - [2.3 波动率类 (volatility)](#23-波动率类-volatility)
  - [2.4 复合类 (composite)](#24-复合类-composite)
- [3. 策略状态说明](#3-策略状态说明)
  - [3.1 状态流转图](#31-状态流转图)
  - [3.2 状态行为详解](#32-状态行为详解)
- [4. 参数配置最佳实践](#4-参数配置最佳实践)
- [5. 常见问题](#5-常见问题)

---

## 1. 快速入门

创建并运行策略的完整流程只需要 4 步:

### 步骤 1: 查看可用模板

先获取所有可用的策略模板，了解每个模板的参数要求:

```bash
curl -s -H "Authorization: Bearer <your_token>" \
  http://localhost:3000/api/v1/strategies/templates | jq '.data[] | {id, name, category}'
```

输出示例:

```json
{
  "id": "ma_crossover",
  "name": "MA Crossover",
  "category": "trend"
}
{
  "id": "rsi",
  "name": "RSI",
  "category": "mean_reversion"
}
```

### 步骤 2: 创建策略

选择一个模板，配置参数，创建策略:

```bash
curl -s -X POST \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My RSI Strategy",
    "template_type": "rsi",
    "parameters": {
      "period": 14,
      "overbought": 75,
      "oversold": 25
    }
  }' \
  http://localhost:3000/api/v1/strategies | jq .
```

创建成功后策略自动处于 `draft`（草稿）状态。

### 步骤 3: 启动策略

将策略从 `draft` 切换到 `active`:

```bash
curl -s -X POST \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{"status": "active"}' \
  http://localhost:3000/api/v1/strategies/<strategy_id>/status | jq .
```

### 步骤 4: 查看策略列表

查看所有策略及其当前状态:

```bash
curl -s -H "Authorization: Bearer <your_token>" \
  "http://localhost:3000/api/v1/strategies?page=1&size=20" | jq .
```

---

## 2. 策略模板详解

系统提供 10 个策略模板，分为 4 个类别。

### 2.1 趋势类 (trend)

趋势跟踪类策略在上升趋势中做多、下降趋势中做空，适合趋势明显的市场环境。

#### MA Crossover (ma_crossover)

双均线交叉策略，使用快速和慢速两条移动平均线。当快线上穿慢线时发出买入信号，下穿时发出卖出信号。

| 参数 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `fast_period` | integer | 5-50 | 10 | 快速移动平均周期 |
| `slow_period` | integer | 10-200 | 30 | 慢速移动平均周期 |

> **约束:** `fast_period` 必须小于 `slow_period`

#### Triple MA (triple_ma)

三均线交叉策略，使用短、中、长三条移动平均线。信号强度由三条线的排列顺序决定。

| 参数 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `short` | integer | 5-20 | 9 | 短期 MA 周期 |
| `medium` | integer | 10-50 | 21 | 中期 MA 周期 |
| `long` | integer | 20-200 | 50 | 长期 MA 周期 |

> **约束:** `short < medium < long`

#### MACD (macd)

MACD 指标策略，基于快慢 EMA 差值与信号线的交叉关系。

| 参数 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `fast` | integer | 2-50 | 12 | 快线 EMA 周期 |
| `slow` | integer | 5-100 | 26 | 慢线 EMA 周期 |
| `signal` | integer | 2-30 | 9 | 信号线周期 |

> **约束:** `fast < slow`

#### Ichimoku Cloud (ichimoku)

一目均衡云图策略，通过转换线、基准线、先行带等综合判断趋势。

| 参数 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `conversion` | integer | 5-20 | 9 | 转换线周期 |
| `base` | integer | 10-60 | 26 | 基准线周期 |
| `span` | integer | 20-120 | 52 | 先行带 B 周期 |
| `displ` | integer | 5-60 | 26 | 位移周期 |

---

### 2.2 均值回归类 (mean_reversion)

均值回归策略假设价格偏离均值后会回归，适合震荡行情。

#### RSI (rsi)

相对强弱指数策略，在超买超卖区域寻找反转信号。

| 参数 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `period` | integer | 5-50 | 14 | RSI 计算周期 |
| `overbought` | float | 65-90 | 75 | 超买阈值（高于此值卖出） |
| `oversold` | float | 10-35 | 25 | 超卖阈值（低于此值买入） |

> **约束:** `oversold < overbought`
>
> **建议:** 默认值适用于多数交易对。趋势强时建议调高 `overbought`（如 80）和调低 `oversold`（如 20）。

#### Mean Reversion (mean_reversion)

统计均值回归策略，使用标准差带判断价格偏离程度。

| 参数 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `period` | integer | 5-100 | 20 | 回看周期 |
| `entry_std` | float | 1-3 | 2.0 | 入场标准差倍数 |
| `exit_std` | float | 0-1 | 0.5 | 出场标准差倍数 |

> **约束:** `exit_std < entry_std`
>
> **说明:** 当价格偏离均值超过 `entry_std` 倍标准差时入场，当价格回到距离均值 `exit_std` 倍标准差时出场。

---

### 2.3 波动率类 (volatility)

基于市场波动率指标进行交易，利用通道和波动率变化。

#### Bollinger Bands (bollinger)

布林带均值回归策略，利用上下轨的超买超卖信号。

| 参数 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `period` | integer | 5-100 | 20 | 布林带计算周期 |
| `std_dev` | float | 1-4 | 2.0 | 标准差倍数 |

> **建议:** `std_dev=2.0` 适用于大多数场景。`std_dev=1.5` 适合短线，信号更多但假信号也多。

#### Keltner Channels (keltner)

肯特纳通道策略，基于 ATR（平均真实波幅）构建通道。

| 参数 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `period` | integer | 5-100 | 20 | Keltner 计算周期 |
| `atr_multiplier` | float | 1-3 | 1.5 | ATR 倍数 |

#### ATR Stop Loss (atr_stop)

基于 ATR 的动态止损策略，根据市场波动率自动调整止损距离。

| 参数 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `period` | integer | 5-50 | 14 | ATR 计算周期 |
| `multiplier` | float | 1-5 | 3.0 | 止损距离倍数 |

> **建议:** 保守交易者使用更高倍数（如 4-5），激进者使用较低倍数（如 1.5-2）。

---

### 2.4 复合类 (composite)

#### Double Bollinger (double_bollinger)

双布林带策略，使用内外两层通道实现分级交易。

| 参数 | 类型 | 范围 | 默认值 | 说明 |
|------|------|------|--------|------|
| `period` | integer | 5-100 | 20 | 计算周期 |
| `inner_std` | float | 1-2.5 | 1.5 | 内层标准差倍数 |
| `outer_std` | float | 2-4 | 2.5 | 外层标准差倍数 |

> **约束:** `inner_std < outer_std`
>
> **说明:** 价格突破外轨时视为强信号，突破内轨时视为弱信号。

---

## 3. 策略状态说明

每个策略有 4 种状态，构成完整的生命周期:

### 3.1 状态流转图

```
                    ┌──────────────────────────────────────────────────┐
                    │                                                  │
                    ▼                                                  │
   ┌───────┐    active    ┌────────┐    paused     ┌─────────┐        │
   │ draft │ ──────────→ │ active │ ←──────────→ │ paused  │ ──→ stopped
   └───────┘              └────────┘              └─────────┘
                                                      │
                                                      │ (也可直接停止)
                                                      ▼
                                                  ┌─────────┐
                                                  │ stopped │ (最终态)
                                                  └─────────┘
```

### 3.2 状态行为详解

| 状态 | 含义 | 允许的操作 |
|------|------|-----------|
| **draft** | 草稿，策略已创建但未启动。可以在参数配置阶段反复修改 | 编辑参数、启动 (→ active) |
| **active** | 运行中，策略信号正在被引擎执行 | 暂停 (→ paused) |
| **paused** | 暂停，策略逻辑停止但保留当前状态和数据。恢复运行继续累计 | 恢复 (→ active)、终止 (→ stopped) |
| **stopped** | 终止，策略完全停止。**不可逆**，如需重启需创建新策略 | 无（查看历史） |

**不允许的转换:**

| 尝试 | 结果 |
|------|------|
| draft → paused | ❌ 必须先启动 |
| draft → stopped | ❌ 必须先启动 |
| active → draft | ❌ 运行中不能回到草稿 |
| active → stopped | ❌ 必须先暂停 |
| stopped → 任何其他状态 | ❌ stopped 为最终态 |

---

## 4. 参数配置最佳实践

### 4.1 通用原则

1. **从默认值开始:** 每个模板有经过测试的默认参数，可作为良好起点
2. **按时间周期调参:** 参数应匹配 K 线周期 —— 短线（5m/15m）用小参数，长线（4h/1d）用大参数
3. **单变量调试:** 每次只修改一个参数，观察效果后再调整下一个
4. **记录变化:** 建议记录每个参数组合的表现，形成自己的调参日志

### 4.2 场景化建议

| 交易场景 | 推荐模板 | 参数策略 |
|----------|---------|---------|
| 强势上涨趋势 | MA Crossover / Triple MA | 使用较小 `fast_period` 快速捕捉趋势 |
| 震荡盘整行情 | RSI / Mean Reversion | 使用标准或略严的阈值（超买 75/超卖 25） |
| 高波动行情 | Bollinger / ATR Stop | 提高 `std_dev` 或 `multiplier` 避免过于敏感 |
| 新手入门 | MA Crossover / Bollinger | 参数少，逻辑直观 |
| 进阶组合 | Double Bollinger / Ichimoku | 多参数协同，信号更可靠 |

### 4.3 避免常见错误

| 错误 | 说明 | 正确做法 |
|------|------|---------|
| 过度优化 | 参数拟合历史数据，无法泛化 | 使用无交集的数据集验证，保持参数简洁 |
| 参数极端化 | 如 `fast_period=5` 且 `slow_period=200` 跨度太大 | 保持在合理范围内，避免物理意义矛盾 |
| 忽略约束关系 | 如 `fast_period >= slow_period` | 创建前确认参数间的约束条件 |
| 名称混淆 | 多策略使用相似名称 | 使用 `{模板名}_{时间}_{序列}` 命名规范 |

---

## 5. 常见问题

### Q: 创建策略后参数还能修改吗？

可以。在 `draft` 或 `paused` 状态下可以调用更新 API 修改参数。`active` 状态下的策略需要先暂停才能修改参数。

### Q: 为什么无法删除某个策略？

确保该策略不属于当前用户。删除操作只验证所有权，不检查状态 —— 任何状态下的策略都可以删除。

### Q: 策略被 `stopped` 后还能恢复吗？

不能。`stopped` 是最终态。如果需重新运行，请创建新策略。

### Q: 如何确认 `template_type` 参数是否正确？

调用 `GET /api/v1/strategies/templates` 获取所有模板 ID，从中选择。

### Q: 分页查询每次最多能取多少条？

每页最大 100 条，默认 20 条。通过 `size` 参数控制。

### Q: 参数校验失败时如何定位问题？

错误响应中会包含具体原因，例如:

```json
{
  "code": 40002,
  "message": "Parameter validation failed: fast_period must be >= 5"
}
```

查阅 [模板参考](#2-策略模板详解) 中的参数范围约束，或调用模板列表 API 查看每个参数的 `min` / `max` 值。
