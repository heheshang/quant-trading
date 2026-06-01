# 设计规格说明书：API Key 管理 ApiKeyManagement

> 设计师：Designer | 版本 v1.0 | 日期：2026-06-01
> 设计灵感：Linear.app 暗色模式 + 加密资产管理风格
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

API Key 管理页面用于管理量化交易系统连接的各交易所 API 密钥，包括添加、编辑、删除、标签分类等功能。用户可为自己的交易账户绑定多个交易所的 API Key。

### 1.2 核心功能

- **API Key 列表**：展示所有已绑定的 API Key，支持按交易所筛选
- **添加 API Key**：通过表单输入交易所、API Key、Secret Key、Passphrase 等
- **编辑 API Key**：修改标签、备注、状态
- **删除 API Key**：解除绑定（需二次确认）
- **交易所标签**：Binance / OKX / Gate.io / Bybit 等
- **连接测试**：验证 API Key 有效性

### 1.3 安全要求

- Secret Key 仅在创建/编辑时可见，之后脱敏显示
- 删除前需输入 API Key 前缀确认
- 敏感操作记录审计日志

---

## 2. 布局结构

### 2.1 整体布局

```
┌─────────────────────────────────────────────────────────┐
│ Header: 页面标题「API Key 管理」+ [+ 添加 Key]           │
├─────────────────────────────────────────────────────────┤
│ 交易所筛选标签栏                                          │
│ [全部] [Binance] [OKX] [Gate.io] [Bybit]                │
├─────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────────┐ │
│ │ API Key 表格                                        │ │
│ │ 列：标签/交易所/Key前缀/状态/添加时间/操作            │ │
│ ├─────────────────────────────────────────────────────┤ │
│ │ Key 卡片                                            │ │
│ │ - 交易所图标 + 标签名                                │ │
│ │ - API Key 前缀: BN***8F                             │ │
│ │ - 状态: 正常/异常                                    │ │
│ │ - 添加时间                                          │ │
│ │ - [测试] [编辑] [删除]                              │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### 2.2 区域划分

| 区域 | 组件 | 描述 |
|------|------|------|
| Header | ApiKeyHeader | 标题 + 添加按钮 |
| 交易所筛选 | ExchangeFilterTabs | 交易所标签筛选 |
| API Key 表格 | ApiKeyTable | 列表展示 |
| API Key 卡片 | ApiKeyCard | 卡片展示（移动端） |
| 添加/编辑对话框 | ApiKeyDialog | 表单弹窗 |
| 连接测试对话框 | TestConnectionDialog | 测试结果 |

---

## 3. 核心交互流程

### 3.1 添加 API Key

1. 点击「添加 Key」按钮
2. 弹出添加对话框
3. 选择交易所（必选）
4. 填写标签名称（必选）
5. 填写 API Key（必选）
6. 填写 Secret Key（必选，加密传输）
7. 如需 Passphrase（OKX 等交易所）
8. 点击「测试连接」验证
9. 验证成功后点击「保存」
10. 调用 `POST /api/apikeys`
11. 成功后关闭对话框并刷新列表

### 3.2 编辑 API Key

1. 点击行「编辑」按钮
2. 弹出编辑对话框（标签/备注可改）
3. 保存调用 `PUT /api/apikeys/:id`

### 3.3 删除 API Key

1. 点击「删除」按钮
2. 弹出确认对话框：
   - 显示该 Key 的前缀信息
   - 输入框要求填写 Key 前4位确认
   - 警告文字：「此操作不可撤销」
3. 确认后调用 `DELETE /api/apikeys/:id`
4. 成功后刷新列表

### 3.4 连接测试

1. 点击「测试」按钮
2. 调用 `POST /api/apikeys/:id/test`
3. 显示测试结果：
   - 成功：绿色对勾 + 「连接正常」
   - 失败：红色叉 + 错误信息（权限不足/Key无效等）

### 3.5 交易所筛选

1. 点击交易所标签
2. 调用 `GET /api/apikeys?exchange=binance`
3. 列表仅显示该交易所的 Key

---

## 4. 状态设计

### 4.1 加载状态

- 页面初始：骨架屏
- 测试连接：按钮 loading + 禁用其他按钮

### 4.2 空状态

- 无 API Key：「暂无 API Key，请先添加」
- 空搜索结果：「未找到匹配的 API Key」

### 4.3 错误状态

- 测试失败：红色 Toast + 具体错误
- 保存失败：表单内错误提示
- 网络错误：全局 Toast

### 4.4 表格状态

| 状态 | 样式 |
|------|------|
| 正常 | 绿色圆点 + 「正常」 |
| 异常 | 红色圆点 + 「异常」 |
| 未测试 | 灰色圆点 + 「未测试」 |

---

## 5. API 调用说明

### 5.1 接口列表

| 方法 | 端点 | 描述 |
|------|------|------|
| GET | `/api/apikeys` | 获取 API Key 列表 |
| GET | `/api/apikeys/:id` | 获取单个 API Key |
| POST | `/api/apikeys` | 添加 API Key |
| PUT | `/api/apikeys/:id` | 更新 API Key |
| DELETE | `/api/apikeys/:id` | 删除 API Key |
| POST | `/api/apikeys/:id/test` | 测试连接 |
| GET | `/api/exchanges` | 获取支持的交易所列表 |

### 5.2 数据类型

```typescript
interface ApiKey {
  id: string
  user_id: string
  exchange: 'binance' | 'okx' | 'gateio' | 'bybit'
  label: string
  api_key: string              // 仅返回前缀，如 BN***8F
  status: 'active' | 'error' | 'untested'
  last_tested_at: string | null
  last_error: string | null
  created_at: string
  updated_at: string
}

interface CreateApiKeyRequest {
  exchange: string
  label: string
  api_key: string
  secret_key: string
  passphrase?: string
}

interface Exchange {
  id: string
  name: string
  code: string
  icon: string
  required_fields: string[]     // ['api_key', 'secret', 'passphrase']
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
| `--color-success` | `#10b981` | 成功/正常 |
| `--color-error` | `#e5484d` | 错误/异常 |
| `--color-warning` | `#f5a623` | 警告 |

### 6.2 交易所图标

- Binance：黄色 #F3BA2F
- OKX：蓝色 #121212，白色文字
- Gate.io：橙色 #EA4E1A
- Bybit：黄色 #F7A600

### 6.3 表单规范

- 输入框高度：40px
- 标签字号：14px / font-weight 500
- 必填标记：红色星号
- Secret 输入框：右侧显示/隐藏切换图标

---

## 7. 响应式设计

### 7.1 断点

| 断点 | 宽度 | 布局 |
|------|------|------|
| Desktop | >= 1280px | 完整表格 + 横向卡片 |
| Laptop | 1024-1279px | 表格滚动 |
| Tablet | 768-1023px | 表格简化为卡片 |
| Mobile | < 768px | 纯卡片列表 |

### 7.2 移动端适配

- 交易所筛选改为下拉选择
- 卡片显示交易所图标 + 标签 + 状态
- 操作按钮折叠到「更多」菜单
- 添加按钮固定在底部
