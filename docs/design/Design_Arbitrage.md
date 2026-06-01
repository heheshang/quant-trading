# 设计规格说明书：套利管理 Arbitrage Management

> 设计师：Designer | 版本 v1.0 | 日期：2026-06-01
> 设计灵感：Linear.app 暗色模式 + 量化套利监控风格
> 框架：Element Plus + Vue 3 + TypeScript + ECharts
> 源代码库：/home/ssk/workspace/quant-trading

---

## 目录

1. [页面概述与目标](#1-页面概述与目标)
2. [布局结构](#2-布局结构)
3. [核心交互流程](#3-核心交互流程)
4. [状态设计](#4-状态设计)
5. [API 调用说明](#5-api-调用说明)
6. [样式规范](#6-样式规范)
7. [响应式设计](#7-响应式设计)

---

## 1. 页面概述与目标

### 1.1 页面定位

套利管理页面用于配置和管理跨交易所套利交易对，监控套利信号和交易统计。交易员可创建套利对、设置交易参数、查看套利收益统计。

### 1.2 核心功能

- **套利对管理**：创建、编辑、删除套利对
- **套利信号监控**：实时显示跨交易所价差信号
- **交易统计**：套利次数、成功率、累计收益
- **参数配置**：价差阈值、手数、执行模式

### 1.3 套利对概念

套利对由两个交易所的同种交易对组成，如：
- Binance BTC/USDT ↔ OKX BTC/USDT
- 价差达到阈值时触发套利信号

---

## 2. 布局结构

### 2.1 整体布局

```
┌─────────────────────────────────────────────────────────┐
│ Header: 页面标题「套利管理」+ [+ 新建套利对]               │
├─────────────────────────────────────────────────────────┤
│ 信号监控面板（实时价差图表）                               │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ ECharts 价差实时图表                                 │ │
│ │ X轴：时间  Y轴：价差（%）                            │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ 套利统计卡片行                                            │
│ ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐ │
│ │ 总套利次数 │ │ 成功次数  │ │ 成功率    │ │ 累计收益   │ │
│ └───────────┘ └───────────┘ └───────────┘ └───────────┘ │
├─────────────────────────────────────────────────────────┤
│ 套利对列表                                                │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ 表格：套利对/交易所A/交易所B/当前价差/状态/操作        │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### 2.2 区域划分

| 区域 | 组件 | 描述 |
|------|------|------|
| Header | ArbitrageHeader | 标题 + 新建按钮 |
| 信号监控图表 | SpreadChart | 实时价差 ECharts |
| 统计卡片 | ArbitrageStatsCards | 4个统计指标 |
| 套利对表格 | ArbitragePairTable | 套利对列表 |
| 新建/编辑对话框 | ArbitragePairDialog | 表单弹窗 |
| 套利详情抽屉 | ArbitrageDetailDrawer | 详细统计 |

---

## 3. 核心交互流程

### 3.1 创建套利对

1. 点击「新建套利对」按钮
2. 弹出对话框，填写：
   - 交易对（如 BTC/USDT）
   - 交易所 A + 交易对
   - 交易所 B + 交易对
   - 价差阈值（%）
   - 每笔交易数量
   - 执行模式（自动/手动）
3. 调用 `POST /api/arbitrage/pairs`
4. 成功后刷新列表

### 3.2 监控套利信号

1. WebSocket 订阅价差数据
2. 价差 >= 阈值时高亮显示
3. 点击「执行套利」按钮手动触发
4. 或开启自动模式自动下单

### 3.3 查看套利统计

1. 点击套利对行「详情」按钮
2. 右侧抽屉显示详细统计：
   - 历史套利记录
   - 收益曲线
   - 成功率趋势

### 3.4 启用/暂停套利对

1. 点击「启用」/「暂停」切换按钮
2. 调用 `PUT /api/arbitrage/pairs/:id/status`
3. 状态变更刷新表格

---

## 4. 状态设计

### 4.1 加载状态

- 页面初始：骨架屏 + 图表占位
- 实时数据：loading spinner

### 4.2 空状态

- 无套利对：「暂无套利对，点击新建」
- 无信号数据：「等待价差信号...」

### 4.3 信号状态

| 状态 | 颜色 | 含义 |
|------|------|------|
| 价差正常 | `--color-success` | < 阈值 |
| 价差接近 | `--color-warning` | 70%-99%阈值 |
| 价差触发 | `--color-error` | >= 阈值 |
| 套利执行中 | `--color-accent` | 闪烁动画 |

### 4.4 套利对状态

| 状态 | 样式 |
|------|------|
| 运行中 | 绿色标签 |
| 已暂停 | 灰色标签 |
| 异常 | 红色标签 |

---

## 5. API 调用说明

### 5.1 接口列表

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/arbitrage/pairs` | 获取套利对列表 |
| GET | `/api/arbitrage/pairs/:id` | 获取单个套利对 |
| POST | `/api/arbitrage/pairs` | 创建套利对 |
| PUT | `/api/arbitrage/pairs/:id` | 更新套利对 |
| DELETE | `/api/arbitrage/pairs/:id` | 删除套利对 |
| PUT | `/api/arbitrage/pairs/:id/status` | 启用/暂停 |
| POST | `/api/arbitrage/pairs/:id/execute` | 手动执行套利 |
| GET | `/api/arbitrage/stats` | 获取统计 |
| GET | `/api/arbitrage/signals` | 获取信号历史 |
| WS | `/ws/arbitrage/spread` | 实时价差数据 |

### 5.2 数据类型

```typescript
interface ArbitragePair {
  id: string
  symbol: string
  exchange_a: 'binance' | 'okx' | 'gateio' | 'bybit'
  exchange_b: 'binance' | 'okx' | 'gateio' | 'bybit'
  symbol_a: string
  symbol_b: string
  spread_threshold: number       // 百分比
  quantity: number
  mode: 'auto' | 'manual'
  status: 'active' | 'paused' | 'error'
  current_spread: number | null
  last_signal_at: string | null
  stats: ArbitrageStats
  created_at: string
}

interface ArbitrageStats {
  total_trades: number
  successful_trades: number
  success_rate: number
  total_profit: string
  created_at: string
}

interface SpreadSignal {
  pair_id: string
  spread: number
  timestamp: string
}
```

---

## 6. 样式规范

### 6.1 设计 Token

| Token | 值 | 用途 |
|-------|-----|------|
| `--color-bg` | `#08090a` | 背景 |
| `--color-surface` | `#191a1b` | 卡片背景 |
| `--color-surface-elevated` | `#212223` | 对话框背景 |
| `--color-text-primary` | `#f7f8f8` | 主文字 |
| `--color-text-secondary` | `#d0d6e0` | 次要文字 |
| `--color-accent` | `#7170ff` | 主色调 |
| `--color-border` | `rgba(255,255,255,0.08)` | 边框 |
| `--color-success` | `#10b981` | 成功 |
| `--color-error` | `#e5484d` | 错误/触发 |
| `--color-warning` | `#f5a623` | 警告 |

### 6.2 图表规范

- 图表背景：透明
- 线色：`--color-accent`
- 触发线：虚线 `--color-error`
- 区域填充：渐变 20% → 0%

### 6.3 套利对卡片

- 卡片高度：自适应
- 内边距：16px
- 状态标签右上角

---

## 7. 响应式设计

### 7.1 断点

| 断点 | 宽度 | 布局 |
|------|------|------|
| Desktop | >= 1280px | 图表 + 表格双列 |
| Laptop | 1024-1279px | 图表上方 + 表格下方 |
| Tablet | 768-1023px | 图表可折叠 + 简化表格 |
| Mobile | < 768px | 卡片式展示 |

### 7.2 移动端适配

- 图表高度减少
- 统计卡片 2x2 排列
- 表格水平滚动
- 对话框全屏
