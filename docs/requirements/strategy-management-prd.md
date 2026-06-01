# PRD: 策略管理模块

> 版本: v1.0
> 状态: Draft
> 作者: PM
> 关联: PRD v1.0 策略沙箱 / 策略回测引擎

---

## 1. 背景

当前系统已有策略沙箱模块（`PRD-strategy-sandbox-mvp`）支持内置模板创建策略，策略回测引擎（`PRD-backtest-engine`）支持历史K线回测与绩效分析。

本模块（策略管理）负责策略实例的全生命周期管理：除基础的增删改查外，还包含：
- **状态机**：draft / active / paused / stopped
- **导入/导出**：JSON 格式备份与迁移
- **权限控制**：用户只能操作自己创建的策略

---

## 2. 目标

| 目标 | 指标 |
|------|------|
| 策略 CRUD | 接口 P99 < 200ms |
| 状态切换 | 所有状态转换原子性，非法转换返回明确错误 |
| 导入/导出 | 支持单条策略 JSON 导出，平台可完整导入恢复 |
| 权限隔离 | 用户 A 无法读取/修改/删除用户 B 的策略 |

### 非目标 (Non-Goals)

- ❌ 策略运行与调度（属于策略沙箱模块）
- ❌ 策略回测（属于策略回测引擎模块）
- ❌ 策略市场 / 分享（P2+）
- ❌ 批量导入导出多条策略（P2）

---

## 3. 用户角色与权限

| 角色 | 策略权限 | 说明 |
|------|---------|------|
| **trader** (普通用户) | CRUD 自有策略 | 默认角色 |
| **pro-trader** (专业交易员) | CRUD 自有策略 | 需管理员开通 |
| **admin** (管理员) | 查看所有策略，不可编辑他人策略 | 审计用途，可强制停止 |

> 注：MVP 阶段所有策略均为"模拟运行"，不涉及实盘资金。

---

## 4. 用户故事 (User Stories)

### US-SM-01: 查看策略列表 (P0)

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

  Scenario: 筛选策略列表 by 状态
    Given 用户拥有多个策略，状态分别为 draft/active/paused/stopped
    When 选择状态筛选器 "active"
    Then 仅显示状态为 active 的策略
      And 其他状态策略被隐藏

  Scenario: 搜索策略名称
    Given 用户拥有策略 "BTC双均线" 和 "ETH布林带"
    When 输入搜索词 "BTC"
    Then 仅显示名称包含 "BTC" 的策略
```

### US-SM-02: 创建策略 (P0)

> **As a** 交易者
> **I want to** 通过选择内置模板并配置参数来创建一个新策略
> **So that** 我不需要编写代码就能快速创建量化策略

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
      And 点击"保存"
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

  Scenario: 策略名称重复时拒绝创建
    Given 用户已有名为 "我的双均线" 的策略
    When 创建新策略并输入名称 "我的双均线"
    Then 保存失败
      And 提示"策略名称已存在，请使用其他名称"
```

### US-SM-03: 策略状态机 (P0)

> **As a** 交易者
> **I want to** 通过状态机控制策略的运行生命周期
> **So that** 我可以灵活地管理策略何时运行、何时停止

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
      And 按钮变为"暂停"和"停止"

  Scenario: 暂停运行中的策略
    Given 策略状态为 "active"
    When 点击"暂停"按钮
    Then 弹出确认对话框"确定要暂停该策略吗？"
      And 用户确认后
    Then 状态变为 "paused"
      And 按钮变为"启用"和"停止"

  Scenario: 停止策略（终止运行）
    Given 策略状态为 "active" 或 "paused"
    When 点击"停止"按钮
    Then 弹出确认对话框"策略停止后不可恢复，确定吗？"
      And 用户确认后
    Then 状态变为 "stopped"
      And 仅可删除或复制

  Scenario: 恢复暂停的策略
    Given 策略状态为 "paused"
    When 点击"启用"按钮
    Then 状态恢复为 "active"

  Scenario: 已停止的策略无法重新启用
    Given 策略状态为 "stopped"
    When 点击"启用"
    Then "启用"按钮不可点击
      And 工具提示"已停止的策略无法重新启用"

  Scenario: 非法状态转换返回错误
    Given 策略状态为 "draft"
    When 前端直接发送 PATCH /api/v1/strategies/{id}/status { "status": "paused" }
    Then 返回 400 错误
      And 提示"草稿状态不能直接转为暂停，请先启用"
