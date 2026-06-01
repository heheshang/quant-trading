# PRD: 交易执行模块 — 手动下单、委托管理与模拟撮合引擎

> 版本: v1.0
> 状态: Draft
> 作者: PM
> 关联: PRD.md US-11/US-12, PRD-market-module.md (行情联动), PRD-backtest-engine.md (回测→实盘桥接), TECH_CHARTER-quant.md

---

## 1. 背景

交易执行模块是量化交易系统从"观察"到"行动"的关键跃迁——用户通过它将交易决策转化为实际委托。主 PRD (v1.0) 将交易执行列为 P1 二期功能，定义了 US-11 (手动下单) 和 US-12 (委托管理) 两个用户故事。

### 当前状态 (2026-05-14)

**已完成的基础设施:**
- 前端 `types/trade.ts`: 已定义 `Order`、`Position`、`Trade`、`Portfolio` 四个 TypeScript 接口
- 前端 `api/trades.ts`: 已定义 7 个 API 调用 (listOrders / createOrder / cancelOrder / listPositions / closePosition / getPortfolio / listTrades)，但后端未实现对应端点
- 前端 `views/trade/TradingView.vue`: 仅为占位组件，显示 "Trading interface coming soon"
- 前端 `views/portfolio/PortfolioView.vue`: 占位组件
- 后端: 无 `handlers/trade.rs`，无 `services/trading_engine.rs`，无 `db/order.rs`
- WebSocket 基础框架已就绪 (`handlers/ws.rs`)，可扩展用于委托状态推送
- 行情模块 (PRD-market-module.md) 已规划深度/盘口数据，可为下单界面提供实时价格参考
- 回测引擎 (PRD-backtest-engine.md) 已实现模拟撮合逻辑 (SL/TP/爆仓检测)，可复用于模拟交易撮合

**待构建:**
- 限价单 / 市价单下单表单与校验
- 模拟撮合引擎 (基于深度数据模拟成交)
- 委托状态机 (pending → partial_filled → filled / cancelled / expired / rejected)
- 委托列表 (当前委托 + 历史委托) 与撤单功能
- 委托 WebSocket 实时推送
- 风控前置检查 (余额不足 / 权限不足 / 交易对限制)

### 本 PRD 与其他 PRD 的关系

| 范围 | PRD-market-module | PRD-backtest-engine | 本 PRD (交易执行) |
|------|-------------------|---------------------|-------------------|
| 行情数据展示 | ✅ | — | — |
| 实时价格推送 | ✅ | — | 下单页面消费 (只读) |
| 回测模拟撮合 | — | ✅ | — |
| 实盘/模拟交易撮合 | — | — | ✅ |
| 手动下单 | — | — | ✅ |
| 委托管理/撤单 | — | — | ✅ |
| 持仓管理 | — | — | ✅ (本 PRD 范围内) |
| 策略自动下单 | — | — | ❌ (P2，策略引擎对接) |

---

## 2. 目标

| 目标 | 指标 | 当前状态 |
|------|------|---------|
| 手动下单 | 限价单 + 市价单，下单响应 < 200ms | TradingView 为占位组件 |
| 模拟撮合 | 基于实时深度数据撮合，成交延迟 < 1s | 不存在 |
| 委托状态流转 | pending → partial_filled → filled / cancelled / expired / rejected | 不存在 |
| 委托列表 | 当前委托 + 历史委托，支持筛选/分页 | 不存在 |
| 撤单 | 待成交委托一键撤单，撤单响应 < 200ms | 不存在 |
| WebSocket 推送 | 委托状态变更实时推送，延迟 < 500ms | WS 基础框架已就绪 |
| 风控前置 | 余额/权限/交易对校验，拦截非法委托 | 不存在 |
| 并发下单 | 单用户同时最多 50 个未成交委托 | 不存在 |

### 非目标 (Non-Goals)

- ❌ 策略自动下单 / 策略信号→委托转换 (P2，需策略引擎对接)
- ❌ 止损/止盈单 (stop-limit / take-profit) (P2，需更复杂的状态机)
- ❌ 条件单 / OCO 单 / 冰山单 (P2+)
- ❌ 实盘交易所对接 (Binance/OKX API 下单) (P2，需 API Key 管理)
- ❌ 多账户交易 (P2+)
- ❌ 保证金交易 / 杠杆交易 (P2+)
- ❌ 移动端适配 (P2+)

---

## 3. 用户角色与权限

| 角色 | 交易权限 | 说明 |
|------|---------|------|
| **trader** (普通用户) | 模拟交易: 下单/撤单/查看委托/查看持仓 | 模拟账户，无真实资金 |
| **pro-trader** (专业交易员) | 模拟交易 + 实盘交易 (本期仅预埋权限) | 实盘 API 对接为 P2 |
| **admin** (管理员) | 查看所有用户委托记录 + 审计日志 | 可查看但不可代替用户下单 |

> 注：v1.1 仅实现模拟交易。实盘交易权限在 pro-trader 角色上预埋，但下单时统一走模拟撮合引擎。实盘交易所对接在 v2.0 中实现。

---

## 4. 用户故事 (User Stories)

### US-TE-01: 限价单下单 (P0)

> **As a** 交易者
> **I want to** 提交限价委托
> **So that** 我可以在指定价格买入或卖出

**验收条件：**

```gherkin
Feature: 限价单下单

  Background:
    Given 用户已登录
      And 用户角色为 "trader"
      And 模拟账户余额充足

  Scenario: 成功提交限价买单
    Given 交易页面已打开
      And 当前交易对为 "BTC/USDT"
      And 当前卖一价为 ¥50,000
    When 用户选择委托类型 "限价单"
      And 输入价格 ¥49,500
      And 输入数量 0.1 BTC
      And 选择方向 "买入"
      And 点击 "提交委托"
    Then 弹出确认对话框，显示: 交易对 BTC/USDT、方向 买入、类型 限价、价格 ¥49,500、数量 0.1 BTC、总金额 ¥4,950
      And 用户点击 "确认" 后
    Then 委托提交成功
      And 系统返回 201
      And 委托出现在 "当前委托" 列表中
      And 委托状态为 "待成交"
      And 模拟账户冻结 ¥4,950 保证金
      And 页面显示 Toast "委托已提交"

  Scenario: 成功提交限价卖单
    Given 用户持有 0.5 BTC
      And 当前买一价为 ¥50,000
    When 用户选择方向 "卖出"
      And 输入价格 ¥51,000
      And 输入数量 0.1 BTC
      And 点击 "提交委托"
      And 确认
    Then 委托提交成功
      And 委托状态为 "待成交"
      And 持仓中冻结 0.1 BTC

  Scenario: 限价单部分成交
    Given 存在状态为 "待成交" 的限价买单 (价格 ¥49,500, 数量 0.1 BTC)
      And 模拟撮合引擎检测到市场卖一价 ≤ ¥49,500
    When 撮合引擎匹配
    Then 委托状态变为 "部分成交"
      And 已成交数量更新
      And 通过 WebSocket 推送成交通知
      And 成交记录写入 trades 表

  Scenario: 限价单完全成交
    Given 存在状态为 "部分成交" 的限价买单
      And 剩余数量已全部撮合
    Then 委托状态变为 "已成交"
      And 冻结保证金释放/结算
      And 持仓数量更新
      And 委托从 "当前委托" 移至 "历史委托"

  Scenario: 限价单价格等于市场价时立即撮合
    Given 当前卖一价为 ¥50,000
    When 用户提交限价买单，价格 ¥50,000
    Then 撮合引擎立即匹配
      And 委托直接进入 "已成交" 状态 (跳过 "待成交")

  Scenario: 限价单价格不合理时提示
    Given 当前市场价为 ¥50,000
    When 用户输入限价买单价格 ¥100,000 (偏离市场价 100%)
    Then 弹出风险提示 "委托价格偏离当前市场价较大 (100%)，是否确认提交？"
      And 用户可选择 "确认提交" 或 "取消"

  Scenario: 卖出数量超过持仓
    Given 用户持有 0.3 BTC
    When 用户提交限价卖单，数量 0.5 BTC
    Then 提示 "卖出数量超过可用持仓 (0.3 BTC)"
      And 提交按钮禁用
```

