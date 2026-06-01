# PRD: 交易执行模块 — 下单、委托管理、撮合与持仓

> 版本: v1.0
> 状态: Draft
> 作者: PM
> 关联: PRD.md US-11/US-12/US-13/US-15, ADR-006 (撮合引擎), data-model.md (交易域), PRD-market-module.md, PRD-strategy-sandbox-mvp.md

---

## 1. 背景

交易执行是量化交易系统的"手"——用户通过它将策略信号或手动判断转化为实际委托，完成从下单到成交到持仓的完整交易闭环。主 PRD (v1.0) 将交易执行列为 P1 核心功能，定义了 US-11 (手动下单)、US-12 (委托管理)、US-13 (持仓概览)、US-15 (风控指标) 四个用户故事。

### 当前状态 (2026-05-14)

- 前端 `views/trade/` 目录存在占位组件 `TradeView.vue`，显示 "Trade execution coming soon"
- 前端 `api/trade.ts` 不存在
- 前端 `types/trade.ts` 不存在
- 后端 `handlers/trade.rs` 在 TECH_CHARTER 中有占位但未创建
- 后端 `services/risk_manager.rs` 在 TECH_CHARTER 中有占位但未创建
- 数据库 `orders` / `trades` / `positions` 三张表已在 data-model.md 中定义，但尚未迁移
- ADR-006 已批准撮合引擎方案：SIMULATE (内存 Order Book) + BACKTEST (OHLC) + LIVE (交易所透传)
- PRD-market-module.md 已覆盖行情数据，本模块可复用实时行情推送
- PRD-strategy-sandbox-mvp.md 已覆盖策略管理，策略信号通过 Redis PubSub 发出

### 本 PRD 与其他 PRD 的关系

| 范围 | PRD-market-module | PRD-strategy-sandbox-mvp | PRD-backtest-engine | 本 PRD (交易执行) |
|------|-------------------|--------------------------|---------------------|-------------------|
| 行情数据展示 | ✅ | — | — | 依赖 (实时价格) |
| 策略信号生成 | — | ✅ | — | 消费 (信号→委托) |
| 回测撮合 | — | — | ✅ | 互补 (回测用独立撮合) |
| 手动下单 | — | — | — | ✅ |
| 委托管理 | — | — | — | ✅ |
| 模拟撮合 | — | — | — | ✅ |
| 持仓管理 | — | — | — | ✅ |
| 风控拦截 | — | — | — | ✅ |
| 成交记录 | — | — | — | ✅ |

---

## 2. 目标

| 目标 | 指标 | 当前状态 |
|------|------|----------|
| 手动下单 | 支持限价单/市价单/止损单，3 步内完成下单 | TradeView 为占位组件 |
| 模拟撮合 | 限价单/市价单可成交，延迟 < 500ms | 撮合引擎未实现 |
| 委托管理 | 实时查看当前委托，一键撤单 | 不存在 |
| 成交记录 | 完整成交历史，支持筛选/导出 | 不存在 |
| 持仓管理 | 实时持仓概览 + 浮动盈亏 + 平仓操作 | 不存在 |
| 风控拦截 | 下单前风控校验，超标拒绝/警告 | risk_manager.rs 未创建 |
| 策略信号→委托 | 策略信号自动生成委托，无需人工 | 策略信号 PubSub 已定义 |
| API 响应 | 下单/撤单接口 P99 < 200ms | 未实现 |

### 非目标 (Non-Goals)

- ❌ 实盘交易 (LIVE 模式透传交易所 API，第二阶段)
- ❌ 高级订单类型 (冰山单/OCO/条件单，P2+)
- ❌ 杠杆/保证金交易 (仅支持现货全额交易，MVP 不涉及杠杆)
- ❌ 跨交易所套利执行 (P2+)
- ❌ 移动端适配 (P2+)
- ❌ 策略回测内撮合 (PRD-backtest-engine 范围)

---

## 3. 用户角色与权限

| 角色 | 交易权限 | 说明 |
|------|---------|------|
| **trader** (普通用户) | 模拟交易：下单/撤单/查看委托/查看持仓/查看成交 | 默认角色，仅模拟模式 |
| **pro-trader** (专业交易员) | 模拟交易 + 实盘交易 (第二阶段启用) | 需管理员开通 |
| **admin** (管理员) | 查看所有用户委托/持仓/成交，不可代操作 | 审计用途，可强制撤单 |

> 注：MVP 阶段所有交易均为"模拟模式" (simulation)，不涉及真实资金。pro-trader 权限预埋但不启用实盘开关。

---

## 4. 用户故事 (User Stories)

### US-TE-01: 下单面板 (P0)

> **As a** 交易者
> **I want to** 在行情页面右侧快速下单
> **So that** 我可以边看行情边执行交易，无需切换页面

**验收条件：**

```gherkin
Feature: 下单面板

  Background:
    Given 用户已登录
      And 当前处于行情页面 /market 或交易页面 /trade

  Scenario: 下单面板展示
    Given 用户在行情页面
    Then 右侧面板显示"下单"区域
      And 包含: 交易对选择器、方向选择(买入/卖出)、委托类型选择、价格输入、数量输入、金额显示、提交按钮
      And 买入按钮为绿色，卖出按钮为红色
      And 默认交易对与当前行情页选中的交易对一致

  Scenario: 限价单下单
    Given 下单面板已打开
    When 选择委托类型 "限价单"
      And 输入价格 50000.00
      And 输入数量 0.1
      And 金额显示为 5000.00 USDT
      And 选择方向 "买入"
    When 点击"买入限价"按钮
    Then 弹出确认对话框:
      | 字段 | 值 |
      | 交易对 | BTC/USDT |
      | 方向 | 买入 |
      | 类型 | 限价单 |
      | 价格 | 50000.00 |
      | 数量 | 0.1 BTC |
      | 总金额 | 5000.00 USDT |
      | 模式 | 模拟 |
    When 确认提交
    Then 委托提交成功
      And Toast 提示 "委托已提交"
      And 委托出现在"当前委托"列表
      And 状态为 "pending"

  Scenario: 市价单下单
    Given 下单面板已打开
    When 选择委托类型 "市价单"
    Then 价格输入框隐藏
      And 显示 "以市场最优价成交" 提示
    When 输入数量 0.1
      And 选择方向 "卖出"
      And 点击"卖出市价"按钮
      And 确认提交
    Then 委托提交成功
      And 委托立即进入撮合引擎
      And 成交后状态更新为 "filled"

  Scenario: 止损单下单
    Given 下单面板已打开
    When 选择委托类型 "止损单"
    Then 显示触发价输入框和委托价输入框
    When 输入触发价 48000.00
      And 输入委托价 47900.00
      And 输入数量 0.1
      And 选择方向 "卖出"
    Then 金额显示为触发说明 "当价格跌至 48000.00 时，以 47900.00 卖出 0.1 BTC"

  Scenario: 价格联动行情
    Given 下单面板已打开
      And 当前交易对为 BTC/USDT
      And 最新价为 50500.00
    When 点击价格输入框旁的"市价"按钮
    Then 价格自动填充为 50500.00
    When 行情推送新价格 50600.00
    Then 已填入的价格不自动更新（避免误操作）
      And 价格输入框旁显示最新价参考

  Scenario: 数量快捷选择
    Given 下单面板已打开
    Then 显示快捷百分比按钮: 25% / 50% / 75% / 100%
    When 用户可用余额为 10000 USDT
      And 点击 "25%" 按钮
    Then 数量自动计算为: (10000 * 0.25 / 当前价) 并填入

  Scenario: 输入校验 - 必填项
    Given 下单面板已打开
    When 价格为空
      And 数量已填写
    Then "提交"按钮禁用
      And 价格输入框显示红色边框
    When 价格填写为 0 或负数
    Then 提示 "价格必须大于0"

  Scenario: 输入校验 - 数量精度
    Given 当前交易对 BTC/USDT 最小下单量为 0.001
    When 输入数量 0.0001
    Then 提示 "最小下单量: 0.001 BTC"
      And 提交按钮禁用
    When 输入数量 0.12345678 (超过8位小数)
    Then 数量自动截断为 0.12345678

  Scenario: 余额不足提示
    Given 用户可用余额为 1000 USDT
    When 输入限价买单: 价格 50000, 数量 0.1 (总金额 5000 USDT)
    Then 金额显示红色
      And 提示 "余额不足，可用: 1000.00 USDT"
      And 提交按钮禁用

  Scenario: 模拟模式标识
    Given 当前为模拟交易模式
    Then 下单面板顶部显示 "模拟交易" 标签 (橙色)
      And 确认对话框包含 "模拟交易 — 不涉及真实资金" 提示
```

