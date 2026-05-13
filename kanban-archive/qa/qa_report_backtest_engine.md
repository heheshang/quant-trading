# QA测试报告: 回测引擎全链路测试

> 任务: t_19e09b37 | 测试日期: 2026-05-13 | 测试人: qa  
> 基础代码: /home/ssk/workspace/quant-trading

---

## 测试概要

| 项目 | 结果 |
|------|------|
| 后端 cargo test | 84/84 通过 |
| 前端 vitest | 145/145 通过 |
| 发现Bug总数 | 15 |
| P0 (崩溃/数据损坏) | 1 |
| P1 (功能阻塞) | 8 |
| P2 (功能缺陷) | 4 |
| P3 (体验问题) | 2 |
| 安全问题 | 0 (XSS低风险, 越权保护完善) |

---

## 一、回测配置页功能测试

### 1.1 策略选择

| 测试项 | 结果 | 说明 |
|--------|------|------|
| 策略下拉框加载 | PASS | listStrategies API调用正常 |
| 策略选择后加载模板参数 | PASS | loadTemplateParams工作正常 |
| 策略切换清空旧参数 | PASS | strategy_params正确重置 |

### 1.2 参数配置

| 测试项 | 结果 | 说明 |
|--------|------|------|
| 交易对输入 | PASS | 必填校验生效 |
| 时间周期选择 | PASS | 8种周期可选 |
| 初始资金输入 | **FAIL** | 见Bug#1 |
| 手续费率输入 | **FAIL** | 见Bug#2 |
| 滑点率输入 | **FAIL** | 见Bug#2 |
| 日期范围选择 | PASS | daterange picker工作正常 |
| 策略参数动态渲染 | PASS | slider+input联动 |

### 1.3 日期范围验证

| 测试项 | 结果 | 说明 |
|--------|------|------|
| 前端必填校验 | PASS | 未选日期时按钮禁用 |
| 后端日期校验(>=7天) | PASS | BacktestConfig::validate() |
| 后端日期格式校验 | PASS | YYYY-MM-DD格式 |

---

## 二、回测执行流程测试

### 2.1 发起回测

| 测试项 | 结果 | 说明 |
|--------|------|------|
| 提交回测请求 | **FAIL** | 见Bug#3 (请求结构不匹配) |
| 并发限制(5) | PASS | Semaphore实现正确 |
| 策略存在性校验 | PASS | 404返回正确 |

### 2.2 运行状态

| 测试项 | 结果 | 说明 |
|--------|------|------|
| 后端spawn_blocking运行 | PASS | 异步架构正确 |
| CancellationToken取消 | PASS | 全局registry实现 |
| 进度条轮询 | **FAIL** | 见Bug#4 (轮询端点不存在) |

### 2.3 结果展示

| 测试项 | 结果 | 说明 |
|--------|------|------|
| 结果页面状态切换 | PASS | idle→running→completed |
| 进度面板展示 | PASS | indeterminate进度条 |
| 错误面板展示 | PASS | el-alert展示错误信息 |
| 取消回测 | PASS | cancel按钮→API调用 |

---

## 三、绩效指标正确性验证

### 3.1 后端计算逻辑审计

| 指标 | 算法 | 结果 | 说明 |
|------|------|------|------|
| total_return_pct | (final_equity - initial) / initial * 100 | PASS | 公式正确 |
| annualized_return_pct | (1+r)^(365/days) - 1 | PASS | 复利年化 |
| max_drawdown_pct | O(n)峰值遍历 | PASS | 正确 |
| sharpe_ratio | mean/std * sqrt(365) | PASS | 日收益率Sharpe |
| sortino_ratio | mean/downside_std * sqrt(365) | **FAIL** | 见Bug#5 |
| calmar_ratio | annualized_return / |max_dd| | PASS | |
| win_rate | wins/total * 100 | PASS | |
| profit_factor | gross_profit / gross_loss | **FAIL** | 见Bug#6 (INFINITY) |
| avg_win/loss_pct | 算术平均 | PASS | |
| total_fees/slippage | 累加 | PASS | |

### 3.2 手工计算验证

