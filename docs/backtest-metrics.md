# 回测绩效指标定义

> 本文档详细说明回测引擎输出的各项绩效指标的含义和计算公式。

---

## 指标总览

| 指标 | 字段名 | 单位 | 方向 | 简述 |
|------|--------|------|------|------|
| 总收益率 | `total_return_pct` | % | 越高越好 | 回测期间的总体盈亏百分比 |
| 年化收益率 | `annualized_return_pct` | % | 越高越好 | 折算为年化的收益率 |
| 最大回撤 | `max_drawdown_pct` | % | 负值，越小（绝对值越大）越差 | 权益从峰值到谷值的最大跌幅 |
| Sharpe 比率 | `sharpe_ratio` | — | 越高越好 | 风险调整后收益（总波动率） |
| Sortino 比率 | `sortino_ratio` | — | 越高越好 | 风险调整后收益（下行波动率） |
| Calmar 比率 | `calmar_ratio` | — | 越高越好 | 年化收益与最大回撤之比 |
| 胜率 | `win_rate` | % | 越高越好 | 盈利交易占总交易的比例 |
| 总交易次数 | `total_trades` | 次 | — | 回测期间完成的交易总数 |
| 盈亏比 | `profit_factor` | — | > 1 盈利，< 1 亏损 | 总盈利与总亏损之比 |
| 平均盈利 | `avg_win_pct` | % | 越高越好 | 盈利交易的平均收益率 |
| 平均亏损 | `avg_loss_pct` | % | 绝对值越小越好 | 亏损交易的平均收益率 |
| 平均交易收益 | `avg_trade_pct` | % | 越高越好 | 所有交易的平均收益率 |
| 总手续费 | `total_fees` | USDT | 越低越好 | 所有交易的手续费总额 |
| 总滑点成本 | `total_slippage` | USDT | 越低越好 | 所有交易的滑点成本总额 |

---

## 收益类指标

### 总收益率 (total_return_pct)

$$
\text{total\_return\_pct} = \frac{E_{final} - E_{initial}}{E_{initial}} \times 100
$$

其中：
- $E_{final}$ = 最终权益（权益曲线最后一个点）
- $E_{initial}$ = 初始资金 (`initial_capital`)

**解读**：正值表示盈利，负值表示亏损。如 +25.3% 表示初始 100,000 USDT 最终变为 125,300 USDT。

---

### 年化收益率 (annualized_return_pct)

$$
\text{annualized\_return\_pct} = \left( (1 + r_{total})^{\frac{365}{D}} - 1 \right) \times 100
$$

其中：
- $r_{total}$ = 总收益率（小数形式，如 0.253）
- $D$ = 回测天数 = `(最后时间戳 - 首个时间戳) / 86,400,000`

**解读**：将回测期间的总收益折算为年化收益，便于不同时间跨度的策略对比。

**注意**：若回测天数 ≤ 0，年化收益率为 0。

---

## 风险类指标

### 最大回撤 (max_drawdown_pct)

$$
\text{max\_drawdown\_pct} = \min_{i} \left( \frac{E_i - \text{peak}_i}{\text{peak}_i} \right) \times 100
$$

其中：
- $E_i$ = 第 $i$ 个权益点的权益值
- $\text{peak}_i$ = 到第 $i$ 个点为止的历史最高权益值（$\text{peak}_i = \max(E_0, E_1, \ldots, E_i)$）

**解读**：始终为负值或 0。如 -12.5% 表示权益从峰值最多下跌了 12.5%。这是衡量策略风险的最重要指标之一。

**计算过程**：单次遍历权益曲线，维护运行峰值，O(n) 时间复杂度。

---

### Sharpe 比率 (sharpe_ratio)

$$
\text{sharpe\_ratio} = \frac{\bar{r}}{\sigma_r} \times \sqrt{365}
$$

其中：
- $\bar{r}$ = 期间收益率的均值 = $\frac{1}{n} \sum_{i=1}^{n} r_i$
- $\sigma_r$ = 期间收益率的标准差 = $\sqrt{\frac{1}{n-1} \sum_{i=1}^{n} (r_i - \bar{r})^2}$
- $r_i = \frac{E_i - E_{i-1}}{E_{i-1}}$ = 相邻权益点之间的收益率
- $\sqrt{365}$ = 年化因子（将日频 Sharpe 转为年化）

