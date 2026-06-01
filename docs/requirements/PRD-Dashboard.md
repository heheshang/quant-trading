---
status: WIP
date: 20260601
---

# PRD-Dashboard-20260601 — 数据仪表盘

## 1. 功能概述

Dashboard 模块为交易员提供全局统计概览和盈亏历史曲线，帮助用户快速掌握账户健康状况、策略表现和风险水平。

**目标用户**：量化交易员、账户管理员

---

## 2. 用户故事

- **US-D1**：作为交易员，我希望在打开 Dashboard 时看到总资产、当日盈亏、持仓数量等核心指标，以便快速了解账户状态
- **US-D2**：作为交易员，我希望查看近 7/30/90 天的累计盈亏曲线，以便评估策略在不同时间维度的表现
- **US-D3**：作为管理员，我希望查看所有用户的汇总统计（按用户分组），以便进行风控监督

---

## 3. API 端点

### 3.1 GET /api/v1/dashboard/stats

**描述**：获取 Dashboard 核心统计数据（总览指标）

**认证**：JWT

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": {
    "total_equity": "100000.00",
    "total_pnl": "1234.56",
    "total_pnl_pct": "1.23",
    "today_pnl": "56.78",
    "today_pnl_pct": "0.06",
    "open_positions": 5,
    "total_orders": 128,
    "winning_rate": "0.62",
    "max_drawdown": "-2345.67",
    "sharpe_ratio": "1.45",
    "updated_at": "2026-06-01T12:00:00Z"
  },
  "message": "success"
}
```

**响应字段说明**：
| 字段 | 类型 | 说明 |
|------|------|------|
| total_equity | string | 当前总权益（USDT） |
| total_pnl | string | 累计盈亏金额 |
| total_pnl_pct | string | 累计收益率（%） |
| today_pnl | string | 当日盈亏 |
| today_pnl_pct | string | 当日收益率（%） |
| open_positions | int | 持仓数量 |
| total_orders | int | 累计成交订单数 |
| winning_rate | string | 胜率（0~1） |
| max_drawdown | string | 最大回撤 |
| sharpe_ratio | string | 夏普比率 |

---

### 3.2 GET /api/v1/dashboard/pnl

**描述**：获取盈亏历史时间序列数据

**认证**：JWT

**Query 参数**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| range | string | 否 | 时间范围：`7d`（默认）/ `30d` / `90d` |

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": {
    "range": "30d",
    "points": [
      {
        "date": "2026-05-01",
        "pnl": "123.45",
        "pnl_pct": "0.12",
        "equity": "100123.45"
      }
    ],
    "summary": {
      "total_pnl": "4567.89",
      "total_pnl_pct": "4.57",
      "max_drawdown": "-890.12",
      "best_day": "2026-05-15",
      "worst_day": "2026-05-20"
    }
  },
  "message": "success"
}
```

---

## 4. 数据库模型

### 4.1 表：`dashboard_stats`（可选快照表）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| user_id | UUID | 用户ID |
| total_equity | DECIMAL(20,8) | 总权益 |
| total_pnl | DECIMAL(20,8) | 累计盈亏 |
| today_pnl | DECIMAL(20,8) | 当日盈亏 |
| open_positions | INT | 持仓数 |
| updated_at | TIMESTAMP | 更新时间 |

> **说明**：主要统计数据通过实时查询 `positions`、`orders` 表聚合计算，无需持久化。

---

## 5. 业务流程

```
用户打开 Dashboard
    │
    ├─→ GET /api/v1/dashboard/stats
    │       ├─→ 查询 positions 表（open_positions）
    │       ├─→ 查询 orders 表（total_orders, winning_rate）
    │       ├─→ 查询 portfolio 表（total_equity, total_pnl）
    │       └─→ 返回聚合结果
    │
    └─→ GET /api/v1/dashboard/pnl?range=30d
            ├─→ 查询 ticker_snapshots 或 portfolio_history 表
            ├─→ 按日期聚合计算每日 PnL
            └─→ 返回时间序列 points[]
```

---

## 6. 边界条件

- 用户无任何持仓/订单时，`stats` 返回全 0 值，不报错
- `range` 参数非法时默认使用 `7d`
- 数据聚合时若某日无快照，用前一日权益补充

---

## 7. 验收标准（Gherkin 格式）

```gherkin
Feature: Dashboard 数据展示

  Scenario: 用户打开 Dashboard 看到统计数据
    Given 用户已登录
    When 用户访问 Dashboard 页面
    Then 系统显示总权益、累计盈亏、当日盈亏、持仓数量
    And 系统显示胜率、夏普比率、最大回撤

  Scenario: 用户查看 30 天盈亏曲线
    Given 用户已登录
    When 用户切换时间范围为 "30d"
    Then 系统显示 30 天的 PnL 曲线
    And 曲线包含每日数据点和汇总统计

  Scenario: 新用户 Dashboard 显示零值
    Given 用户刚注册无任何交易记录
    When 用户访问 Dashboard
    Then 所有统计指标显示为 0
    And 不显示错误信息

  Scenario: Dashboard 数据实时刷新
    Given 用户在 Dashboard 页面
    When 用户完成一笔新交易
    Then Dashboard 统计数字在 5 秒内自动更新
```
