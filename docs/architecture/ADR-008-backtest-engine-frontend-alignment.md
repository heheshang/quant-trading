# ADR-008: 回测引擎前后端契约对齐与实施决策

| 字段 | 值 |
|------|------|
| **ID** | ADR-008 |
| **状态** | 待批准 |
| **日期** | 2026-05-13 |
| **决策者** | Tech Lead |
| **影响范围** | 回测引擎完整链路（handler → engine → db → frontend） |
| **基于** | ADR-004（顶层架构）、ADR-007（详细设计）、PRD-backtest-engine v1.1 |
| **替代/修正** | 修正 ADR-007 中未涉及的前后端契约问题 |

## 背景

ADR-004 和 ADR-007 批准后，后端回测引擎（backtest_engine.rs）和 10 个策略模板已完成实现。前端组件（BacktestView/ConfigForm/MetricsCards/EquityChart/TradesTable）也已完成开发。

但在集成评审中发现：**前后端 API 契约存在严重不一致**，且后端 DB 层仍为 TODO 占位，导致完整链路无法跑通。本 ADR 针对这些间隙做出决策。

## 审核发现

### 发现 1: 前后端 API 路由不一致（CRITICAL）

| 维度 | 前端 (api/backtest.ts) | 后端 (handlers/backtest.rs) | 影响 |
|------|----------------------|---------------------------|------|
| POST 启动 | `/backtest` | `/backtest/run` | ❌ 404 |
| GET 结果 | `/backtest/{id}` | `/backtest/{id}/result` | ❌ 404 |
| 轮询进度 | `/backtest/jobs/{jobId}` | `/backtest/{id}/progress` | ❌ 无此路由 |
| GET 历史 | `/backtest/history` | `/backtest/history` | ✅ 一致 |
| DELETE | `/backtest/{id}` | `/backtest/{id}` | ✅ 一致 |

**根源：** 前端采用 v1.0 PRD 的路由设计（`/backtest` + 独立 job 概念），后端遵循 v1.1 PRD + ADR-007 的路由设计（`/backtest/run` + `/backtest/{id}/result` + `/backtest/{id}/progress`）。两份文档未在路由级别对齐。

### 发现 2: 请求体结构不一致（CRITICAL）

前端 `BacktestParams`:
```typescript
{
  strategy_id: string,
  symbol: string,
  start_date: string,
  end_date: string,
  initial_capital: number
  // 缺少: interval, fee_rate, slippage_rate
}
```

后端 `BacktestRunRequest`:
```rust
pub struct BacktestRunRequest {
    pub strategy_id: Uuid,
    pub config: BacktestConfig {  // 嵌套结构
        pub symbol: String,
        pub interval: String,      // 前端缺失
        pub start_date: String,
        pub end_date: String,
        pub initial_capital: f64,
        pub fee_rate: f64,         // 前端缺失
        pub slippage_rate: f64,    // 前端缺失
    }
}
```

**影响：** 请求体结构完全不同（扁平 vs 嵌套），字段缺少 3 个，无法反序列化。

### 发现 3: 响应数据结构不一致（CRITICAL）

前端 `BacktestResultDetail` 期望扁平结构：
```typescript
{
  id: number,          // ← number, 后端是 UUID string
  strategy_name: string, // ← 后端不返回
  symbol: string,        // ← 后端放在 config 中
  total_return: number,  // ← 后端在 metrics.total_return_pct
  equity_curve: [{ date: string, equity: number }], // ← 字段名不同
  trades: [{ id, open_time, close_time, pnl, pnl_percent }], // ← 字段结构不同
}
```

后端 `BacktestResultResponse` 返回：
```rust
pub struct BacktestResultResponse {
    pub id: Uuid,
    pub strategy_id: Uuid,
    pub config: BacktestConfig,   // 嵌套
    pub metrics: Option<BacktestMetrics>, // 嵌套的 14 个指标
    pub trades: Option<Vec<TradeRecord>>, // 不同的字段
    pub equity_curve: Option<Vec<EquityPoint>>, // time/equity/drawdown_pct
    // ...
}
```

### 发现 4: 前端轮询模型与后端不匹配（HIGH）

前端流程：`runBacktest()` → 返回 `BacktestJob { id, status }` → `getBacktestJob(jobId)` 轮询 → `getBacktestResult(result_id)`

后端流程：`POST /backtest/run` → 返回 `BacktestRunResponse { id, status }` → `GET /backtest/{id}/progress` 轮询 → `GET /backtest/{id}/result`

前端使用独立的 "job" 资源（`/backtest/jobs/{jobId}`），后端则直接操作 backtest_results 表，无独立 job 概念。

### 发现 5: DB 层全部为 TODO 占位（CRITICAL）

`db/backtest.rs` 中 9 个函数全部是 no-op 占位：
- create_backtest_run, update_completed, update_failed, find_backtest_result, get_backtest_progress, list_backtest_history, delete_backtest_record, load_klines, find_strategy

完整链路不能跑通。`spawn_blocking` 内尝试 `tokio::runtime::Handle::current()` 将 panic。

### 发现 6: CancellationToken 未注册（HIGH）