---

### US-TE-02: 市价单下单 (P0)

> **As a** 交易者
> **I want to** 提交市价委托
> **So that** 我可以按当前最优价格立即成交

**验收条件：**

```gherkin
Feature: 市价单下单

  Background:
    Given 用户已登录
      And 用户角色为 "trader"

  Scenario: 成功提交市价买单
    Given 交易页面已打开
      And 当前交易对为 "BTC/USDT"
      And 模拟账户可用余额 ¥10,000
      And 当前卖一价为 ¥50,000
    When 用户选择委托类型 "市价单"
      And 输入数量 0.1 BTC
      And 选择方向 "买入"
      And 点击 "提交委托"
    Then 弹出确认对话框，显示: 交易对 BTC/USDT、方向 买入、类型 市价、数量 0.1 BTC、预估金额 ¥5,000 (以当前价估算)
      And 提示 "市价单将按市场最优价成交，实际成交价可能有偏差"
      And 用户确认后
    Then 委托提交至模拟撮合引擎
      And 按当前卖一价撮合成交
      And 委托状态为 "已成交"
      And 成交记录出现在 "历史委托" 列表
      And 持仓更新

  Scenario: 成功提交市价卖单
    Given 用户持有 0.5 BTC
      And 当前买一价为 ¥50,000
    When 用户选择方向 "卖出"
      And 选择委托类型 "市价单"
      And 输入数量 0.1 BTC
      And 确认
    Then 按当前买一价撮合成交
      And 委托状态为 "已成交"
      And 持仓减少 0.1 BTC

  Scenario: 市价单深度不足时部分成交
    Given 当前卖一至卖五总量为 0.05 BTC
    When 用户提交市价买单，数量 0.1 BTC
    Then 先按卖一~卖五价成交 0.05 BTC
      And 委托状态为 "部分成交"
      And 系统继续等待新卖单进入
      And 剩余 0.05 BTC 在新卖单出现后继续撮合

  Scenario: 市价单无对手盘
    Given 当前深度数据为空 (无卖单)
    When 用户提交市价买单
    Then 提示 "当前市场无对手盘，市价单无法成交"
      And 委托不提交

  Scenario: 市价单不显示价格输入框
    Given 下单表单
    When 用户选择委托类型 "市价单"
    Then 价格输入框隐藏
      And 显示当前买一/卖一价参考
      And 表单标签显示 "数量" (非 "金额")
```

---

### US-TE-03: 下单风控校验 (P0)

> **As a** 系统
> **I want to** 在委托提交前进行风控校验
> **So that** 非法委托不会进入撮合引擎

**验收条件：**

```gherkin
Feature: 下单风控校验

  Background:
    Given 用户已登录

  Scenario: 余额不足拦截
    Given 用户模拟账户可用余额 ¥1,000
    When 用户提交限价买单，总金额 ¥10,000
    Then 系统返回 400
      And 错误码为 40001
      And 提示 "账户余额不足，可用余额 ¥1,000，需要 ¥10,000"
      And 委托不提交

  Scenario: 卖出数量超过持仓拦截
    Given 用户持有 0.1 BTC
    When 用户提交卖单，数量 0.5 BTC
    Then 系统返回 400
      And 错误码为 40002
      And 提示 "可用持仓不足，持有 0.1 BTC，卖出 0.5 BTC"

  Scenario: 交易对不支持
    Given 系统支持的交易对为 ["BTC/USDT", "ETH/USDT", "SOL/USDT"]
    When 用户提交委托，交易对为 "DOGE/USDT"
    Then 系统返回 400
      And 错误码为 40003
      And 提示 "不支持的交易对: DOGE/USDT"

  Scenario: 数量为零或负数
    Given 下单表单
    When 用户输入数量 0 或 -1
    Then 前端实时校验，提示 "数量必须大于0"
      And 提交按钮禁用

  Scenario: 价格为零或负数 (限价单)
    Given 下单表单，委托类型为 "限价单"
    When 用户输入价格 0 或 -100
    Then 前端实时校验，提示 "价格必须大于0"
      And 提交按钮禁用

  Scenario: 未成交委托数量上限
    Given 用户当前有 50 个未成交委托
    When 用户提交新委托
    Then 系统返回 429
      And 错误码为 42901
      And 提示 "未成交委托数量已达上限 (50)，请先撤销部分委托"

  Scenario: trader 角色无法提交实盘委托 (预埋)
    Given 用户角色为 "trader"
    When 委托 mode 为 "live"
    Then 系统返回 403
      And 错误码为 40301
      And 提示 "权限不足，实盘交易需要 pro-trader 角色"

  Scenario: 最小下单数量校验
    Given 交易对 "BTC/USDT" 最小下单量为 0.001 BTC
    When 用户输入数量 0.0001 BTC
    Then 前端实时校验，提示 "最小下单数量为 0.001 BTC"
      And 提交按钮禁用

  Scenario: 价格精度校验
    Given 交易对 "BTC/USDT" 价格精度为 2 位小数 (0.01)
    When 用户输入价格 ¥50,000.123
    Then 前端实时校验，提示 "价格精度不超过 2 位小数"
      And 提交按钮禁用

  Scenario: 数量精度校验
    Given 交易对 "BTC/USDT" 数量精度为 4 位小数 (0.0001)
    When 用户输入数量 0.12345
    Then 前端实时校验，提示 "数量精度不超过 4 位小数"
      And 提交按钮禁用
```

---

### US-TE-04: 查看当前委托 (P0)

> **As a** 交易者
> **I want to** 查看所有未成交委托
> **So that** 我可以跟踪挂单状态

**验收条件：**

```gherkin
Feature: 查看当前委托

  Background:
    Given 用户已登录

  Scenario: 当前委托列表显示
    Given 用户有 3 个未成交委托
    When 打开 "当前委托" 页面
    Then 显示所有未完全成交的委托
      And 每行包含: 时间、交易对、方向(买入绿色/卖出红色)、类型、价格、数量、已成交、状态、操作
      And 默认按时间降序排列 (最新在前)
      And 数据通过 REST API 初始加载

  Scenario: 当前委托实时更新
    Given 当前委托页面已打开
    When 某委托状态发生变化 (部分成交/已成交/已撤销)
    Then 通过 WebSocket 推送状态变更
      And 列表实时更新 (不刷新页面)
      And 已成交的委托从列表移除，出现在 "历史委托"

  Scenario: 按交易对筛选
    Given 当前委托列表已显示
    When 用户在筛选框选择 "BTC/USDT"
    Then 列表仅显示 BTC/USDT 的委托

  Scenario: 按方向筛选
    Given 当前委托列表已显示
    When 用户选择 "仅显示买入"
    Then 列表仅显示方向为 "买入" 的委托

  Scenario: 无未成交委托
    Given 用户无未成交委托
    When 打开 "当前委托" 页面
    Then 显示空状态 "暂无未成交委托"
      And 提供快捷入口 "去下单"

  Scenario: 委托状态颜色标记
    Given 当前委托列表已显示
    Then "待成交" 状态显示为黄色标签
      And "部分成交" 状态显示为蓝色标签
```

---

### US-TE-05: 撤单 (P0)

> **As a** 交易者
> **I want to** 撤销未成交的委托
> **So that** 我可以取消不再需要的挂单

**验收条件：**