---

### US-TE-02: 模拟撮合引擎 (P0)

> **As a** 系统
> **I want to** 对模拟模式委托进行本地撮合
> **So that** 用户可以在无真实资金的情况下体验完整的交易流程

**验收条件：**

```gherkin
Feature: 模拟撮合引擎

  Background:
    Given 系统已启动
      And 撮合引擎为 SIMULATE 模式
      And Market Data Collector 已提供实时行情

  Scenario: 市价单即时成交
    Given 用户提交 BTC/USDT 市价买入单 数量 0.1
      And 最新卖一价为 50500.00
    When 委托进入撮合引擎
    Then 以 50500.00 价格成交 0.1 BTC
      And 生成成交记录
      And 委托状态更新为 "filled"
      And 用户的 BTC 持仓增加 0.1
      And 用户的 USDT 余额减少 5050.00
      And WebSocket 推送成交通知

  Scenario: 限价单挂单
    Given 用户提交 BTC/USDT 限价买入单 价格 50000.00 数量 0.1
      And 当前卖一价为 50500.00 (高于限价)
    When 委托进入撮合引擎
    Then 委托进入 Order Book 买盘
      And 委托状态为 "pending"
      And 不产生成交记录

  Scenario: 限价单部分成交
    Given 用户提交 BTC/USDT 限价买入单 价格 50500.00 数量 0.5
      And Order Book 卖一价 50500.00 数量 0.3
    When 委托进入撮合引擎
    Then 成交 0.3 BTC @ 50500.00
      And 委托状态为 "partial_filled"
      And filled 更新为 0.3
      And 剩余 0.2 继续挂在 Order Book
      And WebSocket 推送部分成交通知

  Scenario: 限价单完全成交
    Given Order Book 中有卖一 50500.00 x 0.3, 卖二 50501.00 x 0.3
      And 用户提交限价买入 价格 50501.00 数量 0.5
    When 委托进入撮合引擎
    Then 成交 0.3 @ 50500.00
      And 成交 0.2 @ 50501.00
      And 委托状态为 "filled"
      And filled 更新为 0.5
      And 生成 2 条成交记录

  Scenario: 止损单触发
    Given 用户提交止损卖出单 触发价 48000.00 委托价 47900.00 数量 0.1
      And 当前价格 49000.00
    When 行情推送最新价 48000.00
    Then 止损单被触发
      And 生成限价卖出委托 价格 47900.00 数量 0.1
      And 委托进入 Order Book 卖盘

  Scenario: 撮合价格优先时间优先
    Given Order Book 买盘有:
      | 价格 | 数量 | 时间 |
      | 50100 | 0.2 | T1 |
      | 50000 | 0.3 | T2 |
      | 50000 | 0.1 | T3 |
    When 新卖出委托 价格 50000 数量 0.2 进入
    Then 优先成交 50100 x 0.2 (价格优先)
      And 剩余卖出 0 挂入卖盘

  Scenario: 模拟手续费计算
    Given 系统配置模拟手续费率 0.001 (0.1%)
    When 成交 0.1 BTC @ 50500.00
    Then 手续费 = 5050.00 * 0.001 = 5.05 USDT
      And 成交记录中 fee 字段为 5.05
      And 用户余额扣除手续费

  Scenario: 撮合引擎初始化
    Given 系统启动
    When 撮合引擎初始化
    Then 从 Redis 加载各交易对最新深度数据初始化 Order Book
      And 从数据库加载 pending 状态的委托恢复 Order Book
      And 撮合引擎就绪状态通过健康检查暴露
```

---

### US-TE-03: 委托管理 (P0)

> **As a** 交易者
> **I want to** 查看和管理我的所有委托
> **So that** 我可以跟踪订单状态和及时撤单

**验收条件：**

```gherkin
Feature: 委托管理

  Background:
    Given 用户已登录

  Scenario: 当前委托列表
    Given 用户有 3 个未完全成交的委托
    When 导航至 /trade 页面
      And 切换至"当前委托" Tab
    Then 显示未完全成交的委托列表
      And 每行包含: 时间、交易对、方向(buy/sell)、类型、价格、数量、已成交、状态、操作
      And 买入方向显示绿色标签 "Buy"
      And 卖出方向显示红色标签 "Sell"
      And 状态为 "pending" 的委托显示"撤单"按钮
      And 状态为 "partial_filled" 的委托显示"撤单"按钮
      And 列表按时间降序排列

  Scenario: 撤单操作
    Given 存在状态为 "pending" 的委托
    When 点击"撤单"按钮
    Then 弹出确认对话框 "确认撤销该委托？"
    When 确认撤销
    Then 发送撤单请求
      And 委托状态更新为 "cancelled"
      And 已成交部分保留 (partial_filled→cancelled 时 filled 量不变)
      And Toast 提示 "委托已撤销"
      And 委托从"当前委托"列表移至"历史委托"

  Scenario: 撤单 - 部分成交
    Given 存在状态为 "partial_filled" 的委托 (已成交 0.3/0.5)
    When 撤销该委托
    Then 未成交部分 0.2 被撤销
      And 已成交部分 0.3 保留
      And 生成持仓记录 (0.3 BTC)
      And 委托状态变为 "cancelled"

  Scenario: 历史委托列表
    Given 用户有 50 条历史委托
    When 切换至"历史委托" Tab
    Then 显示已成交/已撤销/已过期的委托
      And 每页 20 条，底部显示分页
      And 支持按日期范围筛选
      And 支持按交易对筛选
      And 支持按状态筛选 (filled/cancelled/expired/rejected)

  Scenario: 委托状态实时更新
    Given 用户在"当前委托"页面
      And 有一个 pending 状态的限价买入单
    When 撮合引擎成交了该委托
    Then 委托状态实时更新为 "filled" (通过 WebSocket 推送)
      And 委托行从"当前委托"消失
      And 自动出现在"历史委托"列表顶部
      And 显示成交价格和成交时间

  Scenario: 委托详情
    Given 委托列表已显示
    When 点击某委托行
    Then 展开委托详情面板:
      | 字段 | 说明 |
      | 委托ID | UUID |
      | 关联策略 | 如有 (否则显示"手动下单") |
      | 创建时间 | 精确到毫秒 |
      | 更新时间 | 最后状态变更时间 |
      | 成交明细 | 该委托的所有成交记录列表 |
      | 手续费 | 累计手续费 |
```

