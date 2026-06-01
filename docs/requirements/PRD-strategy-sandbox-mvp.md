# PRD: 策略沙箱 MVP — 内置策略模板与参数配置

> 版本: v1.0
> 状态: Draft
> 作者: PM
> 基于: ADR-003 第一阶段
> 关联: PRD v1.0 US-07 / US-08 (模板化替代代码编辑)

---

## 1. 背景

主 PRD (v1.0) US-07 原规划用户通过代码编辑器创建量化策略。经 ADR-003 架构评审决策：

> **第一阶段（MVP）**：用户不直接写代码，而是通过配置参数组合内置策略模板。策略逻辑由 Rust 实现，编译在主程序中。后续阶段再逐步开放 WASM 沙箱和 Python 桥接。

当前状态：
- 前端 `/strategies` 路由已配置，但 `StrategiesView.vue` 仅为占位组件
- 后端尚无 `strategies` 模块（handler / service / model 均不存在）
- 数据库尚无策略相关表

本 PRD 定义 MVP 范围内全部功能、交互流程和验收条件。

---

## 2. 目标

| 目标 | 指标 |
|------|------|
| 内置策略模板 | ≥ 10 个，覆盖趋势跟踪、均值回归、波动率三大类 |
| 策略创建 | 选择模板 + 配置参数 → 保存，不需要写一行代码 |
| 策略生命周期 | draft → active → paused → stopped，前端可见状态切换 |
| 并发运行 | 单个用户同时运行 ≤ 5 个 active 策略，平台总并发无上限 |
| 接口响应 | 策略 CRUD 接口 P99 < 200ms |

### 非目标 (Non-Goals)

- ❌ 用户编写自定义策略代码（第二阶段 WASM 沙箱）
- ❌ 策略运行时可视化日志 / K线回放（第三阶段）
- ❌ 策略收益率实时计算与图表（在数据积累后追加）
- ❌ 策略分享 / 市场 (P2+)

---

## 3. 用户角色与权限

| 角色 | 策略权限 | 说明 |
|------|---------|------|
| **trader** (普通用户) | CRUD 自有策略，仅限模拟运行 | 默认角色 |
| **pro-trader** (专业交易员) | CRUD 自有策略，可实盘运行 | 需管理员开通 |
| **admin** (管理员) | 查看所有策略，不可编辑他人策略 | 审计用途，可强制停止 |

> 注：MVP 阶段所有策略均为"模拟运行"，不涉及实盘资金。pro-trader 权限预埋但不启用实盘开关。

---

## 4. 用户故事 (User Stories)

### US-SS-01: 查看策略列表 (P0)

> **As a** 交易者
> **I want to** 查看我的所有策略
> **So that** 我可以了解已创建的策略和它们的状态

**验收条件：**

```gherkin
Feature: 策略列表展示
  Background:
    Given 用户已登录

  Scenario: 成功加载策略列表
    Given 用户拥有 3 个策略实例
    When 导航至 /strategies 页面
    Then 显示策略列表
      And 每行包含: 策略名称、模板类型、参数摘要、状态、创建时间
      And 页面标题显示"策略管理"

  Scenario: 策略列表为空
    Given 用户没有任何策略
    When 导航至 /strategies 页面
    Then 显示空状态组件
      And 提示"还没有策略，点击创建你的第一个策略"
      And 显示"创建策略"按钮

  Scenario: 策略列表分页
    Given 用户拥有 25 个策略
    When 加载策略列表
    Then 每页显示 20 条
      And 底部显示分页组件
      And 翻页后加载下一页数据
```

### US-SS-02: 创建策略 (选择模板 + 配置参数) (P0)

> **As a** 交易者
> **I want to** 通过选择内置模板并配置参数来创建一个新策略
> **So that** 我不需要编写代码就能快速搭建量化策略

**验收条件：**