```gherkin
Feature: 撤单

  Background:
    Given 用户已登录

  Scenario: 撤销待成交委托
    Given 存在状态为 "待成交" 的委托 (限价买单，价格 ¥49,500，数量 0.1 BTC)
    When 用户点击该委托的 "撤单" 按钮
    Then 弹出确认对话框 "确认撤销该委托？"
      And 用户确认后
    Then 撤单请求提交成功
      And 委托状态变为 "已撤销"
      And 冻结保证金释放回可用余额
      And WebSocket 推送委托状态变更
      And 该委托从 "当前委托" 移至 "历史委托"
      And 页面显示 Toast "委托已撤销"

  Scenario: 撤销部分成交委托
    Given 存在状态为 "部分成交" 的委托 (已成交 0.05 BTC，剩余 0.05 BTC)
    When 用户点击 "撤单"
    Then 委托状态变为 "已撤销"
      And 已成交的 0.05 BTC 保留 (持仓不变)
      And 未成交部分的冻结保证金释放
      And 委托记录标记最终成交量为 0.05 BTC

  Scenario: 已成交委托无法撤单
    Given 存在状态为 "已成交" 的委托
    When 用户查看该委托
    Then "撤单" 按钮不显示或禁用

  Scenario: 撤单并发竞争
    Given 委托状态为 "待成交"
      And 撮合引擎正在处理该委托
    When 用户同时点击撤单
    Then 如果委托在撤单前已成交，撤单失败
      And 提示 "委托已成交，无法撤销"
      And 如果委托尚未成交，撤单成功
      And 状态一致性保证 (不会出现既成交又撤销)

  Scenario: 批量撤单
    Given 用户有 5 个未成交委托
    When 用户点击 "全部撤单" 按钮
    Then 弹出确认 "确认撤销全部 5 个未成交委托？"
      And 用户确认后
    Then 所有未成交委托依次撤销
      And 显示撤单结果: "成功撤销 4 个，1 个已成交无法撤销"
```

---

### US-TE-06: 查看历史委托 (P1)

> **As a** 交易者
> **I want to** 查看历史委托记录
> **So that** 我可以回顾过去的交易行为

**验收条件：**

```gherkin
Feature: 查看历史委托

  Background:
    Given 用户已登录

  Scenario: 历史委托列表显示
    Given 用户有 20 条历史委托记录
    When 打开 "历史委托" 页面
    Then 显示所有已成交/已撤销/已过期的委托
      And 每行包含: 时间、交易对、方向、类型、价格、数量、已成交、状态、成交均价
      And 默认按时间降序排列
      And 分页显示 (每页 20 条)

  Scenario: 按日期范围筛选
    Given 历史委托列表已显示
    When 用户选择日期范围 "2026-05-01" ~ "2026-05-14"
    Then 列表仅显示该时间范围内的委托

  Scenario: 按状态筛选
    Given 历史委托列表已显示
    When 用户选择状态 "已成交"
    Then 列表仅显示已成交的委托

  Scenario: 按交易对筛选
    Given 历史委托列表已显示
    When 用户搜索 "BTC"
    Then 列表仅显示交易对包含 "BTC" 的委托

  Scenario: 委托详情查看
    Given 历史委托列表已显示
    When 用户点击某条委托
    Then 展开详情面板，显示: 委托ID、创建时间、成交时间、手续费、成交明细 (每笔成交的价格和数量)

  Scenario: 无历史委托
    Given 用户无历史委托
    When 打开 "历史委托" 页面
    Then 显示空状态 "暂无历史委托"
```

---

### US-TE-07: 模拟撮合引擎 (P0)

> **As a** 系统
> **I want to** 基于实时深度数据模拟委托撮合
> **So that** 模拟交易体验接近真实交易所

**验收条件：**

```gherkin
Feature: 模拟撮合引擎

  Background:
    Given 系统已连接到市场数据源
      And 深度数据通过 WebSocket 实时更新

  Scenario: 限价买单撮合
    Given 存在待成交限价买单 (价格 ¥49,500，数量 0.1 BTC)
      And 市场卖一价为 ¥49,500
    When 深度数据更新，卖一价降至 ¥49,500 或更低
    Then 撮合引擎匹配该委托
      And 按卖一价成交
      And 委托状态更新为 "已成交"
      And 生成成交记录

  Scenario: 市价买单撮合
    Given 用户提交市价买单 (数量 0.1 BTC)
      And 当前深度数据:
        | 档位 | 价格 | 数量 |
        | 卖一 | ¥50,000 | 0.03 BTC |
        | 卖二 | ¥50,010 | 0.05 BTC |
        | 卖三 | ¥50,020 | 0.10 BTC |
    When 撮合引擎处理
    Then 按卖一价成交 0.03 BTC
      And 按卖二价成交 0.05 BTC
      And 按卖三价成交 0.02 BTC
      And 委托状态为 "已成交"
      And 成交均价为 ¥50,006.4

  Scenario: 限价卖单撮合
    Given 存在待成交限价卖单 (价格 ¥51,000，数量 0.1 BTC)
      And 市场买一价为 ¥51,000
    When 深度数据更新，买一价升至 ¥51,000 或更高
    Then 撮合引擎匹配该委托
      And 按买一价成交

  Scenario: 深度不足时部分撮合
    Given 存在待成交限价买单 (价格 ¥49,500，数量 0.5 BTC)
      And 卖一至卖五累计数量为 0.3 BTC (价格均 ≤ ¥49,500)
    When 撮合引擎处理
    Then 成交 0.3 BTC
      And 委托状态为 "部分成交"
      And 剩余 0.2 BTC 继续等待

  Scenario: 撮合引擎延迟模拟
    Given 模拟撮合配置 delay_ms = 200
    When 委托进入撮合引擎
    Then 撮合结果在 200ms ± 50ms 内返回
      And 模拟真实交易所网络延迟

  Scenario: 委托过期
    Given 存在待成交限价委托，创建时间为 24h 前
      And 委托设置了 GTC (Good-Till-Cancel) 为 24h
    When 系统定时检查过期委托
    Then 委托状态变为 "已过期"
      And 冻结保证金释放
      And WebSocket 推送状态变更

  Scenario: 撮合引擎与深度数据断开
    Given 行情数据源断开
      And 深度数据不可用
    When 用户提交市价单
    Then 提示 "行情数据暂不可用，无法提交市价单"
      And 委托不提交
    When 用户提交限价单
    Then 限价单正常提交 (进入 "待成交" 状态)
      And 待深度数据恢复后撮合
```

---

### US-TE-08: 交易页面布局与交互 (P0)

> **As a** 交易者
> **I want to** 在一个整合页面完成下单和查看委托
> **So that** 我可以高效执行交易操作

**验收条件：**

