# 设计规格说明书：K线数据导入 Kline Import

> 设计师：Designer | 版本 v1.0 | 日期：2026-06-01
> 设计灵感：Linear.app 暗色模式 + 数据导入向导风格
> 框架：Element Plus + Vue 3 + TypeScript
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

K线数据导入页面用于批量导入历史K线数据到系统，支持 CSV 文件导入、API 拉取和交易所直连三种方式。

### 1.2 核心功能

- **CSV 导入**：上传 CSV 文件映射字段导入
- **API 拉取**：配置数据源 API 自动拉取
- **交易所向导**：选择交易所 + 交易对 + 日期范围直接导入
- **导入进度**：实时显示导入进度和结果
- **历史记录**：查看历史导入任务

### 1.3 支持的交易所

- Binance
- OKX
- Gate.io
- Bybit

---

## 2. 布局结构

### 2.1 整体布局

```
┌─────────────────────────────────────────────────────────┐
│ Header: 页面标题「K线导入」                               │
├─────────────────────────────────────────────────────────┤
│ 导入方式 Tab                                             │
│ [CSV导入] [API拉取] [交易所向导]                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  [CSV 导入 Tab]                                         │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ 文件上传区域（拖拽 + 点击上传）                       │ │
│  │ 支持 .csv 文件，最大 100MB                           │ │
│  └─────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ 字段映射配置                                         │ │
│  │ 时间映射 / 开高低收卷映射                            │ │
│  └─────────────────────────────────────────────────────┘ │
│                                                         │
│  [API 拉取 Tab]                                         │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ 数据源配置：URL / 认证 Token                         │ │
│  │ 交易对 / 周期 / 日期范围                             │ │
│  └─────────────────────────────────────────────────────┘ │
│                                                         │
│  [交易所向导 Tab]                                        │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ Step 1: 选择交易所                                   │ │
│  │ Step 2: 选择交易对                                   │ │
│  │ Step 3: 选择周期                                     │ │
│  │ Step 4: 选择日期范围                                 │ │
│  │ [开始导入]                                          │ │
│  └─────────────────────────────────────────────────────┘ │
│                                                         │
├─────────────────────────────────────────────────────────┤
│ 导入历史记录表格                                          │
└─────────────────────────────────────────────────────────┘
```

### 2.2 区域划分

| 区域 | 组件 | 描述 |
|------|------|------|
| Header | KlineImportHeader | 标题 |
| 导入方式 Tab | ImportMethodTabs | 3种导入方式 |
| CSV 导入面板 | CsvImportPanel | 文件上传 + 字段映射 |
| API 拉取面板 | ApiImportPanel | API 配置 |
| 交易所向导面板 | ExchangeWizardPanel | 分步向导 |
| 导入进度 | ImportProgress | 进度条 + 日志 |
| 导入历史 | ImportHistoryTable | 历史记录 |

---

## 3. 核心交互流程

### 3.1 CSV 导入流程

1. 选择/拖拽 CSV 文件到上传区域
2. 系统解析文件头，自动匹配字段
3. 用户确认/调整字段映射：
   - 时间戳
   - 开盘价
   - 最高价
   - 最低价
   - 收盘价
   - 成交量
4. 预览前10条数据
5. 点击「开始导入」
6. 显示导入进度条
7. 完成后显示结果统计

### 3.2 API 拉取流程

1. 填写数据源 URL
2. 如需认证填写 Token
3. 填写目标交易对和周期
4. 选择日期范围
5. 点击「测试连接」验证
6. 验证成功后点击「开始拉取」
7. 实时显示拉取进度

### 3.3 交易所向导流程

1. Step 1：选择交易所（Binance/OKX/Gate.io/Bybit）
2. Step 2：选择交易对（搜索下拉）
3. Step 3：选择K线周期
4. Step 4：选择日期范围（起始-结束）
5. 点击「开始导入」
6. 显示导入进度
7. 支持后台导入