```

### US-SM-04: 编辑策略 (P1)

> **As a** 交易者
> **I want to** 修改已创建策略的名称、描述和参数
> **So that** 我可以优化策略配置而不用重新创建

**验收条件：**

```gherkin
Feature: 编辑策略
  Background:
    Given 用户已登录
      And 拥有 1 个策略

  Scenario: 编辑 draft 策略
    Given 策略状态为 "draft"
    When 点击"编辑"按钮
    Then 打开参数配置弹窗（同创建弹窗，预填当前值）
    When 修改 fast_period 从 10 改为 5
      And 点击"保存"
    Then 策略参数更新成功
      And 列表参数摘要更新为新值

  Scenario: 编辑 active 策略被拒绝
    Given 策略状态为 "active"
    When 点击"编辑"按钮
    Then 提示"策略运行中，请先暂停后再编辑"
      And 不允许编辑

  Scenario: 编辑 paused 策略
    Given 策略状态为 "paused"
    When 点击"编辑"按钮
    Then 可修改参数
      And 保存后状态保持 paused

  Scenario: 编辑后取消
    Given 编辑弹窗已打开并修改了参数
    When 点击"取消"
    Then 参数恢复为修改前的值
      And 不触发后端更新

  Scenario: 修改策略名称
    Given 策略名称为 "旧名称"
    When 编辑并修改名称为 "新名称"
    Then 保存成功
      And 列表中显示新名称
```

### US-SM-05: 删除策略 (P1)

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

  Scenario: 删除 active 状态策略被阻止
    Given 策略状态为 "active"
    When 点击"删除"按钮
    Then 提示"请先停止策略后再删除"
      And 阻止删除操作

  Scenario: 删除 paused 状态策略被阻止
    Given 策略状态为 "paused"
    When 点击"删除"按钮
    Then 提示"请先停止策略后再删除"
      And 阻止删除操作

  Scenario: 删除 draft 状态策略
    Given 策略状态为 "draft"
    When 点击"删除"按钮
    Then 直接删除（无需确认）
      And 策略从列表中消失
```

### US-SM-06: 策略导入/导出 (P1)

> **As a** 交易者
> **I want to** 将策略导出为 JSON，以及从 JSON 导入策略
> **So that** 我可以备份策略配置或在平台间迁移

**验收条件：**

```gherkin
Feature: 策略导入导出
  Background:
    Given 用户已登录
      And 用户拥有 1 个 "stopped" 状态的策略

  Scenario: 导出单条策略为 JSON
    Given 用户点击某策略的"导出"按钮
    Then 浏览器下载 strategy_export_<id>.json 文件
      And JSON 包含: name, template_id, params, symbol, timeframe, status, created_at

  Scenario: 导出 JSON 可完整导入恢复
    Given 导出了策略 JSON
    When 在策略列表页点击"导入策略"
      And 上传该 JSON 文件
    Then 创建成功
      And 新策略名称后缀" (导入)"
      And 其他参数与原策略一致

  Scenario: 导入损坏的 JSON 文件
    Given 用户上传一个格式错误的 JSON
    When 点击"导入"
    Then 提示"文件格式错误，请上传有效的策略 JSON"
      And 不创建任何策略

  Scenario: 导入时模板 ID 不存在
    Given 导出的 JSON 中 template_id 为一个不存在的模板
    When 导入该文件
    Then 提示"策略模板不存在，导入失败"

  Scenario: 导入时模板版本不匹配
    Given 导出的 JSON 中模板版本低于当前版本
    When 导入该文件
    Then 提示"模板版本已更新，部分参数可能不兼容"
      And 允许用户手动调整后继续导入
```

### US-SM-07: 策略模板浏览与管理 (P1)

> **As a** 交易者
> **I want to** 查看所有可用的内置策略模板，并管理自己的自定义模板
> **So that** 我可以快速选择适合的策略框架

**验收条件：**

```gherkin
Feature: 策略模板浏览
  Background:
    Given 用户已登录

  Scenario: 查看内置模板列表
    Given 用户点击"创建策略"
    When 模板选择面板显示
    Then 显示所有内置模板
      And 每个模板显示: 名称、简要描述、分类标签
      And 模板按分类分组展示
      And 支持搜索模板名称

  Scenario: 查看模板详情
    Given 模板选择面板
    When 点击某个模板卡片
    Then 展开显示模板的完整描述
      And 显示该模板的所有可配置参数及默认值
      And 显示"使用此模板"按钮

  Scenario: 查看自定义模板列表
    Given 用户拥有自定义模板
    When 在创建策略面板切换到"我的模板"标签
    Then 显示用户创建的所有自定义模板
      And 每个显示: 名称、描述、参数数量

  Scenario: 创建自定义模板
    Given 用户点击"新建模板"
    When 输入模板名称、描述
      And 定义参数列表（名称/类型/默认值/范围）
      And 点击"保存"
    Then 自定义模板创建成功
      And 在"我的模板"标签页可见

  Scenario: 编辑自定义模板
    Given 用户拥有 1 个自定义模板
    When 点击"编辑"该自定义模板
    Then 可修改名称、描述、参数定义
      And 保存后不影响已有策略实例

  Scenario: 删除自定义模板
    Given 用户拥有 1 个自定义模板
      And 该模板没有被任何策略使用
    When 点击"删除"该自定义模板
    Then 删除成功
      And 模板从列表消失

  Scenario: 删除已被策略使用的自定义模板
    Given 自定义模板正被 1 个策略实例引用
    When 点击"删除"该自定义模板
    Then 提示"该模板正在被 N 个策略使用，无法删除"
      And 删除操作被阻止
```