---

### US-TE-04: 成交记录 (P0)

> **As a** 交易者
> **I want to** 查看我的所有成交记录
> **So that** 我可以分析交易执行情况和复盘交易

**验收条件：**

```gherkin
Feature: 成交记录

  Background:
    Given 用户已登录

  Scenario: 成交记录列表
    Given 用户有成交记录
    When 导航至 /trade 页面
      And 切换至"成交记录" Tab
    Then 显示成交记录列表
      And 每行包含: 成交时间、交易对、方向、价格、数量、金额、手续费、关联委托ID
      And 列表按时间降序排列

  Scenario: 成交记录筛选
    Given 成交记录列表已显示
    When 选择筛选条件:
      | 筛选项 | 说明 |
      | 交易对 | 下拉选择 |
      | 方向 | 买入/卖出 |
      | 日期范围 | 起止日期 |
    Then 列表按条件过滤
      And 显示匹配的记录数

  Scenario: 成交记录分页
    Given 用户有 100 条成交记录
    When 查看成交记录列表
    Then 每页 20 条
      And 底部显示分页组件
      And 翻页后加载下一页数据

  Scenario: 成交记录实时推送
    Given 用户在成交记录页面
    When 撮合引擎产生新成交
    Then 成交记录列表顶部插入新记录 (WebSocket 推送)
      And 新记录高亮显示 3 秒
```

---

### US-TE-05: 持仓管理 (P0)

> **As a** 交易者
> **I want to** 查看和管理我的持仓
> **So that** 我可以了解当前资产配置和浮动盈亏

**验收条件：**

```gherkin
Feature: 持仓管理

  Background:
    Given 用户已登录

  Scenario: 持仓列表
    Given 用户有 3 个持仓
    When 导航至 /trade 页面
      And 切换至"持仓" Tab
    Then 显示持仓列表
      And 每行包含: 交易对、方向(多/空)、持仓数量、开仓均价、当前价、浮动盈亏、盈亏率、操作
      And 浮动盈亏实时更新 (通过 WebSocket 行情推送)
      And 盈利显示绿色，亏损显示红色

  Scenario: 持仓盈亏实时更新
    Given 持仓列表已显示
      And 用户持有 0.5 BTC 均价 50000
      And 当前 BTC 价格 50500
    Then 浮动盈亏显示 +250.00 USDT
      And 盈亏率显示 +0.50%
    When 行情推送价格变为 49000
    Then 浮动盈亏更新为 -500.00 USDT
      And 盈亏率更新为 -1.00%
      And 颜色变为红色

  Scenario: 平仓操作
    Given 用户持有 0.5 BTC
    When 点击"平仓"按钮
    Then 弹出平仓确认对话框:
      | 字段 | 值 |
      | 交易对 | BTC/USDT |
      | 平仓方式 | 市价 (默认) |
      | 平仓数量 | 全部 (0.5 BTC) / 部分输入 |
    When 选择 "全部" 并确认
    Then 生成市价卖出委托 0.5 BTC
      And 委托进入撮合引擎
      And 成交后持仓清零
      And 盈亏计入已实现盈亏
      And Toast 提示 "平仓委托已提交"

  Scenario: 部分平仓
    Given 用户持有 0.5 BTC
    When 选择部分平仓 0.2 BTC
      And 确认
    Then 生成市价卖出委托 0.2 BTC
      And 成交后持仓数量变为 0.3 BTC
      And 开仓均价不变
      And 已实现盈亏增加 (0.2 * (成交价 - 均价))

  Scenario: 持仓为空
    Given 用户没有任何持仓
    When 查看持仓 Tab
    Then 显示空状态 "暂无持仓"
      And 提示 "下单后持仓将在这里显示"

  Scenario: 持仓汇总
    Given 持仓列表已显示
    Then 底部显示汇总行:
      | 字段 | 说明 |
      | 总持仓市值 | 所有持仓按当前价计算的总市值 |
      | 总浮动盈亏 | 所有持仓浮动盈亏之和 |
      | 总盈亏率 | 总浮动盈亏 / 总开仓金额 |
```

---

### US-TE-06: 风控拦截 (P1)

> **As a** 交易者
> **I want to** 在下单前获得风控校验
> **So that** 我不会因为误操作或过度交易导致不可控的风险

**验收条件：**

```gherkin
Feature: 风控拦截

  Background:
    Given 用户已登录
      And 风控规则已配置

  Scenario: 余额不足拦截
    Given 用户可用余额为 1000 USDT
    When 提交限价买入 BTC/USDT 价格 50000 数量 0.1 (总金额 5000 USDT)
    Then 下单被拒绝
      And 返回错误 "余额不足: 可用 1000.00 USDT, 需要 5000.00 USDT"
      And 前端显示错误提示

  Scenario: 最大持仓限制
    Given 风控规则: 单交易对最大持仓 5 BTC
      And 用户已持有 4.8 BTC
    When 尝试买入 0.5 BTC
    Then 下单被拒绝
      And 返回错误 "超过最大持仓限制: BTC/USDT 最大 5 BTC, 当前 4.8 BTC, 欲买 0.5 BTC"
      And 前端显示警告

  Scenario: 每日最大亏损限制
    Given 风控规则: 每日最大亏损 500 USDT
      And 今日已实现亏损 480 USDT
    When 尝试开新仓
    Then 弹出风控警告 "今日已亏损 480.00 / 500.00 USDT，接近每日亏损上限"
      And 用户可选择 "继续" 或 "取消"

  Scenario: 每日最大亏损触发
    Given 风控规则: 每日最大亏损 500 USDT
      And 今日已实现亏损 500 USDT
    When 尝试开新仓
    Then 下单被拒绝
      And 返回错误 "已达到每日最大亏损限制 (500.00 USDT)，今日不可再开新仓"

  Scenario: 单笔最大亏损预警
    Given 风控规则: 单笔最大亏损 200 USDT
      And 用户提交的委托预估亏损可能超过 200 USDT
    When 下单
    Then 弹出确认对话框 "该委托预估最大亏损可能超过 200.00 USDT，确认继续？"
      And 需用户二次确认

  Scenario: 风控规则可配置
    Given 用户为 admin
    When 导航至风控配置页面
    Then 可查看和修改以下规则:
      | 规则 | 说明 |
      | max_position | 单交易对最大持仓量 |
      | max_daily_loss | 每日最大亏损额 |
      | max_single_loss | 单笔最大亏损额 |
      | max_position_concentration | 最大持仓集中度 |
    When 修改 max_daily_loss 为 1000 USDT
    Then 规则立即生效
      And 新的下单校验使用新规则
```