```gherkin
Feature: 创建策略
  Background:
    Given 用户已登录

  Scenario: 成功创建策略
    Given 用户点击"创建策略"按钮
    When 系统显示模板选择面板
      And 用户选择"双均线交叉"模板
      And 输入策略名称 "我的双均线策略"
      And 配置 fast_period = 10, slow_period = 30
      And 选择交易对 "BTC/USDT"
      And 选择时间周期 "1h"
    When 点击"保存"
    Then 策略创建成功
      And 返回策略列表
      And 列表中出现新策略
      And 状态显示为 "draft"

  Scenario: 模板选择后自动填充默认参数
    Given 用户选择"布林带反转"模板
    Then 参数表单自动填充默认值: period=20, std_dev=2.0
      And 用户可手动修改任意参数

  Scenario: 必填参数未填时阻止保存
    Given 创建策略弹窗打开
    When 策略名称为空
      And 其他参数已填写
    Then "保存"按钮置灰/禁用
      And 策略名称输入框显示错误提示"策略名称不能为空"

  Scenario: 参数值超出范围拒绝保存
    Given 选择"双均线交叉"模板
    When fast_period 设置为 100
    Then 输入框显示错误提示"fast_period 有效范围: 5-50"
      And "保存"按钮禁用

  Scenario: 取消创建不保存
    Given 创建策略弹窗打开
    When 填写了部分参数
      And 点击"取消"或关闭弹窗
    Then 不保存任何数据
      And 返回策略列表页
```

### US-SS-03: 启用 / 暂停 / 停止策略 (P0)

> **As a** 交易者
> **I want to** 控制策略的运行状态（启用、暂停、停止）
> **So that** 我可以管理策略何时运行

**验收条件：**

```gherkin
Feature: 策略状态管理
  Background:
    Given 用户已登录
      And 用户拥有 1 个 "draft" 状态的策略

  Scenario: 启用草稿策略
    Given 策略状态为 "draft"
    When 点击"启用"按钮
    Then 状态变为 "active"
      And 后端开始为该策略分配行情数据流
      And 按钮变为"暂停"和"停止"

  Scenario: 暂停运行中的策略
    Given 策略状态为 "active"
    When 点击"暂停"按钮
    Then 弹出确认对话框"确定要暂停该策略吗？"
      And 用户确认后
    Then 状态变为 "paused"
      And 策略暂停接收新的行情 tick
      And 按钮变为"启用"和"停止"

  Scenario: 停止策略（终止运行）
    Given 策略状态为 "active" 或 "paused"
    When 点击"停止"按钮
    Then 弹出确认对话框"策略停止后不可恢复，确定吗？"
      And 用户确认后
    Then 状态变为 "stopped"
      And 策略彻底终止（不可恢复为 active）
      And 仅可删除或复制

  Scenario: 恢复暂停的策略
    Given 策略状态为 "paused"
    When 点击"启用"按钮
    Then 状态恢复为 "active"
      And 策略继续接收行情 tick 继续执行

  Scenario: 状态转换校验
    Given 策略状态为 "stopped"
    When 点击"启用"
    Then "启用"按钮不可点击
      And 工具提示"已停止的策略无法重新启用"
```

### US-SS-04: 编辑策略参数 (P1)

> **As a** 交易者
> **I want to** 修改已创建策略的参数
> **So that** 我可以优化策略配置而不用重新创建

**验收条件：**

```gherkin
Feature: 编辑策略参数
  Background:
    Given 用户已登录
      And 拥有 1 个策略

  Scenario: 编辑 draft 策略的参数
    Given 策略状态为 "draft"
    When 点击"编辑"按钮
    Then 打开参数配置弹窗（同创建弹窗，预填当前值）
    When 修改 fast_period 从 10 改为 5
      And 点击"保存"
    Then 策略参数更新成功
      And 列表参数摘要更新为新值

  Scenario: 编辑 active 策略的参数
    Given 策略状态为 "active"
    When 点击"编辑"按钮
    Then 提示"策略运行中，请先暂停后再编辑"
      And 不允许编辑

  Scenario: 编辑后取消
    Given 编辑弹窗已打开并修改了参数
    When 点击"取消"
    Then 参数恢复为修改前的值
      And 不触发后端更新
```

### US-SS-05: 删除策略 (P1)

> **As a** 交易者
> **I want to** 删除不再需要的策略
> **So that** 我可以保持策略列表整洁

**验收条件：**

```gherkin
Feature: 删除策略
  Background:
    Given 用户已登录

  Scenario: 删除 stopped 状态的策略
    Given 策略状态为 "stopped"
    When 点击"删除"按钮
    Then 弹出确认对话框"确定要删除策略「xxx」吗？此操作不可恢复"
      And 用户确认后
    Then 策略被删除
      And 策略从列表中消失
      And 提示"策略已删除"

  Scenario: 删除 active 状态策略
    Given 策略状态为 "active"
    When 点击"删除"按钮
    Then 提示"请先停止策略后再删除"
      And 阻止删除操作

  Scenario: 删除时后端返回错误
    Given 策略 ID 不存在或已被删除
    When 执行删除操作
    Then 提示"策略不存在或已被删除"
      And 刷新列表
```

