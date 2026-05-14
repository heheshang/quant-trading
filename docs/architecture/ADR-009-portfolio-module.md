# ADR-009: Portfolio 组合权益模块技术方案

**版本**: v1.0
**状态**: 已完成
**日期**: 2026-05-14
**负责人**: Tech-Lead

---

## 1. 背景

Portfolio 组合权益模块为用户提供多策略账户的统一视图，包括权益汇总、持仓聚合、策略绩效分析。

---

## 2. 表结构设计

### 2.1 portfolio_summary（组合汇总，非持久化，按需聚合）

> 注：`portfolio_summary` 不单独存储，通过 SQL 聚合 `positions` + `order` + `backtest_results` 实时计算。

```sql
-- 持仓聚合视图
CREATE VIEW v_portfolio_positions AS
SELECT
    p.user_id,
    p.symbol,
    p.side,
    SUM(p.quantity)       AS total_quantity,
    AVG(p.avg_price)      AS avg_price,
    CURRENT_VALUE(symbol) AS current_price,
    SUM(p.unrealized_pnl) AS unrealized_pnl
FROM positions p
WHERE p.status = 'open'
GROUP BY p.user_id, p.symbol, p.side;
```

### 2.2 新增表（权益曲线历史，需要持久化存储）

```sql
CREATE TABLE portfolio_equity_history (
    id          BIGSERIAL PRIMARY KEY,
    user_id     UUID    NOT NULL REFERENCES users(id),
    equity      DECIMAL(20, 8) NOT NULL,
    timestamp   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_equity_history_user_time ON portfolio_equity_history(user_id, timestamp DESC);
```

### 2.3 现有可用表

| 表名 | 用途 |
|------|------|
| `strategy` | 策略基础信息 |
| `order` | 订单（含 filled 状态） |
| `positions` | 实时持仓 |
| `backtest_results` | 回测绩效 |
| `paper_accounts` | 模拟账户 |

---

## 3. 后端实现

### 3.1 文件结构

```
backend/src/
├── db/
│   ├── mod.rs              # 注册 portfolio_equity_history 表
│   └── portfolio.rs         # Entity 定义（新增）
├── handlers/
│   ├── mod.rs              # 注册 portfolio router
│   └── portfolio.rs         # Handler 实现（新增）
└── services/
    └── portfolio.rs          # 聚合计算逻辑（新增）
```

### 3.2 Entity 定义 (db/portfolio.rs)

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "portfolio_equity_history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    #[sea_orm]
    pub user_id: Uuid,
    #[sea_orm]
    pub equity: Decimal,
    #[sea_orm]
    pub timestamp: ChronoDateTime,
    #[sea_orm]
    pub created_at: ChronoDateTime,
}

impl ActiveModelBehavior for ActiveModel {}
```

### 3.3 Handler (handlers/portfolio.rs)

```rust
// GET /api/v1/portfolio/summary
pub async fn get_portfolio_summary(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<PortfolioSummaryQuery>,
) -> Result<Json<ApiResponse<PortfolioSummary>>, AppError> {
    let user_id = params.user_id.unwrap_or(user.user_id);
    // 权限校验：只能查看自己或 admin
    if user_id != user.user_id && user.role != "admin" {
        return Err(AppError::Forbidden("无权限查看该组合".into()));
    }
    let summary = portfolio_service::get_summary(&db, user_id).await?;
    Ok(Json(ApiResponse::success(summary)))
}

