# PRD: 策略回测引擎 — 历史K线回测与绩效分析

> 版本: v1.1 (更新版)
> 状态: Draft
> 作者: PM
> 基于: ADR-004 (回测引擎架构决策), ADR-007 (回测引擎实施决策)
> 关联: PRD-strategy-sandbox-mvp.md (US-SS-01~SS-06), TECH_CHARTER-quant.md

---

## 1. 背景

主 PRD (v1.0) 中回测引擎被列为核心差异化功能。ADR-004 已在架构层面决策采用"全内存向量化计算"方案——一次性加载 K 线数据到内存，逐根执行策略逻辑，纯 CPU 计算，无 IO 等待。

### 当前状态 (2026-05-13)

**已完成的后端实现:**
- `handlers/backtest.rs`: 6 个 API 端点（run/result/progress/cancel/history/delete）
- `services/backtest_engine.rs`: 回测引擎核心（信号生成、逐根K线计算、SL/TP、爆仓检测、14 项绩效指标）
- `models/backtest.rs`: 完整类型系统（Kline/Signal/TradeRecord/EquityPoint/BacktestMetrics）
- `services/strategy.rs`: StrategyTemplate trait 扩展 generate_signal，10 个模板已实现
- 并发控制: tokio Semaphore (5并发) + CancellationToken + spawn_blocking
- 10 个策略模板已实现 generate_signal

**已完成的前端实现:**
- `BacktestView.vue`: 配置模式 + 结果模式双视图，含空状态/运行中/错误态
- `BacktestConfigForm.vue`: 策略选择下拉框、交易对、初始资金、日期范围
- `BacktestMetricsCards.vue`: 6 个 KPI 卡片（总收益率、年化收益率、夏普比率、最大回撤、胜率、交易次数）
- `BacktestEquityChart.vue`: ECharts 权益曲线，渐变色填充，tooltip 交互
- `BacktestTradesTable.vue`: 交易明细表格，盈亏颜色标记，排序，CSV 导出预留
- `api/backtest.ts`: API 封装（run/result/job/history/delete）
- `types/backtest.ts`: TypeScript 类型定义
- 7 个测试文件覆盖主要组件

**待完成 (TODO):**
- `db/backtest.rs`: 所有 SeaORM 数据库操作为 TODO 占位（create/update/find/delete）
- CancellationToken 注册表未实现（DashMap<Uuid, CancellationToken>）
- WS 进度推送未实现（前端使用轮询 polling 替代）
- 回撤曲线图未实现（只有权益曲线）
- 回测历史标签页未实现
- 配置表单缺少 `interval` / `fee_rate` / `slippage_rate` 字段
- "高级设置"折叠面板未实现

### 为何这个功能重要

回测是量化交易的核心：它在无风险的环境中验证策略的盈利能力、风险特征和稳健性，避免"纸上交易"的假象。不可靠的回测比没有回测更危险——误差必须控制在可接受范围内。

---

## 2. 目标

| 目标 | 指标 | 当前状态 |
|------|------|---------|
| 回测执行 | 1 年 1h 数据回测时间 < 5s (ADR-004 承诺) | 引擎已实现，待 DB 层集成后验证 |
| 性能指标精度 | 夏普比率计算误差 < 5% | O(n) 向量化计算已实现 |
| 核心指标覆盖 | ≥ 8 个绩效指标 | 14 个指标已实现 |
| 结果可视化 | 权益曲线 + 回撤曲线 + 交易标注 | 权益曲线完成，回撤曲线 TODO |
| 并发回测 | 单用户同时最多 5 个回测任务 | Semaphore 已实现 |
| 进度推送 | WS 每 10% 推送进度事件 | 前端轮询替代（P1），WS 推送 TODO |
| DB 持久化 | 结果完整写入 backtest_results 表 | TODO（db/backtest.rs 占位） |

### 非目标 (Non-Goals)

- ❌ 参数优化/网格搜索（第二阶段）
- ❌ 参数敏感度热力图（第二阶段）
- ❌ 蒙特卡洛模拟（第三阶段）
- ❌ 多品种/投资组合回测（第三阶段）
- ❌ 并行对比回测（第三阶段）
- ❌ 逐笔 level-2 数据回测（ADR-004 已排除）
- ❌ 历史回测结果的统计分析/比较面板（P2+）

---

## 3. 用户角色与权限

| 角色 | 回测权限 | 说明 |
|------|---------|------|
| **trader** (普通用户) | 基于自有策略运行回测，查看结果 | 模拟模式 |
| **pro-trader** (专业交易员) | 同 trader，无额外回测权限 | 实盘权限预埋 |
| **admin** (管理员) | 查看所有用户的回测记录 | 审计用途，可取消任意回测任务 |

> 所有角色均无法编辑他人策略进行回测——回测必须基于用户自有策略。

---

## 4. 用户故事 (User Stories)

### US-BT-01: 运行回测 (P0)

> **As a** 交易者
> **I want to** 从我的策略列表中选择一个策略并配置回测参数来运行回测
> **So that** 我可以验证策略在历史行情数据上的表现