### US-SS-06: 浏览策略模板库 (P2)

> **As a** 交易者
> **I want to** 浏览所有可用的内置策略模板
> **So that** 我可以了解系统提供了哪些策略能力

**验收条件：**

```gherkin
Feature: 策略模板浏览
  Background:
    Given 用户已登录

  Scenario: 查看模板列表
    Given 用户点击"创建策略"
    When 模板选择面板显示
    Then 显示所有 10 个内置模板
      And 每个模板显示: 名称、简要描述、分类标签（趋势跟踪/均值回归/波动率）
      And 模板按分类分组展示
      And 支持搜索模板名称

  Scenario: 查看模板详情
    Given 模板选择面板
    When 点击某个模板卡片
    Then 展开显示模板的完整描述
      And 显示该模板的所有可配置参数及默认值
      And 显示"使用此模板"按钮
```

---

## 5. 策略状态机

```
                ┌──────────┐
                │  draft   │ ◄── 新建策略默认状态
                └────┬─────┘
                     │ 启用
                     ▼
                ┌──────────┐
         ┌─────▶│  active  │◀────┐
         │      └────┬─────┘     │
         │           │           │
         │       暂停 │           │ 启用
         │           ▼           │
         │      ┌──────────┐     │
         │      │  paused  │─────┘
         │      └────┬─────┘
         │           │
         │       停止 │
         │           ▼
         │      ┌──────────┐
         └──────│ stopped  │──仅在存档/删除时离开
                └──────────┘
```

**状态转换规则：**

| 当前状态 | 允许操作 | 目标状态 | 约束 |
|---------|---------|---------|------|
| draft | 启用 | active | — |
| draft | 编辑 | draft | 可修改参数 |
| draft | 删除 | — (已删除) | 确认弹窗 |
| active | 暂停 | paused | 确认弹窗 |
| active | 停止 | stopped | 确认弹窗 |
| active | 编辑 | (拒绝) | 提示先暂停 |
| paused | 启用 | active | — |
| paused | 停止 | stopped | 确认弹窗 |
| paused | 编辑 | paused | 可修改参数 |
| stopped | 删除 | — (已删除) | 确认弹窗 |
| stopped | 启用 | (拒绝) | 不允许 |
| stopped | 编辑 | (拒绝) | 不允许 |

---

## 6. 数据模型

### 6.1 策略模板 (后端内置，不入库)

Rust 定义，编译在主程序中：

```rust
/// 策略模板分类
enum TemplateCategory {
    TrendFollowing,  // 趋势跟踪
    MeanReversion,   // 均值回归
    Volatility,      // 波动率
    Composite,       // 复合策略 (预留)
}

/// 参数定义
struct ParameterDef {
    name: &'static str,            // 参数名，如 "fast_period"
    display_name: &'static str,    // 展示名，如 "快线周期"
    description: &'static str,     // 描述
    param_type: ParamType,         // 类型
    default_value: serde_json::Value,
    min_value: Option<f64>,
    max_value: Option<f64>,
    step: Option<f64>,             // 步长 (滑块用)
    required: bool,
}

enum ParamType {
    Integer,    // 整数
    Float,      // 浮点数
    Select,     // 枚举选择
    Boolean,    // 开关
}

/// 策略模板
struct StrategyTemplate {
    id: &'static str,              // 如 "ma_cross"
    name: &'static str,            // 如 "双均线交叉"
    description: &'static str,
    category: TemplateCategory,
    params: &'static [ParameterDef],
}
```

### 6.2 用户策略 (数据库)

**表名: `user_strategies`**

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | |
| user_id | UUID | FK → users.id, NOT NULL | 拥有者 |
| name | VARCHAR(100) | NOT NULL | 策略名称，用户唯一 |
| template_id | VARCHAR(50) | NOT NULL | 引用内置模板 ID |
| params | JSONB | NOT NULL | 用户配置的参数键值对 |
| status | VARCHAR(20) | NOT NULL, DEFAULT 'draft' | draft/active/paused/stopped |
| symbol | VARCHAR(20) | NOT NULL | 交易对 |
| timeframe | VARCHAR(10) | NOT NULL | 时间周期: 1m/5m/15m/1h/4h/1d |
| created_at | TIMESTAMPTZ | NOT NULL | |
| updated_at | TIMESTAMPTZ | NOT NULL | |

**唯一约束：** `(user_id, name)` — 同一用户下策略名称唯一。