`handlers/backtest.rs` 中 `cancel_backtest` 端点无可用注册表查找 token：
```rust
// TODO: Implement CancellationToken registry
// let token = CANCEL_TOKENS.get(&id).ok_or(AppError::NotFound("backtest not running".into()))?;
```

### 发现 7: equity_curve 未按 mark-to-market 计算（MEDIUM）

`record_equity_point` 只记录了 `self.account.cash`，未对持仓做逐日市（MTM）估值。当有持仓时，权益曲线反映的是现金而非真实权益，导致夏普比率、最大回撤等指标计算失真。

### 发现 8: 前端缺失组件和字段

| 缺失项 | 优先级 | PRD 要求 |
|--------|--------|---------|
| 回撤曲线图 (DrawdownChart) | P0 | US-BT-03 |
| interval 字段 | P0 | US-BT-01 ConfigForm |
| fee_rate / slippage_rate | P0 | US-BT-01 ConfigForm |
| 取消按钮 | P1 | US-BT-06 |
| 回测历史标签页 | P1 | US-BT-05 |
| 高级设置折叠面板 | P2 | US-BT-01 |

## 决策

### 决策 1: API 路由以 ADR-007 / PRD v1.1 为准

**决策：** 后端路由不变（`/backtest/run`, `/backtest/{id}/result`, `/backtest/{id}/progress`），前端 API 层适配。

```typescript
// 修正后的前端 api/backtest.ts
export function runBacktest(params: BacktestRunRequest): Promise<BacktestRunResponse> {
  return client.post('/backtest/run', params)
}
export function getBacktestResult(id: string): Promise<BacktestResultResponse> {
  return client.get(`/backtest/${id}/result`)
}
export function getBacktestProgress(id: string): Promise<BacktestProgressResponse> {
  return client.get(`/backtest/${id}/progress`)
}
export function cancelBacktest(id: string): Promise<void> {
  return client.post(`/backtest/${id}/cancel`)
}
export function listBacktestHistory(strategyId: string, page: number, size?: number): Promise<PaginatedResponse<BacktestSummary>> {
  return client.get('/backtest/history', { params: { strategy_id: strategyId, page, size } })
}
export function deleteBacktestResult(id: string): Promise<void> {
  return client.delete(`/backtest/${id}`)
}
```

**理由：** 避免后端做两套路由兼容。前端适配成本更低，且后端路由设计已在 ADR-007 中决策。

### 决策 2: 请求体使用嵌套 config 结构

**决策：** 前端 `BacktestParams` 改为与后端一致的嵌套结构，补充缺失字段。

```typescript
interface BacktestConfig {
  symbol: string
  interval: string           // 新增
  start_date: string
  end_date: string
  initial_capital: number
  fee_rate: number           // 新增 (默认 0.001)
  slippage_rate: number      // 新增 (默认 0.0005)
}
interface BacktestRunRequest {
  strategy_id: string
  config: BacktestConfig
}
```

**理由：** 与后端模型一致，避免反序列化适配层。`fee_rate` 和 `slippage_rate` 有默认值，前端高级设置折叠面板控制。

### 决策 3: 前端响应适配层（Frontend Response Adapter）

**决策：** 在 API 层增加响应转换函数，将后端返回的 `BacktestResultResponse` 转换为前端组件所需的 `BacktestResultDetail`。

```typescript
function adaptBacktestResult(backend: BacktestResultResponse): BacktestResultDetail {
  return {
    id: backend.id,
    strategy_name: '', // 从策略详情获取后补充
    symbol: backend.config.symbol,
    initial_capital: backend.config.initial_capital,
    total_return: backend.metrics?.total_return_pct ?? 0,
    annual_return: backend.metrics?.annualized_return_pct ?? 0,
    sharpe_ratio: backend.metrics?.sharpe_ratio ?? 0,
    max_drawdown: backend.metrics?.max_drawdown_pct ?? 0,
    win_rate: backend.metrics?.win_rate ?? 0,
    total_trades: backend.metrics?.total_trades ?? 0,
    profit_factor: backend.metrics?.profit_factor ?? 0,
    sortino_ratio: backend.metrics?.sortino_ratio ?? 0,
    calmar_ratio: backend.metrics?.calmar_ratio ?? 0,
    equity_curve: (backend.equity_curve ?? []).map(ep => ({
      date: new Date(ep.time).toISOString(),
      equity: ep.equity,
      drawdown_pct: ep.drawdown_pct,
    })),
    trades: (backend.trades ?? []).map(t => ({
      id: 0, // 后端无此字段
      open_time: new Date(t.entry_time).toISOString(),
      close_time: new Date(t.exit_time).toISOString(),
      direction: t.direction,
      open_price: t.entry_price,
      close_price: t.exit_price,
      quantity: t.quantity,
      pnl: t.pnl_usdt,
      pnl_percent: t.pnl_pct,
      exit_reason: t.exit_reason,
      fee: t.fee,
      slippage: t.slippage,
    })),
    created_at: backend.created_at ?? '',
    duration_ms: backend.duration_ms,
  }
}
```

**理由：** 后端不应为前端特定格式做调整。适配层在 API 层集中处理，保持前后端各自类型系统独立。