### US-SM-08: 权限控制 (P0)

> **As a** 交易者
> **I want to** 确保其他用户无法操作我的策略
> **So that** 我的策略配置和运行安全隔离

**验收条件：**

```gherkin
Feature: 策略权限控制
  Background:
    Given 用户 A 和用户 B 均已登录

  Scenario: 用户无法查看他人策略列表
    Given 用户 A 尝试访问 GET /api/v1/strategies
    Then 仅返回用户 A 自己的策略
      And 不包含用户 B 的任何策略

  Scenario: 用户无法查看他人策略详情
    Given 用户 A 尝试访问 GET /api/v1/strategies/{b_strategy_id}
    Then 返回 403 Forbidden
      And 提示"无权访问该策略"

  Scenario: 用户无法修改他人策略
    Given 用户 A 尝试 PUT /api/v1/strategies/{b_strategy_id}
    Then 返回 403 Forbidden
      And 提示"无权操作该策略"

  Scenario: 用户无法删除他人策略
    Given 用户 A 尝试 DELETE /api/v1/strategies/{b_strategy_id}
    Then 返回 403 Forbidden
      And 提示"无权操作该策略"

  Scenario: 用户无法切换他人策略状态
    Given 用户 A 尝试 PATCH /api/v1/strategies/{b_strategy_id}/status
    Then 返回 403 Forbidden

  Scenario: 管理员可查看所有策略
    Given 管理员尝试访问 GET /api/v1/strategies
    Then 返回所有用户的策略（分页）
      And 包含 user_id 字段用于区分拥有者
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
         │           │           │ 启用
         │       暂停 │           │
         │           ▼           │
         │      ┌──────────┐     │
         │      │  paused  │─────┘
         │      └────┬─────┘
         │           │
         │       停止 │
         │           ▼
         │      ┌──────────┐
         └──────│ stopped  │──仅删除时离开
                └──────────┘
```

**状态转换规则：**

| 当前状态 | 允许操作 | 目标状态 | 约束 |
|---------|---------|---------|------|
| draft | 启用 | active | — |
| draft | 编辑 | draft | 可修改参数 |
| draft | 删除 | — (已删除) | 直接删除 |
| active | 暂停 | paused | 确认弹窗 |
| active | 停止 | stopped | 确认弹窗 |
| active | 编辑 | (拒绝) | 提示先暂停 |
| active | 删除 | (拒绝) | 提示先停止 |
| paused | 启用 | active | — |
| paused | 停止 | stopped | 确认弹窗 |
| paused | 编辑 | paused | 可修改参数 |
| paused | 删除 | (拒绝) | 提示先停止 |
| stopped | 删除 | — (已删除) | 确认弹窗 |
| stopped | 启用 | (拒绝) | 不允许 |
| stopped | 编辑 | (拒绝) | 不允许 |

---

## 6. 数据模型

### 6.1 策略模板（内置 + 自定义）

**表名: `strategy_templates`**

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | |
| name | VARCHAR(100) | NOT NULL | 模板名称 |
| description | TEXT | | 模板描述 |
| template_type | VARCHAR(20) | NOT NULL | 'builtin' / 'custom' |
| params | JSONB | NOT NULL | 参数定义数组 |
| is_active | BOOLEAN | DEFAULT true | 是否启用 |
| created_by | UUID | FK → users.id | 仅 custom 模板有值 |
| created_at | TIMESTAMPTZ | NOT NULL | |
| updated_at | TIMESTAMPTZ | NOT NULL | |

**params JSONB 结构示例：**
```json
[
  {
    "name": "fast_period",
    "display_name": "快线周期",
    "type": "integer",
    "default": 5,
    "min": 5,
    "max": 50,
    "required": true
  }
]
```

### 6.2 用户策略实例

