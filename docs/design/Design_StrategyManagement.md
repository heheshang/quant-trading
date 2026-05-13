# 设计规格说明书：策略管理 UI 界面

> 基于架构 ADR (ADR-STRATEGY-MGMT) + PRD (strategy-management-prd.md) 的完整 UI/UX 设计方案
> 设计师：Designer | 版本 v2.0 | 日期：2026-05-13
> 设计灵感：Linear.app 暗色模式设计系统 + 金融交易终端风格
> 框架：Element Plus + Vue 3 + TypeScript
> 源代码库：/home/ssk/workspace/quant-trading
> **本版本更新：v1.0 → v2.0：增加 symbol/timeframe 必填字段、template_id UUID 引用、模板市场、筛选/分页/批量操作**

---

## 目录

1. [设计系统继承](#1-设计系统继承)
2. [页面1：策略列表页 /strategies](#2-页面1策略列表页-strategies)
3. [页面2：创建/编辑策略页 /strategies/new 和 /strategies/:id/edit](#3-页面2创建编辑策略页-strategiesnew-和-strategiesidedit)
4. [页面3：策略模板市场 /strategies/templates](#4-页面3策略模板市场-strategiestemplates)
5. [组件状态总矩阵](#5-组件状态总矩阵)
6. [路由设计](#6-路由设计)
7. [API 数据流（已对齐 ADR）](#7-api-数据流已对齐-adr)
8. [响应式适配方案](#8-响应式适配方案)

---

## 1. 设计系统继承

### 1.1 完全继承现有 Design Token

本设计继承自 `Design_QuantTrading_UI.md` 的所有定义。以下是关键 token：

| Token | 暗色模式值 | 用途 |
|-------|-----------|------|
| `--color-bg` | `#08090a` | 最深层背景 |
| `--color-surface` | `#191a1b` | 卡片/面板背景 |
| `--color-surface-elevated` | `#212223` | 悬浮/下拉背景 |
| `--color-deep-bg` | `#050607` | 图表深层背景 |
| `--color-text-primary` | `#f7f8f8` | 主文字 |
| `--color-text-secondary` | `#d0d6e0` | 次要文字 |
| `--color-text-tertiary` | `#8a8f98` | 辅助/占位文字 |
| `--color-accent` | `#7170ff` | 主色调 |
| `--color-accent-hover` | `#8b8aff` | 主色调悬停 |
| `--color-border` | `rgba(255,255,255,0.08)` | 边框 |
| `--color-border-hover` | `rgba(255,255,255,0.16)` | 边框悬停 |
| `--color-error` | `#e5484d` | 错误 |
| `--color-success` | `#10b981` | 成功 |
| `--color-warning` | `#f5a623` | 警告 |
| `--color-info` | `#60a5fa` | 信息 |
| `--color-buy` | `#10b981` | 买入/上涨 |
| `--color-sell` | `#e5484d` | 卖出/下跌 |

字体：
- UI：`'Inter', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif`
- 等宽/数字：`'JetBrains Mono', 'SF Mono', Menlo, monospace`

### 1.2 策略管理专用组件列表

| 组件 | 类型 | 描述 |
|------|------|------|
| StrategyTableRow | 展示 | 策略表格行（含 symbol/timeframe 显示） |
| StrategyStatusBadge | 展示 | 策略状态徽标 (draft/active/paused/stopped) |
| TemplateCard | 展示/交互 | 模板选择卡片 |
| TemplateMarketplaceCard | 展示/交互 | 模板市场卡片（带使用量/评分） |
| ParamFormGroup | 表单 | 动态参数配置组 |
| ParamSlider | 表单 | 滑块数值输入 (el-slider) |
| ParamInputNumber | 表单 | 数字输入 (el-input-number) |
| ParamSelect | 表单 | 选项选择 (el-select) |
| ParamSwitch | 表单 | 开关 (el-switch) |
| ParamPreview | 展示 | 参数配置预览摘要 |
| StepIndicator | 展示 | 步骤指示器 |
| EmptyStrategies | 展示 | 策略空状态引导 |
| StrategyFilterBar | 展示/交互 | 状态筛选栏 |
| SymbolTimeframeSelector | 表单 | 交易对+周期联合选择器 |
| BulkActionBar | 交互 | 批量操作栏（选中 N 项时浮现） |

### 1.3 策略状态系统

| 状态 | 枚举值 | 含义 | 颜色 | 图标 |
|------|--------|------|------|------|
| 草稿 | draft | 未完成配置 | `--color-text-tertiary` `#8a8f98` | Edit |
| 运行中 | active | 策略正常交易 | `--color-success` `#10b981` | VideoPlay |
| 已暂停 | paused | 手动暂停 | `--color-warning` `#f5a623` | VideoPause |
| 已停止 | stopped | 彻底停止 | `--color-sell` `#e5484d` | CircleClose |

### 1.4 策略模板类型（Template Category）

| 模板类型 | 标签颜色 | 描述 |
|----------|---------|------|
| 趋势跟踪 | `--color-info` | 均线交叉/布林带 |
| 均值回归 | `--color-accent` | RSI/布林线回归 |
| 网格交易 | `--color-success` | 网格/定投 |
| 套利 | `--color-warning` | 跨交易所/三角套利 |
| 自定义 | `--color-text-tertiary` | 用户自定义脚本 |

---

## 2. 页面1：策略列表页 /strategies

### 2.1 布局结构

```
+--------------------------------------------------------------+
|  策略管理                                    [+ 创建策略]      |  ← PageHeader
|  策略模板市场                                      [>?]      |  ← Template Market Link
|                                                               |
|  [全部] [运行中] [已暂停] [草稿] [已停止]    [搜索...]  [?排序]|  ← FilterBar
|                                                               |
|  已选择 3 项:  [启用] [暂停] [停止] [删除]       [取消全选]    |  ← BulkActionBar (shown when items selected)
|                                                               |
|  +----------------------------------------------------------+ |
|  | [□] 策略名称    | 交易对 | 周期 | 模板类型  | 状态 | 操作 | |  ← TableHeader (checkbox column added)
|  |----|------------|--------|------|-----------|------|------| |
|  | □  | Grid BTC   | BTCUSDT|  1H  | 网格交易  | ●运行|  ⋮  | |
|  | □  | MA CROSS   | ETHUSDT|  4H  | 趋势跟踪  | ●暂停|  ⋮  | |
|  | □  | Arbitrage  | BTCUSDT|  1D  | 套利      | ●停止|  ⋮  | |
|  | □  | RSI Revert | BTCUSDT|  15M | 均值回归  | ○草稿|  ⋮  | |
|  +----------------------------------------------------------+ |
|                                                               |
|  [← 上一页]  1  2  3 ...  [下一页 →]       共 24 条  [20条/页]|  ← Pagination
+--------------------------------------------------------------+
```

**页面规格：**
- 最大宽度：`1344px`（与现有设计一致）
- 内边距：`24px`（桌面端）/ `16px`（移动端）
- 页面标题字号：`24px`，字重 600

### 2.2 页面顶部区域 (PageHeader)

**标题行布局：**
- 左侧：页面标题"策略管理"
- 右侧：`[+ 创建策略]` 按钮 + `[策略模板市场]` 链接按钮

**创建策略按钮规格：**
- 组件：`el-button` type="primary"
- 高度：`36px`
- 内边距：`0 16px`
- 图标：左侧 Plus 图标

**模板市场链接：**
- 组件：`el-button` type="default"（次要样式）
- 文字：`策略模板市场`
- 右侧图标：`ShopIcon` 或 `GridIcon`

### 2.3 筛选栏 (StrategyFilterBar)

**状态筛选 Pills：**
- 组件：`el-radio-group` 样式化为 pills
- 选项：`全部` `运行中` `已暂停` `草稿` `已停止`
- 默认选中：`全部`

| 状态 | 视觉 |
|------|------|
| 未选中 | 背景透明，文字 `--color-text-tertiary`，`13px` |
| 选中 | 背景 `--color-accent`，文字白色，`13px`，圆角 `16px` |

**搜索框：**
- 位置：筛选栏右侧
- 组件：`el-input` 带 search 图标
- 宽度：`240px`
- 占位符：`"搜索策略名称..."`

**排序：**
- 组件：`el-select` size="small"
- 选项：创建时间（最新）、创建时间（最早）、名称 A-Z、名称 Z-A
- 默认：创建时间（最新）

### 2.4 批量操作栏 (BulkActionBar)

**显示条件：** 当有策略被勾选时，筛选栏下方浮现

**布局：**
```
已选择 3 项:  [启用] [暂停] [停止] [删除]  [取消全选]
```

**规格：**
- 背景：`--color-surface-elevated`
- 边框：`1px solid --color-border`
- 内边距：`12px 16px`
- 按钮：`el-button` size="small"

**批量操作逻辑（按状态过滤可见按钮）：**

| 当前选中策略状态 | 显示按钮 |
|----------------|---------|
| 包含 active | 批量暂停、批量停止 |
| 包含 paused | 批量启用、批量停止 |
| 包含 draft | 批量删除 |
| 包含 stopped | 批量删除 |
| 混合状态 | 显示所有适用按钮 |

**删除批量：**
- 需二次确认 Modal（同单条删除确认）

### 2.5 策略表格 (StrategyTable)

**表格规格：**
- 组件：`el-table` size="default" + `el-checkbox`
- 背景：`--color-surface`
- 圆角：`8px`
- 边框：`1px solid --color-border`
- 行高：`56px`

**列定义（v2.0 新增交易对/周期列）：**

| 列 | 宽度 | 对齐 | 内容 |
|----|------|------|------|
| 复选框 | `48px` | 中 | `el-checkbox` 全选/单选 |
| 策略名称 | `min-width: 180px` | 左 | 名称（14px 600）+ 描述（12px tertiary，单行省略） |
| 交易对 | `100px` | 中 | `BTC/USDT` 等宽字体 13px |
| 周期 | `80px` | 中 | `1H` 等宽字体 13px |
| 模板类型 | `120px` | 左 | `el-tag` 显示模板类型标签 |
| 状态 | `100px` | 中 | `StatusBadge` 组件 |
| 操作 | `80px` | 右 | `⋮` 操作菜单按钮 |

**交易对/周期列规格：**
- 交易对：Symbol 等宽显示 `BTC/USDT`，色 `--color-text-primary`
- 周期：等宽显示 `1H/4H/1D`，色 `--color-text-secondary`
- 两列间距紧凑，不设可滚动

**策略名称列规格：**
- 第一行：策略名称，14px，字重 600，色 `--color-text-primary`
- 第二行：策略描述，12px，色 `--color-text-tertiary`，单行省略
- 两行间距：`2px`

**模板类型标签规格：**

| 模板类型 | el-tag type | 规格 |
|----------|------------|------|
| 趋势跟踪 | info | 小尺寸，`effect="plain"` |
| 均值回归 | — | 小尺寸，自定义色（`--color-accent`） |
| 网格交易 | success | 小尺寸，`effect="plain"` |
| 套利 | warning | 小尺寸，`effect="plain"` |
| 自定义 | info | 小尺寸，`effect="dark"`(灰色) |

**状态徽标 (StrategyStatusBadge) 规格：**

```
● 运行中    ← 绿色圆点 + 文字
● 已暂停    ← 黄色圆点 + 文字
● 已停止    ← 红色圆点 + 文字
○ 草稿     ← 灰色圆点 + 文字
```

| 元素 | 规格 |
|------|------|
| 圆点 | 8px 直径圆，与文字间距 `6px` |
| 文字 | 13px，字重 500 |
| 运行中 | 圆点 `--color-success`，文字 `--color-success` |
| 已暂停 | 圆点 `--color-warning`，文字 `--color-warning` |
| 已停止 | 圆点 `--color-sell`，文字 `--color-sell` |
| 草稿 | 圆点 `--color-text-tertiary`，文字 `--color-text-tertiary` |

**操作菜单 (Row Actions)：**
- 组件：`el-dropdown` 以 `⋮` 按钮为 trigger
- 按钮规格：`28x28px`，圆角 `6px`，`⋮` 图标
- 默认背景：透明
- 悬停背景：`rgba(255,255,255,0.04)`

**下拉菜单项（按状态动态显示 v2.0）：**

| 策略状态 | 显示的菜单项 |
|----------|------------|
| draft | 编辑、删除 |
| active | 暂停、停止、编辑、克隆、删除 |
| paused | 启用、停止、编辑、克隆、删除 |
| stopped | 克隆、删除 |

每个菜单项规格：
- 高度：`32px`
- 内边距：`0 12px`
- 图标：左侧 14px 图标，图标文字间距 `8px`
- 删除项颜色：`--color-error`

### 2.6 分页 (Pagination)

- 组件：`el-pagination`
- 布局：`prev, pager, next, total, sizes`
- 每页显示：`20` 条（可选 10/20/50）
- 背景：透明
- 对齐：居中
- 上边距：`16px`

### 2.7 状态矩阵

#### 正常状态（有策略数据）
表格完整显示所有数据行，分页正常。

#### 加载状态 (Table Loading)
```
+--------------------------------------------------------------+
|  策略管理                                    [+ 创建策略]      |
|  [全部] [运行中] [已暂停] [草稿] [已停止]                      |
|                                                               |
|  +----------------------------------------------------------+ |
|  |  □  ████████████  ████  ███  ████████████        ███  ██ | |  ← Skeleton rows
|  |  □  ████████████  ████  ███  ████████████        ███  ██ | |     × 5
|  |  □  ████████████  ████  ███  ████████████        ███  ██ | |
|  |  □  ████████████  ████  ███  ████████████        ███  ██ | |
|  |  □  ████████████  ████  ███  ████████████        ███  ██ | |
|  +----------------------------------------------------------+ |
+--------------------------------------------------------------+
```

- `el-table` 使用 `v-loading` 指令
- 骨架屏行数：5 行
- 骨架动画：`shimmer`

#### 空状态 (EmptyStrategies)
```
+--------------------------------------------------------------+
|  策略管理                                    [+ 创建策略]      |
|                                                               |
|  +----------------------------------------------------------+ |
|  |                                                           | |
|  |              [🧠 图标，48px, --color-text-tertiary]        | |
|  |                                                           | |
|  |              还没有创建策略                                 | |
|  |              创建第一个策略开始自动化交易                    | |
|  |                                                           | |
|  |              [  [+ 创建第一个策略]  ] ← primary button     | |
|  |                                                           | |
|  +----------------------------------------------------------+ |
+--------------------------------------------------------------+
```

#### 过滤为空状态
- 文案："没有匹配的策略"
- 副文案："尝试调整筛选条件或搜索关键词"
- CTA："清除筛选" text 按钮

#### 加载失败状态
- 使用 `el-result` icon="error"
- CTA 按钮：`[重新加载]` primary

---

## 3. 页面2：创建/编辑策略页 /strategies/new 和 /strategies/:id/edit

### 3.1 布局结构（两步向导）

```
+--------------------------------------------------------------+
|  ← 返回策略列表                        创建策略 - 步骤 1/2      |
|                                                               |
|  步骤指示器:  ○═══════○═══════○                                |
|          ① 选择模板  ② 配置参数  ③ 摘要完成                     |
|                                                               |
|  +------------------------------+----------------------------+ |
|  | [步骤内容区域 - 动态切换]      | [步骤内容区域]               | |
|  |  选择模板卡片                  |  实时参数摘要                 | |
|  |                               |                              | |
|  +------------------------------+----------------------------+ |
|                                                               |
|  [上一步]                              [下一步] [保存草稿]      |
+--------------------------------------------------------------+
```

**页面规格：**
- 最大宽度：`960px`
- 居中对齐
- 页面内边距：`24px`

### 3.2 步骤指示器 (StepIndicator)

```
  ●═══════○═══════○
  ① 选择模板  ② 配置参数  ③ 摘要完成
```

| 元素 | 规格 |
|------|------|
| 步骤圆点 | 圆形，直径 `24px` |
| 已完成步骤 | 背景 `--color-accent`，白色数字，✔ 图标 |
| 当前步骤 | 圆圈 `2px` 实线 `--color-accent`，数字 `--color-accent` |
| 未完成步骤 | 圆圈 `2px` 实线 `--color-border`，数字 `--color-text-tertiary` |
| 步骤间连线 | 水平线，`2px` 高，`64px` 宽 |
| 已完成连线 | 背景 `--color-accent` |
| 未完成连线 | 背景 `--color-border` |
| 步骤标签 | 13px，字重 500；完成/当前色 `--color-text-primary`，未完成 `--color-text-tertiary` |

### 3.3 步骤1：选择模板 (Template Selection)

**布局：**
```
+--------------------------------------------------------------+
|  选择策略模板                                                   |
|  选择一个预设模板开始配置，或从空白创建                          |
|                                                               |
|  +----------+ +----------+ +----------+ +----------+          |
|  | 📈       | | 📉       | | 🔲       | | 🔄       |          |
|  | 趋势跟踪  | | 均值回归  | | 网格交易  | | 套利     |          |
|  | 均线交叉  | | RSI回归   | | 等差网格  | | 跨所套利 |          |
|  | ...       | | ...       | | ...       | | ...      |          |
|  | [趋势标签] | | [回归标签] | | [网格标签] | | [套利标签] |      |
|  +----------+ +----------+ +----------+ +----------+          |
|                                                               |
|  +----------+                                                 |
|  | ✏️       |                                                 |
|  | 自定义   |                                                 |
|  | 空白模板  |                                                 |
|  | ...      |                                                 |
|  | [自定义]  |                                                 |
|  +----------+                                                 |
+--------------------------------------------------------------+
```

**模板卡片 (TemplateCard) 规格：**
- 尺寸：`calc(25% - 12px)`（四列网格，间距 `16px`）
- 最小宽度：`200px`
- 背景：`--color-surface`
- 圆角：`8px`
- 边框：`1px solid --color-border`
- 内边距：`20px`
- 高度：`180px`
- 光标：`pointer`

**卡片内容：**

| 区域 | 规格 |
|------|------|
| 图标 | 24px，色 `--color-accent`，上边距 0 |
| 模板名称 | 16px，字重 600，色 `--color-text-primary`，间距上 12px |
| 简短描述 | 13px，色 `--color-text-tertiary`，2行省略 |
| 模板类型标签 | `el-tag` size="small"，在卡片底部 |

**卡片状态：**

| 状态 | 视觉表现 |
|------|---------|
| 默认 | 标准 surface 背景 + border 边框 |
| 悬停 | 边框色 `--color-border-hover`，轻微上移 `-2px`，阴影 `0 4px 12px rgba(0,0,0,0.2)` |
| 选中 | 边框 `2px solid --color-accent`，背景 `rgba(113,112,255,0.04)` |
| 加载中 | Skeleton 卡片 |
| 加载失败 | 卡片显示"模板加载失败" + 重试文字按钮 |

### 3.4 步骤2：配置参数 (Parameter Configuration)

**v2.0 布局（含 symbol/timeframe）：**
```
+--------------------------------------------------------------+
|  配置参数 - 趋势跟踪                                           |
|  配置所选模板的运行参数                                        |
|                                                               |
|  +----------------------------------------------------------+ |
|  | 基本设置                                    [展开/折叠]    | |
|  |                                                            | |
|  | 策略名称  [________________________]                        | |
|  | 策略描述  [________________________]                        | |
|  | 交易对    [BTC/USDT ▼]              ]  ← 必填（ADR D1）    | |
|  | 时间周期  [1H ▼]                    ]  ← 必填（ADR D1）    | |
|  +----------------------------------------------------------+ |
|                                                               |
|  +----------------------------------------------------------+ |
|  | 入场条件                                    [展开/折叠]    | |
|  |                                                            | |
|  | 快线周期  [12] ──●━━━━━━━━━━━━○━━ 50   (el-slider)         | |
|  |  移动平均线快线周期，较短周期敏感度高                        | |
|  |                                                            | |
|  | 慢线周期  [26] ──●━━━━━━━━━━━━○━━ 100  (el-slider)         | |
|  |  移动平均线慢线周期，较长周期趋势稳定                        | |
|  |                                                            | |
|  | 均线类型  [SMA ▼]                           (el-select)      | |
|  |                                                            | |
|  +----------------------------------------------------------+ |
|                                                               |
|  +----------------------------------------------------------+ |
|  | 出场条件                                    [展开/折叠]    | |
|  |                                                            | |
|  | 止盈比例  [ 2.5 ] %   1.0 ─────○━━━━ 10.0  (el-slider)    | |
|  | 止损比例  [ 5.0 ] %   1.0 ──○━━━━━━ 10.0  (el-slider)     | |
|  |                                                            | |
|  | 追踪止损  [○ 关闭  ● 开启]                (el-switch)       | |
|  |  开启后止损会随盈利方向移动                                   | |
|  |                                                            | |
|  | 追踪距离  [ 1.0 ] %                       (el-input-number) | |
|  +----------------------------------------------------------+ |
|                                                               |
|  +----------------------------------------------------------+ |
|  | 📋 策略摘要                                                | |
|  |  · 交易 BTC/USDT，1H 周期                                   | |  ← v2.0 新增
|  |  · 快线 12 / 慢线 26 SMA 交叉                               | |
|  |  · 止盈 2.5% / 止损 5.0%                                    | |
|  |  · 追踪止损已开启（距离 1.0%）                               | |
|  |  · 预计算力：低（约 2ms/计算）                               | |
|  +----------------------------------------------------------+ |
|                                                               |
|  [← 返回选择模板]                          [保存草稿] [保存并启动] |
+--------------------------------------------------------------+
```

#### 交易对+周期选择器 (SymbolTimeframeSelector)

**交易对下拉 (SymbolSelector)：**
- 组件：`el-select`
- 宽度：`160px`
- 选项：BTC/USDT, ETH/USDT, BNB/USDT, SOL/USDT, XRP/USDT 等（从预置列表选择）
- 必填标识：左侧红色星号
- 支持搜索过滤

**时间周期下拉 (TimeframeSelector)：**
- 组件：`el-select`
- 宽度：`120px`
- 选项：1m, 5m, 15m, 30m, 1H, 4H, 1D, 1W
- 必填标识：左侧红色星号

**字段只读性（ADR D3）：**
- symbol 和 timeframe 在创建后 **不可编辑**
- 编辑模式时：两个下拉框 `disabled`，显示当前值

#### 参数分组 (ParamFormGroup)

**分组标题：**
- 高度：`40px`
- 分组名：`14px`，字重 600，色 `--color-text-primary`
- 右侧折叠/展开按钮：`+`/`-` 图标，`20px`，色 `--color-text-tertiary`
- 默认：全部展开
- 切换动画：高度过渡 0.2s

#### 各控件规格

**数值参数 (el-slider)：**
- 组件：`el-slider` 紧凑模式
- 对齐：滑块居中，左右显示 min/max 值 `12px` 灰色
- 显示当前值 tooltip（悬停时）
- 同步显示 el-input-number 侧边（靠右）
- 拉杆色：`--color-accent`

**数值参数 (el-input-number)：**
- 组件：`el-input-number` size="small"
- 宽度：`120px`
- 后缀单位标签（如 `%`，`x`）

**选项参数 (el-select)：**
- 组件：`el-select` size="default"
- 最小宽度：`200px`

**开关参数 (el-switch)：**
- 组件：`el-switch`
- 激活色：`--color-accent`
- 关闭色：`#62666d`

#### 策略摘要面板 (ParamPreview)

**规格：**
- 背景：`--color-surface-elevated`
- 圆角：`8px`
- 边框：`1px solid --color-border`
- 内边距：`16px`

**v2.0 新增摘要项：**
- 交易对和周期显示在摘要第一行：`· 交易 BTC/USDT，1H 周期`

### 3.5 底部操作栏

**布局：**
- 固定底部，上边框 `1px solid --color-border`
- 背景：`--color-bg`
- 内边距：`16px 0`
- 左侧：`[← 返回选择模板]` link/text 按钮
- 右侧：`[保存草稿]` default 按钮 + `[保存并启动]` primary 按钮

**按钮规格：**

| 按钮 | 组件 | 规格 |
|------|------|------|
| ← 返回选择模板 | `el-button` text | 图标 `ArrowLeft`，色 `--color-text-tertiary` |
| 保存草稿 | `el-button` | type="default"，高度 `36px` |
| 保存并启动 | `el-button` | type="primary"，高度 `36px`，图标 `VideoPlay`（仅 active 状态时可用） |

**保存并启动按钮状态（ADR D4 状态机）：**
- draft → active：可用
- paused → active：可用
- 编辑已有策略：按钮文字变为"保存"（状态不变）
- stopped 状态：按钮隐藏

### 3.6 表单验证状态

| 状态 | 触发条件 | 视觉表现 |
|------|---------|---------|
| **默认** | 页面加载 | 字段空白或预设值 |
| **编辑模式** | /strategies/:id/edit | 自动填充，symbol/timeframe disabled |
| **必填缺失** | 必填字段为空 | 红色边框 + 红色提示文字 `12px` |
| **格式错误** | 数值超范围 | 红色边框 + "值应在 X-Y 之间" |
| **保存中** | 提交请求 | 按钮显示 spinner，"保存中..."，表单 disabled |
| **保存成功** | API 返回 200 | Toast "策略保存成功" + 返回列表 |
| **保存失败** | API 异常 | Toast 红色 + 错误信息 |
| **名称重复** | 已存在同名 | 表单级错误提示"策略名称已存在" |
| **草稿自动保存** | 编辑中每30秒 | 右上角 toast "草稿已保存" |

### 3.7 状态转换逻辑（ADR D4 前端实现）

**状态徽标点击/操作菜单状态转换规则：**

| 当前状态 | 允许的目标状态 | 操作按钮/菜单项 |
|---------|--------------|---------------|
| draft | active（启用） | 操作菜单"启用" |
| active | paused（暂停）、stopped（停止） | 操作菜单"暂停"/"停止" |
| paused | active（启用）、stopped（停止） | 操作菜单"启用"/"停止" |
| stopped | 无（不可直接启用） | 仅显示"克隆"、"删除" |

**非法转换的 UX 处理：**
- stopped 策略启用需通过克隆新策略实现
- 编辑/删除在 stopped 状态下有条件限制（见下拉菜单）

### 3.8 编辑模式特殊逻辑

**编辑时 symbol/timeframe 只读（ADR D3）：**
- 两个选择器显示当前值（disabled 样式）
- 右侧显示文字标签：`创建后不可更改`
- hover 显示 tooltip 提示

---

## 4. 页面3：策略模板市场 /strategies/templates

### 4.1 布局结构

```
+--------------------------------------------------------------+
|  策略模板市场                                [我的模板] [创建模板]| ← PageHeader
|  浏览社区模板，发现更多交易策略                               |
|                                                               |
|  [全部] [趋势跟踪] [均值回归] [网格交易] [套利] [自定义]   [搜索] | ← Category Filter
|                                                               |
|  排序: [热门 ▼]   显示: [网格 ▼] [列表 ▼]                    | ← Sort & View Toggle
|                                                               |
|  +----------+ +----------+ +----------+ +----------+          |
|  | 🏆       | | 📈       | | 🔲       | | ⚡       |          |
|  | 双均线交叉| | RSI 回归  | | 无限网格  | | 三角套利  |          |
|  | 作者: Al | | 作者: Bo  | | 作者: Ch  | | 作者: Di  |          |
|  | ⭐4.8  🔥123 | ⭐4.5  🔥89 | ⭐4.9  🔥200 | ⭐4.2  🔥45 |          |
|  | [趋势标签] | | [回归标签] | | [网格标签] | | [套利标签] |      |
|  | [立即使用] | | [立即使用] | | [立即使用] | | [立即使用] |      |
|  +----------+ +----------+ +----------+ +----------+          |
|                                                               |
|  +----------+ +----------+ +----------+ +----------+          |
|  | ...      | | ...      | | ...      | | ...      |          |
|  +----------+ +----------+ +----------+ +----------+          |
|                                                               |
|  [← 上一页]  1  2  3 ...  [下一页 →]       共 156 个模板      |
+--------------------------------------------------------------+
```

**页面规格：**
- 最大宽度：`1344px`
- 内边距：`24px`

### 4.2 模板市场卡片 (TemplateMarketplaceCard)

**卡片规格：**
- 尺寸：`calc(25% - 12px)`（四列网格，间距 `16px`）
- 最小宽度：`240px`
- 背景：`--color-surface`
- 圆角：`8px`
- 边框：`1px solid --color-border`
- 内边距：`20px`
- 高度：`auto`（最小 200px）

**卡片内容：**

| 区域 | 规格 |
|------|------|
| 排名/徽章 | 左上角 `#1` / `HOT` 等徽章，色 `--color-warning` |
| 图标 | 32px，色 `--color-accent` |
| 模板名称 | 16px，字重 600，色 `--color-text-primary` |
| 作者 | 13px，色 `--color-text-tertiary` |
| 评分 | `⭐ 4.8` 格式，13px |
| 使用量 | `🔥 123 人使用` 格式，13px，色 `--color-text-tertiary` |
| 模板类型标签 | `el-tag` size="small" |
| CTA 按钮 | `[立即使用]` el-button type="primary" size="small" |

**卡片状态：**

| 状态 | 视觉表现 |
|------|---------|
| 默认 | 标准 surface 背景 |
| 悬停 | 边框色 `--color-border-hover`，`-2px` 上浮，阴影 |
| 已安装 | 按钮变为 `[已安装 ✓]` 灰色 disabled 按钮 |
| 加载中 | Skeleton |

### 4.3 模板详情侧滑面板 (TemplateDetailPanel)

**触发：** 点击卡片或"查看详情"按钮

**布局：** 右侧 `el-drawer`，宽度 `480px`

**内容：**
```
+----------------------------------+
|  ×                               |
|                                   |
|  📈  双均线交叉策略                  |
|  作者: AlgoTrader_001             |
|  ⭐ 4.8 (256 次评分)               |
|                                   |
|  [趋势跟踪标签] [官方认证标签]        |
|                                   |
|  --- 描述 ---                     |
|  基于快速/慢速均线交叉的经典趋势跟踪... |
|                                   |
|  --- 策略参数 ---                  |
|  · 快线周期: 12                   |
|  · 慢线周期: 26                   |
|  · 均线类型: SMA                  |
|  · 止盈比例: 2.5%                 |
|  · 止损比例: 5.0%                 |
|                                   |
|  --- 预计回测收益 ---              |
|  年化收益: 45.2%                   |
|  夏普比率: 1.85                    |
|  最大回撤: 12.3%                   |
|                                   |
|  [查看详细回测报告 →]               |
|                                   |
|  [立即使用此模板创建策略]            | ← Primary CTA
+----------------------------------+
```

### 4.4 我的模板页 (/strategies/templates/mine)

**切换方式：** 顶部 Tab 或 URL 路由

**布局与列表页类似，增加：**

| 区域 | 内容 |
|------|------|
| 页面标题 | "我的模板" |
| 操作栏 | `[+ 创建模板]` 按钮 |
| 模板列表 | 用户自己创建的模板 |
| 状态标签 | 草稿/已发布 |

**创建/编辑模板表单：**
- 模板名称（必填）
- 模板描述（富文本）
- 模板分类（下拉选择）
- 参数 Schema 定义（可视化 JSON 编辑器）
- 设为公开/私有开关

---

## 5. 组件状态总矩阵

### 5.1 StrategyStatusBadge

| 状态 | 触发条件 | 圆点 | 文字 |
|------|---------|------|------|
| 默认 | 正常 | 对应状态色 | 对应状态名 |
| 未知状态 | API 返回未知值 | `--color-text-tertiary` | "未知" |

### 5.2 TemplateCard

| 状态 | 触发条件 | 视觉表现 |
|------|---------|---------|
| 默认 | 页面加载 | 标准 surface 卡片 |
| 悬停 | 鼠标移入 | border 高亮 + 上浮 2px |
| 选中 | 点击选中 | accent 边框 + 浅色背景 |
| 加载中 | 模板 API 请求中 | Skeleton 动画 |
| 加载失败 | 异常 | 错误文案 + 重试 |
| disabled | 条件不满足 | 透明度 0.5，禁止点击 |

### 5.3 TemplateMarketplaceCard

| 状态 | 触发条件 | 视觉表现 |
|------|---------|---------|
| 默认 | 页面加载 | 标准卡片 |
| 悬停 | 鼠标移入 | border 高亮 + 上浮 2px |
| 已安装 | 用户已安装 | `[已安装 ✓]` disabled 按钮 |
| 加载中 | 请求中 | Skeleton |

### 5.4 SymbolTimeframeSelector

| 状态 | 触发条件 | 视觉表现 |
|------|---------|---------|
| 默认 | 正常 | 标准下拉 |
| 聚焦 | 点击展开 | 边框 accent 色 |
| 选中有值 | 已选 | 显示选中值 |
| 错误 | 未选必填项 | 红色边框 + 提示 |
| 只读（编辑模式） | ADR D3 | disabled 样式 + tooltip |

### 5.5 BulkActionBar

| 状态 | 触发条件 | 视觉表现 |
|------|---------|---------|
| 隐藏 | 无选中项 | `display: none` |
| 显示 | ≥1 项选中 | 浮现动画，从上方滑入 |
| 删除确认 | 点击删除 | 弹出确认 Modal |

### 5.6 StepIndicator

| 状态 | 视觉 |
|------|------|
| 已完成步骤 | 实心 accent 圆 + ✔ |
| 当前步骤 | 空心 accent 圆 + 数字 |
| 未来步骤 | 空心灰色圆 + 数字 |
| 连线已完成 | accent 色实线 |
| 连线未完成 | 灰色虚线 |

### 5.7 删除确认模态框

```
+--------------------------------------------------+
|  删除策略                                          |
|  确定要删除策略「Grid BTC」吗？                      |
|  删除后策略配置和数据将不可恢复。                     |
|                                                    |
|          [取消]           [确认删除]                 |
+--------------------------------------------------+
```

| 元素 | 规格 |
|------|------|
| 标题 | "删除策略"，16px，字重 600 |
| 正文 | "确定要删除策略「{name}」吗？删除后将不可恢复"，14px |
| 取消按钮 | `el-button` type="default" |
| 确认按钮 | `el-button` type="danger" |
| 加载中 | 确认按钮显示 spinner，"删除中..." |
| 宽度 | `420px` |

### 5.8 状态转换确认模态框

**暂停确认：**
```
+--------------------------------------------------+
|  暂停策略                                          |
|  确定要暂停策略「Grid BTC」吗？                      |
|  暂停后策略将停止交易，但配置保留。                   |
|                                                    |
|          [取消]           [确认暂停]                 |
+--------------------------------------------------+
```

**停止确认：**
```
+--------------------------------------------------+
|  停止策略                                          |
|  确定要停止策略「Grid BTC」吗？                      |
|  停止后将无法恢复运行，只能克隆或删除。                |
|                                                    |
|          [取消]           [确认停止]                 |
+--------------------------------------------------+
```

### 5.9 ParamSlider

| 状态 | 触发条件 | 视觉表现 |
|------|---------|---------|
| 默认 | 正常 | 标准滑块 |
| 悬停 | 鼠标悬停拉杆 | 拉杆放大 1.1x |
| 拖动 | 正在拖动 | tooltip 显示值 |
| 错误 | 值超范围 | 拉杆下方红色提示 |

### 5.10 ParamSwitch

| 状态 | 触发条件 | 视觉表现 |
|------|---------|---------|
| 关闭 | 默认 | 灰色 |
| 开启 | 用户开启 | accent 色 |
| disabled | 条件依赖 | 透明度 0.5 |

---

## 6. 路由设计

| 路由 | 组件 | 描述 |
|------|------|------|
| `/strategies` | StrategiesView | 策略列表页 |
| `/strategies/new` | StrategyFormView | 创建新策略（两步向导） |
| `/strategies/:id/edit` | StrategyFormView | 编辑已有策略（两步向导，预填） |
| `/strategies/templates` | TemplateMarketplaceView | 模板市场 |
| `/strategies/templates/mine` | MyTemplatesView | 我的模板 |

路由配置示例：
```typescript
{
  path: 'strategies',
  name: 'Strategies',
  component: StrategiesView,
  meta: { requiresAuth: true, title: '策略' },
  children: [
    { path: 'new', name: 'StrategyNew', component: StrategyFormView },
    { path: ':id/edit', name: 'StrategyEdit', component: StrategyFormView },
    { path: 'templates', name: 'TemplateMarketplace', component: TemplateMarketplaceView },
    { path: 'templates/mine', name: 'MyTemplates', component: MyTemplatesView },
  ]
}
```

---

## 7. API 数据流（已对齐 ADR）

### 7.1 策略列表

```
GET /api/v1/strategies?status=active&search=btc&page=1&page_size=20&sort=created_at&order=desc

Response:
{
  "data": [
    {
      "id": "uuid-1",
      "user_id": "uuid-user",
      "name": "Grid BTC",
      "description": "网格策略",
      "template_id": "uuid-template",    ← ADR D6: UUID 引用
      "parameters": {
        "layers": 20,
        "spread": 1.0,
        "quantity": 0.01
      },
      "symbol": "BTCUSDT",               ← ADR D2: 新增
      "timeframe": "1h",                 ← ADR D2: 新增
      "status": "active",                ← ADR D5: 状态机
      "created_at": "2026-05-01T10:00:00Z",
      "updated_at": "2026-05-12T14:30:00Z"
    }
  ],
  "total": 24,
  "page": 1,
  "page_size": 20
}
```

### 7.2 模板列表

```
GET /api/v1/strategies/templates

Response:
{
  "data": [
    {
      "id": "uuid-template",
      "name": "趋势跟踪",
      "description": "基于移动平均线交叉的趋势跟踪策略",
      "category": "trend",
      "default_parameters": {},
      "parameter_schema": [
        {
          "name": "fast_period",
          "label": "快线周期",
          "description": "移动平均线快线周期",
          "type": "number",              ← ADR D6: B3 对齐
          "default": 12,
          "min": 2,
          "max": 50
        },
        {
          "name": "slow_period",
          "label": "慢线周期",
          "type": "number",
          "default": 26,
          "min": 5,
          "max": 200
        }
      ]
    }
  ]
}
```

### 7.3 策略创建（ADR D1）

```
POST /api/v1/strategies

Body:
{
  "name": "Grid BTC",
  "description": "网格做市策略",
  "template_id": "uuid-template",       ← ADR D6: UUID
  "symbol": "BTCUSDT",                   ← ADR D1: 必填
  "timeframe": "1h",                     ← ADR D1: 必填
  "parameters": {
    "layers": 20,
    "spread": 1.0
  }
}

Response: StrategyResponse (包含 symbol/timeframe)
```

### 7.4 策略更新（ADR D3）

```
PUT /api/v1/strategies/:id

Body:
{
  "name": "Grid BTC v2",
  "description": "更新描述",
  "parameters": { ... }
}
注意：symbol 和 timeframe 不可更改，API 会忽略或拒绝
```

### 7.5 策略状态操作（ADR D4/D5）

```
PATCH /api/v1/strategies/:id/status
Body: { "status": "active" | "paused" | "stopped" }

状态机校验（service 层）:
- draft → active: ✅ 允许
- active → paused: ✅ 允许
- active → stopped: ✅ 允许
- paused → active: ✅ 允许
- paused → stopped: ✅ 允许
- stopped → active/paused: ❌ 拒绝 (42201 ERR_STRATEGY_INVALID_TRANSITION)
- draft → paused/stopped: ❌ 拒绝
```

### 7.6 模板市场 API（Phase 2）

```
GET /api/v1/strategy-templates              ← 公开模板列表
GET /api/v1/strategy-templates/:id           ← 模板详情 (P1)
POST /api/v1/strategy-templates              ← 创建自定义模板 (P1)
PUT /api/v1/strategy-templates/:id           ← 更新模板 (P2)
DELETE /api/v1/strategy-templates/:id        ← 删除模板 (P2)
```

### 7.7 导入导出 API（Phase 2）

```
POST /api/v1/strategies/:id/export          ← 导出策略 JSON
POST /api/v1/strategies/import              ← 导入策略 JSON
```

---

## 8. 响应式适配方案

### 8.1 桌面端（≥1024px）
- 策略列表：全宽表格，7列（含 checkbox/symbol/timeframe）
- 选择模板：4列网格
- 配置参数：左右分栏（参数表单 + 摘要面板）
- 模板市场：4列网格
- 页面内边距：`24px`

### 8.2 平板端（768px ~ 1023px）
- 策略列表：表格保持，描述列或模板列宽度缩减
- 选择模板：3列网格
- 配置参数：单列（摘要面板在底部）
- 模板市场：3列网格
- 页面内边距：`16px`

### 8.3 移动端（<768px）
- 策略列表：切换为卡片列表
  - 卡片规格：全宽，`--color-surface` 背景，`8px` 圆角，`16px` 内边距
  - 每张卡片显示：名称、交易对、周期、状态徽标、关键参数摘要
  - 操作按钮：底部操作按钮组
  - 筛选栏：横向滚动（可滑动）
  - 复选框列隐藏（移动端批量操作改为长按触发）
- 选择模板：2列网格
- 配置参数：全屏表单，摘要折叠在底部
- 模板市场：2列网格
- 页面内边距：`16px`
- 底部操作栏：固定底部

### 8.4 断点总结

| 断点 | 列表布局 | 模板网格 | 配置布局 | 市场网格 |
|------|---------|---------|---------|---------|
| ≥1024px | 7列表格 | 4列 | 左右分栏 | 4列 |
| 768~1023px | 7列表格（列宽缩减） | 3列 | 单列（摘要置底） | 3列 |
| <768px | 卡片列表 | 2列 | 全屏表单 + 固定底部 | 2列 |

---

## 9. TypeScript 类型定义（v2.0 对齐 ADR）

```typescript
// 策略状态枚举（ADR D5）
export type StrategyStatus = 'active' | 'paused' | 'stopped' | 'draft'

// 模板分类（ADR D6）
export type TemplateCategory = 'trend' | 'mean-reversion' | 'grid' | 'arbitrage' | 'custom'

// 策略模板（ADR D6: template_id UUID）
export interface StrategyTemplate {
  id: string                    // UUID
  name: string
  description: string
  category: TemplateCategory
  default_parameters: Record<string, any>
  parameter_schema: StrategyParamDef[]
  estimated_cost?: string
}

// 参数定义（ADR B3 对齐: param_type → type via serde rename）
export interface StrategyParamDef {
  name: string
  label: string
  description?: string
  type: 'integer' | 'float' | 'select' | 'boolean' | 'string'  // 后端 serde rename = "type"
  default?: any
  min?: number
  max?: number
  options?: { label: string; value: string }[]
  required?: boolean
}

// 策略完整信息（ADR D2: symbol/timeframe 新增）
export interface StrategyFull {
  id: string                    // UUID
  user_id: string
  name: string
  description: string
  template_id: string           // ADR D6: UUID 引用
  parameters: Record<string, any>
  symbol: string                // ADR D1: 必填
  timeframe: string             // ADR D1: 必填
  status: StrategyStatus        // ADR D5: 枚举
  created_at: string
  updated_at: string
}

// 创建策略 Payload（ADR D1: symbol/timeframe/template_id 必填）
export interface CreateStrategyPayload {
  name: string
  description?: string
  template_id: string          // ADR D6: UUID
  symbol: string                // ADR D1: 必填
  timeframe: string             // ADR D1: 必填
  parameters: Record<string, any>
  status?: StrategyStatus       // 默认 draft
}

// 更新策略 Payload（ADR D3: symbol/timeframe 不可改）
export interface UpdateStrategyPayload {
  name?: string
  description?: string
  parameters?: Record<string, any>
  // 注意: symbol 和 timeframe 不可通过此接口修改
}

// 状态更新请求（ADR D4/D5: service 层状态机校验）
export interface UpdateStatusRequest {
  status: StrategyStatus
}
```

---

## 10. 与 v1.0 设计的关键差异（变更说明）

| 变更项 | v1.0 | v2.0 | 驱动 ADR |
|-------|------|------|---------|
| symbol 字段 | 缺失 | 必填，创建时选择 | ADR D1 |
| timeframe 字段 | 缺失 | 必填，创建时选择 | ADR D1 |
| template_type → template_id | String | UUID | ADR D6 |
| 状态枚举 | String（无约束） | StrategyStatus 枚举 | ADR D5 |
| 状态机校验 | 后端无校验 | service 层校验，非法转换返回 42201 | ADR D4 |
| 策略列表增加列 | 5列 | 7列（含 checkbox/s交易对/周期） | 本次需求 |
| 批量操作 | 缺失 | BulkActionBar | 本次需求 |
| 模板市场页 | 缺失 | 新增页面 | 本次需求 |
| 筛选 Pills | 5个选项 | 5个选项 + 搜索 + 排序 | 本次需求 |
| symbol/timeframe 只读性 | 无定义 | 编辑时 disabled | ADR D3 |