```gherkin
Feature: 交易页面布局与交互

  Background:
    Given 用户已登录

  Scenario: 交易页面三栏布局
    When 用户导航至 /trade 页面
    Then 页面显示三栏布局:
      | 区域 | 宽度 | 内容 |
      | 左侧图表区 | flex:1 | K线图 + 深度图 |
      | 中间下单区 | 360px | 下单表单 + 实时价格 |
      | 右侧委托区 | flex:1 | 当前委托 + 历史委托 |
    And 三栏可拖拽调整宽度 (最小宽度 300px)

  Scenario: 下单表单布局
    Given 交易页面已打开
    Then 下单区显示:
      - 交易对选择器 (可切换)
      - 方向切换 Tab: [买入] [卖出] (买入绿色/卖出红色)
      - 委托类型选择: [限价] [市价]
      - 价格输入 (限价单时显示，市价单时隐藏)
      - 数量输入
      - 可用余额/持仓显示
      - 滑块快捷选择数量 (25%/50%/75%/100%)
      - 预估总金额显示
      - 提交委托按钮

  Scenario: 下单表单与行情联动
    Given 交易页面已打开
      And 当前交易对为 "BTC/USDT"
      And 实时卖一价为 ¥50,100
    When 用户切换方向为 "买入"
    Then 价格输入框默认填入卖一价 ¥50,100
      And 显示 "买一: ¥50,099 / 卖一: ¥50,100"
    When 用户切换方向为 "卖出"
    Then 价格输入框默认填入买一价 ¥50,099

  Scenario: 交易对切换
    Given 交易页面已打开，当前为 "BTC/USDT"
    When 用户从交易对选择器切换至 "ETH/USDT"
    Then K线图切换至 ETH/USDT
      And 深度数据切换至 ETH/USDT
      And 下单表单重置 (价格/数量清空)
      And 可用余额/持仓更新

  Scenario: 数量滑块
    Given 用户可用余额 ¥10,000
      And 当前价格为 ¥50,000
    When 用户点击 "25%" 滑块
    Then 数量输入框填入 0.05 BTC (¥10,000 * 25% / ¥50,000)
      And 预估金额显示 ¥2,500

  Scenario: 交易模式标识
    Given 当前为模拟交易模式
    Then 交易页面顶部显示 "模拟交易" 标签 (蓝色)
      And 下单确认对话框包含 "模拟交易" 提示
    Given 当前为实盘交易模式 (pro-trader)
    Then 交易页面顶部显示 "实盘交易" 标签 (红色)
      And 下单确认对话框包含风险提示

  Scenario: 委托区 Tab 切换
    Given 交易页面已打开
    Then 右侧委托区显示两个 Tab: [当前委托] [历史委托]
    When 用户点击 "历史委托" Tab
    Then 显示历史委托列表
      And 当前委托 Tab 角标显示未成交数量
```

---

### US-TE-09: 委托 WebSocket 实时推送 (P1)

> **As a** 交易者
> **I want to** 实时接收委托状态变更通知
> **So that** 我可以及时了解订单执行情况

**验收条件：**

```gherkin
Feature: 委托 WebSocket 实时推送

  Background:
    Given 用户已登录
      And WebSocket 已连接

  Scenario: 订阅委托频道
    Given 用户已连接 WebSocket
    When 用户发送订阅消息:
      | action | channels |
      | subscribe | ["order:all"] |
    Then 服务端返回订阅确认:
      | type | channels |
      | subscribed | ["order:all"] |
      And 后续该用户的所有委托状态变更通过 WebSocket 推送

  Scenario: 委托状态变更推送
    Given 用户已订阅 "order:all" 频道
    When 委托 #123 状态从 "待成交" 变为 "部分成交"
    Then WebSocket 推送消息:
      ```json
      {
        "type": "order_update",
        "data": {
          "order_id": "123",
          "symbol": "BTC/USDT",
          "side": "buy",
          "order_type": "limit",
          "price": "49500.00",
          "quantity": "0.1000",
          "filled_quantity": "0.0500",
          "status": "partial_filled",
          "updated_at": 1715500900000
        },
        "ts": 1715500900000
      }
      ```

  Scenario: 成交通知推送
    Given 用户已订阅 "order:all" 频道
    When 委托 #123 完全成交
    Then WebSocket 推送成交消息:
      ```json
      {
        "type": "trade",
        "data": {
          "trade_id": "456",
          "order_id": "123",
          "symbol": "BTC/USDT",
          "side": "buy",
          "price": "49500.00",
          "quantity": "0.0500",
          "fee": "0.4950",
          "timestamp": 1715500900000
        },
        "ts": 1715500900000
      }
      ```

  Scenario: 断线重连后补发
    Given WebSocket 断线后重连成功
    When 用户重新订阅 "order:all"
    Then 服务端补发断线期间的委托状态变更
      And 按时间顺序推送
      And 客户端去重 (基于 order_id + status)

  Scenario: 未订阅时委托状态仍可通过 REST 查询
    Given 用户未连接 WebSocket
    When 用户打开当前委托页面
    Then 通过 REST API 轮询获取最新状态 (每 5s)
```

---

### US-TE-10: 持仓管理 (P1)

> **As a** 交易者
> **I want to** 查看和管理我的持仓
> **So that** 我可以了解当前资产配置

**验收条件：**

```gherkin
Feature: 持仓管理

  Background:
    Given 用户已登录
      And 用户有模拟交易持仓

  Scenario: 持仓列表显示
    Given 用户有 3 个持仓
    When 打开 "持仓" 页面
    Then 显示所有持仓汇总
      And 每行包含: 交易对、方向(多/空)、持仓数量、开仓均价、当前价、浮动盈亏、盈亏率
      And 浮动盈亏实时更新 (通过 WebSocket ticker 推送)
      And 盈利显示绿色、亏损显示红色

  Scenario: 一键平仓
    Given 用户持有 "BTC/USDT" 多头 0.3 BTC
    When 用户点击 "平仓" 按钮
    Then 弹出确认 "确认平仓 BTC/USDT 0.3 BTC？将以市价卖出"
      And 用户确认后
    Then 生成市价卖单 0.3 BTC
      And 成交后持仓清零
      And 盈亏计入已实现盈亏

  Scenario: 部分平仓
    Given 用户持有 "BTC/USDT" 多头 0.5 BTC
    When 用户点击 "部分平仓"
      And 输入平仓数量 0.2 BTC
    Then 生成市价卖单 0.2 BTC
      And 成交后持仓数量变为 0.3 BTC
      And 开仓均价不变

  Scenario: 无持仓
    Given 用户无持仓
    When 打开 "持仓" 页面
    Then 显示空状态 "暂无持仓"
      And 提供快捷入口 "去下单"

  Scenario: 持仓盈亏实时更新
    Given 持仓页面已打开
      And 用户持有 "BTC/USDT" 多头，开仓均价 ¥49,000
    When BTC/USDT 当前价从 ¥50,000 变为 ¥51,000
    Then 浮动盈亏从 +¥100 更新为 +¥200
      And 盈亏率从 +2.04% 更新为 +4.08%
      And 数字闪烁效果 (同 Ticker 闪烁规则)
```

---

## 5. 数据模型

### 5.1 新增数据库表

#### orders 表 (委托)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | BIGSERIAL PK | 委托 ID |
| user_id | BIGINT NOT NULL FK → users.id | 用户 ID |
| strategy_id | BIGINT NULL FK → strategies.id | 关联策略 (手动下单为 NULL) |
| symbol | VARCHAR(20) NOT NULL | 交易对 (BTC/USDT) |
| side | VARCHAR(4) NOT NULL | 方向: buy / sell |
| order_type | VARCHAR(10) NOT NULL | 类型: limit / market |
| price | DECIMAL(20,8) NULL | 委托价格 (市价单为 NULL) |
| quantity | DECIMAL(20,8) NOT NULL | 委托数量 |
| filled_quantity | DECIMAL(20,8) DEFAULT 0 | 已成交数量 |
| avg_fill_price | DECIMAL(20,8) NULL | 成交均价 |
| status | VARCHAR(20) NOT NULL | 状态: pending / partial_filled / filled / cancelled / expired / rejected |
| mode | VARCHAR(10) NOT NULL DEFAULT 'paper' | 交易模式: paper / live |
| fee | DECIMAL(20,8) DEFAULT 0 | 手续费 |
| reject_reason | VARCHAR(200) NULL | 拒绝原因 |
| time_in_force | VARCHAR(10) DEFAULT 'GTC' | 有效期: GTC / IOC / FOK |
| expire_at | TIMESTAMPTZ NULL | 过期时间 |
| created_at | TIMESTAMPTZ NOT NULL DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMPTZ NOT NULL DEFAULT NOW() | 更新时间 |
| cancelled_at | TIMESTAMPTZ NULL | 撤单时间 |
| filled_at | TIMESTAMPTZ NULL | 完全成交时间 |