---

## 7. API 设计

遵循 TECH_CHARTER 规范：`/api/v1/{resource}` + snake_case JSON。

### 7.1 策略模板 API

| 方法 | 路径 | 说明 | 优先级 |
|------|------|------|--------|
| GET | `/api/v1/strategies/templates` | 获取所有内置模板列表 | P0 |
| GET | `/api/v1/strategies/templates/{id}` | 获取单个模板详情（含参数定义） | P1 |

**GET /api/v1/strategies/templates 响应示例：**
```json
{
  "code": 0,
  "data": [
    {
      "id": "ma_cross",
      "name": "双均线交叉",
      "description": "快线上穿慢线时买入，下穿时卖出",
      "category": "trend_following",
      "params": [
        {"name": "fast_period", "display_name": "快线周期", "type": "integer", "default": 5, "min": 5, "max": 50, "step": 1, "required": true},
        {"name": "slow_period", "display_name": "慢线周期", "type": "integer", "default": 20, "min": 10, "max": 200, "step": 1, "required": true}
      ]
    }
  ],
  "message": "success"
}
```

### 7.2 用户策略 CRUD

| 方法 | 路径 | 说明 | 优先级 |
|------|------|------|--------|
| GET | `/api/v1/strategies` | 获取当前用户策略列表（分页） | P0 |
| POST | `/api/v1/strategies` | 创建新策略 | P0 |
| GET | `/api/v1/strategies/{id}` | 获取单个策略详情 | P1 |
| PUT | `/api/v1/strategies/{id}` | 更新策略参数/名称 | P1 |
| PATCH | `/api/v1/strategies/{id}/status` | 切换策略状态 | P0 |
| DELETE | `/api/v1/strategies/{id}` | 删除策略 | P1 |

**POST /api/v1/strategies 请求体：**
```json
{
  "name": "我的双均线策略",
  "template_id": "ma_cross",
  "params": {"fast_period": 10, "slow_period": 30},
  "symbol": "BTC/USDT",
  "timeframe": "1h"
}
```

**PATCH /api/v1/strategies/{id}/status 请求体：**
```json
{
  "status": "active"
}
```
> 内部校验状态转换合法性；非法转换返回 400 + 错误消息。

### 7.3 额外错误码

```rust
pub const ERR_STRATEGY_INVALID_TRANSITION: i32 = 42201;  // 非法状态转换
pub const ERR_STRATEGY_NAME_DUPLICATE: i32 = 40901;      // 策略名称重复
pub const ERR_STRATEGY_LIMIT_EXCEEDED: i32 = 42901;       // 超出并发限制
```

---

## 8. 前端路由与页面设计

### 8.1 路由

| 路径 | 组件 | 说明 |
|------|------|------|
| `/strategies` | `StrategiesView.vue` | 策略列表页（已配置） |
| 无子路由 | — | MVP 阶段创建/编辑使用弹窗而非独立页面 |

> 创建和编辑策略使用 Dialog/Modal 组件，避免路由嵌套带来的复杂度。

### 8.2 页面布局

```
┌────────────────────────────────────────────────────┐
│  策略管理                          [创建策略] 按钮   │
├────────────────────────────────────────────────────┤
│                                                    │
│  ┌──────────────────────────────────────────────┐  │
│  │  策略名称    模板类型    参数摘要    状态  操作  │  │
│  ├──────────────────────────────────────────────┤  │
│  │  BTC趋势   双均线交叉  fast=10,slow=30 运行中 [暂停][停止]│
│  │  ETH策略  布林带反转  period=20,std=2.0 暂停  [启用][停止]│
│  │  均值回归  RSI超买超卖  period=14  draft    [启用][删除]│
│  └──────────────────────────────────────────────┘  │
│                                                    │
│  分页: < 1 2 3 ... >                               │
└────────────────────────────────────────────────────┘
```

### 8.3 组件树

```
StrategiesView.vue
├── PageHeader (标题 + 「创建策略」按钮)
├── StrategyTable (el-table)
│   ├── StatusTag (颜色标签: draft=灰/active=绿/paused=橙/stopped=红)
│   └── ActionButtons (根据状态动态渲染)
├── EmptyState (列表为空时)
├── CreateStrategyDialog (创建/编辑弹窗)
│   ├── Step1: TemplateSelector (模板卡片网格 + 搜索)
│   └── Step2: ParameterForm (动态渲染)
│       ├── ElSlider (数值参数)
│       ├── ElSelect (枚举参数)
│       └── ElSwitch (开关参数)
├── ConfirmDialog (操作确认)
└── Pagination (分页)
```