---

### US-TE-07: 策略信号自动下单 (P1)

> **As a** 交易者
> **I want to** 策略运行时自动生成委托
> **So that** 我不需要手动操作就能执行策略信号

**验收条件：**

```gherkin
Feature: 策略信号自动下单

  Background:
    Given 用户已登录
      And 用户有一个 active 状态的策略

  Scenario: 策略信号生成委托
    Given 策略 "双均线交叉" 运行中
      And 策略配置的交易对为 BTC/USDT
    When 策略引擎产生买入信号
      And 信号通过 Redis PubSub (strategy:signal:{id}) 发出
    Then 交易服务接收信号
      And 自动创建限价买入委托 (信号包含价格和数量)
      And 委托的 strategy_id 关联到该策略
      And 委托进入撮合引擎
      And WebSocket 推送通知用户 "策略 [双均线交叉] 生成买入委托"

  Scenario: 策略信号触发风控
    Given 策略生成买入信号
      And 风控规则: 单交易对最大持仓 5 BTC
      And 用户已持有 4.8 BTC
      And 信号数量为 0.5 BTC
    When 交易服务接收信号
    Then 委托被风控拒绝
      And 策略日志记录 "风控拒绝: 超过最大持仓限制"
      And WebSocket 推送通知用户 "策略 [双均线交叉] 信号被风控拒绝"

  Scenario: 策略停止后不再生成委托
    Given 策略 "双均线交叉" 被用户停止
    When 策略引擎不再产生信号
    Then 交易服务不再接收该策略的信号
      And 已有的 pending 委托不受影响 (可手动撤单)

  Scenario: 策略信号与手动委托区分
    Given 委托列表已显示
    Then 策略生成的委托显示关联策略名称
      And 手动下单的委托显示 "手动" 标签
      And 支持按来源筛选 (策略/手动)
```

---

### US-TE-08: 交易页面布局 (P0)

> **As a** 交易者
> **I want to** 在一个页面内同时查看行情、下单、委托和持仓
> **So that** 我可以高效执行交易而不需要频繁切换页面

**验收条件：**

```gherkin
Feature: 交易页面布局

  Background:
    Given 用户已登录

  Scenario: 交易页面三栏布局
    Given 用户导航至 /trade 页面
    When 页面加载完成
    Then 显示三栏布局:
      | 区域 | 位置 | 内容 |
      | 行情面板 | 左侧 (flex:2) | K线图 + 深度图 + Ticker |
      | 下单面板 | 右上 (360px) | 下单表单 + 账户余额 |
      | 委托/成交/持仓 | 右下 (360px) | Tab 切换 |

  Scenario: Tab 切换
    Given 交易页面已加载
      And 右下区域显示 Tab 栏
    Then 包含以下 Tab:
      | Tab | 内容 |
      | 当前委托 | 未完全成交的委托列表 |
      | 历史委托 | 已结束的委托列表 |
      | 成交记录 | 所有成交记录 |
      | 持仓 | 当前持仓列表 + 汇总 |

  Scenario: 行情与下单联动
    Given 交易页面已加载
    When 用户在行情面板切换交易对为 ETH/USDT
    Then 下单面板的交易对自动切换为 ETH/USDT
      And 委托/成交/持仓列表按 ETH/USDT 筛选

  Scenario: 快捷键支持
    Given 交易页面已加载
    When 按下快捷键 "B"
    Then 下单面板方向切换为 "买入"
    When 按下快捷键 "S"
    Then 下单面板方向切换为 "卖出"
    When 按下快捷键 "Ctrl+Enter"
    Then 提交当前委托 (弹出确认框)

  Scenario: 交易对从行情页跳转
    Given 用户在 /market 页面
    When 点击某交易对的"交易"按钮
    Then 跳转至 /trade/{symbol} 页面
      And 下单面板自动选择该交易对
```

---

### US-TE-09: 账户余额与资产概览 (P1)

> **As a** 交易者
> **I want to** 在交易页面查看我的账户余额
> **So that** 我可以了解可用资金和资产分布

**验收条件：**

```gherkin
Feature: 账户余额与资产概览

  Background:
    Given 用户已登录

  Scenario: 余额显示
    Given 交易页面已加载
    Then 下单面板上方显示:
      | 字段 | 说明 |
      | 可用余额 | 可用于下单的金额 |
      | 冻结金额 | 挂单冻结的金额 |
      | 总资产 | 可用 + 冻结 + 持仓市值 |

  Scenario: 余额实时更新
    Given 可用余额为 10000 USDT
    When 用户提交限价买入 5000 USDT
    Then 可用余额更新为 5000 USDT
      And 冻结金额增加 5000 USDT
    When 委托成交
    Then 冻结金额减少 5000 USDT
      And 持仓市值增加
    When 委托撤销
    Then 冻结金额减少
      And 可用余额恢复

  Scenario: 模拟账户初始化
    Given 新用户首次进入交易页面
    Then 系统自动创建模拟账户
      And 初始余额为 100000 USDT
      And 显示提示 "模拟账户已创建，初始资金: 100,000 USDT"
```

---

### US-TE-10: 交易通知 (P1)

> **As a** 交易者
> **I want to** 收到交易相关的实时通知
> **So that** 我不会错过重要的成交和风控事件

**验收条件：**

```gherkin
Feature: 交易通知

  Background:
    Given 用户已登录

  Scenario: 成交通知
    Given 用户有一个 pending 状态的委托
    When 该委托被撮合引擎成交
    Then 前端显示 Toast 通知 "BTC/USDT 买入成交 0.1 @ 50500.00"
      And 通知自动消失 (5秒)

  Scenario: 部分成交通知
    Given 用户有一个数量为 0.5 的委托
    When 部分成交 0.3
    Then 前端显示 Toast "BTC/USDT 买入部分成交 0.3/0.5 @ 50500.00"

  Scenario: 止损触发通知
    Given 用户有一个止损卖出单
    When 价格触及触发价
    Then 前端显示通知 "止损单已触发: BTC/USDT 卖出 47900.00"

  Scenario: 风控预警通知
    Given 风控规则: 每日最大亏损 500 USDT
      And 今日已亏损 400 USDT
    When 风控检测到接近阈值
    Then 前端显示警告通知 "风控预警: 今日已亏损 400/500 USDT"
      And 通知为橙色/红色样式

  Scenario: 通知中心
    Given 用户收到多条通知
    When 点击导航栏通知图标
    Then 显示通知中心列表
      And 按时间降序排列
      And 未读通知加粗显示
      And 支持标记已读/全部已读
```

---

## 5. 数据模型

### 5.1 新增表

#### accounts（模拟账户表）