**表名: `user_strategies`**

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | |
| user_id | UUID | FK → users.id, NOT NULL | 拥有者 |
| name | VARCHAR(100) | NOT NULL | 策略名称，用户唯一 |
| description | TEXT | | 策略描述 |
| template_id | UUID | FK → strategy_templates.id, NOT NULL | 引用模板 |
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
| GET | `/api/v1/strategy-templates` | 获取模板列表（内置+自定义） | P0 |
| GET | `/api/v1/strategy-templates/{id}` | 获取单个模板详情（含参数定义） | P1 |
| POST | `/api/v1/strategy-templates` | 创建自定义模板 | P1 |
| PUT | `/api/v1/strategy-templates/{id}` | 更新自定义模板 | P2 |
| DELETE | `/api/v1/strategy-templates/{id}` | 删除自定义模板 | P2 |

### 7.2 用户策略 CRUD

| 方法 | 路径 | 说明 | 优先级 |
|------|------|------|--------|
| GET | `/api/v1/strategies` | 获取当前用户策略列表（分页/筛选/搜索） | P0 |
| POST | `/api/v1/strategies` | 创建新策略 | P0 |
| GET | `/api/v1/strategies/{id}` | 获取单个策略详情 | P1 |
| PUT | `/api/v1/strategies/{id}` | 更新策略参数/名称/描述 | P1 |
| PATCH | `/api/v1/strategies/{id}/status` | 切换策略状态 | P0 |
| DELETE | `/api/v1/strategies/{id}` | 删除策略 | P1 |

### 7.3 导入/导出 API

| 方法 | 路径 | 说明 | 优先级 |
|------|------|------|--------|
| POST | `/api/v1/strategies/{id}/export` | 导出单条策略为 JSON | P1 |
| POST | `/api/v1/strategies/import` | 从 JSON 导入策略 | P1 |

### 7.4 错误码

```rust
pub const ERR_STRATEGY_NOT_FOUND: i32 = 40401;          // 策略不存在
pub const ERR_STRATEGY_INVALID_TRANSITION: i32 = 42201;  // 非法状态转换
pub const ERR_STRATEGY_NAME_DUPLICATE: i32 = 40901;      // 策略名称重复
pub const ERR_STRATEGY_ACCESS_DENIED: i32 = 40301;      // 无权访问策略
pub const ERR_TEMPLATE_NOT_FOUND: i32 = 40402;          // 模板不存在
pub const ERR_TEMPLATE_IN_USE: i32 = 40902;            // 模板正被使用
pub const ERR_IMPORT_INVALID_JSON: i32 = 40001;        // 导入 JSON 格式错误
pub const ERR_IMPORT_TEMPLATE_MISSING: i32 = 40403;    // 导入时模板不存在
```

---

## 8. 前端路由与页面设计

### 8.1 路由

| 路径 | 组件 | 说明 |
|------|------|------|
| `/strategies` | `StrategiesView.vue` | 策略列表页 |
| `/strategies/:id` | `StrategyDetailView.vue` | 策略详情页（含操作按钮） |

> 创建和编辑策略使用 Dialog/Modal 组件。

### 8.2 页面布局

```
┌────────────────────────────────────────────────────────────┐
│  策略管理                                [导入] [创建策略]    │
├────────────────────────────────────────────────────────────┤
│  [全部] [草稿] [运行中] [已暂停] [已停止]    [🔍 搜索...]     │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 双均线策略    双均线交叉  BTC/USDT 1h   ● 运行中      │  │
│  │ 创建于 2026-05-10                         [启用/暂停] │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 布林带策略    布林带反转  ETH/USDT 15m  ○ 草稿        │  │
│  │ 创建于 2026-05-12              [启用] [编辑] [删除]   │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ 波动率策略    波动率突破  BTC/USDT 4h   ■ 已停止       │  │
│  │ 创建于 2026-05-08           [导出] [删除]             │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                            │
│                        < 1 / 3 >                           │
└────────────────────────────────────────────────────────────┘
```

**状态指示器：** ● active (绿) / ○ draft (灰) / ◐ paused (黄) / ■ stopped (红)

---

## 9. 边界情况

| 场景 | 处理方式 |
|------|---------|
| 删除有运行历史的策略 | 允许删除，保留回测记录（回测模块独立） |
| 导入时策略名称与现有冲突 | 自动重命名为"xxx (导入-2)" |
| 模板参数定义变更（自定义模板） | 已创建策略不受影响，新实例使用新定义 |
| 用户删除自己的最后一个策略 | 列表显示空状态 |
| 超级长/超短策略名称 | 前端限制 1-100 字符 |
| 导出他人策略 ID（构造请求） | 后端校验 user_id，返回 403 |

---

## 10. 非功能需求

| 需求 | 说明 |
|------|------|
| 性能 | 策略列表接口 P99 < 200ms |
| 容量 | 单用户策略上限 100 条（软限制，前端提示） |
| 可用性 | 状态切换原子性，失败不遗留中间状态 |
| 安全 | 所有策略操作鉴权，IDOR 防护 |
| 导出格式 | UTF-8 编码，JSON Schema 稳定版本 |
