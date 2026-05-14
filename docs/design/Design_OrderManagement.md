# 设计规格说明书：订单管理模块 UI 界面

> 基于 ADR-010-order-management 的完整 UI/UX 设计方案
> 设计师：Designer | 版本 v1.0 | 日期：2026-05-14
> 设计灵感：Linear.app 暗色模式设计系统 + 金融交易终端风格
> 框架：Element Plus + Vue 3 + TypeScript + ECharts
> 源代码库：/home/ssk/workspace/quant-trading
> ADR 关联：ADR-010 全覆盖（D1-D11）
> 参考设计：Design_TradingExecution.md, Design_Portfolio.md

---

## 目录

1. [设计系统继承](#1-设计系统继承)
2. [页面1：订单列表页 /orders](#2-页面1订单列表页-orders)
3. [组件：订单创建对话框 OrderCreateDialog](#3-组件订单创建对话框-ordercreatedialog)
4. [组件：订单详情侧滑栏 OrderDetailDrawer](#4-组件订单详情侧滑栏-orderdetaildrawer)
5. [组件：持仓面板 PositionPanel](#5-组件持仓面板-positionpanel)
6. [组件：成交记录 Tab TradeRecordTab](#6-组件成交记录-tab-traderecordtab)
7. [组件状态总矩阵](#7-组件状态总矩阵)
8. [路由设计](#8-路由设计)
9. [API 数据流](#9-api-数据流)
10. [WebSocket 实时更新机制](#10-websocket-实时更新机制)
11. [响应式适配方案](#11-响应式适配方案)
12. [风控与错误处理 UX](#12-风控与错误处理-ux)
13. [交互细节与动画](#13-交互细节与动画)
14. [空状态与错误处理 UX](#14-空状态与错误处理-ux)

---

## 1. 设计系统继承

### 1.1 完全继承 Design Token

本设计继承自 `Design_QuantTrading_UI.md`、`Design_TradingExecution.md` 与 `Design_Portfolio.md` 的所有定义。以下是关键 token：

|| Token | 暗色模式值 | 用途 ||
||-------|-----------|------|
|| `--color-bg` | `#08090a` | 最深层背景 |
|| `--color-surface` | `#191a1b` | 卡片/面板背景 |
|| `--color-surface-elevated` | `#212223` | 悬浮/下拉背景 |
|| `--color-deep-bg` | `#050607` | 图表深层背景 |
|| `--color-text-primary` | `#f7f8f8` | 主文字 |
|| `--color-text-secondary` | `#d0d6e0` | 次要文字 |
|| `--color-text-tertiary` | `#8a8f98` | 辅助/占位文字 |
|| `--color-accent` | `#7170ff` | 主色调 |
|| `--color-accent-hover` | `#8b8aff` | 主色调悬停 |
|| `--color-border` | `rgba(255,255,255,0.08)` | 边框 |
|| `--color-border-hover` | `rgba(255,255,255,0.16)` | 边框悬停 |
|| `--color-error` | `#e5484d` | 错误 |
|| `--color-success` | `#10b981` | 成功/上涨 |
|| `--color-warning` | `#f5a623` | 警告 |
|| `--color-info` | `#60a5fa` | 信息 |
|| `--color-buy` | `#67C23A` | 买入/多头/上涨 |
|| `--color-sell` | `#F56C6C` | 卖出/空头/下跌 |
|| `--color-buy-bg` | `rgba(103,194,58,0.12)` | 上涨背景 |
|| `--color-sell-bg` | `rgba(245,108,108,0.12)` | 下跌背景 |
|| `--color-frozen` | `#f5a623` | 保证金/冻结资金 |
|| `--color-pending` | `#f5a623` | 待成交状态色 |
|| `--color-partial` | `#60a5fa` | 部分成交状态色 |
|| `--color-filled` | `#67C23A` | 已成交状态色 |
|| `--color-cancelled` | `#8a8f98` | 已撤销状态色 |
|| `--color-expired` | `#8a8f98` | 已过期状态色 |
|| `--color-rejected` | `#e5484d` | 已拒绝状态色 |
|| `--color-paper-mode` | `#60a5fa` | 模拟交易标签色 |
|| `--color-live-mode` | `#e5484d` | 实盘交易标签色 |

字体：
- UI：`'Inter', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif`
- 等宽/数字：`'JetBrains Mono', 'SF Mono', Menlo, monospace`

间距：
- 基础间距单元：4px
- 面板内边距：16px
- 卡片内边距：12px
- 元素间距：8px / 12px / 16px / 24px

### 1.2 订单管理模块专用组件列表

| 组件 | 类型 | 描述 | ADR 关联 |
|------|------|------|---------|
| **OrderListView** | 容器 | 订单列表页容器 | D6 |
| **OrderListHeader** | 展示 | 页面标题 + 统计摘要 + 创建按钮 | D2/D6 |
| **OrderSummaryCards** | 容器 | 四大统计卡片行 | D2/D3 |
| **ActiveOrdersCard** | 展示 | 活跃委托数 + 保证金占用 | D2/D3 |
| **TodayFillsCard** | 展示 | 今日成交通知数 | D5 |
| **FrozenBalanceCard** | 展示 | 冻结保证金总额 | D3 |
| **AccountBalanceCard** | 展示 | 可用余额 | D3 |
| **OrderFilterBar** | 交互 | 筛选栏（状态/方向/类型/交易对/日期） | D6 |
| **OrderTable** | 展示 | 订单列表表格 | D2/D6 |
| **OrderRow** | 展示 | 订单行（含状态标签/操作按钮） | D2 |
| **StatusPill** | 展示 | 订单状态 Pill 标签 | D2 |
| **CancelButton** | 交互 | 撤单按钮 | D2/D3 |
| **CancelAllButton** | 交互 | 全部撤单按钮 | D2 |
| **OrderCreateDialog** | 交互 | 创建订单对话框（买卖表单） | D1/D3/D8/D9 |
| **SideToggle** | 交互 | 买入/卖出方向切换 | — |
| **OrderTypeSelect** | 交互 | 限价/市价/止损类型选择 | D1/D9 |
| **PriceInput** | 交互 | 价格输入 | D1/D8 |
| **QuantityInput** | 交互 | 数量输入 + 精度校验 | D8 |
| **QuantitySlider** | 交互 | 数量百分比快捷选择 | D3 |
| **AccountInfo** | 展示 | 可用余额/持仓提示 | D3 |
| **OrderSummary** | 展示 | 预估金额/手续费/冻结 | D3 |
| **OrderConfirmDialog** | 交互 | 提交确认对话框 + 风险提示 | D3/D8 |
| **OrderDetailDrawer** | 展示 | 订单详情侧滑栏 | D2/D6 |
| **FillRecords** | 展示 | 成交明细子表格 | D4/D7 |
| **PositionPanel** | 容器 | 持仓面板 | D3/D7 |
| **PositionSummary** | 展示 | 账户汇总条 | D3/D7 |
| **PositionsTable** | 展示 | 持仓列表表格 | D7 |
| **PositionRow** | 展示 | 持仓行（含平仓按钮） | D3/D7 |
| **ClosePositionDialog** | 交互 | 平仓确认对话框 | D3/D7 |
| **TradeRecordTab** | 容器 | 成交记录 Tab 页 | D4/D5 |
| **TradeRecordTable** | 展示 | 成交记录表格 | D4 |
| **TradeRecordRow** | 展示 | 成交记录行 | D4 |
| **TradeModeBadge** | 展示 | 交易模式标识（模拟/实盘） | — |
| **WsConnectionStatus** | 展示 | WS连接状态指示器 | D5 |
| **RiskAlertBar** | 展示 | 风控预警横幅 | D8 |
| **PriceDeviationWarning** | 交互 | 价格偏离风险提示 | D8 |
| **PaginationBar** | 交互 | 分页控件 | — |
| **EmptyOrderState** | 展示 | 空订单占位图 | — |
| **LoadingSkeleton** | 展示 | 加载骨架屏 | — |

### 1.3 数据类型（ADR D1-D11 对齐）

#### Order 类型

```typescript
export interface Order {
  order_id: string                          // PRD: i64 序列化为 string
  symbol: string                            // 交易对
  side: 'buy' | 'sell'                      // 方向
  order_type: 'limit' | 'market' | 'stop' | 'stop_limit'  // 委托类型 (D9)
  price: string | null                      // 限价单 Decimal 字符串，市价单 null
  stop_price: string | null                 // 止损触发价 (D9)
  quantity: string                          // Decimal 字符串
  filled_quantity: string                   // Decimal 字符串
  avg_fill_price: string | null             // Decimal 字符串
  status: 'pending' | 'partial_filled' | 'filled' | 'cancelled' | 'expired' | 'rejected'
  mode: 'paper' | 'live'                    // 交易模式
  fee: string                               // Decimal 字符串
  time_in_force: 'GTC' | 'IOC' | 'FOK'     // 有效期
  strategy_id: string | null                // 策略信号下单 (D10)
  created_at: string                        // ISO 8601
  updated_at: string                        // ISO 8601
}
```

#### Position 类型

```typescript
export interface Position {
  id: string
  symbol: string
  side: 'long' | 'short'
  quantity: string                          // Decimal
  available_quantity: string                // Decimal (扣除冻结)
  avg_entry_price: string                   // Decimal
  unrealized_pnl: string                    // Decimal
  realized_pnl: string                      // Decimal
  mode: 'paper' | 'live'
  created_at: string
  updated_at: string
}
```

#### Account 类型

```typescript
export interface Account {
  user_id: string
  balance: string                           // 可用余额
  frozen_balance: string                    // 冻结余额
  initial_balance: string                   // 初始资金
  total_pnl: string                         // 累计盈亏
  equity: string                            // 权益
  positions_count: number
  active_orders_count: number
}
```

#### Trade 类型

```typescript
export interface Trade {
  trade_id: string
  order_id: string
  symbol: string
  side: 'buy' | 'sell'
  price: string
  quantity: string
  fee: string
  is_maker: boolean
  created_at: string
}
```

#### CreateOrderRequest (D9 对齐)

```typescript
export interface CreateOrderRequest {
  symbol: string
  side: 'buy' | 'sell'
  order_type: 'limit' | 'market' | 'stop' | 'stop_limit'
  price?: string                            // 限价单/止损限价单必填
  stop_price?: string                       // 止损单必填 (D9)
  quantity: string
  time_in_force: 'GTC' | 'IOC' | 'FOK'
}
```

#### WS 消息类型 (D5 对齐)

```typescript
export interface OrderUpdateData {
  order_id: string
  symbol: string
  side: 'buy' | 'sell'
  order_type: 'limit' | 'market' | 'stop' | 'stop_limit'
  price: string | null
  quantity: string
  filled_quantity: string
  status: 'pending' | 'partial_filled' | 'filled' | 'cancelled' | 'expired' | 'rejected'
  updated_at: number
}

export interface TradeNotificationData {
  trade_id: string
  order_id: string
  symbol: string
  side: 'buy' | 'sell'
  price: string
  quantity: string
  fee: string
  timestamp: number
}

export interface PositionUpdateData {
  symbol: string
  side: 'long' | 'short'
  quantity: string
  avg_entry_price: string
  unrealized_pnl: string
}

export interface RiskAlertData {
  rule_type: string                         // daily_loss / single_loss / max_position
  severity: 'warn' | 'block'
  message: string
  details: Record<string, string>
}

export type TradeWsMessage =
  | { type: 'order_update'; data: OrderUpdateData; ts: number }
  | { type: 'fill'; data: TradeNotificationData; ts: number }
  | { type: 'position_update'; data: PositionUpdateData; ts: number }
  | { type: 'risk_alert'; data: RiskAlertData; ts: number }       // D8 风控预警
  | { type: 'subscribed'; channels: string[] }
  | { type: 'unsubscribed'; channels: string[] }
  | { type: 'error'; code: number; message: string }
```

### 1.4 状态颜色系统

#### 订单状态颜色

| 状态 | 前景色 | 背景色 | 标签样式 | 图标 |
|------|--------|--------|---------|------|
| pending（待成交） | `--color-pending` `#f5a623` | `rgba(245,166,35,0.12)` | 黄色 Pill | ⏱ |
| partial_filled（部分成交） | `--color-partial` `#60a5fa` | `rgba(96,165,250,0.12)` | 蓝色 Pill | ⟳ |
| filled（已成交） | `--color-filled` `#67C23A` | `rgba(103,194,58,0.12)` | 绿色 Pill | ✓ |
| cancelled（已撤销） | `--color-cancelled` `#8a8f98` | `rgba(138,143,152,0.08)` | 灰色 Pill | ✕ |
| expired（已过期） | `--color-expired` `#8a8f98` | `rgba(138,143,152,0.08)` | 灰色 Pill | ⏰ |
| rejected（已拒绝） | `--color-rejected` `#e5484d` | `rgba(229,72,77,0.12)` | 红色 Pill | ✕ |

#### 买卖方向颜色

| 方向 | 前景色 | 背景色 | 按钮色 |
|------|--------|--------|--------|
| 买入 (buy) | `--color-buy` `#67C23A` | `rgba(103,194,58,0.12)` | 绿色实心 |
| 卖出 (sell) | `--color-sell` `#F56C6C` | `rgba(245,108,108,0.12)` | 红色实心 |

#### 交易模式标识

| 模式 | 标签色 | 图标 | 位置 |
|------|--------|------|------|
| 模拟交易 (paper) | `--color-paper-mode` `#60a5fa` | 🧪 | Header 左侧 |
| 实盘交易 (live) | `--color-live-mode` `#e5484d` | 🔴 | Header 左侧 |

---

## 2. 页面1：订单列表页 /orders

> 核心页面，融合订单列表 + 创建对话框 + 详情侧滑栏 + 持仓面板 + 成交记录

### 2.1 整体布局

```
┌──────────────────────────────────────────────────────────────────┐
│  OrderListHeader                                                  │
│  [🧪模拟交易] 订单管理 · [WS ●]                  [+ 新建委托]    │
├──────────────────────────────────────────────────────────────────┤
│  OrderSummaryCards (4列均分)                                      │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐   │
│  │ 活跃委托   │ │ 今日成交   │ │ 冻结保证金 │ │ 可用余额   │   │
│  │ 3          │ │ 12         │ │ ¥4,950.00  │ │ ¥55,050.00 │   │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘   │
├──────────────────────────────────────────────────────────────────┤
│  [委托管理]  [持仓管理]  [成交记录]       Tab 切换                │
├──────────────────────────────────────────────────────────────────┤
│  OrderFilterBar                                                   │
│  [状态▼] [方向▼] [类型▼] [交易对🔍] [日期范围]     [全部撤单]    │
├──────────────────────────────────────────────────────────────────┤
│  OrderTable (全宽)                                                │
│  ┌───────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐   │
│  │时间   │交易对│方向  │类型  │价格  │数量  │已成交│状态  │操作│   │
│  ├───────┼──────┼──────┼──────┼──────┼──────┼──────┼──────┤   │
│  │06:05  │BTC   │买    │限价  │49500 │0.1   │0.05  │部分  │撤单│   │
│  │06:03  │ETH   │卖    │限价  │3200  │1.0   │0     │待成  │撤单│   │
│  │06:01  │BTC   │买    │市价  │—     │0.2   │0.2   │已成交│详情│   │
│  │05:58  │SOL   │买    │限价  │150   │10    │10    │已成交│详情│   │
│  └───────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘   │
│  [分页: < 1 2 3 >]                                                │
└──────────────────────────────────────────────────────────────────┘
```

### 2.2 OrderListHeader

| 属性 | 规格 |
|------|------|
| 高度 | 56px |
| 内边距 | 0 16px |
| 左侧 | TradeModeBadge + 页面标题「订单管理」字号 18px/font-weight 600 |
| 右侧 | WsConnectionStatus + 「+ 新建委托」按钮 |
| 背景 | `--color-bg` |
| 底部边框 | `--color-border` |

**TradeModeBadge**：
- 类型：16px 圆角 Pill，高度 24px，padding 0 8px
- 模拟：文字「🧪 模拟交易」，背景 `rgba(96,165,250,0.12)`，文字 `--color-paper-mode`
- 实盘：文字「🔴 实盘交易」，背景 `rgba(229,72,77,0.12)`，文字 `--color-live-mode`

**WsConnectionStatus**：
- 8px 圆点 + 文字（12px）
- 连接：绿色 `--color-success`，文字「已连接」
- 重连中：黄色 `--color-warning`，文字「重连中」，0.5s 闪烁
- 断开：灰色 `--color-text-tertiary`，文字「已断开」

**新建委托按钮**：
- 类型：`el-button type="primary"`
- 图标：`Plus` (Element Plus Icons)，左侧
- 文字：「新建委托」
- 高度：36px
- 圆角：8px
- 点击 → 打开 OrderCreateDialog

### 2.3 OrderSummaryCards — 四大统计卡片

布局：4 列均分，间距 16px，使用 `el-row :gutter="16"`

#### ActiveOrdersCard（活跃委托）

| 属性 | 规格 |
|------|------|
| 卡片 | `--color-surface`, border-radius 12px, padding 20px |
| 标签 | 「活跃委托」字号 13px, `--color-text-tertiary` |
| 主值 | `3` 字号 28px/font-weight 700, `--color-text-primary`, JetBrains Mono |
| 子值 | 「保证金 ¥4,950」字号 12px, `--color-frozen` |
| 图标 | 右上角 `List` 图标，24px，`--color-accent` 40% 透明 |
| 悬停 | border 从 `--color-border` → `--color-border-hover`，translate-y -2px + box-shadow |

#### TodayFillsCard（今日成交）

| 属性 | 规格 |
|------|------|
| 标签 | 「今日成交」字号 13px, `--color-text-tertiary` |
| 主值 | `12` 字号 28px/font-weight 700, `--color-filled` |
| 子值 | 「成交额 ¥58,400」字号 12px, `--color-text-secondary` |
| 图标 | 右上角 `CircleCheck` 图标，24px，`--color-filled` 40% 透明 |
| 悬停 | 同 ActiveOrdersCard |

#### FrozenBalanceCard（冻结保证金）

| 属性 | 规格 |
|------|------|
| 标签 | 「冻结保证金」字号 13px, `--color-text-tertiary` |
| 主值 | `¥4,950.00` 字号 28px/font-weight 700, `--color-frozen` |
| 子值 | 「占总权益 8.2%」字号 12px, `--color-text-tertiary` |
| 图标 | 右上角 `Lock` 图标，24px，`--color-frozen` 40% 透明 |
| 悬停 | 同上 |

#### AccountBalanceCard（可用余额）

| 属性 | 规格 |
|------|------|
| 标签 | 「可用余额」字号 13px, `--color-text-tertiary` |
| 主值 | `¥55,050.00` 字号 28px/font-weight 700, `--color-text-primary` |
| 子值 | 「权益 ¥60,000」字号 12px, `--color-text-secondary` |
| 图标 | 右上角 `Wallet` 图标，24px，`--color-accent` 40% 透明 |
| 悬停 | 同上 |

**数值格式化规则**：
- 金额：千分位分隔，保留 2 位小数，前缀 ¥
- 正数：前缀 `+`（盈亏/收益率相关）
- 零：无前缀，使用 `--color-neutral`

### 2.4 Tab 切换

```
┌──────────────────────────────────────────────────┐
│  [委托管理 (24)]  [持仓管理 (3)]  [成交记录 (156)] │
│  ─────────────────                               │
└──────────────────────────────────────────────────┘
```

| 属性 | 值 |
|------|------|
| 选中 | 下划线 `--color-accent` `#7170ff`，文字 `--color-text-primary` |
| 未选 | 文字 `--color-text-secondary` |
| 角标 | 红色圆形，对应数量，12px 字体 |
| 高度 | 40px |
| 间距 | 24px |

**交互**：
- 切换 Tab → 加载对应数据
- 数量实时更新（WS 推送时）
- 委托管理 Tab 显示订单列表
- 持仓管理 Tab 显示 PositionPanel
- 成交记录 Tab 显示 TradeRecordTab

### 2.5 OrderFilterBar — 筛选栏

| 属性 | 值 |
|------|------|
| 高度 | 自适应，约 44px |
| 内边距 | 8px 0 |
| 背景 | 透明 |
| 间距 | 元素间距 12px |

**筛选器组件**：

| 筛选器 | 类型 | 宽度 | 选项/说明 |
|--------|------|------|-----------|
| 状态 | `el-select` multiple | 180px | 全部/待成交/部分成交/已成交/已撤销/已过期/已拒绝 |
| 方向 | `el-select` | 120px | 全部/买入/卖出 |
| 类型 | `el-select` | 120px | 全部/限价/市价/止损(MVP禁用)/止损限价(MVP禁用) |
| 交易对 | `el-input` + Search 图标 | 160px | placeholder「搜索交易对」，debounce 300ms |
| 日期范围 | `el-date-picker type="daterange"` | 280px | 格式 YYYY-MM-DD，默认最近7天 |

**全部撤单按钮**：
- 位置：筛选栏最右侧
- 样式：ghost，`--color-error` 文字
- 悬停：`--color-error` 背景，白色文字
- 显示条件：有活跃委托时
- 禁用条件：无活跃委托

**清空筛选按钮**：
- 位置：筛选栏右侧（全部撤单按钮前）
- 样式：ghost，`--color-text-tertiary` 文字
- 显示条件：任一筛选器有值时
- 图标：`CircleClose`

### 2.6 OrderTable — 订单列表表格

| 属性 | 规格 |
|------|------|
| 容器 | `--color-surface`, border-radius 12px, padding 0 |
| 表格 | `el-table`，暗色模式，stripe |
| 行高 | 48px |
| 表头 | `--color-text-tertiary`, 字号 12px, font-weight 600 |

**列定义**：

| 列 | 宽度 | 对齐 | 内容 | 样式 |
|----|------|------|------|------|
| 时间 | 100px | 左 | `MM-DD HH:mm` | 等宽字体 12px, `--color-text-secondary` |
| 交易对 | 100px | 左 | `BTC/USDT` | 14px bold, `--color-text-primary` |
| 方向 | 60px | 中 | SideTag（买/卖） | 买入绿/卖出红 |
| 类型 | 80px | 中 | 限价/市价/止损 | 12px, 止损类型灰色斜体(MVP不可用) |
| 价格 | 120px | 右 | `49,500.00` 或 `市价` | 等宽字体，市价单显示「市价」灰色 |
| 数量 | 100px | 右 | `0.1000` | 等宽字体 |
| 已成交 | 100px | 右 | `0.0500 / 0.1000` | 等宽字体，进度条式 |
| 成交均价 | 120px | 右 | `49,500.00` | 等宽字体，未成交显示「—」 |
| 状态 | 100px | 中 | StatusPill | 见状态颜色系统 |
| 操作 | 100px | 中 | CancelButton / 详情链接 | — |

**已成交列 — 进度条式显示**：

```
┌──────────────────────────┐
│  0.0500 / 0.1000         │
│  ████████░░░░░░  50%     │
└──────────────────────────┘
```

- 上行：数字（等宽字体 12px）
- 下行：微型进度条
  - 高度 3px，宽度 60px
  - 背景色 `--color-border`
  - 填充色：买入 `--color-buy`，卖出 `--color-sell`
  - 百分比：右侧 12px `--color-text-tertiary`
- 已完全成交：填充 100%，无百分比文字
- 未成交（filled_quantity=0）：无进度条，显示「0 / 0.1000」

**SideTag 规格**：
- 买入：`el-tag type="success"` 文字「买」，`--color-buy` + `--color-buy-bg`
- 卖出：`el-tag type="danger"` 文字「卖」，`--color-sell` + `--color-sell-bg`
- 尺寸：small（高度 20px）

**行交互**：
- 悬停：行背景 `rgba(255,255,255,0.04)`
- 点击行（非按钮区域）→ 打开 OrderDetailDrawer
- 状态变化时：行背景闪烁 300ms

**排序**：
- 默认按 `created_at` 降序（最新在前）
- 可点击列头排序：时间、价格、数量、状态

**分页**：
- `el-pagination` small
- 每页 20 条
- 总数显示

### 2.7 CancelButton（撤单按钮）

| 状态 | 样式 |
|------|------|
| 默认 | ghost 样式，`--color-text-secondary` 文字，`--color-border` 边框 1px，文字「撤单」 |
| 悬停 | `--color-error` 边框 + `--color-error` 文字 |
| 禁用/隐藏 | status = filled / cancelled / expired / rejected 时隐藏 |
| 加载 | el-icon-loading 旋转，文字「撤单中...」 |

**点击流程**：
1. 弹出 CancelConfirmDialog
2. 确认 → 按钮变为 loading → 调用 POST /api/v1/orders/:id/cancel
3. 成功 → Toast 「委托已撤销」+ 委托状态更新 + 保证金释放动画
4. 失败(409) → Toast 「委托已成交，无法撤销」+ 刷新列表
5. 失败(其他) → RiskErrorAlert

**CancelConfirmDialog**：
```
┌──────────────────────────────────────┐
│  确认撤单                    [✕]    │
├──────────────────────────────────────┤
│                                      │
│  确认撤销委托 #123456？              │
│  BTC/USDT 买入 限价 49,500.00       │
│  数量: 0.1000 BTC                    │
│                                      │
│  将释放保证金 ≈ 4,950.00 USDT       │
│                                      │
│  [取消]              [确认撤单]      │
└──────────────────────────────────────┘
```

| 属性 | 值 |
|------|------|
| 宽度 | 380px |
| 遮罩 | rgba(0,0,0,0.6) |
| 圆角 | 12px |
| 背景 | `--color-surface-elevated` |
| 确认按钮 | `--color-error` 红色，白色文字 |
| 取消按钮 | ghost 样式 |
| 保证金释放 | `--color-frozen` 文字，显示释放金额 |

**CancelAllButton 点击流程**：
1. 弹出确认「确认撤销全部 N 个未成交委托？」
2. 确认 → 调用 POST /api/v1/orders/cancel-all
3. 成功 → Toast「成功撤销 M 个委托」
4. 部分失败 → Toast「成功撤销 M 个，N 个无法撤销」

---

## 3. 组件：订单创建对话框 OrderCreateDialog

> 从 /trade 页面的 OrderForm 演化，适配对话框模式

### 3.1 对话框布局

```
┌──────────────────────────────────────┐
│  新建委托                    [✕]    │
├──────────────────────────────────────┤
│                                      │
│  [   买入   ]  [   卖出   ]          │
│  ──────────                          │
│                                      │
│  [  限价  ]  [  市价  ]              │
│  ──────────                          │
│                                      │
│  交易对     [BTC/USDT       ▼]      │
│                                      │
│  价格       [49,500.00    ] USDT     │  ← 限价单显示
│             [↑↓]                     │
│                                      │
│  数量       [0.1000       ] BTC      │
│             ≈ 4,950.00 USDT          │
│  [25%] [50%] [75%] [100%]            │
│                                      │
│  ┌──────────────────────────────┐   │
│  │  可用余额: 55,050.00 USDT    │   │
│  │  冻结保证金: 4,950.00 USDT   │   │
│  └──────────────────────────────┘   │
│                                      │
│  ┌──────────────────────────────┐   │
│  │  预估金额: ≈ 4,950.00 USDT  │   │
│  │  手续费(≈): 4.95 USDT        │   │
│  │  合计冻结: ≈ 4,954.95 USDT  │   │
│  └──────────────────────────────┘   │
│                                      │
│  [取消]              [确认提交]      │
└──────────────────────────────────────┘
```

### 3.2 对话框规格

| 属性 | 值 |
|------|------|
| 宽度 | 440px |
| 遮罩 | rgba(0,0,0,0.6) |
| 圆角 | 12px |
| 背景 | `--color-surface-elevated` |
| 最大高度 | 90vh，内容溢出时内部滚动 |
| 打开动画 | slide-up + fade-in，300ms ease-out |
| 关闭动画 | slide-down + fade-out，200ms ease-in |

### 3.3 SideToggle（方向切换）

| 属性 | 买入 | 卖出 |
|------|------|------|
| 选中背景 | `--color-buy` `#67C23A` | `--color-sell` `#F56C6C` |
| 选中文字 | `#FFFFFF` | `#FFFFFF` |
| 未选背景 | `transparent` | `transparent` |
| 未选文字 | `--color-text-secondary` | `--color-text-secondary` |
| 悬停未选 | `rgba(103,194,58,0.08)` | `rgba(245,108,108,0.08)` |
| 高度 | 40px | 40px |
| 圆角 | 8px | 8px |

**交互**：
- 切换方向 → PriceInput 默认填入对手方最优价
- 切换方向 → AccountInfo 显示可用余额（买入）或可用持仓（卖出）
- 切换方向 → 表单校验重新触发

### 3.4 OrderTypeSelect（委托类型选择）

| 属性 | 限价 | 市价 | 止损(MVP禁) | 止损限价(MVP禁) |
|------|------|------|-------------|----------------|
| 选中 | 下划线 `--color-accent` | 下划线 `--color-accent` | 灰色+锁图标 | 灰色+锁图标 |
| 文字 | 14px | 14px | 12px, `--color-text-tertiary` | 12px, `--color-text-tertiary` |
| 高度 | 32px | 32px | 32px | 32px |

**MVP 限制**：
- 止损/止损限价选项显示为灰色+🔒图标
- 点击弹出提示「止损单功能将在下个版本上线」
- 与 ADR D9 对齐：MVP 阶段 OrderType 仅支持 limit + market

**交互**：
- 切换为市价 → PriceInput 隐藏（CSS transition slide-up），显示当前买一/卖一价参考
- 切换为限价 → PriceInput 显示，默认填入最优价
- 市价单显示提示文字：「市价单将按市场最优价成交，实际成交价可能有偏差」

### 3.5 交易对选择器

| 属性 | 值 |
|------|------|
| 类型 | `el-select` |
| 宽度 | 100% |
| 数据源 | GET /api/v1/symbols 返回的 symbolConfigs |
| 禁用选项 | `enabled = false` 的交易对 |
| 切换逻辑 | 更新 price_precision / quantity_precision / min_quantity / max_quantity |
| 搜索 | 支持搜索过滤 |

### 3.6 PriceInput（价格输入）

| 状态 | 样式 |
|------|------|
| 默认 | `--color-surface-elevated` 背景，`--color-border` 边框 1px |
| 聚焦 | `--color-accent` 边框 2px，box-shadow: 0 0 0 3px rgba(113,112,255,0.15) |
| 错误 | `--color-error` 边框 2px，下方显示错误信息 12px 红色 |
| 禁用 | opacity 0.4，cursor: not-allowed |

**校验规则**（实时，on blur + on input debounce 300ms）：

| 校验项 | 规则 | 错误码 | 错误信息 |
|--------|------|--------|---------|
| 必填 | 限价单必填 | — | 「请输入委托价格」 |
| 正数 | > 0 | 40005 | 「价格必须大于0」 |
| 精度 | ≤ price_precision 位小数 | 40005 | 「价格精度不超过 N 位小数」 |
| 偏离 > 10% | 价格偏离当前市价 | — | 弹出 PriceDeviationWarning |
| 偏离 > 100% | — | — | 「委托价格偏离当前市场价较大(100%)，是否确认提交？」 |

**输入增强**：
- 右侧显示交易对计价货币（USDT）
- 上下箭头按钮：步进 = 10^(-price_precision)
- 键盘 ↑↓ 箭头支持

### 3.7 QuantityInput（数量输入）

**尺寸**: 同 PriceInput，高度 40px

**校验规则**：

| 校验项 | 规则 | 错误码 | 错误信息 |
|--------|------|--------|---------|
| 必填 | 必填 | — | 「请输入委托数量」 |
| 正数 | > 0 | 40004 | 「数量必须大于0」 |
| 精度 | ≤ quantity_precision 位小数 | 40004 | 「数量精度不超过 N 位小数」 |
| 最小量 | ≥ min_quantity | 40006 | 「最小下单数量为 {min}」 |
| 最大量 | ≤ max_quantity | 40004 | 「最大下单数量为 {max}」 |
| 余额 | 买入: price × quantity ≤ balance | 40002 | 「账户余额不足」 |
| 持仓 | 卖出: quantity ≤ available_quantity | 40005 | 「可用持仓不足」 |
| 最小金额 | price × quantity ≥ min_notional | 40007 | 「最小下单金额为 {min}」 |

**输入增强**：
- 右侧显示基础货币（BTC）
- 下方显示 `≈ {amount} USDT`（预估金额，等宽字体）

### 3.8 QuantitySlider（数量百分比滑块）

```
┌──────────────────────────────────────────────┐
│  [25%]  [50%]  [75%]  [100%]                 │
└──────────────────────────────────────────────┘
```

| 属性 | 值 |
|------|------|
| 选项 | 25% / 50% / 75% / 100% |
| 按钮样式 | 4个等宽按钮，高度 28px，圆角 4px |
| 选中 | `--color-accent` 背景，白色文字 |
| 未选 | `--color-surface-elevated` 背景，`--color-text-secondary` 文字 |

**计算逻辑**：
- 买入 25% → quantity = (balance × 0.25 / price) 向下取整到 quantity_precision
- 卖出 25% → quantity = available_quantity × 0.25 向下取整
- 手动输入数量时，最近的百分比按钮高亮

### 3.9 AccountInfo（账户信息）

| 属性 | 值 |
|------|------|
| 高度 | 自适应，约 48px |
| 背景 | `--color-surface`，圆角 8px，padding 12px |
| 余额文字 | 等宽字体 14px，`--color-text-primary` |
| 标签文字 | 12px，`--color-text-tertiary` |
| 冻结文字 | 12px，`--color-frozen` `#f5a623` |
| 余额不足 | `--color-error` 红色文字 + 闪烁动画 |

### 3.10 OrderSummary（预估金额）

| 属性 | 值 |
|------|------|
| 背景 | `--color-surface-elevated`，圆角 8px |
| 内边距 | 12px |
| 金额文字 | 等宽字体 14px，`--color-text-primary` |
| 标签文字 | 12px，`--color-text-tertiary` |
| 市价单 | 显示「预估金额 (以当前价估算)」，带 * 号说明 |

### 3.11 提交按钮

| 方向 | 默认 | 悬停 | 禁用 | 加载 |
|------|------|------|------|------|
| 买入 | 背景 `--color-buy`，白色文字 | `#5daf34`，scale 1.02 | opacity 0.4 | el-icon-loading 旋转 |
| 卖出 | 背景 `--color-sell`，白色文字 | `#dd4f4f`，scale 1.02 | opacity 0.4 | el-icon-loading 旋转 |

**禁用条件**（任一即禁用）：
1. 价格/数量为空
2. 实时校验有错误
3. 提交中（loading 状态）
4. WS 断线且无轮询降级

**点击流程**：
1. 按钮变为 loading 状态
2. 弹出 OrderConfirmDialog
3. 确认 → 调用 POST /api/v1/orders
4. 成功 → Toast 「委托已提交」+ 对话框关闭 + 列表刷新
5. 失败 → RiskErrorAlert 显示错误码+信息 + 按钮恢复

### 3.12 OrderConfirmDialog（确认对话框）

```
┌──────────────────────────────────────┐
│  确认提交委托                [✕]    │
├──────────────────────────────────────┤
│                                      │
│  交易对:    BTC/USDT                 │
│  方向:      买入                     │
│  类型:      限价                     │
│  价格:      49,500.00 USDT          │
│  数量:      0.1000 BTC              │
│  预估金额:  ≈ 4,950.00 USDT        │
│  手续费(≈): ≈ 4.95 USDT             │
│                                      │
│  🧪 模拟交易                         │
│                                      │
│  [取消]              [确认提交]      │
└──────────────────────────────────────┘
```

| 属性 | 值 |
|------|------|
| 宽度 | 400px |
| 遮罩 | rgba(0,0,0,0.6) |
| 圆角 | 12px |
| 背景 | `--color-surface-elevated` |
| 确认按钮 | 买入绿色/卖出红色 |
| 取消按钮 | ghost 样式 |

**特殊场景**：
- 市价单：显示「市价单将按市场最优价成交，实际成交价可能有偏差」黄色警告
- 价格偏离 > 10%：显示「委托价格偏离当前市场价较大」黄色警告
- 实盘模式：显示「⚠️ 实盘交易，涉及真实资金」红色警告
- 风控 Warn（D8）：显示风控警告文字 + 二次确认

---

## 4. 组件：订单详情侧滑栏 OrderDetailDrawer

> 从右侧滑出，展示订单全量信息 + 成交明细

### 4.1 布局

```
┌──────────────────────────────────────┐
│  委托详情 #123456            [✕]    │
├──────────────────────────────────────┤
│                                      │
│  ┌──────────────────────────────┐   │
│  │  BTC/USDT                    │   │
│  │  买入 · 限价 · [部分成交]     │   │
│  └──────────────────────────────┘   │
│                                      │
│  ── 委托信息 ──                      │
│  委托价格    49,500.00 USDT          │
│  委托数量    0.1000 BTC              │
│  已成交      0.0500 BTC              │
│  成交均价    49,500.00 USDT          │
│  有效期      GTC                     │
│  手续费      2.4750 USDT             │
│  创建时间    2026-05-14 06:00:00     │
│  更新时间    2026-05-14 06:05:00     │
│                                      │
│  ── 保证金信息 (D3) ──               │
│  冻结金额    4,950.00 USDT           │
│  已释放      2,475.00 USDT           │
│  仍冻结      2,475.00 USDT           │
│                                      │
│  ── 成交明细 ──                      │
│  ┌──────────────────────────────┐   │
│  │ 成交价  │ 数量  │ 手续费 │时间│   │
│  │ 49,500  │ 0.050 │ 2.48  │06:05│  │
│  └──────────────────────────────┘   │
│                                      │
│  [撤销委托]          （可撤时显示）   │
│                                      │
└──────────────────────────────────────┘
```

### 4.2 规格

| 属性 | 值 |
|------|------|
| 宽度 | 420px |
| 位置 | 右侧滑出 |
| 背景 | `--color-surface-elevated` |
| 遮罩 | rgba(0,0,0,0.4) |
| 内边距 | 24px |
| 打开动画 | translate-x(100%) → translate-x(0)，300ms ease-out |
| 关闭动画 | 反向，200ms ease-in |

### 4.3 头部区域

- 交易对名称：24px，font-weight 700，`--color-text-primary`
- 方向+类型+状态：一行显示
  - 方向：SideTag（买/卖）
  - 类型：文字标签「限价」「市价」
  - 状态：StatusPill

### 4.4 委托信息区

| 字段 | 样式 | 说明 |
|------|------|------|
| 委托价格 | 等宽 14px，`--color-text-primary` | 市价单显示「市价」 |
| 委托数量 | 等宽 14px | quantity |
| 已成交 | 等宽 14px，部分成交蓝色 | filled_quantity |
| 成交均价 | 等宽 14px | avg_fill_price，未成交「—」 |
| 有效期 | 14px | time_in_force |
| 手续费 | 等宽 14px，`--color-frozen` | fee |
| 创建时间 | 等宽 12px，`--color-text-secondary` | created_at |
| 更新时间 | 等宽 12px，`--color-text-secondary` | updated_at |
| 策略ID | 12px，`--color-accent`（可点击） | strategy_id (D10)，无则不显示 |
| 止损触发价 | 等宽 14px，`--color-warning` | stop_price (D9)，仅止损单显示 |

### 4.5 保证金信息区（D3 专属）

| 字段 | 样式 |
|------|------|
| 冻结金额 | 等宽 14px，`--color-frozen` |
| 已释放 | 等宽 14px，`--color-success` |
| 仍冻结 | 等宽 14px，`--color-frozen` |

**计算逻辑**：
- 冻结金额 = price × quantity（买入限价单）
- 已释放 = avg_fill_price × filled_quantity
- 仍冻结 = 冻结金额 - 已释放

**仅限价买单显示此区域**。市价单和卖出单不显示。

### 4.6 成交明细区

| 列 | 宽度 | 对齐 | 内容 | 样式 |
|----|------|------|------|------|
| 成交价 | flex | 右 | `49,500.00` | 等宽字体 |
| 数量 | 80px | 右 | `0.0500` | 等宽字体 |
| 手续费 | 80px | 右 | `2.48` | 等宽字体，`--color-frozen` |
| 时间 | 60px | 右 | `06:05` | 等宽字体 12px，`--color-text-secondary` |
| 方向 | — | — | Maker/Taker | `is_maker ? 'M' : 'T'`，小标签 |

**空态**：未成交时显示「暂无成交记录」灰色文字

### 4.7 底部操作

- 可撤销（status = pending / partial_filled）→ 显示红色「撤销委托」按钮
- 不可撤销 → 不显示按钮

---

## 5. 组件：持仓面板 PositionPanel

> 独立 Tab 页，与 /orders 页面内的持仓管理 Tab 共享

### 5.1 布局

```
┌──────────────────────────────────────────────────────────────┐
│  PositionSummary（账户汇总条）                                 │
│  可用余额: 55,000.00  冻结: 4,950.00  权益: 60,050.00        │
│  初始资金: 100,000.00  累计盈亏: -40,450.00                  │
├──────────────────────────────────────────────────────────────┤
│  筛选栏: [方向▼] [交易对🔍]                                   │
├──────────────────────────────────────────────────────────────┤
│  PositionsTable                                                │
│  ┌───────┬──────┬──────┬──────┬──────┬──────────┬──────┐   │
│  │交易对 │方向  │数量  │可用  │均价  │浮动盈亏  │操作  │   │
│  ├───────┼──────┼──────┼──────┼──────┼──────────┼──────┤   │
│  │BTC    │多    │0.3   │0.2   │49k  │+¥300     │平仓  │   │
│  │ETH    │多    │1.0   │0.5   │3.2k │-¥100     │平仓  │   │
│  └───────┴──────┴──────┴──────┴──────┴──────────┴──────┘   │
└──────────────────────────────────────────────────────────────┘
```

### 5.2 PositionSummary（账户汇总条）

| 字段 | 样式 | 说明 |
|------|------|------|
| 可用余额 | 等宽 20px，`--color-text-primary` | balance |
| 冻结余额 | 等宽 14px，`--color-frozen` | frozen_balance |
| 权益 | 等宽 20px，`--color-text-primary` | equity |
| 初始资金 | 等宽 14px，`--color-text-secondary` | initial_balance |
| 累计盈亏 | 等宽 20px，盈利绿/亏损红 | total_pnl |
| 持仓数 | 14px，`--color-text-secondary` | positions_count |
| 活跃委托 | 14px，`--color-text-secondary` | active_orders_count |

### 5.3 PositionsTable

| 列 | 宽度 | 对齐 | 内容 | 样式 |
|----|------|------|------|------|
| 交易对 | 100px | 左 | `BTC/USDT` | 14px bold |
| 方向 | 60px | 中 | SideTag（多/空） | 多=绿，空=红 |
| 持仓数量 | 100px | 右 | `0.3000` | 等宽字体 |
| 可用数量 | 100px | 右 | `0.2000` | 等宽字体，`--color-text-secondary` |
| 开仓均价 | 120px | 右 | `49,000.00` | 等宽字体 |
| 当前价 | 120px | 右 | `50,000.00` | 等宽字体，实时更新+闪烁 |
| 浮动盈亏 | 140px | 右 | `+¥300.00` | 盈利绿/亏损红，等宽字体 |
| 盈亏率 | 80px | 右 | `+2.04%` | 盈利绿/亏损红 |
| 操作 | 160px | 中 | [部分平仓] [全部平仓] | — |

**浮动盈亏实时更新**：
- 通过 Ticker WS 推送当前价
- 前端计算: unrealized_pnl = (current_price - avg_entry_price) × quantity（多头）
- 数字变化时闪烁动画（300ms）

**平仓按钮**：
- 全部平仓 → 生成市价反向单（数量 = 持仓数量）
- 部分平仓 → 弹出输入数量对话框 → 生成市价反向单

**ClosePositionDialog**：
```
┌──────────────────────────────────────┐
│  平仓确认                    [✕]    │
├──────────────────────────────────────┤
│                                      │
│  确认平仓 BTC/USDT 0.3 BTC？        │
│  将以市价卖出                        │
│                                      │
│  预估金额: ≈ 15,000.00 USDT         │
│  将释放保证金 ≈ 14,700.00 USDT      │
│                                      │
│  [取消]              [确认平仓]      │
└──────────────────────────────────────┘
```

---

## 6. 组件：成交记录 Tab TradeRecordTab

> 成交记录独立 Tab，展示所有历史成交

### 6.1 布局

```
┌──────────────────────────────────────────────────────────────┐
│  筛选栏: [交易对🔍] [方向▼] [日期范围]                        │
├──────────────────────────────────────────────────────────────┤
│  TradeRecordTable                                              │
│  ┌─────────┬───────┬──────┬──────┬──────┬──────┬──────┐    │
│  │成交时间 │交易对 │方向  │成交价│数量  │手续费│类型  │    │
│  ├─────────┼───────┼──────┼──────┼──────┼──────┼──────┤    │
│  │06:05:12 │BTC    │买    │49500 │0.05  │2.48  │Taker │    │
│  │06:01:45 │BTC    │买    │49000 │0.2   │9.80  │Maker │    │
│  │05:58:30 │SOL    │买    │150   │10    │1.50  │Taker │    │
│  └─────────┴───────┴──────┴──────┴──────┴──────┴──────┘    │
│  [分页: < 1 2 3 ... 8 >]                                      │
└──────────────────────────────────────────────────────────────┘
```

### 6.2 TradeRecordTable

| 列 | 宽度 | 对齐 | 内容 | 样式 |
|----|------|------|------|------|
| 成交时间 | 100px | 左 | `HH:mm:ss` 或 `MM-DD HH:mm` | 等宽字体 12px |
| 交易对 | 100px | 左 | `BTC/USDT` | 14px bold |
| 方向 | 60px | 中 | SideTag（买/卖） | 买入绿/卖出红 |
| 成交价 | 120px | 右 | `49,500.00` | 等宽字体 |
| 数量 | 100px | 右 | `0.0500` | 等宽字体 |
| 成交额 | 120px | 右 | `2,475.00` | 等宽字体 |
| 手续费 | 80px | 右 | `2.48` | 等宽字体，`--color-frozen` |
| 类型 | 60px | 中 | `M`(Maker) / `T`(Taker) | 小标签 |

**筛选器**：
- 交易对搜索: `el-input` + Search 图标，宽度 160px
- 方向: `el-select`，全部/买入/卖出
- 日期范围: `el-date-picker type="daterange"`

**排序**：默认按 created_at 降序

**分页**：`el-pagination`，每页 20 条

**行点击** → 高亮关联的委托详情（打开 OrderDetailDrawer，定位到该 order_id）

---

## 7. 组件状态总矩阵

### 7.1 OrderSummaryCards 状态

| 状态 | 活跃委托 | 今日成交 | 冻结保证金 | 可用余额 |
|------|---------|---------|-----------|---------|
| **默认** | 数值 | 数值 | 金额 | 金额 |
| **加载中** | 骨架屏 60px | 骨架屏 60px | 骨架屏 100px | 骨架屏 100px |
| **WS更新** | 数字闪烁 300ms | 数字闪烁 | 数字闪烁 | 数字闪烁 |
| **零值** | 0，`--color-neutral` | 0 | ¥0.00，`--color-neutral` | 金额显示 |

### 7.2 OrderTable 状态

| 状态 | 表现 | 触发条件 |
|------|------|---------|
| 加载中 | 骨架屏 (5行) | 首次加载/切换筛选 |
| 有数据 | 表格显示 | 数据加载完成 |
| 空数据 | EmptyOrderState | 无委托 |
| 实时更新 | 行闪烁动画 | WS 推送 order_update |
| 状态变化 | 行背景闪烁 | pending→filled 等 |
| 行删除 | 行滑出动画 | 委托从当前切换到历史 |
| 筛选中 | 加载遮罩 | 筛选条件变更 |

### 7.3 OrderCreateDialog 状态

| 状态 | 表现 | 触发条件 |
|------|------|---------|
| 默认 | 空表单，提交按钮禁用 | 打开对话框 |
| 填写中 | 实时校验，错误提示 | 用户输入 |
| 价格偏离 | 黄色警告条 | 价格偏离 > 10% |
| 余额不足 | 红色错误，提交禁用 | 金额 > 可用余额 |
| 持仓不足 | 红色错误，提交禁用 | 卖出量 > 可用持仓 |
| 提交中 | 按钮 loading | 点击提交确认后 |
| 提交成功 | Toast + 对话框关闭 | API 201 |
| 提交失败 | RiskErrorAlert | API 4xx/5xx |
| WS 断线 | 提示「降级为轮询」 | WS 断开 |
| 行情不可用 | 市价单禁用+提示 | 深度数据为空 |

### 7.4 CancelButton 状态

| 状态 | 样式 | 可用条件 |
|------|------|---------|
| 默认 | ghost，灰色边框 | status = pending / partial_filled |
| 悬停 | 红色边框+文字 | 同上 |
| 加载 | 旋转 icon | 撤单请求中 |
| 隐藏 | display: none | status = filled / cancelled / expired / rejected |

### 7.5 OrderDetailDrawer 状态

| 状态 | 表现 |
|------|------|
| 加载中 | 骨架屏（信息区+成交明细） |
| 有数据 | 完整信息展示 |
| WS更新 | 状态/已成交/保证金数字闪烁 |
| 可撤销 | 底部显示撤单按钮 |
| 不可撤销 | 底部无按钮 |
| 策略单 | 显示策略ID标签 (D10) |

### 7.6 WsConnectionStatus 状态

| 状态 | 圆点颜色 | 文字 | 闪烁 |
|------|---------|------|------|
| connected | `--color-success` | 已连接 | 无 |
| connecting | `--color-warning` | 连接中 | 1s 闪烁 |
| reconnecting | `--color-warning` | 重连中 | 0.5s 闪烁 |
| disconnected | `--color-text-tertiary` | 已断开 | 无 |

---

## 8. 路由设计

| 路径 | 组件 | 说明 |
|------|------|------|
| `/orders` | OrderListView.vue | 订单列表页（含3个Tab） |
| `/orders/:id` | OrderListView.vue + 自动打开 OrderDetailDrawer | 指定订单详情 |
| `/trade` | TradingView.vue | 交易主页面（复用 OrderCreateDialog） |

**Tab 路由映射**（嵌套路由或 query 参数）：
- `/orders?tab=orders` → 委托管理（默认）
- `/orders?tab=positions` → 持仓管理
- `/orders?tab=trades` → 成交记录

**路由守卫**：所有交易路由需 JWT 认证，未登录跳转 `/login`

**与 /trade 页面的关系**：
- `/trade` 页面内嵌了精简版 OrderCreateDialog + 精简版委托列表
- `/orders` 页面是完整版，含筛选、分页、详情、持仓、成交记录
- 点击 `/trade` 委托区的「查看全部」链接跳转到 `/orders`
- 两个页面共享 OrderCreateDialog、OrderDetailDrawer、CancelConfirmDialog 等组件

---

## 9. API 数据流

### 9.1 下单流程

```
用户打开 OrderCreateDialog
    ↓
填写表单 (交易对/方向/类型/价格/数量)
    ↓
前端实时校验 (精度/余额/持仓/最小量/偏离度) — D8 前置
    ↓ 通过
弹出 OrderConfirmDialog
    ↓ 确认
POST /api/v1/orders (CreateOrderRequest)
    ↓
后端 RiskManager 校验 (D8 8项拦截链)
    ↓ Pass
TradingEngine.freeze_margin (D3 三阶段冻结)
    ↓
写入 orders 表 status=pending
    ↓
Redis PubSub("trade:order:{uid}") → TradeWsHub (D5)
    ↓
WS 推送 order_update (status=pending)
    ↓
MatchingEngine 处理 (D1 内存撮合 / D4 深度驱动)
    ↓
成交 → 更新 orders + trades + positions
    ↓
WS 推送 order_update + fill + position_update
    ↓
前端更新 OrderTable + SummaryCards + AccountInfo
```

### 9.2 撤单流程

```
用户点击 CancelButton
    ↓
弹出 CancelConfirmDialog (显示释放保证金)
    ↓ 确认
POST /api/v1/orders/:id/cancel
    ↓
PG 行锁 SELECT FOR UPDATE (D2)
    ↓
校验状态 (pending/partial_filled → 允许)
    ↓
从内存订单簿移除
    ↓
释放冻结保证金 (D3 第三阶段)
    ↓
更新 status = cancelled
    ↓
WS 推送 order_update (status=cancelled)
    ↓
前端更新 OrderTable + SummaryCards + AccountInfo
```

### 9.3 平仓流程

```
用户点击 ClosePositionButton
    ↓
弹出 ClosePositionDialog
    ↓ 确认
POST /api/v1/positions/:symbol/close
    ↓
查找持仓 → 校验平仓数量
    ↓
生成反向市价委托 (D7 加权均价计算)
    ↓
撮合 → 更新 position / account
    ↓
WS 推送 order_update + position_update
    ↓
前端更新 PositionsTable + SummaryCards
```

### 9.4 前端 API 调用列表

| 函数 | 方法 | 路径 | 优先级 | ADR |
|------|------|------|--------|-----|
| `createOrder(req)` | POST | /api/v1/orders | P0 | D1/D3/D8 |
| `listOrders(params)` | GET | /api/v1/orders | P0 | D6 |
| `getOrder(id)` | GET | /api/v1/orders/:id | P0 | D6 |
| `cancelOrder(id)` | POST | /api/v1/orders/:id/cancel | P0 | D2/D3 |
| `cancelAllOrders(filter?)` | POST | /api/v1/orders/cancel-all | P1 | D2 |
| `listTrades(params?)` | GET | /api/v1/trades | P1 | D4 |
| `listPositions()` | GET | /api/v1/positions | P1 | D7 |
| `closePosition(symbol, data?)` | POST | /api/v1/positions/:symbol/close | P0 | D3/D7 |
| `getAccount()` | GET | /api/v1/account | P0 | D3 |
| `getSymbols()` | GET | /api/v1/symbols | P0 | D1 |
| `initAccount()` | POST | /api/v1/account/init | P1 | D3 |
| `getRiskRules()` | GET | /api/v1/trade/risk/rules | P1 | D8 |
| `updateRiskRule(id, data)` | PUT | /api/v1/trade/risk/rules/:id | P1 | D8 |

---

## 10. WebSocket 实时更新机制

### 10.1 频道订阅（D5: TradeWsHub）

| PubSub 频道 | 触发时机 | WS 推送消息类型 | 优先级 |
|-------------|---------|----------------|--------|
| `trade:order:{user_id}` | 委托状态变更 | `order_update` | P0 |
| `trade:fill:{user_id}` | 成交通知 | `fill` | P0 |
| `trade:position:{user_id}` | 持仓变更 | `position_update` | P1 |
| `trade:risk:{user_id}` | 风控预警 | `risk_alert` | P1 |

### 10.2 消息处理

| WS 消息类型 | 前端处理 |
|------------|---------|
| `order_update` | 更新 OrderTable 中对应委托行；若状态变为 filled/cancelled/expired，从当前委托移至历史；更新 SummaryCards |
| `fill` | 成交通知，弹出 Toast 提示；更新成交记录 Tab |
| `position_update` | 更新 PositionsTable；更新浮动盈亏 |
| `risk_alert` | 显示 RiskAlertBar 横幅；severity=block 时禁用下单 |
| `subscribed` | 记录已订阅频道 |
| `error` | 显示错误 Toast |

### 10.3 重连策略

- 指数退避：1s → 2s → 4s → 8s → 16s → 30s（封顶）
- 重连成功后自动恢复订阅
- 补发断线期间状态变更（后端支持）
- 前端幂等去重（基于 order_id + status + updated_at）

### 10.4 降级方案

WS 不可用时：
- 委托列表: REST 轮询每 5s（GET /api/v1/orders?status=pending,partial_filled）
- 持仓列表: REST 轮询每 10s
- 账户: REST 轮询每 10s
- 下单/撤单: REST 仍可用，响应后手动刷新列表

---

## 11. 响应式适配方案

### 11.1 桌面端 (≥ 1440px)

- 全宽布局，最大宽度 1440px，居中
- SummaryCards 4 列均分
- OrderTable 全宽，所有列显示
- OrderDetailDrawer 从右侧滑出 420px

### 11.2 平板端 (768px ~ 1439px)

- SummaryCards 2×2 网格
- OrderTable 隐藏部分列（成交均价、类型列压缩为图标）
- OrderDetailDrawer 全屏覆盖
- OrderCreateDialog 宽度适配屏幕

### 11.3 移动端 (< 768px) — P2+，本期预留

- SummaryCards 2×2 网格
- OrderTable 简化为卡片式列表
- 底部 Tab 栏切换（委托/持仓/成交）
- OrderCreateDialog 全屏
- 筛选栏折叠为抽屉

---

## 12. 风控与错误处理 UX

### 12.1 前端风控校验（实时，D8 对齐）

| 校验项 | 错误码 | 前端处理 | 提示方式 |
|--------|--------|---------|---------|
| 交易对不支持 | 40004 | 从 symbolConfigs 检查 | 下拉不可选 |
| 数量无效 | 40004 | 实时校验 | Input 下方红色文字 |
| 价格无效 | 40005 | 实时校验 | Input 下方红色文字 |
| 低于最小下单量 | 40006 | 实时校验 | Input 下方红色文字 |
| 低于最小下单金额 | 40007 | 实时校验 | OrderSummary 区域红色文字 |
| 余额不足 | 40002 | 比对 account.balance | AccountInfo 红色 + 提交禁用 |
| 持仓不足 | 40005 | 比对 position.available_quantity | AccountInfo 红色 + 提交禁用 |
| 未成交委托达上限 | 42901 | 提交时检查 | 提交后 RiskErrorAlert |
| 风控 Warn | D8 | 后端返回 warn | 二次确认对话框 |
| 风控 Block | 40003 | 后端返回 block | RiskErrorAlert + 禁用提交 |

### 12.2 后端错误响应处理

| HTTP 状态码 | 错误码 | 前端处理 |
|------------|--------|---------|
| 400 | 40001-40007 | RiskErrorAlert 显示具体错误信息 + details |
| 400 | 40003 | 风控拒绝 — RiskAlertBar 红色横幅 |
| 401 | 40101 | 跳转登录页 |
| 403 | 40301 | Toast「权限不足」 |
| 404 | 40401 | Toast「委托不存在」+ 刷新列表 |
| 404 | 40403 | Toast「持仓不存在」+ 刷新持仓 |
| 409 | 40901 | Toast「委托已成交，无法撤销」+ 刷新列表 |
| 429 | 42901 | Toast「未成交委托达上限」+ 引导撤单 |
| 503 | 50301 | Toast「撮合引擎不可用」+ 禁用下单 |

### 12.3 RiskAlertBar 样式

```
┌──────────────────────────────────────────────────┐
│  ⚠ 风控预警: 当日亏损已达 80%，请注意风险      │
└──────────────────────────────────────────────────┘
```

| 属性 | 值 |
|------|------|
| 背景 | `rgba(245,166,35,0.12)` (Warn) / `rgba(229,72,77,0.12)` (Block) |
| 边框 | `--color-warning` / `--color-error` 1px 左边框 |
| 文字 | `--color-warning` / `--color-error` 14px |
| 图标 | ⚠ 黄色三角 |
| 位置 | 页面顶部，全宽，OrderListHeader 下方 |
| 持续 | 不自动消失，WS risk_alert 更新时刷新/消失 |

### 12.4 RiskErrorAlert 样式

```
┌──────────────────────────────────────────────────┐
│  ⚠ 余额不足: 可用余额 1,000.00 USDT，需要 10,000 │
└──────────────────────────────────────────────────┘
```

| 属性 | 值 |
|------|------|
| 背景 | `rgba(229,72,77,0.12)` |
| 边框 | `--color-error` 1px 左边框 |
| 文字 | `--color-error` 14px |
| 图标 | ⚠ 黄色三角 |
| 位置 | OrderCreateDialog 表单顶部 |
| 持续 | 5s 后自动消失，或用户手动关闭 |

---

## 13. 交互细节与动画

### 13.1 动画时间线

| 动画 | 时长 | 缓动 | 触发 |
|------|------|------|------|
| 数值变化闪烁 | 300ms | ease-out | WS 推送 / 数据更新 |
| 行状态变化 | 300ms | ease-out | order_update |
| 行删除滑出 | 200ms | ease-in | 委托完成/撤销 |
| 对话框打开 | 300ms | ease-out | 打开 Create/Confirm |
| 对话框关闭 | 200ms | ease-in | 关闭对话框 |
| Drawer 滑入 | 300ms | ease-out | 打开详情 |
| Drawer 滑出 | 200ms | ease-in | 关闭详情 |
| 按钮悬停 scale | 150ms | ease | hover |
| Tab 切换 | 200ms | ease | 点击 Tab |
| 筛选结果加载 | 300ms | ease | 筛选条件变更 |

### 13.2 数值变化闪烁

- 正数变化：绿色背景闪 `rgba(103,194,58,0.12)` → 恢复
- 负数变化：红色背景闪 `rgba(245,108,108,0.12)` → 恢复
- 中性变化：无闪烁
- 同一元素 300ms 内不重复闪烁（防 WS 高频推送时闪烁过快）

### 13.3 订单状态变化动画

- pending → partial_filled：行背景蓝色闪烁 300ms + StatusPill 更新
- partial_filled → filled：行背景绿色闪烁 300ms + 行从当前委托列表淡出 → 历史列表淡入
- pending → cancelled：行背景灰色闪烁 300ms + 同上
- pending → rejected：行背景红色闪烁 300ms

### 13.4 保证金变化动画

- 冻结：FrozenBalanceCard 数字增加 + 冻结色闪烁
- 释放：FrozenBalanceCard 数字减少 + 绿色闪烁
- 撤单释放：AccountBalanceCard 余额增加 + 闪烁

---

## 14. 空状态与错误处理 UX

### 14.1 EmptyOrderState（无委托）

```
┌──────────────────────────────────┐
│                                  │
│      📋 暂无委托记录             │
│      筛选条件下没有找到委托       │
│                                  │
│      [ 清空筛选 ]  [ 新建委托 ]  │
│                                  │
└──────────────────────────────────┘
```

| 属性 | 值 |
|------|------|
| 图标 | 📋 48px |
| 主文字 | 「暂无委托记录」16px, `--color-text-secondary` |
| 副文字 | 「筛选条件下没有找到委托」13px, `--color-text-tertiary` |
| 按钮 | 清空筛选(ghost) + 新建委托(primary) |

### 14.2 EmptyPositionState（无持仓）

```
┌──────────────────────────────────┐
│                                  │
│      📊 暂无持仓                 │
│      当前没有任何持仓记录         │
│                                  │
│         [ 去下单 ]               │
│                                  │
└──────────────────────────────────┘
```

### 14.3 EmptyTradeState（无成交记录）

```
┌──────────────────────────────────┐
│                                  │
│      📝 暂无成交记录             │
│      还没有成交记录               │
│                                  │
└──────────────────────────────────┘
```

### 14.4 LoadingSkeleton（骨架屏）

- OrderTable：5行骨架，每行高度 48px
- SummaryCards：4个卡片骨架，120px 宽
- OrderDetailDrawer：信息区 6 行骨架 + 成交明细 3 行骨架

### 14.5 ErrorState（错误状态）

```
┌──────────────────────────────────┐
│                                  │
│      ⚠ 加载失败                  │
│      网络错误，请重试             │
│                                  │
│         [ 重新加载 ]             │
│                                  │
└──────────────────────────────────┘
```

---

## 附录 A: ADR 决策覆盖验证

| ADR 决策 | UI 设计体现 |
|----------|-----------|
| D1: 内存撮合+异步DB | OrderCreateDialog 市价单立即撮合UI反馈；限价单挂单等待状态 |
| D2: PG行锁状态机 | CancelButton 撤单 → 409 冲突处理；StatusPill 6态完整；CancelConfirmDialog 显示释放保证金 |
| D3: 三阶段保证金冻结 | AccountInfo/FrozenBalanceCard 显示冻结余额；OrderDetailDrawer 保证金信息区；撤单后释放刷新 |
| D4: 深度事件驱动撮合 | 限价单「待成交」状态；行情不可用时市价单禁用 |
| D5: 独立TradeWsHub | WsConnectionStatus 指示器；4频道订阅；order_update/fill/position_update/risk_alert |
| D6: API 路由设计 | API 调用列表完整（9 P0 + 4 P1）；路由与现有代码对齐 |
| D7: 加权平均均价 | PositionRow 显示 avg_entry_price；ClosePositionDialog 计算逻辑 |
| D8: 风控前置 | 前端8项风控校验实时反馈；后端错误码映射；RiskAlertBar 横幅；风控 Warn 二次确认 |
| D9: 止损单扩展 | OrderTypeSelect 显示止损选项(灰色禁用+提示)；stop_price 字段预留；MVP 禁用止损 |
| D10: 策略信号自动下单 | OrderDetailDrawer 显示 strategy_id；CreateOrderRequest 预留 strategy_id；MVP 不实现策略信号 |
| D11: 错误码体系 | 后端错误响应处理完整映射（40001-40007, 40401-40403, 42901, 50301） |

## 附录 B: Contract 缺口修复清单

| 缺口 | 设计方案 | 优先级 |
|------|---------|--------|
| Order.id → order_id | 全部使用 order_id | P0 |
| Order.type → order_type | SideToggle/OrderTypeSelect 使用 order_type | P0 |
| Order.price: number → string\|null | PriceInput value 为 string，市价单 null | P0 |
| quantity/filled_quantity: string | QuantityInput value 为 string | P0 |
| 缺 avg_fill_price | OrderDetailDrawer 显示 | P0 |
| 缺 partial_filled/expired | StatusPill 6态完整 | P0 |
| 缺 mode/fee/time_in_force/updated_at | OrderDetailDrawer 完整显示 | P0 |
| Position.entry_price → avg_entry_price | PositionRow 使用 avg_entry_price | P0 |
| 缺 available_quantity/realized_pnl/mode | PositionSummary/PositionRow 显示 | P0 |
| Portfolio → Account | PositionSummary 使用 Account 类型 | P0 |
| 缺 CreateOrderRequest | OrderCreateDialog 提交数据结构 | P0 |
| 缺 WS 交易消息类型 | TradeWsMessage 类型完整（含 risk_alert） | P0 |
| 缺 getOrder/getSymbols/cancelAllOrders | API 调用列表完整 | P0 |
| 缺 close_position (symbol路径参数) | ClosePositionDialog 调用 /positions/:symbol/close | P0 |
| Trade 缺 is_maker | TradeRecordTable 类型列显示 M/T | P1 |
| 缺 stop_price (D9 止损) | OrderTypeSelect 预留止损选项 | P1 |
| 缺 strategy_id (D10) | OrderDetailDrawer 预留显示 | P1 |
| 缺 init_account | API 调用列表包含 | P1 |
| 缺 risk_rules API | API 调用列表包含 | P1 |