### 8.4 Pinia Store: `useStrategyStore`

```typescript
interface StrategyState {
  strategies: Strategy[]
  total: number
  page: number
  pageSize: number
  loading: boolean
  templates: Template[]
  currentTemplate: Template | null
}

interface Strategy {
  id: string
  name: string
  template_id: string
  template_name: string
  params: Record<string, any>
  param_summary: string      // 前端拼接的摘要文本
  status: 'draft' | 'active' | 'paused' | 'stopped'
  symbol: string
  timeframe: string
  created_at: string
  updated_at: string
}

interface Template {
  id: string
  name: string
  description: string
  category: string
  params: ParamDef[]
}
```

Actions: `fetchStrategies`, `fetchTemplates`, `createStrategy`, `updateStrategy`, `updateStatus`, `deleteStrategy`.

---

## 9. 内置模板清单 (MVP 10 个)

详细参数定义：

| # | ID | 名称 | 分类 | 参数 |
|---|-----|------|------|------|
| 1 | `ma_cross` | 双均线交叉 | 趋势跟踪 | fast_period[int:5-50:10], slow_period[int:10-200:30] |
| 2 | `triple_ma` | 三均线 | 趋势跟踪 | short[int:5-20:5], medium[int:10-50:20], long[int:20-200:60] |
| 3 | `macd` | MACD 交叉 | 趋势跟踪 | fast[int:5-20:12], slow[int:20-40:26], signal[int:5-15:9] |
| 4 | `bollinger` | 布林带反转 | 均值回归 | period[int:10-50:20], std_dev[float:1.0-3.5:2.0] |
| 5 | `rsi` | RSI 超买超卖 | 均值回归 | period[int:5-30:14], overbought[float:70-90:70], oversold[float:10-30:30] |
| 6 | `keltner` | Keltner 通道 | 波动率 | period[int:10-50:20], atr_multiplier[float:1.0-3.0:2.0] |
| 7 | `atr_stop` | ATR 止损 | 波动率 | period[int:7-30:14], multiplier[float:1.0-5.0:3.0] |
| 8 | `mean_reversion` | 均值回归 | 均值回归 | period[int:10-50:20], entry_std[float:1.0-3.0:2.0], exit_std[float:0.0-1.5:0.5] |
| 9 | `ichimoku` | ICHIMOKU 云图 | 趋势跟踪 | conversion[int:5-20:9], base[int:20-40:26], span[int:40-70:52], displ[int:20-30:26] |
| 10 | `double_bollinger` | 双布林带 | 波动率 | period[int:10-50:20], inner_std[float:0.5-2.0:1.5], outer_std[float:1.5-4.0:2.5] |

> 参数格式: `name[type:range:default]`。所有整数参数 step=1，浮点数 step=0.5。

---

## 10. 验收条件汇总

### P0 (必须有)

| ID | 场景 | 类型 |
|----|------|------|
| SS-P0-01 | 用户打开策略页面，显示策略列表（含模板选择入口） | 列表展示 |
| SS-P0-02 | 用户选择模板并配置参数，创建策略成功 | 创建 |
| SS-P0-03 | 已创建的策略可以暂停 | 状态切换 |
| SS-P0-04 | 已暂停的策略可以重新启用 | 状态切换 |
| SS-P0-05 | 策略列表为空时显示空状态提示 | 边界情况 |
| SS-P0-06 | 用户可切换策略状态 draft→active→paused→stopped 完整链路 | 状态机 |

### P1 (应该有)

| ID | 场景 | 类型 |
|----|------|------|
| SS-P1-01 | 创建策略时必填参数为空，显示表单校验错误 | 校验 |
| SS-P1-02 | 参数值超出定义范围，显示错误提示 | 校验 |
| SS-P1-03 | 已创建的策略可以编辑参数（draft/paused 状态） | 编辑 |
| SS-P1-04 | 运行中的策略不能编辑，提示先暂停 | 约束 |
| SS-P1-05 | 运行中的策略可停止，状态变为 stopped | 状态切换 |
| SS-P1-06 | 已停止的策略可以删除 | 删除 |
| SS-P1-07 | 策略名称在同一用户下重复，返回 409 | 唯一性 |
| SS-P1-08 | 活跃运行策略超过 5 个时创建新策略拒绝 | 并发限制 |
| SS-P1-09 | 非法状态转换返回 400 并提示 | 状态机 |