**验收条件：**

```gherkin
Feature: 运行回测
  Background:
    Given 用户已登录
    And 用户拥有至少 1 个含有有效参数的策略

  Scenario: 成功启动回测
    Given 导航至 /backtest 页面
    When 从策略选择下拉框中选择一个策略
    And 设置回测参数:
      | 参数 | 值 |
      | 交易对 | BTC/USDT |
      | 起始日期 | 2024-01-01 |
      | 结束日期 | 2024-12-31 |
      | 初始资金 | 100,000 USDT |
    When 点击"运行回测"按钮
    Then 显示进度条/加载状态
    And 后端启动回测任务
    And 回测完成后自动展示结果

  Scenario: 参数验证失败阻止运行
    Given 选择策略后
    When 起始日期晚于结束日期
    Then "运行回测"按钮禁用
    And 显示错误提示"起始日期不能晚于结束日期"
    When 初始资金设置为 0
    Then 显示错误提示"初始资金必须大于 0"

  Scenario: 前端表单验证
    Given 回测配置页面
    When 未选择策略
    Then "运行回测"按钮禁用
    When 交易对为空
    Then 显示"请输入交易对"错误提示
```

**实现状态:** ✅ 后端 handler 完成；✅ ConfigForm 完成（含表单验证）；⚠️ 缺少 interval/fee_rate/slippage_rate 字段（当前版本简化）；⚠️ DB 层为 TODO 占位

### US-BT-02: 查看回测绩效指标 (P0)

> **As a** 交易者
> **I want to** 查看回测完成后的绩效指标摘要
> **So that** 我可以快速评估策略的盈利能力与风险

**验收条件：**

```gherkin
Feature: 查看绩效指标
  Background:
    Given 用户已登录
    And 最近一次回测已完成

  Scenario: 查看回测结果摘要
    Given 导航至回测结果页面
    Then KPI 卡片区显示以下指标:
      | 指标 | 示例 |
      | 总收益率 | +25.3% |
      | 年化收益率 | +22.1% |
      | 夏普比率 | 1.85 |
      | 最大回撤 | -12.5% |
      | 胜率 | 62.0% |
      | 交易次数 | 85 |
    And 正收益为绿色（#22c55e），负收益为红色（#ef4444）
    And 支持骨架屏加载态和空数据态

  Scenario: 无交易结果
    Given 回测完成但未产生任何交易
    Then KPI 卡片渲染为零值
    And 显示空状态组件
```

**实现状态:** ✅ MetricsCards 完成（6 个指标，含 loading/empty 状态，正负颜色标记）

### US-BT-03: 查看权益曲线与回撤图 (P0)

> **As a** 交易者
> **I want to** 查看回测的权益曲线和资金回撤图
> **So that** 我可以直观理解策略的收益波动和风险暴露

**验收条件：**

```gherkin
Feature: 权益曲线与回撤图
  Background:
    Given 回测已完成并显示结果页面

  Scenario: 权益曲线展示
    Then 显示权益曲线折线图（时间序列）
    And X 轴为日期/时间
    And Y 轴为账户权益（USDT）
    And 曲线使用渐变色填充
    And 正收益曲线为绿色，负收益为红色
    And 支持骨架屏加载态和空数据态

  Scenario: 回撤曲线 (TODO)
    Then 在权益曲线下方显示回撤曲线（面积图，红色/灰色填充）
    And X 轴与权益曲线同步
    And Y 轴为回撤百分比（0% ~ -X%）
    And 鼠标悬停时显示当前日期的回撤值（待实现）

  Scenario: 图表交互
    Given 权益曲线已渲染
    When 鼠标悬停
    Then tooltip 显示日期和权益值
```

**实现状态:** ✅ EquityChart 完成（ECharts line chart + gradient fill + tooltip + skeleton + empty）；⚠️ 回撤曲线图 TODO；⚠️ 交易标注（买卖点 markers）TODO；⚠️ dataZoom 缩放 TODO

### US-BT-04: 查看回测交易记录 (P1)

> **As a** 交易者
> **I want to** 查看回测产生的所有交易明细
> **So that** 我可以逐笔核对策略的交易逻辑

**验收条件：**

```gherkin
Feature: 查看交易记录
  Background:
    Given 回测已完成
    And 回测产生了 ≥ 1 笔交易

  Scenario: 交易列表展示
    Given 查看回测结果页面
    Then 显示交易明细表格
    And 每行包含:
      | 列 | 说明 |
      | 开仓时间 | 日期时间 |
      | 方向 | 多头/空头 |
      | 开仓价 | 数值 |
      | 平仓价 | 数值 |
      | 盈亏 | +1,540 USDT |
      | 盈亏% | +3.64% |
    And 盈利行颜色标记为绿色
    And 亏损行颜色标记为红色
    And 支持按盈亏列排序
    And 支持骨架屏加载态和空数据态

  Scenario: 导出功能
    Given 有交易记录
    When 点击"导出结果"按钮
    Then 生成 CSV 文件下载（当前为占位，提示"导出功能开发中"）
```