```sql
CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    mode trade_mode NOT NULL DEFAULT 'simulation',
    balance DECIMAL(20,8) NOT NULL DEFAULT 0,
    frozen DECIMAL(20,8) NOT NULL DEFAULT 0,
    currency VARCHAR(10) NOT NULL DEFAULT 'USDT',
    initial_balance DECIMAL(20,8) NOT NULL DEFAULT 100000,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, mode, currency)
);

CREATE INDEX idx_accounts_user_id ON accounts(user_id);
```

#### order_fills（成交明细表 — 扩展 trades 表）

> data-model.md 中已定义 `trades` 表。本 PRD 沿用该表结构，但增加以下字段。

```sql
-- 在 trades 表基础上增加
ALTER TABLE trades ADD COLUMN IF NOT EXISTS fee_currency VARCHAR(10) DEFAULT 'USDT';
ALTER TABLE trades ADD COLUMN IF NOT EXISTS is_maker BOOLEAN DEFAULT false;
```

### 5.2 现有表使用说明

| 表 | 来源 | 说明 |
|----|------|------|
| orders | data-model.md | 委托表，沿用已有定义 |
| trades | data-model.md | 成交记录表，追加 fee_currency / is_maker 字段 |
| positions | data-model.md | 持仓表，沿用已有定义 |
| risk_rules | data-model.md | 风控规则表，沿用已有定义 |
| strategies | PRD-strategy-sandbox-mvp | 策略表，strategy_id 关联到 orders |

### 5.3 Redis 缓存 Key（新增）

| Key | 类型 | TTL | 说明 |
|-----|------|-----|------|
| `account:{user_id}:simulation` | HASH | 5s | 模拟账户余额缓存 |
| `position_cache:{user_id}:{symbol}` | HASH | 5s | 持仓缓存 (沿用 data-model) |
| `orderbook:sim:{symbol}` | STRING(JSON) | 1s | 模拟撮合 Order Book 快照 |
| `order:{order_id}` | HASH | 10min | 委托详情缓存 |
| `risk:check:{user_id}` | STRING | 1s | 风控检查结果缓存 |

### 5.4 Redis Pub/Sub 频道（新增）

| 频道 | 用途 | 发布者 | 订阅者 |
|------|------|--------|--------|
| `trade:order:{user_id}` | 委托状态变更 | Trade Service | WS Hub |
| `trade:fill:{user_id}` | 成交通知 | Matching Engine | WS Hub |
| `trade:position:{user_id}` | 持仓变更 | Trade Service | WS Hub |
| `strategy:signal:{strategy_id}` | 策略信号 | Strategy Engine | Trade Service |

---

## 6. API 端点设计

遵循 TECH_CHARTER 规范：`/api/v1/{resource}` + snake_case JSON。

### 6.1 REST API

| 方法 | 路径 | 说明 | 优先级 |
|------|------|------|--------|
| POST | `/api/v1/trade/orders` | 创建委托 | P0 |
| GET | `/api/v1/trade/orders` | 查询委托列表 | P0 |
| GET | `/api/v1/trade/orders/{id}` | 查询委托详情 | P0 |
| DELETE | `/api/v1/trade/orders/{id}` | 撤销委托 | P0 |
| GET | `/api/v1/trade/fills` | 查询成交记录 | P0 |
| GET | `/api/v1/trade/positions` | 查询持仓列表 | P0 |
| POST | `/api/v1/trade/positions/{symbol}/close` | 平仓操作 | P0 |
| GET | `/api/v1/trade/account` | 查询账户余额 | P1 |
| WS | `/api/v1/trade/ws` | 交易 WebSocket 推送 | P0 |
| GET | `/api/v1/trade/risk/rules` | 查询风控规则 | P1 |
| PUT | `/api/v1/trade/risk/rules/{id}` | 修改风控规则 (admin) | P1 |

### 6.2 API 详细设计

#### POST /api/v1/trade/orders

**请求体：**

```json
{
  "symbol": "BTC/USDT",
  "side": "buy",
  "type": "limit",
  "price": "50000.00",
  "amount": "0.10000000",
  "stop_price": null,
  "strategy_id": null,
  "mode": "simulation",
  "time_in_force": "gtc"
}
```

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| symbol | string | 是 | 交易对 |
| side | string | 是 | "buy" / "sell" |
| type | string | 是 | "limit" / "market" / "stop" / "stop_limit" |
| price | string | 限价单必填 | 委托价格 |
| amount | string | 是 | 委托数量 |
| stop_price | string | 止损单必填 | 触发价 |
| strategy_id | string | 否 | 策略 ID (手动单为 null) |
| mode | string | 是 | "simulation" (MVP 仅支持) |
| time_in_force | string | 否 | "gtc"(默认) / "ioc" / "fok" |

**响应：**

```json
{
  "code": 0,
  "data": {
    "order_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "pending",
    "created_at": "2026-05-14T06:30:00.123Z"
  },
  "message": "success"
}
```

**错误码：**

```json
{"code": 40001, "message": "参数错误", "details": [{"field": "price", "msg": "限价单价格不能为空"}]}
{"code": 40002, "message": "余额不足", "details": [{"available": "1000.00", "required": "5000.00"}]}
{"code": 40003, "message": "风控拒绝", "details": [{"rule": "max_daily_loss", "msg": "已达到每日最大亏损限制"}]}
{"code": 40004, "message": "交易对不可交易", "details": [{"symbol": "XXX/USDT"}]}
```

#### GET /api/v1/trade/orders

**查询参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| symbol | string | 否 | 交易对筛选 |
| status | string | 否 | 状态筛选: pending/partial_filled/filled/cancelled |
| side | string | 否 | 方向: buy/sell |
| source | string | 否 | 来源: manual/strategy |
| page | integer | 否 | 页码，默认 1 |
| size | integer | 否 | 每页条数，默认 20，最大 100 |

#### DELETE /api/v1/trade/orders/{id}

**响应：**

```json
{
  "code": 0,
  "data": {
    "order_id": "550e8400-e29b-41d4-a716-446655440000",
    "status": "cancelled",
    "filled": "0.30000000",
    "cancelled_amount": "0.20000000"
  },
  "message": "success"
}
```

#### POST /api/v1/trade/positions/{symbol}/close

**请求体：**

```json
{
  "amount": "0.50000000",
  "type": "market"
}
```

#### GET /api/v1/trade/account

**响应：**

```json
{
  "code": 0,
  "data": {
    "mode": "simulation",
    "currency": "USDT",
    "balance": "50000.00000000",
    "frozen": "5000.00000000",
    "total_equity": "105000.00000000",
    "initial_balance": "100000.00000000",
    "total_pnl": "5000.00000000",
    "positions_value": "50000.00000000"
  },
  "message": "success"
}
```

### 6.3 WebSocket 消息协议

**连接：** `wss://host/api/v1/trade/ws?token=***`

**客户端→服务端消息：**

```json
// 订阅交易通知
{
  "action": "subscribe",
  "channels": ["order", "fill", "position", "risk_alert"]
}

// 取消订阅
{
  "action": "unsubscribe",
  "channels": ["risk_alert"]
}
```