### P2 (可以有)

| ID | 场景 | 类型 |
|----|------|------|
| SS-P2-01 | 模板列表按分类分组展示 | 体验 |
| SS-P2-02 | 模板卡片名称、描述、分类标签完整 | 体验 |
| SS-P2-03 | 参数表单滑块实时显示当前值 | 体验 |
| SS-P2-04 | 创建/编辑时取消操作回退到列表页不保存 | 体验 |
| SS-P2-05 | 策略列表支持搜索和过滤 | 体验 |
| SS-P2-06 | 列表页显示策略参数摘要 | 体验 |

---

## 11. 边界情况与错误处理

| 场景 | 预期行为 |
|------|---------|
| 用户未登录访问 /strategies | 重定向至登录页（已有路由守卫） |
| 删除已被其他进程删除的策略 | 返回 404，提示"策略不存在或已被删除" |
| 并发操作同一策略的状态 | 后端数据库乐观锁或行级锁，防止竞态 |
| 数据库连接失败 | 返回 500 + 错误码 50002，前端提示"服务异常，请稍后重试" |
| 策略名称为空或仅空白字符 | 前端校验拒绝 + 后端再次校验 |
| 参数类型不匹配（传字符串给数字参数） | 后端反序列化失败 → 400 + 指明字段 |
| 模板 ID 不存在 | 创建时返回 400 "无效的模板 ID" |
| 策略状态长时间卡在 active 但无响应 | 预留 watchdog 心跳接口（二期实现） |
| 切换交易对/周期时参数不改 | 允许，symbol/timeframe 是必填但非模板参数 |

---

## 12. 非功能需求

| 需求 | 要求 |
|------|------|
| 响应时间 | CRUD 接口 P99 < 200ms |
| 并发加载 | 策略列表 100 条同时在线用户 < 1s 加载 |
| 安全性 | 用户只能操作自己的策略（user_id 鉴权） |
| 前端加载 | 策略页面首屏 JS < 200KB |
| 可测试性 | 每个 REST 端点有 E2E 测试 |
| 可观测性 | 策略状态变更事件写审计日志 |

---

## 13. 术语表

| 术语 | 说明 |
|------|------|
| 策略模板 (Template) | 后端内置的不可变策略逻辑定义，含参数元数据 |
| 策略实例 (Strategy) | 用户基于模板创建的具体策略，含实际参数值 |
| 交易对 (Symbol) | 如 "BTC/USDT"，策略运行的标的 |
| 时间周期 (Timeframe) | 如 "1h"，K 线周期 |
| 策略状态 | draft→active→paused→stopped |


## 14. 附录 — 新增后端文件清单

| 文件 | 说明 |
|------|------|
| `backend/src/models/strategy.rs` | Strategy 数据结构、Template 定义、状态枚举 |
| `backend/src/models/schemas.rs` (追加) | 新增 StrategyRequest/Response 等 schema |
| `backend/src/handlers/strategy.rs` | CRUD + 状态切换 handler |
| `backend/src/handlers/mod.rs` (修改) | 注册 strategy handler |
| `backend/src/services/strategy.rs` | 业务逻辑：模板管理、状态机校验 |
| `backend/src/db/strategy.rs` | 数据库操作：CRUD + 分页查询 |
| `backend/src/db/mod.rs` (修改) | 导出 strategy db module |
| `backend/migrations/xxx_create_user_strategies.sql` | 数据库迁移 |
| `frontend/src/views/strategy/StrategiesView.vue` (重写) | 策略列表 + 操作 |
| `frontend/src/views/strategy/CreateStrategyDialog.vue` (新建) | 创建/编辑弹窗 |
| `frontend/src/views/strategy/TemplateSelector.vue` (新建) | 模板选择面板 |
| `frontend/src/views/strategy/ParameterForm.vue` (新建) | 动态参数表单 |
| `frontend/src/views/strategy/StrategyTable.vue` (新建) | 策略表格 |
| `frontend/src/stores/strategy.ts` (新建) | Pinia store |
| `frontend/src/api/strategy.ts` (新建) | Axios API 封装 |
| `frontend/src/types/strategy.ts` (新建) | TypeScript 类型定义 |

---

> **下一步建议：** 本 PRD 完成后，Tech Lead 进行架构评审，确认数据模型和 API 设计，然后按 `Spec First → Test First → Code Last` 流程将 US-SS-01 到 US-SS-06 拆分为开发任务。