**实现状态:** ✅ TradesTable 完成（el-table + 盈亏颜色 + 排序 + 导出按钮/占位）；⚠️ CSV 导出未实现

### US-BT-05: 查看回测历史列表 (P1)

> **As a** 交易者
> **I want to** 查看某策略所有的历史回测记录
> **So that** 我可以对比不同参数配置下的回测结果

**验收条件：**

```gherkin
Feature: 回测历史列表
  Background:
    Given 用户已登录
    And 某策略有 ≥ 3 次历史回测记录

  Scenario: 查看回测历史
    Given 进入回测页面
    When 切换到"回测历史"标签页
    Then 显示按时间倒序排列的回测记录列表
    And 每行包含: 运行时间、日期范围、初始资金、总收益率、夏普比率、最大回撤、交易次数、状态
    And 点击某行进入该次回测的详细结果页

  Scenario: 空历史
    Given 策略从未运行过回测
    Then 显示"该策略暂无回测记录"
```

**实现状态:** ⚠️ 后端 `list_backtest_history` handler 已完成但 DB 层为 TODO；⚠️ 前端历史标签页未实现

### US-BT-06: 回测进度与取消 (P1)

> **As a** 交易者
> **I want to** 在回测运行时查看进度并能取消长时间运行的回测
> **So that** 我可以避免无谓等待或误操作

**验收条件：**

```gherkin
Feature: 回测进度与取消
  Background:
    Given 用户已启动一个回测任务

  Scenario: 进度反馈
    Given 回测任务正在运行
    Then 显示进度条（indeterminate 动画 + 百分比）
    And 提示"回测运行中... 正在计算策略表现，请稍候"

  Scenario: 回测完成
    Given 回测任务完成
    Then 进度条消失
    And 自动展示结果区域
    And 显示"回测完成"标签

  Scenario: 回测失败提示
    Given 回测因错误中止
    Then 显示错误提示 el-alert
    And 展示具体的错误信息

  Scenario: 取消回测 (TODO)
    Given 回测任务正在运行
    When 点击"取消"按钮
    Then 回测任务被取消（待实现: CancellationToken registry）
```

**实现状态:** ✅ 前端进度反馈完成（el-progress indeterminate + polling 状态检测）；⚠️ 取消按钮未实现（需 CancellationToken registry）；⚠️ WS 推送未实现（当前用轮询）

### US-BT-07: 策略编辑页快速回测 (P2)

> **As a** 交易者
> **I want to** 在策略编辑页面直接启动回测
> **So that** 我可以在修改参数后立即验证效果

**验收条件：**

```gherkin
Feature: 策略编辑页面快速回测
  Background:
    Given 用户正在编辑策略参数

  Scenario: 编辑页启动回测
    Given 在策略编辑页面修改了参数
    When 点击"保存并回测"按钮
    Then 先保存参数
    And 跳转至回测页面并预填该策略和最新参数
    And 自动触发回测运行
```

**实现状态:** ❌ 未开始（P2，优先级最低）

---

## 5. 数据模型

### 5.1 backtest_results 表

数据库 `backtest_results` 表定义如下：

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 回测结果 ID |
| strategy_id | UUID | FK → strategies(id), NOT NULL | 关联策略 |
| user_id | UUID | FK → users(id), NOT NULL | 用户 ID |
| config | JSONB | NOT NULL | 回测配置（见下方结构） |
| status | backtest_status | DEFAULT 'pending' | pending/running/completed/failed/cancelled |
| progress | SMALLINT | DEFAULT 0 | 进度 0-100 |
| start_time | TIMESTAMPTZ | | 实际开始时间 |
| end_time | TIMESTAMPTZ | | 实际结束时间 |
| duration_ms | BIGINT | | 回测耗时（毫秒） |
| metrics | JSONB | | 性能指标 |
| trades | JSONB | | 交易明细数组 |
| equity_curve | JSONB | | 权益曲线数组 |
| error | TEXT | | 错误信息 |
| created_at | TIMESTAMPTZ | DEFAULT NOW() | |

```sql
CREATE TYPE backtest_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled');
```

**config JSONB 结构：**
```json
{
  "date_range": {"start": "2024-01-01", "end": "2024-12-31"},
  "initial_capital": 100000.0,
  "fee_rate": 0.001,
  "slippage_rate": 0.0005,
  "symbol": "BTC/USDT",
  "interval": "1h"
}
```

**metrics JSONB 结构（后端已实现 14 项）：**
```json
{
  "total_return_pct": 25.3,
  "annualized_return_pct": 22.1,
  "max_drawdown_pct": -12.5,
  "sharpe_ratio": 1.85,
  "sortino_ratio": 2.1,
  "calmar_ratio": 1.77,
  "win_rate": 62.0,
  "total_trades": 85,
  "profit_factor": 2.3,
  "avg_win_pct": 3.2,
  "avg_loss_pct": -1.8,
  "avg_trade_pct": 0.3,
  "total_fees": 1250.0,
  "total_slippage": 620.0
}
```

