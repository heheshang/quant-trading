# PRD: Portfolio 组合权益管理模块

**版本**: v1.0
**状态**: 已完成
**日期**: 2026-05-14
**负责人**: PM

---

## 1. 功能概述

Portfolio 组合权益模块为交易员提供多策略组合的**权益总览、持仓汇总、绩效分析**能力，帮助用户快速掌握整体账户健康状况。

### 目标用户
- 量化交易员（操盘多策略）
- 账户管理员（查看所有组合）

---

## 2. 功能范围

### P0（必须）
1. **组合权益总览** — 总资产、累计盈亏、当日盈亏、收益率
2. **持仓汇总** — 按交易对分组的持仓（多头/空头方向、数量、均价、当前价、浮动盈亏）
3. **策略绩效列表** — 各策略收益率、最大回撤、交易次数

### P1（应该）
4. **权益曲线** — 每日/每小时组合权益走势
5. **风控指标** — Sharpe Ratio、最大回撤、盈亏比

### P2（可以）
6. 多策略对比柱状图

---

## 3. API 端点

### 3.1 GET /api/v1/portfolio/summary
**描述**: 获取组合权益汇总

**Query 参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| user_id | string (UUID) | 否 | 不传则返回当前用户组合 |

**响应**:
```json
{
  "code": 0,
  "data": {
    "total_equity": "100000.00",
    "daily_pnl": "1234.56",
    "daily_pnl_rate": "1.25",
    "cumulative_pnl": "15000.00",
    "cumulative_pnl_rate": "17.65",
    "total_positions": 5,
    "updated_at": "2026-05-14T10:30:00Z"
  }
}
```

### 3.2 GET /api/v1/portfolio/positions
**描述**: 获取持仓汇总

**Query 参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| user_id | string | 否 | 不传则返回当前用户 |
| symbol | string | 否 | 按交易对筛选 |
| page | integer | 否 | 默认1 |
| size | integer | 否 | 默认20，最大100 |

**响应**:
```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "symbol": "BTCUSDT",
        "side": "long",
        "quantity": "0.50",
        "avg_price": "65000.00",
        "current_price": "66500.00",
        "unrealized_pnl": "750.00",
        "unrealized_pnl_rate": "2.31"
      }
    ],
    "total": 5,
    "page": 1,
    "size": 20
  }
}
```

### 3.3 GET /api/v1/portfolio/performance
**描述**: 获取多策略绩效对比

**Query 参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| user_id | string | 否 | 不传则返回当前用户 |

**响应**:
```json
{
  "code": 0,
  "data": {
    "strategies": [
      {
        "strategy_id": "uuid",
        "strategy_name": "趋势追踪",
        "total_pnl": "5000.00",
        "total_pnl_rate": "8.33",
        "max_drawdown": "-3.21",
        "trade_count": 45,
        "win_rate": "62.22"
      }
    ]
  }
}
```

### 3.4 GET /api/v1/portfolio/equity_curve
**描述**: 获取权益曲线数据

**Query 参数**:
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| user_id | string | 否 | 不传则返回当前用户 |
| start_date | string (YYYY-MM-DD) | 否 | 默认30天前 |
| end_date | string (YYYY-MM-DD) | 否 | 默认今天 |
| granularity | string | 否 | hour / day，默认 day |

**响应**:
```json
{
  "code": 0,
  "data": {
    "points": [
      {"timestamp": "2026-05-01T00:00:00Z", "equity": "98500.00"},
      {"timestamp": "2026-05-02T00:00:00Z", "equity": "99200.00"}
    ]
  }
}
```

---

## 4. Gherkin 验收条件

### Feature: Portfolio Summary

```gherkin
Feature: Portfolio Summary

  Scenario: 用户查看自己的组合汇总
    Given 用户已登录
    When 用户请求 GET /api/v1/portfolio/summary（不传user_id）
    Then 返回当前用户的组合汇总数据
    And HTTP 200

  Scenario: 用户查看他人的组合汇总（有权限）
    Given 用户已登录且为 admin 角色
    When 用户请求 GET /api/v1/portfolio/summary?user_id=xxx
    Then 返回指定用户的组合汇总数据
    And HTTP 200

  Scenario: 未登录用户访问
    Given 用户未登录
    When 用户请求 GET /api/v1/portfolio/summary
    Then HTTP 401 Unauthorized

  Scenario: 持仓为空时返回零值
    Given 用户已登录但无任何持仓
    When 用户请求 GET /api/v1/portfolio/summary
    Then total_equity="0.00", total_positions=0
    And HTTP 200
```

### Feature: Portfolio Positions

```gherkin
Feature: Portfolio Positions

  Scenario: 获取持仓列表（分页）
    Given 用户已登录且有5个持仓
    When 用户请求 GET /api/v1/portfolio/positions?page=1&size=2
    Then 返回2条持仓数据
    And total=5, page=1, size=2
    And HTTP 200

  Scenario: 按交易对筛选持仓
    Given 用户有 BTCUSDT 和 ETHUSDT 持仓
    When 用户请求 GET /api/v1/portfolio/positions?symbol=BTCUSDT
    Then 只返回 BTCUSDT 持仓
    And HTTP 200

  Scenario: 空持仓返回空数组
    Given 用户已登录但无持仓
    When 用户请求 GET /api/v1/portfolio/positions
    Then items=[], total=0
    And HTTP 200
```

### Feature: Portfolio Performance

```gherkin
Feature: Portfolio Performance

  Scenario: 获取策略绩效列表
    Given 用户有3个策略且有交易历史
    When 用户请求 GET /api/v1/portfolio/performance
    Then 返回3个策略的绩效数据
    And HTTP 200

  Scenario: 策略无交易时绩效为零
    Given 用户有策略但无任何成交
    When 用户请求 GET /api/v1/portfolio/performance
    Then 该策略 total_pnl="0.00", trade_count=0
    And HTTP 200
```

---

## 5. 数据来源

### 5.1 关联表
- `strategy` — 策略基础信息（name, user_id）
- `order` — 订单表（symbol, side, price, quantity, status=filled）
- `positions` — 持仓表（symbol, side, quantity, avg_price）
- `backtest_results` — 回测结果（用于历史业绩）

### 5.2 计算规则
- **累计盈亏** = Σ(平仓盈亏) + Σ(浮动盈亏)
- **当日盈亏** = 今日权益 - 昨日权益
- **收益率** = 盈亏 / 初始本金 × 100%
- **最大回撤** = max(peak - equity) / peak × 100%
- **Sharpe Ratio** = (平均日收益 / 日收益标准差) × √252

---

## 6. 非功能需求

| 指标 | 要求 |
|------|------|
| 响应时间 | P95 < 200ms |
| 分页 | 最大100条/页 |
| 实时性 | 权益数据 T+0 |
| 并发 | 支持100+用户同时访问 |

---

## 7. 依赖关系

- 依赖 `strategy` 表存在
- 依赖 `order` 表有 `filled` 状态订单
- 依赖 `positions` 持仓表（或从 order 实时聚合）