**索引:**
- `idx_orders_user_id` ON (user_id)
- `idx_orders_user_status` ON (user_id, status) — 当前委托查询
- `idx_orders_symbol` ON (symbol)
- `idx_orders_created_at` ON (created_at DESC) — 历史委托排序

#### trades 表 (成交记录)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | BIGSERIAL PK | 成交 ID |
| order_id | BIGINT NOT NULL FK → orders.id | 关联委托 |
| user_id | BIGINT NOT NULL FK → users.id | 用户 ID |
| symbol | VARCHAR(20) NOT NULL | 交易对 |
| side | VARCHAR(4) NOT NULL | 方向 |
| price | DECIMAL(20,8) NOT NULL | 成交价格 |
| quantity | DECIMAL(20,8) NOT NULL | 成交数量 |
| fee | DECIMAL(20,8) NOT NULL DEFAULT 0 | 手续费 |
| is_maker | BOOLEAN DEFAULT FALSE | 是否 Maker |
| created_at | TIMESTAMPTZ NOT NULL DEFAULT NOW() | 成交时间 |

**索引:**
- `idx_trades_order_id` ON (order_id)
- `idx_trades_user_id` ON (user_id)
- `idx_trades_created_at` ON (created_at DESC)

#### positions 表 (持仓)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | BIGSERIAL PK | 持仓 ID |
| user_id | BIGINT NOT NULL FK → users.id | 用户 ID |
| symbol | VARCHAR(20) NOT NULL | 交易对 |
| side | VARCHAR(5) NOT NULL | 方向: long / short |
| quantity | DECIMAL(20,8) NOT NULL | 持仓数量 |
| available_quantity | DECIMAL(20,8) NOT NULL | 可用数量 (扣除冻结) |
| avg_entry_price | DECIMAL(20,8) NOT NULL | 开仓均价 |
| unrealized_pnl | DECIMAL(20,8) DEFAULT 0 | 浮动盈亏 |
| realized_pnl | DECIMAL(20,8) DEFAULT 0 | 已实现盈亏 |
| mode | VARCHAR(10) NOT NULL DEFAULT 'paper' | 交易模式 |
| created_at | TIMESTAMPTZ NOT NULL DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMPTZ NOT NULL DEFAULT NOW() | 更新时间 |

**索引:**
- `uniq_positions_user_symbol_mode` UNIQUE ON (user_id, symbol, mode) — 每用户每交易对每模式一个持仓
- `idx_positions_user_id` ON (user_id)

#### paper_accounts 表 (模拟账户)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | BIGSERIAL PK | 账户 ID |
| user_id | BIGINT NOT NULL UNIQUE FK → users.id | 用户 ID |
| balance | DECIMAL(20,8) NOT NULL | 可用余额 |
| frozen_balance | DECIMAL(20,8) NOT NULL DEFAULT 0 | 冻结余额 |
| initial_balance | DECIMAL(20,8) NOT NULL | 初始资金 |
| total_pnl | DECIMAL(20,8) DEFAULT 0 | 累计盈亏 |
| created_at | TIMESTAMPTZ NOT NULL DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMPTZ NOT NULL DEFAULT NOW() | 更新时间 |

**索引:**
- `uniq_paper_accounts_user_id` UNIQUE ON (user_id)

#### symbol_configs 表 (交易对配置)

| 字段 | 类型 | 说明 |
|------|------|------|
| id | SERIAL PK | ID |
| symbol | VARCHAR(20) NOT NULL UNIQUE | 交易对 |
| base_currency | VARCHAR(10) NOT NULL | 基础货币 (BTC) |
| quote_currency | VARCHAR(10) NOT NULL | 计价货币 (USDT) |
| price_precision | SMALLINT NOT NULL | 价格精度 (2) |
| quantity_precision | SMALLINT NOT NULL | 数量精度 (4) |
| min_quantity | DECIMAL(20,8) NOT NULL | 最小下单量 |
| max_quantity | DECIMAL(20,8) NOT NULL | 最大下单量 |
| min_notional | DECIMAL(20,8) NOT NULL | 最小下单金额 |
| fee_rate | DECIMAL(8,6) NOT NULL DEFAULT 0.001 | 手续费率 |
| enabled | BOOLEAN NOT NULL DEFAULT TRUE | 是否启用 |
| created_at | TIMESTAMPTZ NOT NULL DEFAULT NOW() | 创建时间 |

### 5.2 Redis Key 设计

| Key | 类型 | TTL | 说明 |
|-----|------|-----|------|
| `order:{order_id}` | Hash | 24h | 委托详情缓存 |
| `user_orders:{user_id}:active` | Sorted Set | — | 用户活跃委托 (score=created_at) |
| `position:{user_id}:{symbol}:paper` | Hash | — | 持仓缓存 (实时更新) |
| `paper_account:{user_id}` | Hash | — | 模拟账户余额缓存 |
| `symbol_config:{symbol}` | Hash | 1h | 交易对配置缓存 |
| `match_queue:{symbol}` | List | — | 待撮合委托队列 |

### 5.3 SeaORM Entity 模型

```rust
// models/order.rs
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "orders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,
    pub strategy_id: Option<i64>,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub avg_fill_price: Option<Decimal>,
    pub status: OrderStatus,
    pub mode: TradeMode,
    pub fee: Decimal,
    pub reject_reason: Option<String>,
    pub time_in_force: TimeInForce,
    pub expire_at: Option<DateTime>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub cancelled_at: Option<DateTime>,
    pub filled_at: Option<DateTime>,
}

// 枚举
pub enum OrderSide { Buy, Sell }
pub enum OrderType { Limit, Market }
pub enum OrderStatus { Pending, PartialFilled, Filled, Cancelled, Expired, Rejected }
pub enum TradeMode { Paper, Live }
pub enum TimeInForce { GTC, IOC, FOK }
```

---

## 6. API 设计

### 6.1 REST API 端点

| 方法 | 路径 | 说明 | 优先级 |
|------|------|------|--------|
| POST | /api/v1/orders | 创建委托 | P0 |
| GET | /api/v1/orders | 查询委托列表 | P0 |
| GET | /api/v1/orders/:id | 查询委托详情 | P0 |
| POST | /api/v1/orders/:id/cancel | 撤单 | P0 |
| POST | /api/v1/orders/cancel-all | 批量撤单 | P1 |
| GET | /api/v1/trades | 查询成交记录 | P1 |
| GET | /api/v1/positions | 查询持仓列表 | P1 |
| POST | /api/v1/positions/:id/close | 平仓 (生成市价单) | P1 |
| GET | /api/v1/account | 查询模拟账户 | P0 |
| GET | /api/v1/symbols | 查询交易对配置 | P0 |

### 6.2 API 详细设计

#### POST /api/v1/orders — 创建委托

**请求：**
```json
{
  "symbol": "BTC/USDT",
  "side": "buy",
  "order_type": "limit",
  "price": "49500.00",
  "quantity": "0.1000",
  "time_in_force": "GTC"
}
```

**成功响应 (201)：**
```json
{
  "code": 0,
  "data": {
    "order_id": "123456",
    "symbol": "BTC/USDT",
    "side": "buy",
    "order_type": "limit",
    "price": "49500.00",
    "quantity": "0.1000",
    "filled_quantity": "0.0000",
    "status": "pending",
    "mode": "paper",
    "created_at": "2026-05-14T06:00:00Z"
  },
  "message": "success"
}
```

**错误响应 (400)：**
```json
{
  "code": 40001,
  "message": "余额不足",
  "details": {
    "available": "1000.00",
    "required": "4950.00",
    "currency": "USDT"
  }
}
```