**trades 数组元素结构（后端已实现）：**
```json
{
  "entry_time": 1704067200000,
  "exit_time": 1704153600000,
  "direction": "long",
  "entry_price": 42350.0,
  "exit_price": 43120.0,
  "quantity": 2.0,
  "pnl_usdt": 1540.0,
  "pnl_pct": 3.64,
  "holding_period_ms": 86400000,
  "exit_reason": "signal",
  "fee": 42.35,
  "slippage": 21.18
}
```

exit_reason 枚举: `signal`(信号平仓), `stop_loss`(止损), `take_profit`(止盈), `time_exit`(超时), `liquidation`(爆仓)

**equity_curve 数组元素结构（后端已实现）：**
```json
{
  "time": 1704067200000,
  "equity": 100000.0,
  "drawdown_pct": 0.0
}
```

### 5.2 索引

```sql
CREATE INDEX idx_backtest_strategy_id ON backtest_results(strategy_id);
CREATE INDEX idx_backtest_user_id ON backtest_results(user_id);
CREATE INDEX idx_backtest_created_at ON backtest_results(created_at DESC);
CREATE INDEX idx_backtest_status ON backtest_results(status);
```

### 5.3 后端 Rust 类型（已实现）

```rust
// models/backtest.rs - 全部已完成
pub struct BacktestRunRequest { strategy_id: Uuid, config: BacktestConfig }
pub struct BacktestConfig { symbol, interval, start_date, end_date, initial_capital, fee_rate, slippage_rate } // validate() 已实现
pub struct BacktestRunResponse { id: Uuid, status, progress, created_at }
pub struct BacktestResultResponse { id, strategy_id, status, progress, config, metrics, trades, equity_curve, start_time, end_time, duration_ms, error }
pub struct BacktestProgressResponse { id, status, progress, current_bar, total_bars, elapsed_ms }
pub struct BacktestSummary { id, status, config, metrics_preview, start_time, duration_ms, created_at }
pub struct MetricsPreview { total_return_pct, sharpe_ratio, max_drawdown_pct, total_trades }

// 内部类型
pub struct Kline { open_time, open, high, low, close, volume }
pub enum Signal { Buy{quantity_pct}, Sell{quantity_pct}, CloseAll, Hold }
pub enum Direction { Long, Short }
pub struct Account { initial_capital, cash, equity }
pub struct Position { direction, quantity, entry_price, entry_time, stop_loss, take_profit, fee_paid, slippage_paid }
pub struct TradeRecord { entry_time, exit_time, direction, entry_price, exit_price, quantity, pnl_usdt, pnl_pct, holding_period_ms, exit_reason, fee, slippage }
pub struct EquityPoint { time, equity, drawdown_pct }
pub struct BacktestMetrics { total_return_pct, annualized_return_pct, max_drawdown_pct, sharpe_ratio, sortino_ratio, calmar_ratio, win_rate, total_trades, profit_factor, avg_win_pct, avg_loss_pct, avg_trade_pct, total_fees, total_slippage }
```

---

## 6. API 设计

遵循 TECH_CHARTER 规范：`/api/v1/{resource}` + snake_case JSON。

### 6.1 回测 API 端点

| 方法 | 路径 | 说明 | 优先级 | 实现状态 |
|------|------|------|--------|---------|
| POST | `/api/v1/backtest/run` | 启动回测任务 | P0 | ✅ handler 完成，DB 层 TODO |
| GET | `/api/v1/backtest/{id}/result` | 获取回测结果详情 | P0 | ✅ handler 完成，DB 层 TODO |
| GET | `/api/v1/backtest/{id}/progress` | 获取回测进度 | P1 | ✅ handler 完成，DB 层 TODO |
| POST | `/api/v1/backtest/{id}/cancel` | 取消运行中的回测 | P1 | ⚠️ 框架完成，需 CancellationToken registry |
| GET | `/api/v1/backtest/history` | 获取策略回测历史列表 | P1 | ⚠️ handler 完成，DB 层 TODO |
| DELETE | `/api/v1/backtest/{id}` | 删除回测记录 | P2 | ⚠️ handler 完成，DB 层 TODO |

#### POST /api/v1/backtest/run 请求体

```json
{
  "strategy_id": "550e8400-e29b-41d4-a716-446655440000",
  "config": {
    "symbol": "BTC/USDT",
    "interval": "1h",
    "start_date": "2024-01-01",
    "end_date": "2024-12-31",
    "initial_capital": 100000.0,
    "fee_rate": 0.001,
    "slippage_rate": 0.0005
  }
}
```

**验证规则（BacktestConfig::validate() 已实现）：**
- `start_date` < `end_date`，间隔 ≥ 7 天
- `initial_capital` ≥ 100
- `fee_rate` >= 0 且 <= 0.01
- `slippage_rate` >= 0 且 <= 0.01
- `strategy_id` 必须存在且属于当前用户

