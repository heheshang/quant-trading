# 设计规格说明书：Portfolio 组合权益模块 UI 界面

> 基于 ADR-009-portfolio-module 的完整 UI/UX 设计方案
> 设计师：Designer | 版本 v1.0 | 日期：2026-05-14
> 设计灵感：Linear.app 暗色模式设计系统 + 金融交易终端风格
> 框架：Element Plus + Vue 3 + TypeScript + ECharts
> 源代码库：/home/ssk/workspace/quant-trading
> ADR 关联：ADR-009 全覆盖（PortfolioSummary / Positions / Performance / EquityCurve）

---

## 目录

1. [设计系统继承](#1-设计系统继承)
2. [页面1：组合总览仪表板 /portfolio](#2-页面1组合总览仪表板-portfolio)
3. [页面2：持仓汇总页 /portfolio/positions](#3-页面2持仓汇总页-portfoliopositions)
4. [页面3：策略绩效对比页 /portfolio/performance](#4-页面3策略绩效对比页-portfolioperformance)
5. [组件状态总矩阵](#5-组件状态总矩阵)
6. [路由设计](#6-路由设计)
7. [API 数据流](#7-api-数据流)
8. [WebSocket 实时更新机制](#8-websocket-实时更新机制)
9. [响应式适配方案](#9-响应式适配方案)
10. [交互细节与动画](#10-交互细节与动画)
11. [空状态与错误处理 UX](#11-空状态与错误处理-ux)

---

## 1. 设计系统继承

### 1.1 完全继承 Design Token

本设计继承自 `Design_QuantTrading_UI.md` 与 `Design_TradingExecution.md` 的所有定义。以下是关键 token：

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
| `--color-success` | `#10b981` | 成功/上涨 |
| `--color-warning` | `#f5a623` | 警告 |
| `--color-info` | `#60a5fa` | 信息 |
| **`--color-buy`** | **`#67C23A`** | **买入/多头/上涨（Element Plus 绿）** |
| **`--color-sell`** | **`#F56C6C`** | **卖出/空头/下跌（Element Plus 红）** |
| `--color-buy-bg` | `rgba(103,194,58,0.12)` | 上涨背景 |
| `--color-sell-bg` | `rgba(245,108,108,0.12)` | 下跌背景 |
| **`--color-long`** | **`#67C23A`** | **多头持仓标识** |
| **`--color-short`** | **`#F56C6C`** | **空头持仓标识** |
| `--color-long-bg` | `rgba(103,194,58,0.12)` | 多头行背景 |
| `--color-short-bg` | `rgba(245,108,108,0.12)` | 空头行背景 |
| **`--color-positive`** | **`#67C23A`** | **正收益/盈利数值** |
| **`--color-negative`** | **`#F56C6C`** | **负收益/亏损数值** |
| **`--color-neutral`** | **`#8a8f98`** | **零值/持平** |
| **`--color-frozen`** | **`#f5a623`** | **保证金/冻结资金** |

字体：
- UI：`'Inter', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif`
- 等宽/数字：`'JetBrains Mono', 'SF Mono', Menlo, monospace`

间距：
- 基础间距单元：4px
- 面板内边距：16px
- 卡片内边距：12px
- 元素间距：8px / 12px / 16px / 24px

### 1.2 Portfolio 模块专用组件列表

| 组件 | 类型 | 描述 | ADR 关联 |
|------|------|------|---------|
| **PortfolioDashboard** | 容器 | 组合总览页面容器 | 整体 |
| **PortfolioHeader** | 展示 | 页面标题 + 时间区间选择 + 刷新按钮 | — |
| **EquityOverviewCards** | 容器 | 四大指标卡片行 | 4.1 |
| **TotalEquityCard** | 展示 | 总资产卡片（主指标） | 4.1 |
| **CumulativePnlCard** | 展示 | 累计盈亏卡片 | 4.1 |
| **DailyPnlCard** | 展示 | 当日盈亏卡片 | 4.1 |
| **ReturnRateCard** | 展示 | 收益率卡片 | 4.1 |
| **EquityCurveChart** | 展示 | 权益曲线图（ECharts 面积图） | 4.2 |
| **GranularitySelector** | 交互 | 权益曲线粒度切换（小时/日/周） | 4.2 |
| **DateRangePicker** | 交互 | 日期区间选择器 | 4.2 |
| **PositionSummaryTable** | 展示 | 持仓汇总表格（按交易对分组） | 4.1 |
| **PositionRow** | 展示 | 持仓行（多头/空头分组） | 4.1 |
| **SideTag** | 展示 | 多/空方向标签 | 4.1 |
| **UnrealizedPnlCell** | 展示 | 浮动盈亏单元格（带颜色+百分比） | 4.1 |
| **PerformanceMetricsCards** | 容器 | 绩效指标三卡片行 | 4.1 |
| **MaxDrawdownCard** | 展示 | 最大回撤卡片 | 4.1 |
| **SharpeRatioCard** | 展示 | 夏普率卡片 | 4.1 |
| **WinRateCard** | 展示 | 胜率卡片 | 4.1 |
| **StrategyComparisonTable** | 展示 | 策略绩效对比表格 | 4.1 |
| **StrategyRow** | 展示 | 策略行（含迷你 sparkline） | 4.1 |
| **SortHeader** | 交互 | 排序表头（点击切换排序） | — |
| **PaginationBar** | 交互 | 分页控件 | — |
| **EmptyPortfolio** | 展示 | 空组合占位图 | — |
| **LoadingSkeleton** | 展示 | 加载骨架屏 | — |
| **ErrorState** | 展示 | 错误状态提示 | — |

---

## 2. 页面1：组合总览仪表板 /portfolio

> 一页总览，从上到下：指标卡片 → 权益曲线 → 持仓摘要 → 绩效指标 + 策略列表

### 2.1 页面布局

```
┌──────────────────────────────────────────────────────────────┐
│  PortfolioHeader                                              │
│  组合权益 · 日期选择 · 粒度 · 刷新                            │
├──────────────────────────────────────────────────────────────┤
│  EquityOverviewCards (4列均分)                                │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐│
│  │ 总资产     │ │ 累计盈亏   │ │ 当日盈亏   │ │ 收益率     ││
│  │ ¥100,000   │ │ +¥15,000   │ │ +¥1,234    │ │ +17.65%   ││
│  │            │ │ +15.00%    │ │ +1.25%     │ │            ││
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘│
├──────────────────────────────────────────────────────────────┤
│  EquityCurveChart (全宽)                                      │
│  ┌──────────────────────────────────────────────────────────┐│
│  │  ▁▂▃▄▅▆▇█▇▆▅▄▃▂▃▄▅▆▇█████████████████  权益曲线      ││
│  │  面积图 + 十字线 + Tooltip                                ││
│  └──────────────────────────────────────────────────────────┘│
├──────────────────────────────────────────────────────────────┤
│  两栏布局                                                     │
│  ┌─────────────────────────────┐ ┌─────────────────────────┐ │
│  │ PositionSummaryTable        │ │ PerformanceMetricsCards │ │
│  │ 持仓汇总表                  │ │ ┌─────┐┌─────┐┌─────┐  │ │
│  │ ┌───────┬───────┬─────────┐ │ │ │回撤  ││夏普  ││胜率  │  │ │
│  │ │交易对 │方向   │浮动盈亏 │ │ │ └─────┘└─────┘└─────┘  │ │
│  │ ├───────┼───────┼─────────┤ │ │                         │ │
│  │ │BTC/USDT│多    │+¥1,234 │ │ │ StrategyComparisonTable │ │
│  │ │ETH/USDT│空    │-¥567   │ │ │ 策略绩效对比表          │ │
│  │ └───────┴───────┴─────────┘ │ │ ┌────┬────┬────┬────┐  │ │
│  │ 分页控件                     │ │ │策略│盈亏│回撤│胜率│  │ │
│  └─────────────────────────────┘ │ └────┴────┴────┴────┘  │ │
│                                   └─────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

### 2.2 PortfolioHeader

| 属性 | 规格 |
|------|------|
| 高度 | 56px |
| 内边距 | 0 16px |
| 左侧 | 页面标题「组合权益」字号 18px/font-weight 600 |
| 右侧控件组 | DateRangePicker + GranularitySelector + 刷新按钮 |
| 背景 | `--color-bg` |
| 底部边框 | `--color-border` |

**GranularitySelector**：
- 类型：`el-segmented`（Element Plus 分段控制器）
- 选项：`小时` / `日` / `周`
- 默认：`日`
- 宽度：200px
- 高度：32px

**DateRangePicker**：
- 类型：`el-date-picker type="daterange"`
- 格式：YYYY-MM-DD
- 默认值：最近 30 天
- 宽度：280px

**刷新按钮**：
- 类型：`el-button circle plain`
- 图标：`Refresh` (Element Plus Icons)
- 尺寸：32px
- 旋转动画：点击时 icon 旋转 360deg，持续 0.6s

### 2.3 EquityOverviewCards — 四大指标卡片

布局：4 列均分，间距 16px，使用 `el-row :gutter="16"`

#### TotalEquityCard（总资产）

| 属性 | 规格 |
|------|------|
| 卡片 | `--color-surface`, border-radius 12px, padding 20px |
| 标签 | 「总资产」字号 13px, `--color-text-tertiary` |
| 主值 | `¥100,000.00` 字号 28px/font-weight 700, `--color-text-primary`, font-family `JetBrains Mono` |
| 子值 | 「X 个持仓」字号 12px, `--color-text-tertiary` |
| 图标 | 右上角 `Wallet` 图标，24px，`--color-accent` 40% 透明 |
| 悬停 | border 从 `--color-border` → `--color-border-hover`，轻微 translate-y -2px + box-shadow |

#### CumulativePnlCard（累计盈亏）

| 属性 | 规格 |
|------|------|
| 标签 | 「累计盈亏」字号 13px, `--color-text-tertiary` |
| 主值 | `+¥15,000.00` 字号 28px/font-weight 700 |
| 主值颜色 | 正数 `--color-positive`，负数 `--color-negative`，零 `--color-neutral` |
| 子值 | `+15.00%` 字号 13px，同色 |
| 图标 | `TrendingUp` / `TrendingDown`，颜色跟随正负 |
| 悬停 | 同 TotalEquityCard |

#### DailyPnlCard（当日盈亏）

| 属性 | 规格 |
|------|------|
| 标签 | 「当日盈亏」字号 13px, `--color-text-tertiary` |
| 主值 | `+¥1,234.56` 字号 28px/font-weight 700 |
| 主值颜色 | 同 CumulativePnlCard 规则 |
| 子值 | `+1.25%` 字号 13px，同色 |
| 图标 | `Calendar` / `Timer`，颜色跟随 |
| 悬停 | 同上 |

#### ReturnRateCard（收益率）

| 属性 | 规格 |
|------|------|
| 标签 | 「收益率」字号 13px, `--color-text-tertiary` |
| 主值 | `+17.65%` 字号 28px/font-weight 700 |
| 主值颜色 | 同上规则 |
| 子值 | 迷你 sparkline（最近 30 天收益率趋势）40px × 16px |
| 图标 | `DataLine`，颜色跟随 |
| 悬停 | 同上 |

**数值格式化规则**：
- 金额：千分位分隔，保留 2 位小数，前缀 ¥
- 百分比：保留 2 位小数，后缀 %
- 正数：前缀 `+`
- 零：无前缀，使用 `--color-neutral`

### 2.4 EquityCurveChart — 权益曲线

| 属性 | 规格 |
|------|------|
| 容器 | `--color-surface`, border-radius 12px, padding 16px |
| 高度 | 320px (图表区 280px) |
| 标题 | 「权益曲线」字号 14px/font-weight 600, padding-bottom 12px |
| 图表类型 | ECharts 面积图 (area) |
| X轴 | 时间轴，格式自适应（小时→HH:mm, 日→MM-DD, 周→MM-DD） |
| Y轴 | 权益值，千分位分隔 |
| 面积填充 | 线上方渐变 `--color-accent` 20% → 0% |
| 线颜色 | `--color-accent` |
| 线宽 | 2px |
| Tooltip | 十字线 + 数值气泡 `{time}<br/>权益: ¥{value}` |
| 数据点 | hover 时显示，半径 4px，`--color-accent` |
| 空态 | 灰色虚线 + 「暂无权益数据」文字 |
| 加载 | ECharts loading 动画 |

**ECharts 配置要点**：
```typescript
{
  backgroundColor: 'transparent',
  grid: { left: 64, right: 24, top: 16, bottom: 32 },
  xAxis: { type: 'time', axisLine: { lineStyle: { color: 'rgba(255,255,255,0.08)' } } },
  yAxis: { type: 'value', axisLabel: { formatter: val => '¥' + formatNumber(val) } },
  series: [{
    type: 'line',
    smooth: 0.3,
    areaStyle: {
      color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
        { offset: 0, color: 'rgba(113,112,255,0.20)' },
        { offset: 1, color: 'rgba(113,112,255,0.00)' }
      ])
    },
    lineStyle: { color: '#7170ff', width: 2 },
    itemStyle: { color: '#7170ff' },
    showSymbol: false,
  }],
  tooltip: {
    trigger: 'axis',
    backgroundColor: '#212223',
    borderColor: 'rgba(255,255,255,0.08)',
    textStyle: { color: '#f7f8f8', fontFamily: 'JetBrains Mono' }
  }
}
```

### 2.5 底部两栏布局

- 左栏：60% 宽度 — PositionSummaryTable
- 右栏：40% 宽度 — PerformanceMetricsCards + StrategyComparisonTable
- 间距：16px

### 2.6 PositionSummaryTable — 持仓汇总表

| 属性 | 规格 |
|------|------|
| 容器 | `--color-surface`, border-radius 12px, padding 16px |
| 标题行 | 「持仓汇总」字号 14px/font-weight 600 + 右侧持仓数量 Badge |
| 表格 | `el-table`，暗色模式 |

**列定义**：

| 列 | 宽度 | 对齐 | 内容 |
|----|------|------|------|
| 交易对 | flex | 左 | symbol（加粗） |
| 方向 | 80px | 中 | `SideTag`（多/空标签） |
| 数量 | 120px | 右 | quantity（等宽字体） |
| 均价 | 120px | 右 | avg_price（等宽字体） |
| 现价 | 120px | 右 | current_price（等宽字体） |
| 浮动盈亏 | 160px | 右 | `UnrealizedPnlCell`（金额 + 百分比） |

**SideTag 规格**：
- 多头：`el-tag type="success"` 文字「多」，`--color-long` + `--color-long-bg`
- 空头：`el-tag type="danger"` 文字「空」，`--color-short` + `--color-short-bg`
- 尺寸：small（高度 20px）

**UnrealizedPnlCell 规格**：
- 正数：`--color-positive`，前缀 `+`
- 负数：`--color-negative`
- 零：`--color-neutral`
- 格式：金额在上（14px），百分比在下（12px，较浅色）
- 等宽字体

**行交互**：
- 悬停：行背景 `rgba(255,255,255,0.04)`
- 多头行默认背景：`--color-long-bg`（10% 透明度版本）
- 空头行默认背景：`--color-short-bg`（10% 透明度版本）

**排序**：
- 默认按浮动盈亏绝对值降序
- 可点击列头排序（交易对字母序、浮动盈亏数值序）

**分页**：
- `el-pagination` small
- 每页 10 条
- 总数显示

### 2.7 PerformanceMetricsCards — 绩效指标三卡片

三卡片水平排列，间距 12px

#### MaxDrawdownCard（最大回撤）

| 属性 | 规格 |
|------|------|
| 容器 | `--color-surface`, border-radius 12px, padding 16px |
| 标签 | 「最大回撤」字号 12px, `--color-text-tertiary` |
| 主值 | `-12.34%` 字号 22px/font-weight 700, `--color-negative` |
| 迷你图 | 小型面积图（40px × 20px），回撤区间用红色填充 |
| 图标 | `Bottom` 图标，16px，`--color-negative` 50% |

#### SharpeRatioCard（夏普率）

| 属性 | 规格 |
|------|------|
| 标签 | 「夏普率」字号 12px, `--color-text-tertiary` |
| 主值 | `1.85` 字号 22px/font-weight 700 |
| 主值颜色 | ≥2 `--color-positive`，1-2 `--color-warning`，<1 `--color-negative` |
| 图标 | `DataAnalysis` 图标，颜色跟随 |

#### WinRateCard（胜率）

| 属性 | 规格 |
|------|------|
| 标签 | 「胜率」字号 12px, `--color-text-tertiary` |
| 主值 | `62.5%` 字号 22px/font-weight 700 |
| 主值颜色 | ≥60% `--color-positive`，40-60% `--color-warning`，<40% `--color-negative` |
| 迷你图 | 半圆进度环（36px × 20px），颜色跟随主值 |
| 图标 | `SuccessFilled` 图标，颜色跟随 |

### 2.8 StrategyComparisonTable — 策略绩效对比表

| 属性 | 规格 |
|------|------|
| 容器 | `--color-surface`, border-radius 12px, padding 16px, margin-top 16px |
| 标题 | 「策略绩效」字号 14px/font-weight 600 |

**列定义**：

| 列 | 宽度 | 对齐 | 内容 |
|----|------|------|------|
| 策略名称 | flex | 左 | strategy_name（可点击，链接色） |
| 总盈亏 | 120px | 右 | total_pnl（颜色跟随正负） |
| 盈亏率 | 100px | 右 | total_pnl_rate（颜色跟随） |
| 最大回撤 | 100px | 右 | max_drawdown（`--color-negative`） |
| 交易次数 | 80px | 中 | trade_count |
| 胜率 | 80px | 右 | win_rate（颜色跟随阈值） |

**行交互**：
- 悬停：行背景 `rgba(255,255,255,0.04)`
- 点击策略名称：跳转策略详情页（预留）

**排序**：
- 默认按总盈亏降序
- 可点击列头排序

---

## 3. 页面2：持仓汇总页 /portfolio/positions

> 独立的持仓管理页面，比仪表板更详细的持仓列表

### 3.1 页面布局

```
┌──────────────────────────────────────────────────────────────┐
│  PositionHeader                                               │
│  持仓汇总 · 筛选器 · 导出按钮                                 │
├──────────────────────────────────────────────────────────────┤
│  筛选栏                                                       │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐                        │
│  │方向筛选 │ │交易对搜索│ │排序方式 │                        │
│  └─────────┘ └─────────┘ └─────────┘                        │
├──────────────────────────────────────────────────────────────┤
│  PositionTable (全宽)                                         │
│  ┌───────┬──────┬──────┬──────┬──────┬──────────┬──────┐   │
│  │交易对 │方向  │数量  │均价  │现价  │浮动盈亏  │占比  │   │
│  ├───────┼──────┼──────┼──────┼──────┼──────────┼──────┤   │
│  │BTC    │多    │0.5   │¥60k │¥65k │+¥2,500  │45%  │   │
│  │ETH    │空    │10    │¥3.2k│¥3.0k│+¥2,000  │32%  │   │
│  │SOL    │多    │50    │¥150 │¥140 │-¥500    │23%  │   │
│  └───────┴──────┴──────┴──────┴──────┴──────────┴──────┘   │
│  分页控件                                                     │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 筛选栏

**方向筛选**：
- 类型：`el-segmented`
- 选项：`全部` / `多头` / `空头`
- 默认：`全部`

**交易对搜索**：
- 类型：`el-input` + `Search` 图标
- 宽度：200px
- placeholder：「搜索交易对」

**排序方式**：
- 类型：`el-select`
- 选项：浮动盈亏↓ / 浮动盈亏↑ / 数量↓ / 交易对 A-Z
- 默认：浮动盈亏↓

### 3.3 PositionTable 详细列（比仪表板多一列「占比」）

| 列 | 宽度 | 对齐 | 内容 |
|----|------|------|------|
| 交易对 | flex | 左 | symbol 加粗 |
| 方向 | 80px | 中 | SideTag |
| 数量 | 120px | 右 | quantity 等宽 |
| 均价 | 120px | 右 | avg_price 等宽 |
| 现价 | 120px | 右 | current_price 等宽 |
| 浮动盈亏 | 160px | 右 | UnrealizedPnlCell |
| 占比 | 80px | 右 | 占总权益百分比 |

**占比列**：
- 数值 + 迷你进度条（背景 `--color-border`，填充跟随方向色）
- 高度 4px，宽度 60px

---

## 4. 页面3：策略绩效对比页 /portfolio/performance

> 多策略并排对比，突出差异

### 4.1 页面布局

```
┌──────────────────────────────────────────────────────────────┐
│  PerformanceHeader                                            │
│  策略绩效 · 时间范围 · 对比模式切换                            │
├──────────────────────────────────────────────────────────────┤
│  PerformanceOverviewCards (3列)                               │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐               │
│  │ 最大回撤   │ │ 夏普率     │ │ 胜率       │               │
│  │ -12.34%    │ │ 1.85       │ │ 62.5%     │               │
│  └────────────┘ └────────────┘ └────────────┘               │
├──────────────────────────────────────────────────────────────┤
│  StrategyComparisonChart (全宽)                               │
│  ┌──────────────────────────────────────────────────────────┐│
│  │  多策略权益曲线叠加图 (ECharts multi-line)               ││
│  └──────────────────────────────────────────────────────────┘│
├──────────────────────────────────────────────────────────────┤
│  StrategyComparisonTable (全宽)                               │
│  ┌──────┬──────┬──────┬──────┬──────┬──────┬──────┐        │
│  │策略  │盈亏  │盈亏率│回撤  │Sharpe│交易数│胜率  │        │
│  └──────┴──────┴──────┴──────┴──────┴──────┴──────┘        │
└──────────────────────────────────────────────────────────────┘
```

### 4.2 StrategyComparisonChart — 多策略权益曲线叠加

| 属性 | 规格 |
|------|------|
| 高度 | 360px |
| 类型 | ECharts 多线图（无面积填充） |
| 颜色 | 每策略分配不同色 `[#7170ff', '#10b981', '#f5a623', '#60a5fa', '#e5484d']` |
| 图例 | 顶部，可点击切换显示/隐藏 |
| Tooltip | 显示所有策略在该时间点的权益值 |

---

## 5. 组件状态总矩阵

### 5.1 EquityOverviewCards 状态

| 状态 | 总资产 | 累计盈亏 | 当日盈亏 | 收益率 |
|------|--------|---------|---------|--------|
| **默认** | 显示数值 | 数值+颜色 | 数值+颜色 | 数值+颜色 |
| **加载中** | 骨架屏 120px | 骨架屏 100px | 骨架屏 100px | 骨架屏 80px |
| **空数据** | ¥0.00 | ¥0.00 灰色 | ¥0.00 灰色 | 0.00% 灰色 |
| **WS更新** | 数值闪烁(2s) | 数值闪烁 | 数值闪烁 | 数值闪烁 |

**数值闪烁动画**：
- 收到 WS 推送时，数值背景 `rgba(113,112,255,0.15)` 渐隐 2s
- CSS: `@keyframes value-flash { from { background: rgba(113,112,255,0.15); } to { background: transparent; } }`

### 5.2 PositionSummaryTable 状态

| 状态 | 表现 |
|------|------|
| **默认** | 分页数据，可排序 |
| **加载中** | `el-table` v-loading，5行骨架 |
| **空数据** | EmptyPortfolio 组件（图标+文字「暂无持仓」） |
| **筛选无结果** | 「未找到匹配持仓」 |
| **WS更新行** | 行背景闪烁对应方向色 2s |
| **排序中** | 列头排序图标旋转 |

### 5.3 StrategyComparisonTable 状态

| 状态 | 表现 |
|------|------|
| **默认** | 分页数据，可排序 |
| **加载中** | v-loading 骨架 |
| **空数据** | 「暂无策略数据」Empty 组件 |
| **单策略** | 表格显示 1 行，叠加图显示 1 线 |

### 5.4 EquityCurveChart 状态

| 状态 | 表现 |
|------|------|
| **默认** | 面积图 + 十字线 |
| **加载中** | ECharts loading 动画 |
| **空数据** | 虚线 + 居中文字 |
| **粒度切换** | 图表重绘动画 300ms |
| **区间变更** | 图表重绘动画 300ms |

---

## 6. 路由设计

| 路由 | 组件 | 描述 |
|------|------|------|
| `/portfolio` | PortfolioDashboard | 组合总览仪表板 |
| `/portfolio/positions` | PositionSummary | 持仓汇总详细页 |
| `/portfolio/performance` | PerformanceComparison | 策略绩效对比页 |

导航入口：
- 侧边栏「组合权益」菜单项
- 图标：`PieChart` (Element Plus Icons)

---

## 7. API 数据流

### 7.1 页面初始化

```
/portfolio 加载:
  1. GET /api/v1/portfolio/summary     → EquityOverviewCards
  2. GET /api/v1/portfolio/equity_curve → EquityCurveChart
  3. GET /api/v1/portfolio/positions    → PositionSummaryTable
  4. GET /api/v1/portfolio/performance  → PerformanceMetricsCards + StrategyComparisonTable
```

### 7.2 筛选/分页操作

```
持仓翻页:
  GET /api/v1/portfolio/positions?page=2&size=10

方向筛选:
  GET /api/v1/portfolio/positions?side=long

粒度切换:
  GET /api/v1/portfolio/equity_curve?granularity=hour

日期变更:
  GET /api/v1/portfolio/equity_curve?start_date=2026-04-14&end_date=2026-05-14
```

### 7.3 composable 设计

```typescript
// composables/usePortfolioSummary.ts
export function usePortfolioSummary() {
  const summary = ref<PortfolioSummary | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetch(userId?: string) { ... }
  function applyWsUpdate(data: Partial<PortfolioSummary>) { ... }

  return { summary, loading, error, fetch, applyWsUpdate }
}

// composables/usePortfolioPositions.ts
export function usePortfolioPositions() {
  const positions = ref<PaginatedPositions | null>(null)
  const loading = ref(false)
  const filters = reactive({ symbol: '', side: '' as '' | 'long' | 'short' })
  const pagination = reactive({ page: 1, size: 10 })

  async function fetch() { ... }
  function applyWsUpdate(position: PortfolioPosition) { ... }

  return { positions, loading, filters, pagination, fetch, applyWsUpdate }
}

// composables/useEquityCurve.ts
export function useEquityCurve() {
  const curve = ref<EquityCurve | null>(null)
  const loading = ref(false)
  const granularity = ref<'hour' | 'day' | 'week'>('day')
  const dateRange = ref<[string, string]>([defaultStart, defaultEnd])

  async function fetch() { ... }

  return { curve, loading, granularity, dateRange, fetch }
}

// composables/usePortfolioPerformance.ts
export function usePortfolioPerformance() {
  const performance = ref<{ strategies: StrategyPerformance[] } | null>(null)
  const loading = ref(false)

  async function fetch(userId?: string) { ... }

  return { performance, loading, fetch }
}
```

---

## 8. WebSocket 实时更新机制

### 8.1 频道设计

| 频道 | 消息类型 | 推送频率 | 影响 |
|------|---------|---------|------|
| `portfolio:summary:{user_id}` | PortfolioSummary 字段增量 | 成交时/1min 轮询 | 四大指标卡片值更新 |
| `portfolio:position:{user_id}` | PortfolioPosition 全量 | 成交时 | 持仓行新增/更新/删除 |
| `portfolio:equity:{user_id}` | EquityCurvePoint | 1min 定时 | 权益曲线追加数据点 |

### 8.2 更新策略

1. **Summary 更新**：收到 WS → 合并字段 → 触发闪烁动画 → 2s 后恢复
2. **Position 更新**：收到 WS → 按 symbol+side 查找行 → 更新数值 → 行背景闪烁
3. **Equity 追加**：收到 WS → ECharts `appendData` → 无全量重绘

### 8.3 频道生命周期

- 进入 `/portfolio` → subscribe 三个频道
- 离开 `/portfolio` → unsubscribe 三个频道
- Tab 切换 → 保持连接，暂停渲染
- 网络断开 → 自动重连 + 重连后全量刷新

---

## 9. 响应式适配方案

### 9.1 桌面端 (≥1200px)

- 四卡片 4 列均分
- 持仓表 + 绩效面板 60/40 分栏
- 表格所有列可见

### 9.2 平板端 (768px - 1199px)

- 四卡片 2×2 网格
- 底部改为单栏（持仓表全宽 → 绩效面板全宽）
- 表格隐藏「均价」列

### 9.3 移动端 (<768px)

- 四卡片纵向堆叠，单列
- 底部单栏
- 表格改为卡片列表模式（每行一张卡片）
- 权益曲线高度 240px
- 筛选栏折叠为 `el-drawer`

---

## 10. 交互细节与动画

### 10.1 卡片悬停

```css
.metric-card {
  transition: transform 0.2s ease, border-color 0.2s ease, box-shadow 0.2s ease;
}
.metric-card:hover {
  transform: translateY(-2px);
  border-color: var(--color-border-hover);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}
```

### 10.2 数值闪烁

```css
@keyframes value-flash-positive {
  from { background-color: rgba(103, 194, 58, 0.15); }
  to { background-color: transparent; }
}
@keyframes value-flash-negative {
  from { background-color: rgba(245, 108, 108, 0.15); }
  to { background-color: transparent; }
}
.flash-positive { animation: value-flash-positive 2s ease-out; }
.flash-negative { animation: value-flash-negative 2s ease-out; }
```

### 10.3 刷新按钮旋转

```css
@keyframes spin-once {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.refresh-spinning .el-icon { animation: spin-once 0.6s ease; }
```

### 10.4 表格行闪烁

```css
@keyframes row-flash-long {
  from { background-color: rgba(103, 194, 58, 0.15); }
  to { background-color: transparent; }
}
@keyframes row-flash-short {
  from { background-color: rgba(245, 108, 108, 0.15); }
  to { background-color: transparent; }
}
```

### 10.5 加载骨架屏

```css
.skeleton-line {
  background: linear-gradient(90deg, #191a1b 25%, #212223 50%, #191a1b 75%);
  background-size: 200% 100%;
  animation: skeleton-shimmer 1.5s infinite;
  border-radius: 4px;
}
@keyframes skeleton-shimmer {
  from { background-position: 200% 0; }
  to { background-position: -200% 0; }
}
```

---

## 11. 空状态与错误处理 UX

### 11.1 空状态

| 场景 | 组件 | 图标 | 文案 | 操作 |
|------|------|------|------|------|
| 新用户无组合数据 | EmptyPortfolio | `PieChart` 64px 灰色 | 「还没有组合数据」<br/>「开始交易后，这里会显示您的持仓和收益」 | 「前往交易」按钮 → /trade |
| 无持仓 | EmptyPosition | `Wallet` 64px 灰色 | 「当前没有持仓」 | — |
| 无策略 | EmptyStrategy | `DataLine` 64px 灰色 | 「暂无策略绩效数据」 | — |

### 11.2 错误状态

| 场景 | 表现 | 操作 |
|------|------|------|
| API 401 | 跳转登录页 | — |
| API 403 | `el-result` 403 页 | 「返回首页」 |
| API 500 | `el-result` 500 页 + 错误信息 | 「重试」按钮 |
| WS 断开 | 顶部横幅「实时数据连接断开，正在重连...」 | 手动「重新连接」 |
| 超时无响应 | 卡片/表格 loading 超过 10s → 「加载超时」 | 「重新加载」 |

### 11.3 数据精度

- 所有金额使用 `string` 类型传输（ADR B4 契约）
- 前端使用 `decimal.js` 或 `BigInt` 计算，避免浮点误差
- 显示时统一格式化函数 `formatMoney(value: string): string`
- 百分比显示 `formatPercent(value: string): string`