#### GET /api/v1/orders — 查询委托列表

**参数：**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| status | string | 否 | 筛选状态: pending/partial_filled/filled/cancelled/expired/rejected |
| symbol | string | 否 | 筛选交易对 |
| side | string | 否 | 筛选方向: buy/sell |
| start_date | string | 否 | 开始日期 (ISO 8601) |
| end_date | string | 否 | 结束日期 (ISO 8601) |
| page | int | 否 | 页码 (默认 1) |
| size | int | 否 | 每页条数 (默认 20, 最大 100) |

**响应 (200)：**
```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "order_id": "123456",
        "symbol": "BTC/USDT",
        "side": "buy",
        "order_type": "limit",
        "price": "49500.00",
        "quantity": "0.1000",
        "filled_quantity": "0.0500",
        "avg_fill_price": "49500.00",
        "status": "partial_filled",
        "created_at": "2026-05-14T06:00:00Z",
        "updated_at": "2026-05-14T06:05:00Z"
      }
    ],
    "total": 42,
    "page": 1,
    "size": 20
  },
  "message": "success"
}
```

#### POST /api/v1/orders/:id/cancel — 撤单

**成功响应 (200)：**
```json
{
  "code": 0,
  "data": {
    "order_id": "123456",
    "status": "cancelled",
    "filled_quantity": "0.0500",
    "released_amount": "2475.00"
  },
  "message": "success"
}
```

**错误响应 (409)：**
```json
{
  "code": 40901,
  "message": "委托已成交，无法撤销",
  "details": {
    "order_id": "123456",
    "current_status": "filled"
  }
}
```

#### POST /api/v1/orders/cancel-all — 批量撤单

**请求：**
```json
{
  "symbol": "BTC/USDT",
  "side": "buy"
}
```

**响应 (200)：**
```json
{
  "code": 0,
  "data": {
    "cancelled_count": 4,
    "failed_count": 1,
    "failed_orders": [
      {"order_id": "123460", "reason": "已成交"}
    ]
  },
  "message": "success"
}
```

#### GET /api/v1/account — 查询模拟账户

**响应 (200)：**
```json
{
  "code": 0,
  "data": {
    "user_id": 1,
    "balance": "55000.00",
    "frozen_balance": "4950.00",
    "initial_balance": "100000.00",
    "total_pnl": "-40450.00",
    "equity": "60050.00",
    "positions_count": 3,
    "active_orders_count": 5
  },
  "message": "success"
}
```

#### GET /api/v1/symbols — 查询交易对配置

**响应 (200)：**
```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "symbol": "BTC/USDT",
        "base_currency": "BTC",
        "quote_currency": "USDT",
        "price_precision": 2,
        "quantity_precision": 4,
        "min_quantity": "0.0010",
        "max_quantity": "1000.0000",
        "min_notional": "10.00",
        "fee_rate": "0.001000",
        "enabled": true
      }
    ]
  },
  "message": "success"
}
```

### 6.3 WebSocket 消息协议

**连接：** `wss://host/api/v1/trade/ws?token=***`

**客户端→服务端：**
```json
// 订阅委托更新
{
  "action": "subscribe",
  "channels": ["order:all"]
}

// 取消订阅
{
  "action": "unsubscribe",
  "channels": ["order:all"]
}
```

**服务端→客户端：**
```json
// 委托状态变更
{
  "type": "order_update",
  "data": {
    "order_id": "123456",
    "symbol": "BTC/USDT",
    "side": "buy",
    "order_type": "limit",
    "price": "49500.00",
    "quantity": "0.1000",
    "filled_quantity": "0.0500",
    "status": "partial_filled",
    "updated_at": 1715500900000
  },
  "ts": 1715500900000
}

// 成交通知
{
  "type": "trade",
  "data": {
    "trade_id": "789",
    "order_id": "123456",
    "symbol": "BTC/USDT",
    "side": "buy",
    "price": "49500.00",
    "quantity": "0.0500",
    "fee": "2.4750",
    "timestamp": 1715500900000
  },
  "ts": 1715500900000
}

// 持仓更新
{
  "type": "position_update",
  "data": {
    "symbol": "BTC/USDT",
    "side": "long",
    "quantity": "0.0500",
    "avg_entry_price": "49500.00",
    "unrealized_pnl": "50.00"
  },
  "ts": 1715500900000
}

// 订阅确认
{
  "type": "subscribed",
  "channels": ["order:all"]
}

// 错误
{
  "type": "error",
  "code": 40001,
  "message": "Invalid channel"
}
```

### 6.4 错误码

| 错误码 | HTTP | 说明 |
|--------|------|------|
| 40001 | 400 | 余额不足 |
| 40002 | 400 | 持仓不足 |
| 40003 | 400 | 不支持的交易对 |
| 40004 | 400 | 数量无效 (零/负/超精度) |
| 40005 | 400 | 价格无效 (零/负/超精度) |
| 40006 | 400 | 低于最小下单量 |
| 40007 | 400 | 低于最小下单金额 |
| 40101 | 401 | 未登录 |
| 40301 | 403 | 权限不足 (trader 无法实盘) |
| 40401 | 404 | 委托不存在 |
| 40901 | 409 | 委托状态冲突 (已成交无法撤单) |
| 42901 | 429 | 未成交委托数量达上限 |
| 50301 | 503 | 行情数据不可用 (市价单) |

---

## 7. 前端路由与页面设计

### 7.1 路由

| 路径 | 组件 | 说明 |
|------|------|------|
| `/trade` | `TradingView.vue` | 交易主页面 (三栏布局) |
| `/trade/:symbol` | `TradingView.vue` | 指定交易对的交易页面 |
| `/orders` | `OrdersView.vue` | 委托管理页面 (当前+历史) |
| `/portfolio` | `PortfolioView.vue` | 持仓页面 |

### 7.2 交易页面布局

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  [模拟交易]  BTC/USDT ▼                              买一 50,099 / 卖一 50,100 │
├───────────────────────────┬────────────────┬─────────────────────────────────┤
│  [左: 图表区 flex:1]      │ [中: 下单 360px]│  [右: 委托区 flex:1]            │
│                           │                │                                 │
│  ┌─────────────────────┐  │ ┌────────────┐ │  ┌───────────────────────────┐ │
│  │ K线图               │  │ │ [买入] [卖出]│ │  │ [当前委托(3)] [历史委托]  │ │
│  │ (复用 MarketChart)  │  │ ├────────────┤ │  ├───────────────────────────┤ │
│  │                     │  │ │ [限价] [市价]│ │  │ 时间 | 交易对 | 方向 | ... │ │
│  │                     │  │ ├────────────┤ │  │ 06:05 BTC/USDT 买 49500  │ │
│  ├─────────────────────┤  │ │ 价格: 49500 │ │  │                     0.1  │ │
│  │ 深度图              │  │ │ 数量: 0.1   │ │  │ 06:03 ETH/USDT 卖 3200  │ │
│  │ (复用 DepthChart)   │  │ │ [25%][50%]  │ │  │                     1.0  │ │
│  │                     │  │ │ [75%][100%] │ │  │ 06:01 BTC/USDT 买 49000  │ │
│  │                     │  │ ├────────────┤ │  │                     0.2  │ │
│  │                     │  │ │ 可用: ¥10k  │ │  ├───────────────────────────┤ │
│  │                     │  │ │ 预估: ¥4950 │ │  │ [撤单] 按钮在每行末尾     │ │
│  │                     │  │ ├────────────┤ │  │                           │ │
│  │                     │  │ │ [提交委托]  │ │  └───────────────────────────┘ │
│  └─────────────────────┘  │ └────────────┘ │                                 │
└───────────────────────────┴────────────────┴─────────────────────────────────┘
```

### 7.3 组件树

```
TradingView.vue
├── TradingHeader.vue                   # 交易模式标识 + 交易对选择 + 实时价格
├── TradingChartPanel.vue               # 左侧图表区
│   ├── MarketChart.vue                 # K线图 (复用行情模块)
│   └── DepthChart.vue                  # 深度图 (复用行情模块)
├── OrderForm.vue                       # 中间下单区
│   ├── SideToggle.vue                  # 买入/卖出切换
│   ├── OrderTypeSelect.vue             # 限价/市价选择
│   ├── PriceInput.vue                  # 价格输入 (限价单)
│   ├── QuantityInput.vue               # 数量输入 + 精度校验
│   ├── QuantitySlider.vue              # 数量百分比滑块 (25%/50%/75%/100%)
│   ├── AccountInfo.vue                 # 可用余额/持仓信息
│   ├── OrderSummary.vue                # 预估金额/手续费
│   └── OrderConfirmDialog.vue          # 确认对话框
└── OrdersPanel.vue                     # 右侧委托区
    ├── ActiveOrdersTab.vue             # 当前委托
    │   ├── OrderRow.vue                # 委托行
    │   └── CancelAllButton.vue         # 全部撤单
    └── HistoryOrdersTab.vue            # 历史委托
        ├── OrderFilter.vue             # 筛选器 (日期/状态/交易对)
        └── OrderDetailDrawer.vue       # 委托详情抽屉