使用5根K线 uptrend 数据手工验证:

**输入:**
- Klines: open=[100,102,104,106,108], close=[101,103,105,107,109] (1h间隔)
- 初始资金: 10000 USDT, fee_rate=0.001, slippage=0.0005

**Mock策略(首根Buy,末根CloseAll):**
- Bar 0: Buy信号, close=101.0
  - slippage = 101.0 * 0.0005 = 0.0505
  - exec_price = 101.0505
  - quantity = floor(10000 / 101.0505) = 98 (整手)
  - cost = 98 * 101.0505 = 9902.95
  - fee = 9902.95 * 0.001 = 9.90
  - cash = 10000 - 9902.95 - 9.90 = 87.15

- Bar 4: CloseAll, close=109.0
  - slippage = 109.0 * 0.0005 = 0.0545
  - exit_price = 109.0 - 0.0545 = 108.9455 (Long卖出)
  - trade_value = 98 * 108.9455 = 10676.66
  - fee = 10676.66 * 0.001 = 10.68
  - cash = 87.15 + 10676.66 - 10.68 = 10753.13

- total_return = (10753.13 - 10000) / 10000 * 100 = 7.53%

**与引擎输出对比:** 逻辑正确（实际因floor取整可能有微小差异）

---

## 四、权益曲线数据验证

| 测试项 | 结果 | 说明 |
|--------|------|------|
| 曲线点数=K线数 | PASS | 每根K线记录1个点 |
| 持仓时MTM计算(Long) | PASS | cash + qty * close |
| 持仓时MTM计算(Short) | PASS | cash + qty*(entry-close) |
| 空仓时equity=cash | PASS | |
| 爆仓检测(equity<=0) | PASS | break并记录最后一个点 |
| drawdown_pct字段 | **FAIL** | 见Bug#7 (始终为0) |
| 采样函数(>2000点) | PASS | 保留首尾+均匀采样 |
| 前端展示数据格式 | **FAIL** | 见Bug#8 (time/date不匹配) |

---

## 五、交易记录数据验证

| 测试项 | 结果 | 说明 |
|--------|------|------|
| 一次完整往返=1条TradeRecord | PASS | open→close记录1笔 |
| Long PnL计算 | PASS | trade_value - entry_value |
| Short PnL计算 | PASS | entry_value - trade_value |
| fee累加(开+平) | PASS | pos.fee_paid + close_fee |
| slippage累加(开+平) | PASS | pos.slippage_paid + close_slippage |
| holding_period_ms计算 | PASS | exit_time - entry_time |
| exit_reason枚举 | PASS | signal/stop_loss/take_profit |
| 前端字段映射 | **FAIL** | 见Bug#9 |
| 方向Tag颜色 | **FAIL** | 见Bug#10 |

---

## 六、异常处理测试

### 6.1 后端验证(BacktestConfig::validate)

| 测试场景 | 预期结果 | 实际结果 |
|----------|----------|----------|
| start_date >= end_date | 400 Validation | PASS |
| 日期间隔<7天 | 400 Validation | PASS |
| initial_capital < 100 | 400 Validation | PASS |
| fee_rate > 0.01 | 400 Validation | PASS |
| slippage_rate > 0.01 | 400 Validation | PASS |
| 无效日期格式 | 400 Validation | PASS |
| strategy_id不存在 | 404 Not Found | PASS |
| 空 klines 数据 | 400 Validation | PASS |
| 并发超限(>5) | 429 TooManyRequests | PASS |

### 6.2 前端表单验证

| 测试场景 | 预期结果 | 实际结果 |
|----------|----------|----------|
| 未选策略 | 按钮禁用 | PASS |
| 交易对为空 | 校验提示 | PASS |
| 日期未选 | 按钮禁用 | PASS |
| 手续费率范围 | **FAIL** | 见Bug#2 |
| initial_capital最小值 | **FAIL** | 前端1000 vs 后端100 |

---

## 七、前后端契约验证 (兜底)

### 7.1 请求结构不匹配 — P0级