#### POST /api/v1/backtest/run 响应

```json
{
  "code": 0,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440001",
    "status": "pending",
    "progress": 0,
    "created_at": "2026-05-13T07:00:00Z"
  },
  "message": "success"
}
```

#### GET /api/v1/backtest/{id}/result 响应

```json
{
  "code": 0,
  "data": {
    "id": "660e8400-...",
    "strategy_id": "550e8400-...",
    "status": "completed",
    "progress": 100,
    "config": { "symbol": "BTC/USDT", "interval": "1h", ... },
    "metrics": {
      "total_return_pct": 25.3,
      "annualized_return_pct": 22.1,
      "max_drawdown_pct": -12.5,
      "sharpe_ratio": 1.85,
      "sortino_ratio": 2.1,
      "calmar_ratio": 1.77,
      "win_rate": 62.0,
      "total_trades": 85,
      "profit_factor": 2.3,
      "avg_win_pct": 3.2,
      "avg_loss_pct": -1.8,
      "avg_trade_pct": 0.3,
      "total_fees": 1250.0,
      "total_slippage": 620.0
    },
    "trades": [ ... ],
    "equity_curve": [ ... ],
    "start_time": "2026-05-13T07:00:01Z",
    "end_time": "2026-05-13T07:00:05Z",
    "duration_ms": 4230,
    "error": null
  },
  "message": "success"
}
```

> equity_curve 采样：后端提供 `sample_equity_curve()` 函数，均匀采样最大点数以避免大响应体。

#### POST /api/v1/backtest/{id}/cancel

请求体：无。路径参数为回测 ID。

实现方式：CancellationToken registry（DashMap<Uuid, CancellationToken>）存储每个运行中回测的 token，调用 `token.cancel()`。

### 6.2 WebSocket 事件推送 (P1, TODO)

通过现有的 WebSocket 连接，推送回测进度事件：

```json
{ "type": "backtest_progress", "data": { "backtest_id": "660e8400-...", "status": "running", "progress": 45 } }
{ "type": "backtest_completed", "data": { "backtest_id": "660e8400-...", "status": "completed", "duration_ms": 4230 } }
{ "type": "backtest_failed", "data": { "backtest_id": "660e8400-...", "status": "failed", "error": "Insufficient kline data" } }
```

> 当前前端使用 HTTP polling（GET /progress，每 2 秒轮询，最多 60 次），WS 推送作为 P1 优化。

### 6.3 错误码

| 错误码 | 常量 | 说明 |
|-------|------|------|
| 42301 | ERR_BACKTEST_INVALID_RANGE | 日期范围无效 |
| 42302 | ERR_BACKTEST_NO_DATA | K线数据不足 |
| 42303 | ERR_BACKTEST_CONCURRENT_LIMIT | 并发回测超限 (5) |
| 40401 | ERR_BACKTEST_NOT_FOUND | 回测记录不存在 |
| 42304 | ERR_BACKTEST_CANCEL_FAILED | 取消失败（非 running 状态） |

---

## 7. 前端页面设计

### 7.1 路由

| 路径 | 组件 | 说明 | 状态 |
|------|------|------|------|
| `/backtest` | `BacktestView.vue` | 回测配置 + 结果页面 | ✅ 已实现 |
| `/history` | 无 | 回测历史（可后续作为独立路由） | ❌ 待定 |

MVP 阶段回测结果在 `BacktestView.vue` 内部切换视图（配置模式/运行中/结果模式），不增加子路由。

### 7.2 页面结构与状态

BacktestView.vue 有四种视图状态：

1. **空状态**（默认）：显示 `el-empty` + "配置参数后点击运行回测"
2. **配置模式**：显示 `BacktestConfigForm`（策略选择下拉搜索框 + 交易对输入 + 初始资金输入 + 日期范围选择器）
3. **运行中**：显示 `el-progress` indeterminate 进度条 + "回测运行中..." 提示
4. **结果模式**：显示 `BacktestMetricsCards` + `BacktestEquityChart` + `BacktestTradesTable`
5. **错误态**：显示 `el-alert` 类型 error，可关闭

### 7.3 组件设计与复用

| 组件 | 职责 | 数据流 | 状态覆盖 |
|------|------|--------|---------|
| `BacktestConfigForm` | 回测参数配置表单 | emit('run') → parent | loading(提交中), empty(首次), normal |
| `BacktestMetricsCards` | 6 个 KPI 指标卡片 | props: result | loading(skeleton), empty, normal |
| `BacktestEquityChart` | ECharts 权益曲线 | props: data, initialCapital | loading(skeleton), empty, normal |
| `BacktestTradesTable` | 交易明细 el-table | props: trades | loading(skeleton), empty, normal |

### 7.4 状态管理

前端当前使用组件内 `ref()` 管理状态（无需 Pinia store，因为回测是独立页面）：

```typescript
// BacktestView.vue 内部状态
const result = ref<BacktestResultDetail | null>(null)    // 回测结果数据
const isRunning = ref(false)       // 运行中标志
const error = ref('')              // 错误信息
const pollProgress = ref(0)        // 轮询进度百分比
```