**服务端→客户端消息：**

```json
// 委托状态变更
{
  "type": "order_update",
  "data": {
    "order_id": "550e8400-...",
    "symbol": "BTC/USDT",
    "side": "buy",
    "status": "filled",
    "price": "50500.00",
    "amount": "0.10000000",
    "filled": "0.10000000",
    "updated_at": "2026-05-14T06:30:01.000Z"
  },
  "ts": 1715500900000
}

// 成交通知
{
  "type": "fill",
  "data": {
    "trade_id": 12345,
    "order_id": "550e8400-...",
    "symbol": "BTC/USDT",
    "side": "buy",
    "price": "50500.00",
    "amount": "0.10000000",
    "fee": "5.05",
    "is_maker": false
  },
  "ts": 1715500900000
}

// 持仓变更
{
  "type": "position_update",
  "data": {
    "symbol": "BTC/USDT",
    "quantity": "0.10000000",
    "avg_price": "50500.00",
    "unrealized_pnl": "5.00"
  },
  "ts": 1715500900000
}

// 风控预警
{
  "type": "risk_alert",
  "data": {
    "level": "warning",
    "rule": "max_daily_loss",
    "message": "今日已亏损 400/500 USDT",
    "action": "notify"
  },
  "ts": 1715500900000
}

// 风控拒绝
{
  "type": "risk_alert",
  "data": {
    "level": "blocked",
    "rule": "max_daily_loss",
    "message": "已达到每日最大亏损限制",
    "action": "block"
  },
  "ts": 1715500900000
}
```

### 6.4 错误码汇总

```rust
// 交易模块错误码 40xxx
pub const ERR_TRADE_INVALID_PARAMS: i32 = 40001;         // 参数错误
pub const ERR_TRADE_INSUFFICIENT_BALANCE: i32 = 40002;   // 余额不足
pub const ERR_TRADE_RISK_REJECTED: i32 = 40003;          // 风控拒绝
pub const ERR_TRADE_SYMBOL_NOT_TRADEABLE: i32 = 40004;   // 交易对不可交易
pub const ERR_TRADE_ORDER_NOT_FOUND: i32 = 40401;        // 委托不存在
pub const ERR_TRADE_ORDER_NOT_CANCELLABLE: i32 = 40402;  // 委托不可撤销
pub const ERR_TRADE_POSITION_NOT_FOUND: i32 = 40403;     // 持仓不存在
pub const ERR_TRADE_POSITION_INSUFFICIENT: i32 = 40005;  // 持仓不足 (平仓)
pub const ERR_TRADE_PRICE_OUT_OF_BOUNDS: i32 = 40006;    // 价格超出涨跌幅限制
pub const ERR_TRADE_AMOUNT_TOO_SMALL: i32 = 40007;       // 数量小于最小下单量
pub const ERR_TRADE_RATE_LIMITED: i32 = 42901;           // 下单频率限制
pub const ERR_TRADE_SIMULATION_ONLY: i32 = 40008;        // 仅支持模拟模式
pub const ERR_TRADE_MATCHING_ENGINE_DOWN: i32 = 50301;   // 撮合引擎不可用
```

---

## 7. 前端路由与页面设计

### 7.1 路由

| 路径 | 组件 | 说明 |
|------|------|------|
| `/trade` | `TradeView.vue` | 交易主页面（三栏布局） |
| `/trade/:symbol` | `TradeView.vue` | 指定交易对的交易页面 |

### 7.2 页面布局

```
┌──────────────────────────────────────────────────────────────────┐
│  [行情面板 flex:2]          │  [右侧面板 360px]                  │
│                              │                                    │
│  ┌──────────────────────┐    │  ┌────────────────────────────┐   │
│  │ 周期选择: 1m..1w      │    │  │ 模拟交易 🔶  余额: 50,000│   │
│  ├──────────────────────┤    │  │ 可用: 45,000  冻结: 5,000 │   │
│  │                      │    │  ├────────────────────────────┤   │
│  │   K线图               │    │  │ [买入🟢] [卖出🔴]         │   │
│  │   Lightweight-charts │    │  │ 类型: [限价▾] [市价] [止损]│   │
│  │                      │    │  │ 价格: [50000.00]  [市价]  │   │
│  │                      │    │  │ 数量: [0.1    ] [25%][50%]│   │
│  │                      │    │  │ 金额: 5000.00 USDT        │   │
│  │                      │    │  │ [     买入限价 BTC      ]  │   │
│  ├──────────────────────┤    │  └────────────────────────────┘   │
│  │ 深度图               │    │  ┌────────────────────────────┐   │
│  └──────────────────────┘    │  │[当前委托][历史][成交][持仓]│   │
│                              │  ├────────────────────────────┤   │
│                              │  │ 时间 | 交易对 | 方向 | ...│   │
│                              │  │ 06:30 BTC  Buy  50000 ... │   │
│                              │  │ 06:28 ETH  Sell 3200  ... │   │
│                              │  │          [撤单]            │   │
│                              │  └────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

### 7.3 组件树

```
TradeView.vue
├── TradeChart.vue                    # 左侧行情面板
│   ├── CandlestickChart.vue          # K线图 (复用 MarketModule)
│   ├── PeriodSelector.vue            # 周期选择器
│   └── DepthChart.vue               # 深度图 (简化版)
├── OrderPanel.vue                    # 右上-下单面板
│   ├── SymbolSelector.vue           # 交易对选择
│   ├── SideToggle.vue               # 买入/卖出切换
│   ├── OrderTypeSelect.vue          # 委托类型选择
│   ├── PriceInput.vue               # 价格输入 (含市价快捷)
│   ├── AmountInput.vue              # 数量输入 (含百分比快捷)
│   ├── OrderSummary.vue             # 金额/费用预估
│   └── OrderConfirmDialog.vue       # 确认对话框
└── TradeTabs.vue                     # 右下-委托/成交/持仓
    ├── OpenOrders.vue               # 当前委托
    │   └── OrderRow.vue             # 委托行 (含撤单)
    ├── OrderHistory.vue             # 历史委托
    ├── TradeHistory.vue             # 成交记录
    └── PositionList.vue             # 持仓列表
        ├── PositionRow.vue          # 持仓行 (含平仓)
        └── PositionSummary.vue      # 持仓汇总
```

### 7.4 前端状态管理 (Pinia Store)

```typescript
// stores/trade.ts
interface TradeState {
  // 下单
  currentSymbol: string              // 当前交易对
  orderSide: 'buy' | 'sell'          // 当前方向
  orderType: 'limit' | 'market' | 'stop' | 'stop_limit'
  orderPrice: string                 // 价格
  orderAmount: string                // 数量
  stopPrice: string                  // 触发价

  // 委托
  openOrders: Order[]                // 当前委托
  orderHistory: Order[]              // 历史委托
  orderHistoryTotal: number          // 历史委托总数

  // 成交
  fills: Trade[]                     // 成交记录
  fillsTotal: number                 // 成交总数

  // 持仓
  positions: Position[]              // 持仓列表