### 决策 4: DB 层为 P0 实现

**决策：** `db/backtest.rs` 中的 9 个函数为 P0 优先级，与前端适配并行开发。具体实现：
- 创建 `backtest_results` SeaORM entity（迁移文件 + 实体定义）
- `load_klines` 调用 `kline_data` 表的 SeaORM 查询
- `find_strategy` 调用 `strategies` 表的 SeaORM 查询
- 所有函数使用真实 SQL 而非 tracing::info! 占位

**重要约束：** `spawn_blocking` 内不能使用 `tokio::runtime::Handle::current()`。应使用以下方案之一：
1. **方案 A (推荐):** 将 DB 连接通过 channel 发送到外部 tokio task 处理
2. **方案 B:** 在 `spawn_blocking` 中使用 `tokio::runtime::Builder::new_current_thread()` 创建临时运行时（消耗大，不推荐）
3. **方案 C (推荐):** 异步执行引擎，不使用 `spawn_blocking`，直接 `tokio::task::spawn`

**选定方案 C：** 引擎函数本身是纯 CPU 计算，但不在同步代码中进行异步 DB 操作。具体：
- 引擎 `run()` 改为 `async`，内部不再使用 `spawn_blocking`
- Semaphore 控制并发不变
- 引擎完成后直接 await DB 更新

### 决策 5: CancellationToken 注册表实现

**决策：** 使用 `std::sync::LazyLock<DashMap<Uuid, CancellationToken>>` 全局注册表。

```rust
use std::sync::LazyLock;
use dashmap::DashMap;

static CANCEL_TOKENS: LazyLock<DashMap<Uuid, CancellationToken>> = LazyLock::new(|| DashMap::new());
```

- `run_backtest` 中：`CANCEL_TOKENS.insert(result_id, cancel_token);`
- `cancel_backtest` 中：`let (_, token) = CANCEL_TOKENS.remove(&id).ok_or(...)?; token.cancel();`
- `spawn` 的 task 完成后：`CANCEL_TOKENS.remove(&id);`

### 决策 6: equity_curve 恢复 MTM 估值

**决策：** `record_equity_point` 方法需要 MTM 持仓估值：

```rust
fn record_equity_point(&mut self, time: i64, kline: &Kline) {
    let equity = if let Some(pos) = &self.open_position {
        match pos.direction {
            Direction::Long => self.account.cash + pos.quantity * kline.close,
            Direction::Short => self.account.cash - pos.quantity * kline.close,
        }
    } else {
        self.account.cash
    };
    // ... 计算 drawdown 并记录
}
```

**理由：** MTM 估值是权益曲线的核心语义要求。当前实现只记录现金，当有空仓时 equity 高于真实值，有持多时 equity 偏低，导致夏普比率和最大回撤指标偏差。

## 实施计划

### Phase 1 — 后端完整性 (P0)

| # | 任务 | 估计工时 |
|--|------|---------|
| 1 | 创建 backtest_results SeaORM entity + migration | 2h |
| 2 | 实现 db/backtest.rs 所有 9 个函数 | 4h |
| 3 | 改造 handlers/backtest.rs: 异步引擎 + DB 更新 | 3h |
| 4 | CancellationToken 注册表: DashMap 集成 | 1h |
| 5 | equity_curve MTM 修正 | 1h |

### Phase 2 — 前端适配 (P0)

| # | 任务 | 估计工时 |
|--|------|---------|
| 1 | 更新 api/backtest.ts 路由和请求体 | 1h |
| 2 | 添加响应适配层 (adaptBacktestResult) | 1h |
| 3 | 更新 types/backtest.ts 类型定义 | 1h |
| 4 | BacktestView 轮询逻辑改用 /progress | 1h |
| 5 | ConfigForm 增加 interval 下拉 + fee/slippage 高级设置 | 2h |

### Phase 3 — 前端新组件 (P1)

| # | 任务 | 估计工时 |
|--|------|---------|
| 1 | DrawdownChart 回撤曲线图 | 2h |
| 2 | HistoryTab 历史标签页 | 2h |
| 3 | 取消按钮 + 状态管理 | 1h |

## 关联 ADR

- **ADR-004** — 顶层架构决策（全内存向量化计算）→ 继续有效
- **ADR-007** — 回测引擎详细设计 → 本 ADR 修正其未涉及的前后端契约
- **ADR-003** — 策略沙箱 → 策略模板共享不变

## 预期后果

**正面：**
- 前后端契约统一，集成测试通过
- 完整链路可运行（handler → engine → db → response）
- 取消回测功能可用
- 权益曲线 MTM 估值准确

**负面：**
- 前端 API 层需要中度改造（~5 个文件，约 100 行修改）
- DB 层实现（Phase 1）是当前最大工作块
- 后端 handler 改造涉及异步模型变更

**风险缓解：**
- 前后端类型适配层隔离变更影响范围
- Phase 1 和 Phase 2 可并行（前端用 mock 数据开发）
- 保留后端 `/backtest/{id}` 重定向（`GET /backtest/{id} → /backtest/{id}/result`）作为兼容性过渡