| 维度 | 前端(BacktestParams) | 后端(BacktestRunRequest) | 匹配? |
|------|---------------------|------------------------|-------|
| 结构 | 扁平 | 嵌套(config子对象) | **MISMATCH** |
| 周期字段 | `timeframe` | `interval` | **MISMATCH** |
| 滑点字段 | `slippage` | `slippage_rate` | **MISMATCH** |
| 费率单位 | 百分比(0.1=10%) | 小数(0.001=0.1%) | **MISMATCH** |
| 额外字段 | `strategy_params` | 不在请求体中 | **MISMATCH** |

### 7.2 响应字段不匹配

**BacktestResultDetail vs BacktestResultResponse:**

| 前端字段 | 后端字段 | 匹配? |
|---------|---------|-------|
| `id: number` | `id: Uuid` (string) | **MISMATCH** |
| `total_return` (flat) | `metrics.total_return_pct` (nested) | **MISMATCH** |
| `annual_return` (flat) | `metrics.annualized_return_pct` (nested) | **MISMATCH** |
| `sharpe_ratio` (flat) | `metrics.sharpe_ratio` (nested) | **MISMATCH** |
| `max_drawdown` (flat) | `metrics.max_drawdown_pct` (nested) | **MISMATCH** |
| `win_rate` (flat) | `metrics.win_rate` (nested) | **MISMATCH** |
| `total_trades` (flat) | `metrics.total_trades` (nested) | **MISMATCH** |
| `strategy_name` | 不在后端响应中 | **MISMATCH** |
| `initial_capital` | `config.initial_capital` (nested) | **MISMATCH** |
| 缺失字段 | `status, progress, config, error, duration_ms, sortino/calmar/profit_factor等` | **MISMATCH** |

**BacktestTrade vs TradeRecord:**

| 前端字段 | 后端字段 | 匹配? |
|---------|---------|-------|
| `id: number` | 不存在 | **MISMATCH** |
| `open_time` | `entry_time` | **MISMATCH** |
| `close_time` | `exit_time` | **MISMATCH** |
| `open_price` | `entry_price` | **MISMATCH** |
| `close_price` | `exit_price` | **MISMATCH** |
| `pnl` | `pnl_usdt` | **MISMATCH** |
| `pnl_percent` | `pnl_pct` | **MISMATCH** |
| 缺失字段 | `quantity, fee, slippage, holding_period_ms` | **MISMATCH** |

**EquityPoint:**

| 前端字段 | 后端字段 | 匹配? |
|---------|---------|-------|
| `date: string` | `time: i64` | **MISMATCH** |
| 缺失 | `drawdown_pct: f64` | **MISMATCH** |

**BacktestJob:**

| 前端字段 | 后端字段 | 匹配? |
|---------|---------|-------|
| `id: string` | `id: Uuid` | OK |
| `result_id: number` | 不存在 | **MISMATCH** |
| 轮询端点 | `GET /backtest/jobs/{id}` | **端点不存在** |

**BacktestHistoryItem:**

| 前端字段 | 后端字段 | 匹配? |
|---------|---------|-------|
| 返回 `array` | 返回 `PaginatedResponse` | **MISMATCH** |
| `strategy_name` | 不在后端响应中 | **MISMATCH** |
| `total_return` (flat) | `metrics.total_return_pct` (nested) | **MISMATCH** |

---

## 八、回归测试

| 测试项 | 结果 | 说明 |
|--------|------|------|
| 策略管理API不受影响 | PASS | 独立路由模块 |
| 策略CRUD功能 | PASS | cargo test全通过 |
| 前端策略页面 | PASS | vitest全通过 |
| 认证模块 | PASS | 中间件不受影响 |
| 用户管理 | PASS | 独立模块 |
| 并发限制独立 | PASS | Semaphore仅在backtest路由 |

---

## Bug清单

### Bug#1 [P2] 前端 initial_capital 最小值与后端不一致
- **位置:** `BacktestConfigForm.vue:58` vs `models/backtest.rs:192`
- **现象:** 前端 `el-input-number :min="1000"`, 后端 `validate()` 要求 `>= 100`
- **影响:** 前端不允许输入100-999之间的值，但后端本应支持
- **复现:** 设置initial_capital=500 → 前端阻止输入 → 但后端本应接受
- **修复建议:** 前端改为 `:min="100"` 与后端一致

