# 回测引擎使用说明

> 本文档介绍回测引擎的工作原理、使用流程和最佳实践。

---

## 目录

- [概述](#概述)
- [核心概念](#核心概念)
- [使用流程](#使用流程)
- [策略信号机制](#策略信号机制)
- [资金与交易模拟](#资金与交易模拟)
- [进度与取消](#进度与取消)
- [权益曲线与数据采样](#权益曲线与数据采样)
- [合成 K 线数据](#合成-k-线数据)
- [最佳实践](#最佳实践)
- [常见问题](#常见问题)

---

## 概述

回测引擎（BacktestEngine）是量化交易系统的核心模块，用于验证交易策略在历史数据上的表现。引擎采用**全内存向量化计算**架构，单次遍历 K 线数据完成策略回放和权益计算，O(n) 时间复杂度。

### 关键特性

- **异步执行**：回测任务在独立线程池中运行，API 立即返回，不阻塞请求
- **并发控制**：Semaphore 信号量限制同时运行 5 个回测任务
- **实时进度**：通过 `AtomicU32` 每 ~10% 更新一次进度
- **可取消**：基于 `CancellationToken`，每 100 根 K 线检查取消信号
- **双向交易**：支持做多（long）和做空（short）
- **手续费与滑点**：真实的交易成本模拟
- **爆仓检测**：权益归零时自动终止回测

---

## 核心概念

### BacktestConfig（回测配置）

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `symbol` | 交易对，如 `BTC/USDT` | — |
| `interval` | K 线周期 | — |
| `start_date` / `end_date` | 回测时间范围 | — |
| `initial_capital` | 初始资金（USDT） | — |
| `fee_rate` | 手续费率 | `0.001`（0.1%） |
| `slippage_rate` | 滑点率 | `0.0005`（0.05%） |

### StrategyTemplate（策略模板）

策略模板定义了交易信号的生成逻辑。每个模板实现 `StrategyTemplate` trait 的 `generate_signal()` 方法，接收完整 K 线序列和当前索引，返回交易信号。

系统提供 10 个预定义模板，覆盖四大类别：

| 类别 | 模板 |
|------|------|
| 趋势 (trend) | MA Crossover、Triple MA、MACD、Ichimoku Cloud |
| 均值回归 (mean_reversion) | RSI、Mean Reversion |
| 波动率 (volatility) | Bollinger Bands、Keltner Channels、ATR Stop Loss |
| 复合 (composite) | Double Bollinger Bands |

### 信号类型 (Signal)

| 信号 | 说明 |
|------|------|
| `Buy { quantity_pct }` | 做多，使用 `quantity_pct`（0.0 ~ 1.0）比例的可用资金 |
| `Sell { quantity_pct }` | 做空，使用 `quantity_pct` 比例的可用资金 |
| `CloseAll` | 平掉当前所有持仓 |
| `Hold` | 不操作，继续持有或空仓 |

---

## 使用流程

### 第 1 步：创建策略

在运行回测前，需要先创建一个策略（基于某个模板）。参考策略管理 API 创建策略。

```bash
# 创建 MA Crossover 策略
curl -X POST http://localhost:3000/api/v1/strategies \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "BTC 双均线策略",
    "template_type": "ma_crossover",
    "parameters": {"fast_period": 10, "slow_period": 30}
  }'
```

### 第 2 步：运行回测

使用策略 ID 和回测配置启动回测。

```bash
curl -X POST http://localhost:3000/api/v1/backtest \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "strategy_id": "<上一步返回的策略 ID>",
    "config": {
      "symbol": "BTC/USDT",
      "interval": "1h",
      "start_date": "2024-01-01",
      "end_date": "2024-12-31",
      "initial_capital": 100000.0
    }
  }'
```

返回示例：
```json
{
  "code": 0,
  "data": {
    "id": "660e8400-...",
    "status": "running",
    "progress": 0,
    "created_at": "2026-05-13T07:00:00Z"
  }
}
```

### 第 3 步：轮询进度

回测执行期间，可轮询结果接口检查进度：

```bash
curl http://localhost:3000/api/v1/backtest/660e8400-... \
  -H "Authorization: Bearer <access_token>"
```

当 `status` 变为 `completed` 时，`metrics`/`trades`/`equity_curve` 将被填充。

> 也可以通过 WebSocket 接收 `backtest_progress` 和 `backtest_completed` 事件。

### 第 4 步：查看结果

回测完成后，分别获取指标、交易和权益数据：

```bash
# 获取完整结果
curl http://localhost:3000/api/v1/backtest/660e8400-... \
  -H "Authorization: Bearer <access_token>"

# 仅获取交易明细
curl http://localhost:3000/api/v1/backtest/660e8400-.../trades \
  -H "Authorization: Bearer <access_token>"

# 仅获取权益曲线
curl http://localhost:3000/api/v1/backtest/660e8400-.../equity \
  -H "Authorization: Bearer <access_token>"
```

### 第 5 步：取消回测（可选）

如果需要中途取消回测：

```bash
curl -X POST http://localhost:3000/api/v1/backtest/660e8400-.../cancel \
  -H "Authorization: Bearer <access_token>"
```

---

## 策略信号机制

### 信号处理逻辑

引擎逐根 K 线处理，根据策略信号执行相应操作：

| 当前状态 | 收到信号 | 引擎行为 |
|----------|---------|---------|
| 空仓 | `Buy` | 开多仓 |
| 空仓 | `Sell` | 开空仓 |
| 持多仓 | `Sell` | 平多仓 → 开空仓 |
| 持空仓 | `Buy` | 平空仓 → 开多仓 |
| 持仓 | `CloseAll` | 平仓 |
| 任何状态 | `Hold` | 检查止损/止盈，无操作 |

### 止损与止盈

在 `Hold` 信号时，引擎检查当前持仓的止损/止盈触发条件：

- **多头止损**：K 线最低价 ≤ 止损价
- **空头止损**：K 线最高价 ≥ 止损价
- **多头止盈**：K 线最高价 ≥ 止盈价
- **空头止盈**：K 线最低价 ≤ 止盈价

> 止损/止盈价格由策略模板在生成信号时设定。当前模板实现中暂未设置止损止盈，后续版本将支持。

---

## 资金与交易模拟

### 开仓（做多）

1. 计算滑点：`slippage = close_price × slippage_rate`
2. 执行价格：`exec_price = close_price + slippage`（买入价上浮）
3. 可用资金：`allocated = cash × quantity_pct`
4. 买入数量：`quantity = floor(allocated / exec_price)`
5. 交易成本：`cost = quantity × exec_price`
6. 手续费：`fee = cost × fee_rate`
7. 扣减现金：`cash -= cost + fee`

### 开仓（做空）

1. 计算滑点：`slippage = close_price × slippage_rate`
2. 执行价格：`exec_price = close_price - slippage`（卖出价下浮）
3. 卖出数量：`quantity = floor(allocated / exec_price)`
4. 收到金额：`received = quantity × exec_price`
5. 手续费：`fee = received × fee_rate`
6. 增加现金：`cash += received - fee`

### 平仓

- **多头平仓**：`exit_price = current_price - slippage`（卖出价下浮）
- **空头平仓**：`exit_price = current_price + slippage`（买回价上浮）
- 盈亏计算：
  - 多头：`pnl = quantity × exit_price - quantity × entry_price`
  - 空头：`pnl = quantity × entry_price - quantity × exit_price`

### 标记价格（Mark-to-Market）

持仓期间的权益按当前 K 线收盘价计算：

- **多头**：`equity = cash + quantity × close_price`
- **空头**：`equity = cash + quantity × (2 × entry_price - close_price)`

---

## 进度与取消

### 进度更新

- 引擎每处理 ~10% 的 K 线更新一次进度（`AtomicU32`）
- 进度范围：0 ~ 99（运行中），100（完成）
- 前端建议轮询间隔：2 秒

### 取消机制

- 引擎每 100 根 K 线检查一次 `CancellationToken`
- 收到取消信号后，引擎立即停止并返回 `"cancelled"` 错误
- 取消的回测记录状态更新为 `failed`，`error` 字段为 `"cancelled"`

---

## 权益曲线与数据采样

### 权益曲线

每个 K 线周期记录一个权益点（`EquityPoint`），包含：

- `time`：K 线开盘时间（毫秒时间戳）
- `equity`：账户权益（USDT）
- `drawdown_pct`：当前回撤百分比

### 数据采样

当权益曲线数据点超过 2000 个时，`GET /backtest/{id}/equity` 接口自动均匀采样：

- 采样算法：每隔 `N/2000` 个点取一个，始终保留第一个和最后一个数据点
- 采样后的曲线最多包含 2002 个点
- 完整数据仍保存在数据库中，可通过 `GET /backtest/{id}` 获取

---

## 合成 K 线数据

当数据库中不存在指定交易对/周期/时间范围的 K 线数据时，引擎自动生成合成 K 线数据用于演示和测试。

合成数据特征：
- 价格从 100 USDT 起始，随机游走（漂移偏多）
- K 线数量 = 天数 × 每日 K 线数
- 每根 K 线的 OHLCV 按随机波动生成
- 价格不低于 1 USDT

> 合成数据仅用于功能验证，不反映真实市场行为。生产环境请接入真实行情数据。

---

## 最佳实践

### 回测配置

1. **时间范围**：建议至少 3 个月以上，7 天为最低要求
2. **初始资金**：根据交易对价格合理设置，避免因资金不足导致无法开仓
3. **手续费率**：不同交易所费率不同，Binance 默认 0.1%（`0.001`），建议按实际费率设置
4. **滑点率**：流动性好的交易对可设 0.05%（`0.0005`），小币种建议提高到 0.1% ~ 0.3%

### 策略参数

1. **均线周期**：短期参数对噪音更敏感，长期参数更平滑但滞后
2. **RSI 阈值**：默认超买 75 / 超卖 25 较保守，可适当调整
3. **多策略对比**：对同一交易对运行不同策略，横向比较 Sharpe 和最大回撤

### 结果解读

1. **Sharpe Ratio**：> 1.0 为良好，> 2.0 为优秀
2. **Max Drawdown**：-20% 以内为可接受范围，超过 -30% 需要重新评估策略
3. **Win Rate**：不单独看胜率，需结合盈亏比（`avg_win_pct / |avg_loss_pct|`）
4. **Profit Factor**：> 1.5 为良好，< 1.0 意味着亏损

---

## 常见问题

### Q: 回测一直处于 running 状态？

**A:** 检查以下几点：
1. 回测时间范围是否过大（如 1m 周期 + 1 年数据 = ~500K 根 K 线）
2. 数据库连接是否正常
3. 可以通过 `POST /backtest/{id}/cancel` 取消卡住的任务

### Q: 运行回测返回并发超限错误？

**A:** 系统限制同时运行 5 个回测任务。请等待已有任务完成或取消后再试。

### Q: 回测结果显示 0 笔交易？

**A:** 可能原因：
1. 策略参数与 K 线周期不匹配（如均线周期 50 但数据只有 30 根 K 线）
2. 价格在观察期内未触发信号
3. 初始资金不足以开仓（交易对价格过高）

### Q: 做空交易的盈亏计算逻辑是什么？

**A:** 做空时，借入资产卖出获得现金；平仓时以当前价格买回。盈利条件是平仓价格 < 开仓价格。`pnl_usdt = quantity × (entry_price - exit_price)`。

### Q: 合成 K 线数据与真实数据的差异？

**A:** 合成数据使用随机游走生成，不包含真实市场的趋势延续、波动率聚集、成交量模式等特征。合成数据仅用于 API 功能验证，不可用于策略评估。