  // 账户
  account: Account | null            // 账户余额

  // WebSocket
  wsConnected: boolean               // 交易 WS 连接状态
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
  onOrderUpdate: (callback: (data: OrderUpdate) => void) => void
  onFill: (callback: (data: FillData) => void) => void
  onPositionUpdate: (callback: (data: PositionUpdate) => void) => void
  onRiskAlert: (callback: (data: RiskAlert) => void) => void
  connectionStatus: Ref<'connecting' | 'connected' | 'disconnected' | 'reconnecting'>
}
```

**重连策略：** 与行情 WS 相同 — 指数退避 1s→2s→4s→8s→16s→30s (最大)，重连后自动恢复订阅。

---

## 8. 后端设计

### 8.1 新增模块

```
backend/src/
├── handlers/
│   └── trade.rs                    # 交易 REST API handlers
├── services/
│   ├── trade_service.rs            # 交易业务逻辑 (下单/撤单/持仓)
│   ├── matching_engine.rs          # 撮合引擎 (SIMULATE 模式)
│   │   ├── OrderBook               # 内存 Order Book
│   │   ├── Matcher                  # 撮合逻辑
│   │   └── OrderBookManager         # 多交易对 Order Book 管理
│   └── risk_manager.rs             # 风控服务 (检查/拦截)
└── models/
    └── schemas.rs                   # 新增交易相关 schemas
```

### 8.2 交易服务 (TradeService)

```rust
// services/trade_service.rs

pub struct TradeService {
    db: Arc<DatabaseConnection>,
    redis: Arc<redis::Client>,
    matching_engine: Arc<MatchingEngine>,
    risk_manager: Arc<RiskManager>,
    ws_hub: Arc<TradeWsHub>,
}

impl TradeService {
    /// 创建委托 (手动/策略信号)
    pub async fn create_order(&self, req: CreateOrderRequest) -> Result<Order>;

    /// 撤销委托
    pub async fn cancel_order(&self, user_id: &Uuid, order_id: &Uuid) -> Result<CancelResult>;

    /// 处理策略信号 (订阅 Redis PubSub)
    pub async fn handle_strategy_signal(&self, signal: StrategySignal) -> Result<()>;

    /// 查询委托列表
    pub async fn list_orders(&self, user_id: &Uuid, filter: OrderFilter) -> Result<Vec<Order>>;

    /// 查询成交记录
    pub async fn list_fills(&self, user_id: &Uuid, filter: FillFilter) -> Result<Vec<Trade>>;

    /// 查询持仓
    pub async fn list_positions(&self, user_id: &Uuid) -> Result<Vec<Position>>;

    /// 平仓操作
    pub async fn close_position(&self, user_id: &Uuid, symbol: &str, amount: Decimal) -> Result<Order>;

    /// 查询账户余额
    pub async fn get_account(&self, user_id: &Uuid) -> Result<Account>;

    /// 初始化模拟账户
    pub async fn init_simulation_account(&self, user_id: &Uuid) -> Result<Account>;
}
```

### 8.3 撮合引擎 (MatchingEngine)

```rust
// services/matching_engine.rs

/// 单个交易对的 Order Book
pub struct OrderBook {
    symbol: String,
    bids: BTreeMap<Reverse<Decimal>, Vec<Order>>,  // 买盘：价格降序
    asks: BTreeMap<Decimal, Vec<Order>>,             // 卖盘：价格升序
    last_price: Decimal,
    last_update: NaiveDateTime,
}

/// 撮合引擎 (管理多交易对)
pub struct MatchingEngine {
    order_books: DashMap<String, OrderBook>,
    redis: Arc<redis::Client>,
    db: Arc<DatabaseConnection>,
    fee_rate: Decimal,                                    // 手续费率
    market_data_rx: tokio::sync::broadcast::Receiver<MarketTick>,  // 行情接收
}

impl MatchingEngine {
    /// 启动撮合引擎
    pub async fn start(&self) -> Result<()>;

    /// 提交委托到撮合
    pub async fn submit_order(&self, order: Order) -> Result<SubmitResult>;

    /// 撤销委托
    pub async fn cancel_order(&self, symbol: &str, order_id: &Uuid) -> Result<()>;

    /// 行情触发 (止损单/市价单)
    pub async fn on_market_tick(&self, tick: &MarketTick);

    /// 撮合核心逻辑
    fn match_order(&mut self, symbol: &str, order: &Order) -> Vec<Fill>;

    /// 生成成交记录
    async fn create_fills(&self, fills: Vec<Fill>) -> Result<()>;

    /// 更新持仓
    async fn update_positions(&self, fills: &[Fill]) -> Result<()>;

    /// 恢复 Order Book (从数据库加载 pending 委托)
    async fn restore_from_db(&self) -> Result<()>;
}
```

### 8.4 风控服务 (RiskManager)

```rust
// services/risk_manager.rs

pub struct RiskManager {
    db: Arc<DatabaseConnection>,
    redis: Arc<redis::Client>,
}

/// 风控检查结果
pub enum RiskCheckResult {
    Pass,
    Warn(RiskWarning),
    Block(RiskBlock),
}

impl RiskManager {
    /// 下单前风控检查
    pub async fn check_order(&self, user_id: &Uuid, order: &CreateOrderRequest) -> RiskCheckResult;

    /// 检查余额
    async fn check_balance(&self, user_id: &Uuid, required: Decimal) -> RiskCheckResult;

    /// 检查最大持仓
    async fn check_max_position(&self, user_id: &Uuid, symbol: &str, additional: Decimal) -> RiskCheckResult;

    /// 检查每日最大亏损
    async fn check_daily_loss(&self, user_id: &Uuid) -> RiskCheckResult;

    /// 检查单笔最大亏损
    async fn check_single_loss(&self, user_id: &Uuid, order: &CreateOrderRequest) -> RiskCheckResult;

    /// 检查持仓集中度
    async fn check_concentration(&self, user_id: &Uuid, symbol: &str) -> RiskCheckResult;

    /// 获取用户风控规则 (用户级 > 全局级)
    async fn get_rules(&self, user_id: &Uuid) -> Vec<RiskRule>;
}
```

### 8.5 交易 WebSocket Hub

```rust
// 扩展现有 ws.rs

pub struct TradeWsHub {
    /// 频道 → 订阅者集合
    channels: DashMap<String, HashSet<String>>,
    /// 用户会话 → 发送端
    sessions: DashMap<String, mpsc::Sender<String>>,
    /// Redis 订阅端
    redis_sub: redis::aio::PubSub,
}

impl TradeWsHub {
    pub async fn handle_subscribe(&self, session_id: &str, channels: &[String]);
    pub async fn handle_unsubscribe(&self, session_id: &str, channels: &[String]);
    pub async fn broadcast_to_user(&self, user_id: &Uuid, message: &str);
    pub async fn on_redis_message(&self, channel: &str, data: &str);
}
```

### 8.6 数据流

```
[用户/策略信号]
    ↓
[TradeService: 创建委托 + 风控检查]
    ↓ Pass
[MatchingEngine: 撮合]
    ↓ Fill(s)