> 如后续需要跨组件共享回测状态（策略编辑页 → 回测页跳转），可提取到 Pinia store。

### 7.5 前端类型定义

```typescript
// types/backtest.ts - 已实现
export interface BacktestTrade {
  id: number; open_time: string; close_time: string
  direction: 'long' | 'short'; open_price: number; close_price: number
  quantity: number; pnl: number; pnl_percent: number
}

export interface EquityPoint { date: string; equity: number }

export interface BacktestResultDetail {
  id: number; strategy_id: string; strategy_name: string; symbol: string
  total_return: number; annual_return: number; sharpe_ratio: number
  max_drawdown: number; win_rate: number; total_trades: number
  initial_capital: number; equity_curve: EquityPoint[]
  trades: BacktestTrade[]; created_at: string
}

export type BacktestStatus = 'pending' | 'running' | 'completed' | 'failed'

export interface BacktestJob {
  id: string; status: BacktestStatus; result_id?: number; error?: string; created_at: string
}

export interface BacktestParams {
  strategy_id: string; symbol: string; start_date: string; end_date: string; initial_capital: number
}

export interface BacktestHistoryItem {
  id: number; strategy_id: string; strategy_name: string; symbol: string
  total_return: number; sharpe_ratio: number; created_at: string; status: string
}
```

### 7.6 现有组件与 PRD 设计的差异

| PRD 设计 | 实际实现 | 差距说明 |
|---------|---------|---------|
| 策略参数只读展示 | ❌ 未实现 | 选中策略后展示参数（fast_period/slow_period） |
| 时间周期 Interval 选择 | ❌ 未实现 | ConfigForm 缺少 interval 字段 |
| 高级设置（费率、滑点） | ❌ 未实现 | fee_rate/slippage_rate 未在表单中暴露 |
| 回撤曲线图 | ❌ 未实现 | 只有权益曲线 |
| 交易标注（买卖点） | ❌ 未实现 | 权益曲线上无标记 |
| 缩放控制(dataZoom) | ❌ 未实现 | ECharts 未启用缩放 |
| 标签页（摘要/交易/信息） | ⚠️ 部分实现 | 指标 + 图表 + 表格全部展示，未用标签页切换 |
| 回测历史列表 | ❌ 未实现 | 无相关 UI |

---

## 8. 后端设计

### 8.1 模块结构（已实现）

```
backend/src/
├── handlers/
│   └── backtest.rs          # 6 个 API 处理器 ✅
├── services/
│   ├── backtest_engine.rs    # 回测引擎核心逻辑 ✅
│   └── strategy.rs           # strategy_template + strategy_service ✅
├── db/
│   └── backtest.rs           # 所有 DB 操作为 TODO ⚠️
└── models/
    ├── backtest.rs           # 请求/响应/内部类型 ✅
    └── schemas.rs            # PaginatedResponse, ParameterDef 等通用类型
```

### 8.2 回测引擎核心流程（已完成）

```
1. 参数校验 → BacktestConfig::validate()
2. 验证策略存在 → find_strategy()
3. 加载模板 → strategy::get_template()
4. 加载 K 线 → load_klines() (按 symbol + interval + timestamp 排序)
5. 获取信号量 → BACKTEST_SEMAPHORE.try_acquire() (最多 5 并发)
6. 创建结果记录 → create_backtest_run()
7. 启动回测 → spawn_blocking { engine.run() }
8. engine.run() 流程:
   a. 逐根 K 线遍历:
      - 每 100 根检查 CancellationToken
      - 每 10% 更新进度 AtomicU32
      - generate_signal() → Buy/Sell/Hold/CloseAll
      - 开/平仓（市价单 + 滑点 + 手续费）
      - SL/TP 检查（High/Low 穿透）
      - 爆仓检测（equity <= 0）
      - 记录 equity_curve 点
   b. 最后 bar 强制平仓
   c. 计算 14 项绩效指标（O(n) 向量化）
9. 持久化结果 → update_completed() (TODO)
10. 推送 WS 完成事件 (TODO)
```

### 8.3 StrategyTemplate 扩展（已完成）

```rust
pub trait StrategyTemplate: Send + Sync {
    // 已有方法
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> &str;
    fn default_parameters(&self) -> Value;
    fn parameter_schema(&self) -> Vec<ParameterDef>;
    fn validate(&self, params: &Value) -> Result<(), String>;
    // 新增方法
    fn generate_signal(&self, klines: &[Kline], current_idx: usize, params: &Value) -> Signal;
}
```

10 个策略模板已实现 generate_signal: MA Cross, RSI, Bollinger Bands, MACD, Keltner Channel, ATR Trailing, Ichimoku, Parabolic SAR, Supertrend, Volume Profile

### 8.4 并发控制（已完成）