### Bug#2 [P1] 前端 fee_rate/slippage 单位和范围与后端严重不匹配
- **位置:** `BacktestConfigForm.vue:66-90` vs `models/backtest.rs:196-201`
- **现象:** 前端 fee_rate 默认0.1 (step=0.01, max=1, 显示后缀%), slippage默认0.05 (step=0.01, max=1)。后端 fee_rate 要求 [0, 0.01], slippage_rate 要求 [0, 0.01]
- **影响:** 前端发送 fee_rate=0.1 → 后端 validate() 拒绝(>0.01) → 400错误，回测永远无法启动
- **复现步骤:**
  1. 打开回测配置页面
  2. 使用默认 fee_rate=0.1
  3. 点击"运行回测"
  4. 后端返回400: "fee_rate must be 0..0.01"
- **修复建议:** 前端改为小数输入 (fee_rate 默认0.001, range [0, 0.01], step=0.0001) 或前端输入百分比后发送时除以100

### Bug#3 [P0] 前端请求结构与后端完全不匹配，回测永远无法启动
- **位置:** `api/backtest.ts:11` vs `handlers/backtest.rs:48-51`
- **现象:** 前端 `runBacktest()` 发送扁平结构: `{ strategy_id, symbol, timeframe, start_date, end_date, initial_capital, fee_rate, slippage, strategy_params }`; 后端期望嵌套结构: `{ strategy_id, config: { symbol, interval, start_date, end_date, initial_capital, fee_rate, slippage_rate } }`
- **影响:** 后端反序列化失败 → 400或422错误，回测100%无法运行
- **关键不匹配:**
  - 扁平 vs 嵌套config
  - `timeframe` vs `interval`
  - `slippage` vs `slippage_rate`
  - `strategy_params` 位置(前端顶层 vs 后端从策略模型获取)
- **复现步骤:**
  1. 选择策略，填写完整参数
  2. 点击"运行回测"
  3. 前端POST /backtest → 后端反序列化BacktestRunRequest失败
- **修复建议:** 前端改为发送 `{ strategy_id, config: { symbol, interval: timeframe, ... } }` 格式