**解读**：
- > 2.0：优秀
- 1.0 ~ 2.0：良好
- 0 ~ 1.0：一般
- < 0：策略亏损

**注意**：Sharpe 比率对上行和下行波动一视同仁。高波动但盈利的策略可能 Sharpe 较低。

---

### Sortino 比率 (sortino_ratio)

$$
\text{sortino\_ratio} = \frac{\bar{r}}{\sigma_{down}} \times \sqrt{365}
$$

其中：
- $\bar{r}$ = 期间收益率的均值（同 Sharpe）
- $\sigma_{down}$ = 下行标准差 = $\sqrt{\frac{1}{n-1} \sum_{r_i < 0} (r_i - \bar{r})^2}$
- 只计入 $r_i < 0$ 的收益率

**解读**：Sortino 比率只惩罚下行波动，对盈利策略更友好。通常 Sortino > Sharpe，差异越大说明上行波动占比越高。

---

### Calmar 比率 (calmar_ratio)

$$
\text{calmar\_ratio} = \frac{\text{annualized\_return\_pct}}{|\text{max\_drawdown\_pct}|}
$$

**解读**：年化收益与最大回撤的比值。Calmar = 2.0 意味着每承受 1% 的最大回撤，获得 2% 的年化收益。通常 > 1.0 为可接受。

**注意**：若最大回撤为 0（权益从未下跌），Calmar 比率为 0。

---

## 交易类指标

### 胜率 (win_rate)

$$
\text{win\_rate} = \frac{N_{win}}{N_{total}} \times 100
$$

其中：
- $N_{win}$ = `pnl_usdt > 0` 的交易数量
- $N_{total}$ = 总交易数量

**解读**：胜率本身不能单独评价策略，需结合盈亏比。高胜率 + 低盈亏比可能不如低胜率 + 高盈亏比。

---

### 盈亏比 (profit_factor)

$$
\text{profit\_factor} = \frac{\sum |pnl_i| \text{ where } pnl_i > 0}{\sum |pnl_i| \text{ where } pnl_i \leq 0}
$$

**解读**：
- > 1.5：良好
- = 1.0：盈亏相抵
- < 1.0：亏损策略
- `Infinity`：所有交易均盈利（总亏损为 0）

---

### 平均盈利 (avg_win_pct)

$$
\text{avg\_win\_pct} = \frac{1}{N_{win}} \sum_{pnl_i > 0} pnl\_pct_i
$$

### 平均亏损 (avg_loss_pct)

$$
\text{avg\_loss\_pct} = \frac{1}{N_{loss}} \sum_{pnl_i \leq 0} pnl\_pct_i
$$

### 平均交易收益 (avg_trade_pct)

$$
\text{avg\_trade\_pct} = \frac{1}{N_{total}} \sum_{i} pnl\_pct_i
$$

---

## 成本类指标

### 总手续费 (total_fees)

$$
\text{total\_fees} = \sum_{i=1}^{N_{total}} fee_i
$$

每笔交易的 `fee` 包含开仓手续费和平仓手续费。

### 总滑点成本 (total_slippage)

$$
\text{total\_slippage} = \sum_{i=1}^{N_{total}} slippage_i
$$

每笔交易的 `slippage` 包含开仓滑点和平仓滑点。

**解读**：手续费和滑点是策略的现实成本。高频策略（交易次数多）需特别关注，过高的成本可能抵消策略收益。

---

## 指标参考标准

| 指标 | 较差 | 一般 | 良好 | 优秀 |
|------|------|------|------|------|
| Sharpe Ratio | < 0 | 0 ~ 1 | 1 ~ 2 | > 2 |
| Sortino Ratio | < 0 | 0 ~ 1.5 | 1.5 ~ 3 | > 3 |
| Max Drawdown | > -30% | -20% ~ -30% | -10% ~ -20% | < -10% |
| Profit Factor | < 1.0 | 1.0 ~ 1.5 | 1.5 ~ 2.0 | > 2.0 |
| Win Rate | < 40% | 40% ~ 50% | 50% ~ 60% | > 60% |
| Calmar Ratio | < 0.5 | 0.5 ~ 1.0 | 1.0 ~ 2.0 | > 2.0 |

> 以上标准仅供参考，实际评判需结合策略类型、交易频率和市场环境。