- 每个回测任务运行在独立的 `tokio::task::spawn_blocking` 中
- 使用 `tokio::sync::Semaphore` 限制并发数为 5 (static BACKTEST_SEMAPHORE: Semaphore)
- `try_acquire()` 立即返回，而非阻塞等待
- 超出限制返回 429 + "backtest concurrency limit reached (5)"
- SemaphorePermit 通过 move 进入 spawn_blocking，引擎退出时自动释放

### 8.5 CancellationToken 注册表 (TODO)

当前 `cancel_backtest` handler 为框架代码（TODO 注释），需实现：

```rust
use std::sync::OnceLock;
use dashmap::DashMap;
use tokio_util::sync::CancellationToken;

static CANCEL_TOKENS: OnceLock<DashMap<Uuid, CancellationToken>> = OnceLock::new();

// run_backtest 中注册
CANCEL_TOKENS.get_or_init(Default::default).insert(result_id, cancel_token);

// cancel_backtest 中查找并取消
CANCEL_TOKENS.get_or_init(Default::default).get(&id).map(|token| token.cancel());
// 引擎完成后移除
CANCEL_TOKENS.get_or_init(Default::default).remove(&result_id);
```

### 8.6 DB 层实现 (TODO, 高优先级)

`db/backtest.rs` 中所有函数目前为 TODO 占位，需要实现 SeaORM 实体操作：

| 函数 | 说明 | 优先级 |
|------|------|--------|
| create_backtest_run | INSERT backtest_results (status=pending) | P0 ❌ |
| update_completed | UPDATE metrics/trades/equity_curve + status=completed | P0 ❌ |
| update_failed | UPDATE status=failed + error | P0 ❌ |
| find_backtest_result | SELECT by id, map to BacktestResultResponse | P0 ❌ |
| get_backtest_progress | SELECT status + progress by id | P1 ❌ |
| list_backtest_history | Paginated SELECT by strategy_id | P1 ❌ |
| delete_backtest_record | DELETE by id | P2 ❌ |
| load_klines | SELECT from kline_data by symbol/interval/time range | P0 ❌ |
| find_strategy | SELECT from strategies by id | P0 ❌ |

### 8.7 前端 API 路由映射

当前前端 API 路径与 PRD 设计不完全一致，建议统一：

| PRD 设计路径 | 当前前端调用 | 建议 |
|-------------|------------|------|
| POST /api/v1/backtest/run | POST /backtest | 统一为 /api/v1/backtest/run |
| GET /api/v1/backtest/{id}/result | GET /backtest/{id} | 统一为 /api/v1/backtest/{id}/result |
| GET /api/v1/backtest/{id}/progress | 无（前端轮询 job 端点） | 新增 /api/v1/backtest/{id}/progress |
| POST /api/v1/backtest/{id}/cancel | 无 | 新增 (P1) |
| GET /api/v1/backtest/history | GET /backtest/history | 统一为 /api/v1/backtest/history |
| DELETE /api/v1/backtest/{id} | DELETE /backtest/{id} | 统一为 /api/v1/backtest/{id} |

---

## 9. 边界情况

| 场景 | 预期行为 | 实现状态 |
|------|---------|---------|
| 选择的时间范围内没有 K 线数据 | API 返回 400 + "所选时间范围内无可用 K 线数据" | ✅ 引擎校验 |
| 策略 generate_signal 始终返回 Hold | 回测正常完成，产生 0 笔交易，metrics 全为 0/N/A | ✅ 引擎处理 |
| 初始资金在交易过程中耗尽（爆仓） | 停止后续交易，记录至爆仓点为止的权益曲线 | ✅ 引擎 equity <= 0 检查 |
| 日期范围跨多个年份（5 年 1h） | 正常加载到内存（约 43K bars），计算时间相应增加 | ✅ O(n) 算法 |
| 同时运行第 6 个回测 | 返回 429 + "回测并发数已达上限 (5)" | ✅ Semaphore |
| 回测中途删除策略 | 正在运行的回测不受影响 | ✅ 运行时 snapshot 策略参数 |
| 网络断开后查看回测结果 | 结果已持久化到数据库，重新连接后正常查询 | ⚠️ DB 层 TODO |
| 重复点击"运行回测" | 按钮 loading 状态防止重复提交 | ✅ 前端 loading 控制 |
| 滑点+手续费导致亏损超过本金 | 权益曲线正常展示为负值 | ✅ 引擎支持负权益 |
| 回测完成但线程未能写库 | 保留 error 状态，前端展示"回测失败" | ⚠️ DB 层 TODO |
| interval 与 K 线数据不匹配 | 查询 kline_data 时带 interval 过滤 | ⚠️ 待 DB 层实现 |
| BacktestConfig 中 fee_rate=0 | 允许零费率测试 | ✅ validate() 通过 |

---

## 10. 非功能需求

