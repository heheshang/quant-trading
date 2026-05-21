# PRD — P2-F4 订单/账单 Excel 导出

## 1. 功能概述

| 字段 | 内容 |
|------|------|
| 功能名称 | 交易数据导出（CSV/Excel） |
| 功能编号 | P2-F4 |
| 类型 | 后端 API + 前端 UI |
| 优先级 | P2 |
| 描述 | 导出订单历史、成交记录、账户账单为 CSV 或 Excel 格式 |
| 所需工时 | 2天（后端1天 + 前端0.5天 + 测试0.5天） |

## 2. 业务背景

用户需要将交易数据导出到本地进行二次分析、对账、报表制作。现有多渠道数据查询，但缺少批量导出能力。

## 3. 用户故事

**作为** 交易员/运营
**我希望** 一键导出订单历史和账户账单为 CSV/Excel
**以便** 进行对账、生成报表、离线分析

**作为** 管理员
**我希望** 导出完整交易记录
**以便** 合规审计和数据备份

## 4. 技术方案

### 4.1 后端实现

**新增 API 端点**：

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/exports/orders` | GET | 导出订单列表 |
| `/api/v1/exports/trades` | GET | 导出成交记录 |
| `/api/v1/exports/account` | GET | 导出账户快照 |

**Query 参数**：

| 参数 | 类型 | 默认 | 描述 |
|------|------|------|------|
| `format` | string | `csv` | 格式：csv 或 xlsx |
| `status` | string | - | 订单状态过滤（仅 orders） |
| `symbol` | string | - | 交易对过滤 |
| `start_time` | ISO8601 | - | 开始时间 |
| `end_time` | ISO8601 | - | 结束时间 |
| `page` | int | 1 | 页码（分页导出） |
| `page_size` | int | 5000 | 每页条数 |

**导出字段 — 订单 (OrderResponse)**：
- id, symbol, side, order_type, price, quantity, filled_qty, status, created_at, updated_at

**导出字段 — 成交 (TradeResponse)**：
- id, order_id, symbol, side, price, quantity, fee, fee_token, realized_pnl, created_at

**导出字段 — 账户 (AccountResponse)**：
- total_equity, available, position_value, unrealized_pnl, realized_pnl, total_fee, margin_used, leverage, updated_at

**技术选型**：
- CSV：`csv` crate（Rust 标准，轻量）
- Excel：`calamine` crate（支持 .xlsx/.ods，无需系统 Excel）
- 分页：复用现有 ListOrdersQuery / ListTradesQuery，追加 page + page_size

**实现位置**：
- `backend/src/handlers/export.rs`（新文件）
- `backend/src/services/export.rs`（复用现有 list_orders/trades/positions/account 逻辑）

### 4.2 前端实现

**入口页面**：
1. `OrderManagementView.vue` — 订单管理页，Tab: 活跃/历史
2. `PortfolioView.vue` — 账户/资产页

**UI 组件**：
- 导出按钮：Dropdown（CSV / Excel 两个选项）
- 位置：页面右上角操作区

**实现方式**：
- 调用 API，format=csv|xlsx
- 响应类型：`Content-Type: application/octet-stream`
- 前端：Blob → URL.createObjectURL → `<a download>` 触发下载

## 5. 数据流程

```
User 点击「导出 CSV」
  → Frontend: GET /api/v1/exports/orders?format=csv
  → Backend: list_orders(pagination) → 写入 CSV buffer
  → Backend: StreamResponse (Content-Disposition: attachment; filename=orders_20260521.csv)
  → Frontend: Blob.download()
```

## 6. 验收条件

### 后端
- [ ] `GET /api/v1/exports/orders?format=csv` → 200 + CSV 文件下载
- [ ] `GET /api/v1/exports/orders?format=xlsx` → 200 + xlsx 文件下载
- [ ] `GET /api/v1/exports/trades?format=csv` → 200 + CSV
- [ ] `GET /api/v1/exports/account?format=csv` → 200 + CSV
- [ ] 参数过滤：status/symbol/time_range 正确过滤
- [ ] 大数据量（>5000条）分页导出正常
- [ ] 单元测试覆盖

### 前端
- [ ] 导出按钮存在于订单管理页
- [ ] 导出按钮存在于账户页
- [ ] CSV 和 Excel 两个选项可用
- [ ] 点击后浏览器触发下载
- [ ] Vitest 测试覆盖

## 7. 风险

| 风险 | 缓解 |
|------|------|
| 数据量大 OOM | 分页 5000 条/页，流式写入 |
| Excel 格式兼容 | calamine 生成标准 xlsx，Numbers/Excel 均可打开 |
| 前端下载大文件 | Blob URL 方式，浏览器自动处理 |

## 8. 非目标

- 不支持 PDF 导出（后续 P2-F4 扩展）
- 不支持定时自动导出（P2-F4 扩展）
- 不支持导出回测结果（已有单独回测报告）