[TradeService: 生成成交记录 + 更新持仓 + 更新余额 + 通知]
    ├─→ PostgreSQL (orders, trades, positions, accounts)
    ├─→ Redis (缓存更新)
    └─→ Redis PubSub (trade:order, trade:fill, trade:position)
          ↓
    [TradeWsHub: 接收 PubSub 消息]
          ↓
    [分发至订阅了对应频道的客户端]
```

### 8.7 降级策略

| 故障场景 | 降级行为 | 恢复条件 |
|----------|---------|---------|
| 撮合引擎不可用 | 拒绝新委托，返回 503 | 引擎重启恢复 |
| Redis 不可用 | 余额查询降级为数据库直查 | Redis 恢复 |
| PostgreSQL 不可用 | 拒绝新委托，查询降级为缓存 | PG 恢复后补写 |
| 行情数据中断 | 撮合引擎暂停，pending 委托保留 | 行情恢复后继续撮合 |
| 策略信号 PubSub 中断 | 信号丢失，策略日志记录 | PubSub 恢复 |

---

## 9. 边界情况

| 边界情况 | 处理方式 |
|----------|---------|
| 下单频率过高 | 每用户 10 次/分钟，超出返回 429 |
| 同一委托重复提交 | 幂等键 (client_order_id) 防重复 |
| 撤单时委托已成交 | 返回当前状态，不执行撤单 |
| 撤单时委托已部分成交 | 撤销未成交部分，保留已成交部分 |
| 持仓精度不足 (无法全量平仓) | 允许部分平仓，最小平仓量 = 最小下单量 |
| 交易对暂停交易 | 拒绝新委托，保留现有委托和持仓 |
| 模拟账户余额耗尽 | 提示 "余额不足"，不自动充值 (可手动重置) |
| 止损单触发后撮合失败 | 止损单进入 Order Book 挂单，等待成交 |
| 策略信号与手动委托竞争 | 均进入撮合引擎，不区分优先级 |
| 多标签页同时操作 | 每个标签页独立 WebSocket，委托操作需互斥 (乐观锁) |
| 价格精度超限 | 自动截断到交易对支持的精度 |
| 数量精度超限 | 自动截断到交易对支持的精度 |
| 浮动盈亏计算延迟 | 使用最近一次行情价格，标注 "数据可能有延迟" |
| 撮合引擎重启 | 从数据库恢复 pending 委托，重建 Order Book |
| 并发下单 (同用户) | 余额检查使用 Redis 乐观锁 (DECRBY) |

---

## 10. 非功能性需求

### 10.1 性能

| 指标 | 目标值 |
|------|--------|
| 下单接口响应 | P99 < 200ms |
| 撮合延迟 | < 100ms (委托提交到成交) |
| WebSocket 推送延迟 | < 500ms (成交→通知) |
| 委托列表加载 | < 500ms |
| 持仓列表加载 | < 300ms |
| 并发下单 | ≥ 100 TPS |

### 10.2 可靠性

| 要求 | 说明 |
|------|------|
| 委托不丢失 | 委托先写 DB 再撮合，即使宕机也可恢复 |
| 成交不丢失 | 成交记录同步写入 DB，异步更新缓存 |
| 撮合幂等 | 同一委托重复提交不会产生重复成交 |
| 余额一致 | 余额变更使用数据库事务保证一致性 |
| 断线恢复 | WS 重连后自动恢复订阅，补发断线期间状态变更 |

### 10.3 安全

| 要求 | 说明 |
|------|------|
| WS 鉴权 | JWT token 通过 query param 传递 (同行情 WS) |
| 限流 | 每用户 10 次/分钟 (下单)，100 次/分钟 (查询) |
| 权限隔离 | 用户只能操作自己的委托/持仓 |
| 敏感操作确认 | 下单/撤单需前端二次确认 |
| 审计日志 | 所有委托操作写入 audit_log |

---

## 11. 优先级矩阵

| 用户故事 | 优先级 | 说明 |
|----------|--------|------|
| US-TE-01: 下单面板 | P0 | 交易核心入口，无下单则无交易 |
| US-TE-02: 模拟撮合引擎 | P0 | 交易闭环核心，无撮合则无成交 |
| US-TE-03: 委托管理 | P0 | 委托状态跟踪+撤单，基本操作 |
| US-TE-04: 成交记录 | P0 | 交易确认凭证，复盘分析基础 |
| US-TE-05: 持仓管理 | P0 | 交易闭环终点，浮动盈亏+平仓 |
| US-TE-08: 交易页面布局 | P0 | 用户体验，整合所有交易功能 |
| US-TE-06: 风控拦截 | P1 | 安全保护，MVP 可用简单规则 |
| US-TE-07: 策略信号自动下单 | P1 | 策略闭环，依赖策略模块 |
| US-TE-09: 账户余额与资产概览 | P1 | 辅助信息，下单面板可含余额 |
| US-TE-10: 交易通知 | P1 | 信息触达，提升体验 |

**总计: P0 x 6 / P1 x 4**

---

## 12. 实施分阶段建议

### Phase 1: 后端核心 (1-2 周)

1. 创建 `accounts` 表 + 数据迁移 (orders/trades/positions 已有 schema)
2. 实现 `TradeService` (create_order, cancel_order, list_orders, list_fills, list_positions)
3. 实现 `MatchingEngine` SIMULATE 模式 (OrderBook + 限价单/市价单撮合)
4. 实现 `RiskManager` 基础风控 (余额检查、最大持仓)
5. 实现 `handlers/trade.rs` REST API
6. 扩展 `ws.rs` 为 `TradeWsHub` (委托/成交/持仓推送)

### Phase 2: 前端核心 (1-2 周)

1. 实现 `TradeView.vue` 三栏布局
2. 实现 `OrderPanel.vue` (限价/市价/止损下单)
3. 实现 `TradeTabs.vue` (当前委托/历史委托/成交/持仓)
4. 实现 `useTradeWs.ts` composable (连接/订阅/重连)
5. 实现交易通知 (Toast + 通知中心)

### Phase 3: 完善功能 (1 周)

1. 实现策略信号自动下单 (订阅 strategy:signal PubSub)
2. 实现风控规则管理页面 (admin)
3. 实现账户余额与资产概览
4. 实现止损单触发逻辑
5. 实现降级和边界情况处理
6. 性能测试和优化

---

## 13. 关联文档

| 文档 | 关系 |
|------|------|
| ADR-006 | 撮合引擎技术选型决策 |
| ADR-003 | 策略引擎沙箱 (策略信号源) |
| data-model.md | 数据库 schema (orders/trades/positions/risk_rules) |
| PRD.md US-11/12/13/15 | 原始用户故事定义 |
| PRD-market-module.md | 行情数据 (依赖实时价格) |
| PRD-strategy-sandbox-mvp.md | 策略管理 (信号生成) |
| PRD-backtest-engine.md | 回测撮合 (互补，独立) |
| risk-analysis.md | 风险点 CR-02 (撮合引擎缺失) |
| TECH_CHARTER.md | 技术栈和开发规范 |
