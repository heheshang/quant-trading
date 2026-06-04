# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

量化交易系统 - 基于 Rust + Axum 后端和 Vue 3 + TypeScript 前端的全栈量化交易平台。

## 常用命令

### Backend (Rust)

```bash
# 开发构建（约1分钟冷构建，增量~0.5秒）
cd backend && cargo build

# 类型检查
cargo check

# 格式化
cargo fmt --all

# Lint（严格模式，警告当错误）
cargo clippy --all-targets --all-features -- -D warnings

# 运行测试
cargo test

# 运行单个测试
cargo test test_name_here

# 完整检查（构建+测试+lint）
make check
```

### Frontend (Vue/TypeScript)

```bash
cd frontend

# 开发服务器
npm run dev

# 类型检查
npm run typecheck

# Lint
npm run lint

# 测试
npm run test

# 构建生产版本
npm run build
```

### Docker

```bash
# 启动数据库
docker compose up -d db

# 完整环境启动
docker compose up -d
```

## 架构概览

### Backend (`backend/src/`)

- `main.rs` / `lib.rs` — 应用入口和模块导出
- `handlers/` — HTTP 处理器层（认证、策略、回测、市场数据、WebSocket）
- `services/` — 业务逻辑层（认证、策略服务、回测引擎、风险管理）
- `db/` — SeaORM 数据库实体层
- `models/` — 请求/响应数据模型和 Schema
- `middleware/` — JWT 认证中间件
- `bin/stress_test/` — 压力测试工具

**关键设计**：
- 双 Token 认证（access + refresh）
- 策略状态机：`draft → active → paused → stopped`
- 回测引擎：异步执行，Semaphore 并发控制（上限5），支持做多/做空双向交易

### Frontend (`frontend/src/`)

- `api/` — Axios HTTP 客户端，API 响应自动解包 `data` 字段
- `stores/` — Pinia 状态管理（auth, trading, strategy 等）
- `views/` — 页面组件（按功能模块组织）
- `types/` — TypeScript 类型定义
- `router/` — Vue Router 路由配置

**API 约定**：
- 响应格式：`{ code: number, data: T, message: string }`
- `code !== 0` 时拦截器自动 reject
- 分页响应包含 `meta: { page, size, total }`

### 多交易所支持

后端支持 Binance、OKX、Gate.io、Bybit 四个交易所的 REST API 行情获取，详见 `backend/src/services/exchange/`。

### 多周期 K线联动 (P2-3)

`frontend/src/components/charts/MultiTimeframeChart.vue` 实现主图 + 副图
的 X 轴联动布局：

- **主图**：由父组件传入 `mainData` / `mainInterval`，通常是 1m/5m 等较
  短周期
- **副图**：1h / 4h / 1d 三选一，组件内部自行调用
  `GET /api/v1/kline/query?symbol=...&interval=...` 拉数据
- **X 轴同步**：主图 `KlineChart` 在 `chart.timeScale().subscribeVisibleTimeRangeChange`
  里 dispatch `window` 自定义事件 `kline-visible-range-change`；副图
  在 `MultiTimeframeChart` 的 `onVisibleRangeEvent` 监听并调用
  `setVisibleRange({ from, to })` 跟随。组件透出 `addSubBar(bar)` 让父
  组件把 WS 推来的当前 sub 周期 K 线推入副图

使用方式（见 `frontend/src/views/trade/TradingView.vue`）：

```vue
<MultiTimeframeChart
  ref="mtfChartRef"
  :main-data="klineData"
  :main-symbol="selectedSymbol"
  :main-interval="chartInterval"
  :initial-sub-interval="subInterval"
  @sub-interval-change="onSubIntervalChange"
/>
```

副图 K 线更新路径（WS 推送 → store → 副图）：

```ts
tradingStore.$subscribe(() => {
  if (tradingStore.lastKline && tradingStore.lastKlineInterval === subInterval.value) {
    mtfChartRef.value?.addSubBar(tradingStore.lastKline)
  }
})
```

指示器（MA/EMA/MACD/KDJ 等）走的是主图内部 `KlineChart` 的 props，不
需要在 `MultiTimeframeChart` 上重复声明。

## 代码规范

### 提交流程

1. 确保 lint 和 typecheck 通过后再提交
2. 使用 `git commit`（会触发 pre-commit hooks）
3. pre-commit 会运行：cargo fmt → cargo clippy → cargo deny → frontend-typecheck

### 类型处理注意

- API 响应拦截器在 `frontend/src/api/client.ts`，统一解包 `body.data`
- 后端使用 snake_case，前端接收后仍是 snake_case（**不是** camelCase）
- 枚举类型使用 TypeScript `type` 别名而非 `enum`

### 数据库迁移

- 迁移文件在 `backend/migrations/`
- 使用 SeaORM 的 `Entity` 模式定义表结构

### TimescaleDB 时序表 (P3-6)

K线 / 订单 / 风控事件走 TimescaleDB hypertable (基于 PostgreSQL 16):

- **主写入路径**: `klines_phase4` (KlineWriter 写入),自动按 7 天分 chunk
- **历史路径**: `klines` (旧表,仍保留给 admin 导入/导出),7d chunk
- **订单执行**: `trades` / `orders`,1d chunk
- **连续聚合**: `klines_1m` / `klines_5m` / `klines_1h` — 后台 worker 增量维护
  OHLCV,`/api/v1/kline/aggregate` 端点直接读视图
- **压缩**: 7 天前 chunk 自动列存 (segment by symbol,interval)
- **保留**: 1 年后自动 drop

启动用 `timescale/timescaledb:latest-pg16` 镜像 (替代 `postgres:16-alpine`),
DATABASE_URL 完全不变。运行时入口在 `src/db/mod.rs::install_timescaledb_extensions`,
vanilla Postgres 上整段静默跳过,不影响 dev。详细文档见 `docs/timescaledb.md`。