### 3.4 导入进度监控

1. 实时进度条（百分比）
2. 当前处理行数 / 总行数
3. 导入日志滚动显示
4. 支持取消导入
5. 完成后显示成功/失败统计

---

## 4. 状态设计

### 4.1 加载状态

- 文件解析中：上传区域 spinner
- 字段映射中：映射表单 skeleton
- 导入中：进度条 + 禁止操作

### 4.2 空状态

- 无导入历史：「暂无导入记录」
- 未选择文件：上传区域默认状态

### 4.3 错误状态

| 状态 | 提示 |
|------|------|
| 文件格式错误 | 「仅支持 CSV 格式」 |
| 文件过大 | 「文件超过 100MB 限制」 |
| 字段映射缺失 | 红色标记必填字段 |
| API 连接失败 | Toast + 错误详情 |
| 导入失败 | 失败行数 + 原因 |

### 4.4 导入状态

| 状态 | 样式 |
|------|------|
| 待导入 | 灰色 |
| 导入中 | 蓝色 + 进度条 |
| 成功 | 绿色 + 对勾 |
| 部分成功 | 黄色 + 感叹号 |
| 失败 | 红色 + 叉 |

---

## 5. API 调用说明

### 5.1 接口列表

| 方法 | 端点 | 描述 |
|------|------|------|
| POST | `/api/klines/import/csv` | CSV 文件导入 |
| POST | `/api/klines/import/api` | API 拉取导入 |
| POST | `/api/klines/import/exchange` | 交易所导入 |
| GET | `/api/klines/import/tasks` | 获取导入任务列表 |
| GET | `/api/klines/import/tasks/:id` | 获取任务详情 |
| DELETE | `/api/klines/import/tasks/:id` | 取消导入任务 |
| GET | `/api/exchanges` | 获取交易所列表 |
| GET | `/api/symbols?exchange=xxx` | 获取交易对列表 |

### 5.2 数据类型

```typescript
interface ImportTask {
  id: string
  type: 'csv' | 'api' | 'exchange'
  status: 'pending' | 'running' | 'completed' | 'failed' | 'cancelled'
  source: string
  symbol: string
  interval: string
  total_rows: number
  processed_rows: number
  success_rows: number
  failed_rows: number
  error_message: string | null
  created_at: string
  completed_at: string | null
}

interface CsvImportRequest {
  file_id: string
  field_mapping: {
    timestamp: string
    open: string
    high: string
    low: string
    close: string
    volume: string
  }
  exchange: string
  symbol: string
  interval: string
}

interface ExchangeImportRequest {
  exchange: string
  symbol: string
  interval: string
  start_date: string
  end_date: string
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
| `--color-error` | `#e5484d` | 错误 |
| `--color-warning` | `#f5a623` | 警告 |

### 6.2 上传区域

- 虚线边框：2px dashed `--color-border`
- 悬停：边框变为主色调
- 拖拽中：背景色 `--color-accent-bg`
- 圆角：12px
- 内边距：48px

### 6.3 向导步骤

- 步骤条：横向 4 步
- 当前步骤：数字圆圈 + 主色调
- 完成步骤：绿色对勾
- 未完成步骤：灰色数字

### 6.4 进度条

- 高度：8px
- 圆角：4px
- 背景：`--color-surface`
- 填充：渐变 `--color-accent`

---

## 7. 响应式设计

### 7.1 断点

| 断点 | 宽度 | 布局 |
|------|------|------|
| Desktop | >= 1280px | 完整布局 |
| Laptop | 1024-1279px | 字段映射两列 |
| Tablet | 768-1023px | 向导垂直排列 |
| Mobile | < 768px | 单列 + 全屏弹窗 |

### 7.2 移动端适配

- Tab 改为下拉选择
- 上传区域全宽
- 向导步骤改为垂直步骤条
- 历史记录卡片式展示