OrdersView.vue                          # 独立委托管理页
├── OrdersTabs.vue                      # 当前/历史 Tab
├── ActiveOrdersTable.vue               # 当前委托表格
├── HistoryOrdersTable.vue              # 历史委托表格
└── OrderFilterBar.vue                  # 筛选栏

PortfolioView.vue                       # 持仓页面
├── PortfolioSummary.vue                # 账户汇总 (余额/权益/盈亏)
├── PositionsTable.vue                  # 持仓列表
│   └── PositionRow.vue                 # 持仓行 (含平仓按钮)
└── ClosePositionDialog.vue             # 平仓确认
```

### 7.4 前端状态管理 (Pinia Store)

```typescript
// stores/trade.ts
interface TradeState {
  currentSymbol: string                  // 当前交易对
  tradeMode: 'paper' | 'live'           // 交易模式
  orderForm: OrderFormState              // 下单表单状态
  activeOrders: Order[]                  // 当前委托
  historyOrders: Order[]                 // 历史委托
  positions: Position[]                  // 持仓列表
  account: PaperAccount | null           // 模拟账户
  symbolConfigs: Record<string, SymbolConfig>  // 交易对配置
  wsConnected: boolean                   // WebSocket 连接状态
}

interface OrderFormState {
  side: 'buy' | 'sell'
  orderType: 'limit' | 'market'
  price: string
  quantity: string
  timeInForce: 'GTC'
}
```

### 7.5 前端 WebSocket 客户端设计

```typescript
// composables/useTradeWs.ts
interface UseTradeWs {
  connect: () => void
  disconnect: () => void
  subscribe: (channels: string[]) => void
  unsubscribe: (channels: string[]) => void
  onOrderUpdate: (callback: (data: OrderUpdateMessage) => void) => void
  onTrade: (callback: (data: TradeMessage) => void) => void
  onPositionUpdate: (callback: (data: PositionUpdateMessage) => void) => void
  connectionStatus: Ref<'connecting' | 'connected' | 'disconnected' | 'reconnecting'>
}
```

**重连策略：** 同行情模块 — 指数退避 1s → 30s，重连后自动恢复订阅，补发断线期间状态变更。

---

## 8. 后端设计

### 8.1 新增模块

```
backend/src/
├── handlers/
│   └── trade.rs              # 交易 REST API handlers
├── services/
│   ├── trading_engine.rs     # 交易核心逻辑 (下单/撤单/风控)
│   └── matching_engine.rs    # 模拟撮合引擎
├── db/
│   ├── order.rs              # Order CRUD
│   ├── trade.rs              # Trade CRUD
│   ├── position.rs           # Position CRUD
│   └── paper_account.rs      # PaperAccount CRUD
└── models/
    ├── order.rs              # Order Entity + 枚举
    ├── trade.rs              # Trade Entity
    └── position.rs           # Position Entity
```

### 8.2 TradingEngine (交易核心)

```rust
// services/trading_engine.rs

pub struct TradingEngine {
    db: Arc<DatabaseConnection>,
    redis: redis::Client,
    matching_engine: Arc<MatchingEngine>,
    risk_checker: RiskChecker,
    ws_hub: Arc<TradeWsHub>,
}

impl TradingEngine {
    /// 创建委托 (入口)
    pub async fn place_order(&self, req: PlaceOrderRequest) -> Result<Order>;

    /// 风控前置校验
    async fn validate_order(&self, req: &PlaceOrderRequest) -> Result<()>;

    /// 冻结保证金/持仓
    async fn freeze_margin(&self, order: &Order) -> Result<()>;

    /// 撤单
    pub async fn cancel_order(&self, user_id: i64, order_id: i64) -> Result<CancelResult>;

    /// 批量撤单
    pub async fn cancel_all_orders(&self, user_id: i64, filter: CancelFilter) -> Result<CancelAllResult>;

    /// 查询委托列表
    pub async fn list_orders(&self, user_id: i64, filter: OrderFilter) -> Result<PaginatedResult<Order>>;

    /// 查询持仓
    pub async fn list_positions(&self, user_id: i64) -> Result<Vec<Position>>;

    /// 查询账户
    pub async fn get_account(&self, user_id: i64) -> Result<PaperAccount>;
}
```

### 8.3 MatchingEngine (模拟撮合)

```rust
// services/matching_engine.rs

pub struct MatchingEngine {
    db: Arc<DatabaseConnection>,
    redis: redis::Client,
    ws_hub: Arc<TradeWsHub>,
    depth_cache: Arc<DepthCache>,        // 从 Redis 读取实时深度
    delay_ms: u64,                         // 延迟模拟
}

impl MatchingEngine {
    /// 尝试撮合委托 (限价单)
    pub async fn try_match_limit(&self, order: &Order) -> Result<MatchResult>;

    /// 执行市价单撮合
    pub async fn match_market(&self, order: &Order) -> Result<MatchResult>;

    /// 逐档撮合 (按深度数据)
    async fn match_by_depth(&self, symbol: &str, side: &OrderSide, quantity: Decimal) -> Result<Vec<Trade>>;

    /// 生成成交记录
    async fn record_trades(&self, order_id: i64, trades: Vec<Trade>) -> Result<()>;

    /// 更新持仓
    async fn update_position(&self, user_id: i64, symbol: &str, side: &OrderSide, quantity: Decimal, price: Decimal) -> Result<()>;

    /// 释放保证金
    async fn release_margin(&self, user_id: i64, amount: Decimal) -> Result<()>;

    /// 定时检查: 限价单是否可撮合 (深度变化时触发)
    pub async fn on_depth_update(&self, symbol: &str, depth: &DepthData) -> Result<()>;

    /// 定时检查: 过期委托
    pub async fn check_expired_orders(&self) -> Result<u64>;
}
```

### 8.4 数据流

```
[用户提交委托]
    ↓
[TradingEngine: 风控校验 (余额/持仓/交易对/数量/精度)]
    ↓ 通过
[TradingEngine: 冻结保证金 → 写入 orders 表 (status=pending)]
    ↓
[Redis: 写入 match_queue + user_orders:active]
    ↓
[WebSocket: 推送 order_update (status=pending)]
    ↓