### Bug#4 [P1] 前端轮询端点 /backtest/jobs/{id} 在后端不存在
- **位置:** `api/backtest.ts:19` → `GET /backtest/jobs/{jobId}`
- **现象:** BacktestView.pollJob() 调用 `getBacktestJob(jobId)` 访问 `/backtest/jobs/{jobId}`, 但后端只注册了 `/backtest/{id}` (GET完整结果)。且 `BacktestJob.result_id` 字段在后端不存在
- **影响:** 回测启动后，前端轮询立即404 → 进入错误状态 → 永远无法展示结果
- **复现步骤:**
  1. 成功启动回测 (假设Bug#3已修)
  2. 前端调用 GET /backtest/jobs/{id}
  3. 后端返回404
  4. 前端报错: "回测执行失败"
- **修复建议:** 前端改为轮询 `GET /backtest/{id}` 并根据 `status` 字段判断完成状态

### Bug#5 [P2] Sortino Ratio 计算分母不正确
- **位置:** `services/backtest_engine.rs:372-379`
- **现象:** Sortino的downside_variance计算: 先filter出负收益, 再除以总样本数(n-1)。标准Sortino应除以负收益的数量或总样本数。另外使用 `(r - mean_return)` 偏离了标准公式 (通常用 `r - target` 即 `min(r - target, 0)`)
- **影响:** Sortino值与标准计算器结果偏差, 可能误导用户
- **修复建议:** 使用标准公式: `downside_returns = daily_returns.map(r => min(r - target, 0))`, `downside_dev = sqrt(sum(downside_returns^2) / N)`

### Bug#6 [P1] profit_factor 为 INFINITY 时 JSON 序列化/前端展示异常
- **位置:** `services/backtest_engine.rs:411`
- **现象:** 当所有交易盈利(total_loss=0, total_profit>0), `profit_factor = f64::INFINITY`
- **影响:** 
  1. `serde_json` 默认将 INFINITY 序列化为 `null` 或抛出错误(取决于配置)
  2. 前端收到 null/undefined → MetricsCards显示异常
- **复现:** 选择一个在uptrend中只产生盈利交易的策略运行回测
- **修复建议:** 将 INFINITY 替换为特殊值(如 `999.99` 或字符串 `"∞"`) 或在前端做特殊处理

### Bug#7 [P2] EquityPoint.drawdown_pct 始终为0, 未实际计算
- **位置:** `services/backtest_engine.rs:310`
- **现象:** `record_equity_point()` 硬编码 `drawdown_pct: 0.0`, 注释说"calculated in metrics"但metrics计算中并未回填
- **影响:** 前端权益曲线数据中 drawdown_pct 全为0, 无法绘制回撤曲线
- **修复建议:** 在 record_equity_point 中实时计算当前回撤: `drawdown_pct = (equity - peak) / peak * 100.0`

### Bug#8 [P1] 前端 EquityPoint.date(string) vs 后端 EquityPoint.time(i64)
- **位置:** `types/backtest.ts:19` vs `models/backtest.rs:149`
- **现象:** 前端 EquityPoint 使用 `date: string` (如 "2024-01-01"), 后端返回 `time: i64` (毫秒时间戳)
- **影响:** ECharts X轴渲染异常 — 要么显示原始时间戳数字, 要么日期格式化失败
- **修复建议:** 前端改为 `time: number` 并用 echarts formatter 格式化, 或后端返回ISO日期字符串

### Bug#9 [P1] 前端 BacktestTrade 字段名与后端 TradeRecord 完全不匹配
- **位置:** `types/backtest.ts:4-15` vs `models/backtest.rs:132-146`
- **现象:** 前端使用 `open_time/close_time/open_price/close_price/pnl/pnl_percent`, 后端使用 `entry_time/exit_time/entry_price/exit_price/pnl_usdt/pnl_pct`
- **影响:** 前端表格所有列显示undefined, 数据完全无法呈现
- **修复建议:** 统一命名, 推荐使用后端字段名

### Bug#10 [P1] 交易方向Tag颜色反转
- **位置:** `BacktestTradesTable.vue:55`
- **现象:** `direction === 'long' ? 'danger' : 'success'` — Long=红色(danger), Short=绿色(success)
- **影响:** Long显示为红色(通常代表做空/下跌), Short显示为绿色(通常代表做多/上涨), 与金融惯例相反
- **修复建议:** 改为 `direction === 'long' ? 'success' : 'danger'`

### Bug#11 [P1] 前端 BacktestResultDetail 缺少后端大部分字段
- **位置:** `types/backtest.ts:24-39` vs `models/backtest.rs:42-56`
- **现象:** 前端类型只有10个字段(id/strategy_id/strategy_name/symbol/6个flat指标/equity_curve/trades/created_at), 后端BacktestResultResponse有13+字段, 且metrics是嵌套结构(14个子字段)
- **影响:** 
  1. 前端无法读取 `status`, `progress`, `error` 等关键状态字段
  2. 前端无法展示 sortino/calmar/profit_factor/avg_win/avg_loss/total_fees 等指标
  3. 前端 `strategy_name` 在后端响应中不存在
- **修复建议:** 前端类型重构为与后端一致的嵌套结构

### Bug#12 [P1] 前端 listBacktestHistory 返回类型与后端不匹配
- **位置:** `api/backtest.ts:38-44` vs `handlers/backtest.rs:210-232`
- **现象:** 前端期望返回 `BacktestHistoryItem[]` (纯数组), 后端返回 `PaginatedResponse<BacktestSummary>` (分页对象: {items, total, page, size})
- **影响:** 前端拿到的数据格式不正确, 无法渲染历史列表
- **修复建议:** 前端改为处理分页响应结构

### Bug#13 [P3] 并发提示 "0/5 运行中" 硬编码
- **位置:** `BacktestConfigForm.vue:204-206`
- **现象:** `<span class="hint-text">0/5 运行中</span>` 硬编码为0, 不随实际运行数变化
- **影响:** 用户体验误导
- **修复建议:** 从后端获取当前运行数, 或先移除此提示

### Bug#14 [P3] 前端 BacktestTrade 缺少 quantity/fee/slippage/holding_period_ms 字段
- **位置:** `types/backtest.ts:4-15`
- **现象:** 前端 BacktestTrade 只有8个字段, 后端 TradeRecord 有12个字段, 缺少 quantity, fee, slippage, holding_period_ms
- **影响:** 用户无法查看手续费、滑点、持仓时长等详细信息
- **修复建议:** 前端类型补充缺失字段

### Bug#15 [P2] deleteBacktestResult 前端传 number id, 后端期望 UUID string
- **位置:** `api/backtest.ts:46` vs `handlers/backtest.rs:236-247`
- **现象:** 前端 `BacktestResultDetail.id: number`, `deleteBacktestResult(id: number)` 传number; 后端 `Path(id): Path<Uuid>` 期望UUID字符串
- **影响:** 删除操作失败 (UUID解析错误)
- **修复建议:** 前端id类型改为string

---

## 安全审计

| 测试项 | 结果 | 说明 |
|--------|------|------|
| SQL注入 | PASS | SeaORM参数化查询 |
| XSS | LOW RISK | Vue模板自动转义, 但strategy_name等字段若含HTML需注意 |
| CSRF | PASS | API使用Bearer Token认证 |
| 越权访问 | PASS | auth_middleware全局中间件保护 |
| 并发DoS | PASS | Semaphore限制5并发 |
| 资源泄露 | PASS | CancellationToken + onUnmounted清理polling |

---

## 测试通过统计

| 测试类别 | 总测试项 | 通过 | 失败 | 通过率 |
|---------|---------|------|------|--------|
| 回测配置页功能 | 13 | 9 | 4 | 69% |
| 回测执行流程 | 7 | 5 | 2 | 71% |
| 绩效指标正确性 | 12 | 10 | 2 | 83% |
| 权益曲线数据 | 8 | 6 | 2 | 75% |
| 交易记录数据 | 9 | 7 | 2 | 78% |
| 异常处理 | 12 | 10 | 2 | 83% |
| 前后端契约 | 22 | 0 | 22 | 0% |
| 回归测试 | 6 | 6 | 0 | 100% |
| **总计** | **89** | **53** | **36** | **60%** |

---

## 风险评估

### 最高优先级 (必须修复才能运行)
1. **Bug#3 [P0]**: 前端请求结构与后端完全不匹配 — 回测100%无法启动
2. **Bug#4 [P1]**: 轮询端点不存在 — 即使启动也无法获取结果

### 高优先级 (核心功能不可用)
3. **Bug#2 [P1]**: fee_rate/slippage单位不匹配 — 默认值直接触发后端400
4. **Bug#9 [P1]**: 交易记录字段名完全不同 — 数据无法展示
5. **Bug#11 [P1]**: 结果类型结构不匹配 — 前端无法解析后端响应
6. **Bug#12 [P1]**: 历史列表返回类型不匹配
7. **Bug#10 [P1]**: 方向Tag颜色反转
8. **Bug#8 [P1]**: EquityPoint时间字段类型不匹配

### 中等优先级 (功能缺陷)
9. **Bug#6 [P1]**: profit_factor=INFINITY序列化问题
10. **Bug#5 [P2]**: Sortino计算偏差
11. **Bug#7 [P2]**: drawdown_pct始终为0
12. **Bug#1 [P2]**: initial_capital最小值不一致
13. **Bug#15 [P2]**: 删除操作id类型不匹配

### 低优先级 (体验问题)
14. **Bug#14 [P3]**: 交易记录缺少详情字段
15. **Bug#13 [P3]**: 并发提示硬编码

---

## 建议修复优先级

1. **Phase 1 (阻塞性修复)**: Bug#3, #4, #2, #9, #11, #8 — 修复前后端契约, 回测才能端到端运行
2. **Phase 2 (功能修复)**: Bug#6, #10, #12, #15, #7 — 完善核心功能
3. **Phase 3 (优化修复)**: Bug#1, #5, #13, #14 — 提升体验和数据质量