| 需求 | 指标 | 当前状态 |
|------|------|---------|
| 性能 | 1 年 1h 数据 (8760 bars) 回测完成 < 5s | ✅ 引擎 O(n)，待 DB 集成后验证 |
| 性能 | 5 年 1h 数据 (43800 bars) 回测完成 < 30s | ✅ O(n) 向量化 |
| 精度 | 夏普比率计算误差 < 5%（与参考实现对比） | ✅ 日收益向量+年化因子 |
| 精度 | 所有金额计算保留 8 位小数 | ✅ f64 + JSON serialization |
| 并发 | 支持最多 5 个回测任务同时运行 | ✅ Semaphore |
| 数据一致 | 回测中途失败不残留半写结果 | ⚠️ 需事务或状态检查 |
| 存储 | 回测结果永久保留，不自动删除 | ⚠️ 待 DB 层实现后确认 |
| 前端性能 | equity_curve 超过 10K 点采样到 ≤ 2K 点 | ✅ sample_equity_curve() |
| 前端兼容 | IE11 以上 + Chrome/Firefox/Safari/Edge 最新版 | ✅ ECharts + ElementPlus |

---

## 11. 优先级总览

### 实施优先级

| 优先级 | ID | 用户故事/任务 | 当前状态 | 负责人建议 |
|--------|----|-------------|---------|-----------|
| P0 | US-BT-01 | 运行回测 | ⚠️ DB 层为唯一阻塞项 | 后端 |
| P0 | US-BT-02 | 查看绩效指标 | ✅ 完成 | — |
| P0 | US-BT-03 | 权益曲线 | ✅ 完成 | — |
| **P0** | **DB-TODO** | **DB 层 SeaORM 实现** | **❌ TODO (最关键阻塞项)** | **后端** |
| P1 | US-BT-04 | 交易记录表格 | ✅ 完成 | — |
| P1 | US-BT-05 | 回测历史列表 | ⚠️ DB 层阻塞 | 后端+前端 |
| P1 | US-BT-06 | 进度与取消 | ⚠️ 取消功能 TODO | 后端 |
| P1 | US-BT-03-2 | 回撤曲线图 | ❌ TODO | 前端 |
| P1 | WS-PUSH | WS 进度推送 | ❌ TODO | 后端 |
| P1 | CANCEL | CancellationToken registry | ❌ TODO | 后端 |
| P1 | CONFIG | 高级设置（费率/滑点） | ❌ TODO | 前端 |
| P2 | US-BT-07 | 策略编辑页快速回测 | ❌ TODO | 后端+前端 |
| P2 | EXPORT | CSV 导出 | ⚠️ 按钮占位 | 前端 |

### 实施阶段建议

#### 第一阶段 (核心链路打通) — P0 聚焦
- **后端**: 实现 db/backtest.rs 全部 SeaORM 操作（create/update/find/load_klines）
- **后端**: 集成 spawn_blocking 中的 DB 写回（目前为 TODO 注释）
- **后端**: 10 个策略模板的 DB 集成验证
- **测试**: 端到端回测流程集成测试

#### 第二阶段 (体验完善) — P1 功能
- **后端**: CancellationToken registry 实现取消
- **后端**: WS 进度推送替代轮询
- **前端**: 回撤曲线图（ECharts 面积图）
- **前端**: 高级设置折叠面板（费率/滑点）
- **前端**: 回测历史列表

#### 第三阶段 (体验优化) — P2 功能
- **前端**: 策略编辑页→回测页快速导航
- **前端**: 交易标注（买卖点 markers）
- **前端**: 权益曲线 dataZoom 缩放
- **前端**: CSV 导出功能实现

---

## 12. 前端 TODO 详细清单

### 高优先级 (P1)
1. **回撤曲线图**: BacktestEquityChart.vue 添加 dual-y 轴图表（上: 权益曲线, 下: 回撤面积图）
2. **表格列补齐**: BacktestTradesTable.vue 添加 持仓周期、平仓原因、手续费列
3. **高级设置**: BacktestConfigForm.vue 添加 fee_rate/slippage_rate/interval 字段，用 collapse 折叠

### 中优先级 (P2)
4. **交易标记**: 权益曲线图中添加买入/卖出 markPoint
5. **缩放控制**: ECharts dataZoom 组件
6. **回测历史**: 添加 `BacktestHistory.vue` 标签页
7. **CSV 导出**: TradesTable handleExport 实现

### 低优先级 (P3)
8. **策略参数预览**: ConfigForm 选中策略后显示只读参数
9. **快速回测**: 策略编辑页跳转
10. **Pinia store**: 如需要跨页面状态管理

---

## 13. 风险评估

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| DB 层 SeaORM 实体定义与现有模型不匹配 | 高 | 中 | 先确认 kline_data 和 strategies 表的实际 schema |
| equity_curve 数据量过大导致 API 响应慢 | 中 | 低 | sample_equity_curve() 已实现均匀采样 |
| CancellationToken 在多线程下的竞态 | 中 | 低 | DashMap 线程安全 + spawn_blocking 隔离 |
| 前端 BacktestResultDetail 字段与后端不一致 | 高 | 中 | 需要前后端联调确认字段映射 |
| 轮询替代 WS 可能导致前端超时（60次×2s=2min） | 低 | 中 | 大数据集回测可能超时，需增加轮询次数或降级提示 |