┌── 限价单 ──────────────────────┐  ┌── 市价单 ─────────────────────┐
│ [MatchingEngine: 等待深度变化]  │  │ [MatchingEngine: 立即撮合]    │
│   ↓                            │  │   ↓                           │
│ [on_depth_update 触发撮合]     │  │ [match_by_depth: 逐档匹配]    │
│   ↓                            │  │   ↓                           │
│ [部分/全部成交 → 更新状态]     │  │ [记录成交 → 更新持仓/余额]     │
│   ↓                            │  │   ↓                           │
│ [WS 推送 order_update + trade] │  │ [WS 推送 order_update + trade]│
└────────────────────────────────┘  └────────────────────────────────┘
```

### 8.5 降级策略

| 故障场景 | 降级行为 | 恢复条件 |
|----------|---------|---------|
| Redis 不可用 | 保证金/余额降级为 PG 事务查询，撮合延迟增加 | Redis 恢复 |
| 深度数据断开 | 限价单正常挂单，市价单拒绝 (提示 "行情不可用") | 深度数据恢复 |
| PostgreSQL 不可用 | 新委托拒绝 (503)，已提交委托撮合暂停 | PG 恢复 |
| MatchingEngine 延迟 | 委托进入队列排队，前端显示 "撮合中" 状态 | 引擎恢复 |
| WebSocket Hub 不可用 | 委托仍可提交，前端降级为 REST 轮询 (5s) | WS Hub 恢复 |

---

## 9. 边界情况

| 边界情况 | 处理方式 |
|----------|---------|
| 委托提交与撤单并发 | 使用 PG 行锁 (SELECT FOR UPDATE) 保证状态一致性 |
| 部分成交后撤单 | 保留已成交部分，仅撤销未成交部分 |
| 市价单深度不足 | 按可用深度部分撮合，剩余等待；若完全无深度则拒绝 |
| 限价单价格偏离过大 | 前端弹出风险提示 (偏离 > 10%)，不阻止提交 |
| 用户同时打开多个标签页 | 每个标签页独立 WS 连接，委托状态通过 WS 广播至所有连接 |
| 撤单时委托刚好被撮合 | 状态竞争: 先检查当前状态，若已成交则撤单失败并提示 |
| 余额精度问题 (浮点) | 全链路使用 Decimal (Rust: rust_decimal, JS: big.js)，避免浮点误差 |
| 模拟账户余额为 0 | 限制下单，提示 "模拟账户余额不足，请联系管理员重置" |
| 交易对精度配置变更 | 已有委托不受影响 (按下单时精度执行) |
| 极端行情 (闪崩) | 撮合引擎照常按深度撮合，前端显示实际成交价 |
| 手续费计算 | 买入: 冻结金额 = 价格 * 数量 * (1 + fee_rate)，卖出: 手续费从成交金额中扣除 |
| 保证金冻结精度 | 冻结金额 = ceil(价格 * 数量 * (1 + fee_rate), quote_precision) |
| 委托过期时间校验 | 过期时间必须 > 当前时间 + 1min，否则拒绝 |
| 批量撤单部分失败 | 返回成功/失败计数 + 失败委托列表，不回滚成功的撤单 |

---

## 10. 非功能性需求

### 10.1 性能

| 指标 | 目标值 |
|------|--------|
| 下单 API 响应 (P99) | < 200ms |
| 撤单 API 响应 (P99) | < 200ms |
| 委托列表加载 | < 500ms |
| 撮合延迟 (模拟) | < 1s (含 200ms 人为延迟) |
| WebSocket 推送延迟 | < 500ms |
| 并发下单 | 单用户同时 50 个未成交委托 |
| 全系统并发 | ≥ 100 QPS (下单+撤单) |

### 10.2 可靠性

| 要求 | 说明 |
|------|------|
| 委托状态一致性 | PG 事务保证，Redis 为缓存层 (可从 PG 恢复) |
| 保证金一致性 | 下单冻结/撤单释放/成交结算 均在 PG 事务内完成 |
| 撮合幂等 | 同一委托不会重复撮合 (基于 status + filled_quantity 校验) |
| 断线重连 | WS 重连后补发状态变更，前端幂等去重 |
| 定时清理 | 每分钟检查过期委托，自动撤销 |

### 10.3 安全

| 要求 | 说明 |
|------|------|
| 权限隔离 | 用户只能查看/操作自己的委托和持仓 |
| 风控前置 | 所有委托必须经过风控校验后才能进入撮合 |
| 操作审计 | 下单/撤单/成交 操作写入审计日志 (user_id, action, order_id, timestamp) |
| 限流 | 每用户 100 req/min (REST)，10 订阅/连接 (WS) |
| 金额精度 | 全链路 Decimal，防止浮点截断攻击 |
| SQL 注入 | SeaORM 参数化查询 |

---

## 11. 优先级矩阵

| 用户故事 | 优先级 | 说明 |
|----------|--------|------|
| US-TE-01: 限价单下单 | P0 | 核心交易功能 |
| US-TE-02: 市价单下单 | P0 | 核心交易功能 |
| US-TE-03: 下单风控校验 | P0 | 安全基础，P0 必须有 |
| US-TE-04: 查看当前委托 | P0 | 交易闭环必要 |
| US-TE-05: 撤单 | P0 | 交易闭环必要 |
| US-TE-07: 模拟撮合引擎 | P0 | 模拟交易核心 |
| US-TE-08: 交易页面布局与交互 | P0 | 用户体验，整合下单+委托 |
| US-TE-06: 查看历史委托 | P1 | 增强功能 |
| US-TE-09: 委托 WebSocket 实时推送 | P1 | 实时性增强，可降级为轮询 |
| US-TE-10: 持仓管理 | P1 | 持仓查看+平仓 |

**总计: P0 x 7 / P1 x 3**

---

## 12. 实施分阶段建议

### Phase 1: 后端核心 (1-2 周)

1. 创建数据库表 (orders / trades / positions / paper_accounts / symbol_configs)
2. 实现 SeaORM Entity + Migration
3. 实现 `TradingEngine` (place_order / cancel_order / list_orders)
4. 实现 `RiskChecker` (余额/持仓/交易对/精度校验)
5. 实现 `MatchingEngine` (市价单即时撮合 + 限价单深度触发撮合)
6. 实现 `handlers/trade.rs` (REST API)
7. 实现 paper_accounts 初始化 (注册时自动创建，初始资金 ¥100,000)
8. 实现 symbol_configs 种子数据 (BTC/USDT, ETH/USDT, SOL/USDT)

### Phase 2: 前端核心 (1-2 周)

1. 实现 `TradingView.vue` 三栏布局
2. 实现 `OrderForm.vue` (限价/市价切换 + 价格/数量输入 + 精度校验)
3. 实现 `OrderConfirmDialog.vue` (确认对话框 + 风险提示)
4. 实现 `ActiveOrdersTab.vue` (当前委托列表)
5. 实现 `HistoryOrdersTab.vue` (历史委托列表 + 筛选)
6. 实现 `PositionsTable.vue` (持仓列表 + 平仓)
7. 实现 `stores/trade.ts` (Pinia 状态管理)
8. 实现 `api/trades.ts` 更新 (对接新 API 端点)

### Phase 3: 实时推送与完善 (1 周)

1. 实现 `TradeWsHub` (委托状态变更 + 成交 + 持仓推送)
2. 实现 `composables/useTradeWs.ts` (前端 WS 客户端)
3. 实现批量撤单
4. 实现委托过期检查 (定时任务)
5. 实现降级和边界情况处理
6. 集成测试 + 性能测试

---

## 13. 关联文档

| 文档 | 关系 |
|------|------|
| PRD.md US-11/US-12 | 原始用户故事定义 |
| PRD-market-module.md | 行情数据联动 (深度/价格) |
| PRD-backtest-engine.md | 回测撮合逻辑复用参考 |
| TECH_CHARTER-quant.md | 技术栈和开发规范 |
| data-model.md | 数据库 schema 定义 |