// GET /api/v1/portfolio/positions
pub async fn list_portfolio_positions(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<PortfolioPositionsQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<PortfolioPosition>>>, AppError> {
    let user_id = params.user_id.unwrap_or(user.user_id);
    if user_id != user.user_id && user.role != "admin" {
        return Err(AppError::Forbidden("无权限查看该组合".into()));
    }
    let positions = portfolio_service::list_positions(&db, user_id, &params).await?;
    Ok(Json(ApiResponse::success(positions)))
}

// GET /api/v1/portfolio/performance
pub async fn get_portfolio_performance(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<PortfolioPerformanceQuery>,
) -> Result<Json<ApiResponse<PortfolioPerformance>>, AppError> {
    let user_id = params.user_id.unwrap_or(user.user_id);
    if user_id != user.user_id && user.role != "admin" {
        return Err(AppError::Forbidden("无权限查看该组合".into()));
    }
    let performance = portfolio_service::get_performance(&db, user_id).await?;
    Ok(Json(ApiResponse::success(performance)))
}

// GET /api/v1/portfolio/equity_curve
pub async fn get_equity_curve(
    user: AuthenticatedUser,
    State(db): State<Arc<DatabaseConnection>>,
    Query(params): Query<EquityCurveQuery>,
) -> Result<Json<ApiResponse<EquityCurve>>, AppError> {
    let user_id = params.user_id.unwrap_or(user.user_id);
    if user_id != user.user_id && user.role != "admin" {
        return Err(AppError::Forbidden("无权限查看该组合".into()));
    }
    let curve = portfolio_service::get_equity_curve(&db, user_id, &params).await?;
    Ok(Json(ApiResponse::success(curve)))
}
```

### 3.4 Service 层 (services/portfolio.rs)

```rust
pub async fn get_summary(db: &DatabaseConnection, user_id: Uuid) -> Result<PortfolioSummary, AppError> {
    // 1. 从 positions 聚合当前权益
    // 2. 从 order 聚合当日盈亏（status=filled, updated_at 今日）
    // 3. 从 backtest_results 聚合累计盈亏
    // 4. 计算当日/累计收益率
    Ok(PortfolioSummary { ... })
}

pub async fn list_positions(
    db: &DatabaseConnection,
    user_id: Uuid,
    params: &PortfolioPositionsQuery,
) -> Result<PaginatedResponse<PortfolioPosition>, AppError> {
    // 1. 关联 positions + market_data(current_price)
    // 2. 计算浮动盈亏 = (current_price - avg_price) * quantity * side_sign
    // 3. 分页返回
    Ok(PaginatedResponse { ... })
}

pub async fn get_performance(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<PortfolioPerformance, AppError> {
    // 1. 查询 user 关联的所有 strategy
    // 2. 关联 backtest_results + order 聚合各策略绩效
    // 3. 计算 Sharpe Ratio、最大回撤、盈亏比
    Ok(PortfolioPerformance { ... })
}

pub async fn get_equity_curve(
    db: &DatabaseConnection,
    user_id: Uuid,
    params: &EquityCurveQuery,
) -> Result<EquityCurve, AppError> {
    // 1. 从 portfolio_equity_history 查询历史数据
    // 2. 按 granularity (hour/day) 聚合
    Ok(EquityCurve { ... })
}
```

### 3.5 路由注册 (main.rs)

```rust
// 在 api_router 下添加
.router("/portfolio/summary", get(portfolio::get_portfolio_summary))
.router("/portfolio/positions", get(portfolio::list_portfolio_positions))
.router("/portfolio/performance", get(portfolio::get_portfolio_performance))
.router("/portfolio/equity_curve", get(portfolio::get_equity_curve))
```

---

## 4. 前端契约 (B1-B4)

### 4.1 TypeScript 类型 (frontend/src/types/portfolio.ts)

```typescript
// B1: snake_case → camelCase
// B2: UUID → string
// B3: Option<T> → T | null
// B4: Decimal → string（避免浮点精度问题）

export interface PortfolioSummary {
  total_equity: string;       // "100000.00"
  daily_pnl: string;           // "1234.56"
  daily_pnl_rate: string;      // "1.25" (%)
  cumulative_pnl: string;      // "15000.00"
  cumulative_pnl_rate: string; // "17.65" (%)
  total_positions: number;
  updated_at: string;          // ISO8601
}

export interface PortfolioPosition {
  symbol: string;
  side: 'long' | 'short';
  quantity: string;
  avg_price: string;
  current_price: string;
  unrealized_pnl: string;
  unrealized_pnl_rate: string; // "%"
}

export interface StrategyPerformance {
  strategy_id: string;
  strategy_name: string;
  total_pnl: string;
  total_pnl_rate: string;
  max_drawdown: string;
  trade_count: number;
  win_rate: string; // "%"
}

export interface EquityCurvePoint {
  timestamp: string;
  equity: string;
}

export interface EquityCurve {
  points: EquityCurvePoint[];
}

export interface PaginatedPositions {
  items: PortfolioPosition[];
  total: number;
  page: number;
  size: number;
}
```

### 4.2 API 调用 (frontend/src/api/portfolio.ts)

```typescript
import axios from 'axios';
import type { PortfolioSummary, PaginatedPositions, StrategyPerformance, EquityCurve } from '@/types/portfolio';

const api = axios.create({ baseURL: '/api/v1' });

export const portfolioApi = {
  getSummary: (userId?: string) =>
    api.get<{ code: number; data: PortfolioSummary }>('/portfolio/summary', { params: { user_id: userId } }),
  listPositions: (params: { symbol?: string; page?: number; size?: number; user_id?: string }) =>
    api.get<{ code: number; data: PaginatedPositions }>('/portfolio/positions', { params }),
  getPerformance: (userId?: string) =>
    api.get<{ code: number; data: { strategies: StrategyPerformance[] } }>('/portfolio/performance', { params: { user_id: userId } }),
  getEquityCurve: (params: { start_date?: string; end_date?: string; granularity?: string; user_id?: string }) =>
    api.get<{ code: number; data: EquityCurve }>('/portfolio/equity_curve', { params }),
};
```

---

## 5. 错误处理

| 场景 | HTTP 状态码 | 错误码 |
|------|------------|--------|
| 未登录 | 401 | UNAUTHORIZED |
| 无权限查看他人组合 | 403 | FORBIDDEN |
| 用户无任何数据 | 200 | 返回全零值 |
| 数据库错误 | 500 | INTERNAL_ERROR |

---

## 6. 安全考虑

1. **越权漏洞**：必须校验 `user_id` 只能查看自己（除非 admin）
2. **SQL 注入**：使用 SeaORM Parameterized Query
3. **浮点精度**：所有金额用 `string` 传输，前端按需格式化
4. **速率限制**：/portfolio 端点加入 rate limit（60 req/min）

---

## 7. 代码骨架

```bash
# 新增文件
backend/src/db/portfolio.rs       # Entity
backend/src/services/portfolio.rs  # Service
backend/src/handlers/portfolio.rs # Handler

# 修改文件
backend/src/db/mod.rs             # 注册 Entity
backend/src/handlers/mod.rs      # 注册 Handler
backend/src/main.rs              # 挂载路由
```

---

## 8. 测试计划

| 层 | 测试内容 |
|----|---------|
| Unit | portfolio_service 各函数：聚合计算正确性 |
| Integration | 4个 API 端点：正常/空数据/越权 |
| E2E | 登录→查看组合总览→查看持仓明细 |
