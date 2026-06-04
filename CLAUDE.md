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

### OpenAPI workflow (P3-3 闭环)

后端的 `utoipa::path` 注解是契约来源。前端通过 `openapi-typescript` 把
`/api/v1/openapi.json` 转成 `frontend/src/types/api-generated.ts`,用
`openapi-fetch` 拿到全类型化的 client。

```bash
# 重新生成类型 (前端 cd 进去)
npm run gen:api   # 走 scripts/gen-types.sh
```

脚本优先用 `docs/openapi.json` 的提交版;若不存在则 `cargo run --bin
export_openapi` 实时导出。CI 也在同一个脚本上跑,所以本地生成的 `api-generated.ts`
直接 commit 即可。

新增 endpoint 的步骤:

1. 后端在 handler 上加 `#[utoipa::path(...)]`,在 `src/lib.rs` 的
   `ApiDoc` 里 `paths(...)` + `components(schemas(...))` 注册
2. 跑 `npm run gen:api` 重生成 types
3. 前端用 `typedApi.GET/POST/...` 调用,IDE 自动补全 request/response

临时打补丁也可以手工编辑 `api-generated.ts`(本仓库曾用此法加
feature-flag 端点),但下次跑 `gen:api` 会被覆盖,记得在 commit message
里说明。

### Feature flag workflow (P3-5)

后端提供 3 个端点 (实现见 `backend/src/handlers/feature_flag.rs`):

| 端点 | 角色 | 说明 |
| --- | --- | --- |
| `GET    /api/v1/feature-flags`           | 任意已登录用户 | 当前用户的逐 flag 布尔评估 (`{flags: Record<string, boolean>}`) |
| `GET    /api/v1/admin/feature-flags`     | admin | 所有 flag 行的原始定义 (含 description, percentageRollout) |
| `POST   /api/v1/admin/feature-flags`     | admin | upsert (按 `key` 写入) |
| `DELETE /api/v1/admin/feature-flags/{key}`| admin | 删除 |

前端使用:

- `frontend/src/stores/featureFlag.ts` — Pinia store,`load()` 启动时拉
  用户 bootstrap 评估,`isEnabled(key)` 查单个 flag
- `frontend/src/composables/useFeatureFlag.ts` — `useFeatureFlag(key)` 包
  装 store,返回 `ComputedRef<boolean>`,在 template 里直接 `v-if`
- `frontend/src/views/admin/FeatureFlagView.vue` — admin UI,`/admin/feature-flags`
  路由 (`roles: ['admin']`)

新增一个 flag 的标准流程:

1. 后端 migration: `backend/migrations/<ts>_add_<flag_key>.sql`,在
   `feature_flags` 表里 `INSERT` 一行 (或后端服务里通过
   `services::feature_flag::upsert` 写)
2. 跑 `npm run gen:api` 让 openapi 同步 (若改了响应结构)
3. 前端组件: `import { useFeatureFlag } from '@/composables/useFeatureFlag'`
   + `const enabled = useFeatureFlag('<flag_key>')` + `v-if="enabled"`
4. 默认关闭 (closed-by-default) — `useFeatureFlag` 在未知 key 上返回
   `false`,所以未 seed 的 flag 不会"误开"

灰度 (percentage rollout / whitelist) 在后端 `services/feature_flag::evaluate_flag`
里实现,前端只拿到布尔结果。Cache 失效由 `svc::upsert` / `svc::delete_flag`
自动处理,无需前端配合。

### PAMM (Percent Allocation Management Module) — §6-1 商业化

PAMM 是 "基金经理 + 多投资人" 的资金归集 / 收益分配模式。后端在
`backend/src/services/pamm.rs` (8 个核心函数 + HWM 高水位线算法),
9 个 REST 端点在 `backend/src/handlers/pamm.rs`,5 张表的 migration
在 `backend/migrations/20260603150000_create_pamm_tables.sql`。

详细文档见 `docs/pamm.md` (含算法推导 + 完整数字例子 + 9 端点
curl 例子 + 投资人 / 经理流程 + 风险说明)。本文只列关键点。

**核心算法** — 三步:

1. `gross_pnl` = NAV₁ − NAV₀
2. `mgmt_fee` = NAV₁ × mgmt_fee_pct × period_days / 365 (无论盈亏都收)
3. `perf_fee` = (NAV₁ > HWM₋) ? (NAV₁ − HWM₋) × perf_fee_pct : 0
   (高水位线之上的盈利才抽成)
4. `net_to_split` = gross_pnl − mgmt_fee − perf_fee
5. 按 `share_pct` 分给每个投资人,新 NAV₁ 推高 HWM = max(HWM₋, NAV₁)

**前端** — `frontend/src/views/pamm/` 下 4 个 view + `stores/pamm.ts`
+ 4 个路由 (`/pamm`, `/pamm/funds/:id`, `/pamm/my`, `/pamm/manager`),
侧边栏 "PAMM 基金" 入口。

**权限** — 两层防御:
- 路由层: `pamm_manager_routes` 包了 `require_admin_middleware` (admin 角色)
- 服务层: `distribute_profits` / `liquidate` 内部再校验 `fund.manager_id == user.user_id`,
  即使 admin token 不拥有这个 fund 也会被 403 拒掉

**Wire 注意事项** — 所有 decimal 字段在 JSON 上是**字符串**(`rust_decimal::Decimal`
用 `to_string()` 序列化保留精度),不是 JS number。`api-generated.ts`
没有这个模块的类型(本任务没跑 `npm run gen:api`),`stores/pamm.ts`
里的 `Fund` / `Investment` interface 手工对齐后端 DTO,后端形状变化时
需要同时改 store + 跑 gen:api。

**禁止清单**:
- 不要加实时交易所路由 (PAMM 走 paper account,跟单实盘是另一个工作流)
- 不要把 `share_pct` 改成 `float` (精度会丢)
- 不要把 manager dashboard 改成后端聚合端点 (现有 list + detail 端点
  已够,前端在 manager dashboard 里本地聚合即可)

### Copy Trading — §6-2 商业化

Copy Trading 是 "交易员 + 多跟单人" 的按比例跟单模式 (区别于 PAMM 的
"经理 + 投资人 NAV 模式")。后端在
`backend/src/services/copy_trading.rs` (fan-out 任务 + 结算) +
`backend/src/services/copy_trading/risk.rs` (per-subscription 风险上限),
10 个 REST 端点在 `backend/src/handlers/copy_trading.rs`,4 张表的
migration 在 `backend/migrations/20260604130000_create_copy_trading_tables.sql`。

详细文档见 `docs/copy-trading.md` (含 fan-out 算法 + 结算算法 + 10
端点 curl 例子 + 跟单人 / 交易员流程 + 风险说明)。本文只列关键点。

**核心算法 — 两段**:
1. **Fan-out** — `handlers/order.rs` 提交订单时
   `spawn_on_trader_order` 异步触发;对每个 active subscription:
   1) 检查 `max_position_size` per-trade (0 = 无上限) →
   2) 检查 `max_loss_per_day` 滚动已实现亏损 (0 = 无上限) →
   3) 算 `mirror_qty = trader_qty × subscription.ratio` (rust_decimal) →
   4) 走同一个 matching-engine 提交 (`user_id = follower_id`) →
   5) 写 `copy_trades` 审计行
2. **Settle** — `POST /calculate-shares` 按
   `(subscription_id, period_start, period_end)` 聚合 `copy_trades`:
   `gross_pnl = Σ closed_trade_pnl` →
   `trader_share = gross_pnl × trader_fee_pct` (默认 0.20) →
   `follower_share = gross_pnl − trader_share` →
   写 `copy_profit_shares` 行,UNIQUE(subscription_id, period_start, period_end)
   保证 idempotent,re-run 返 409。

**前端** — `frontend/src/views/copy/` 下 4 个 view + `stores/copyTrading.ts`
+ 4 个路由 (`/copy`, `/copy/traders/:id`, `/copy/my`, `/copy/dashboard`),
侧边栏 "跟单交易" 入口。`stores/copyTrading.ts` 用
`openapi-fetch` typed client (`typedApi`),请求/响应形状在编译时
由 `api-generated.ts` 校验。

**OpenAPI 闭环** — `backend/src/lib.rs` 的 `ApiDoc.paths()` + `components(schemas)`
都注册了 10 个 copy-trading path + 16 个 schema
(`RegisterRequest`/`SubscribeRequest`/`UnsubscribeRequest`/`CalculateSharesRequest`
请求体 + 6 个响应 wrapper + 5 个 service view)。重新生成:
```
cd backend && cargo run --bin export_openapi > ../docs/openapi.json
cd ../frontend && npx openapi-typescript ../docs/openapi.json -o ./src/types/api-generated.ts
```

**权限** — 两层防御:
- 路由层: 4 个 copy-trading 路由都 `requiresAuth: true`,`/copy/dashboard`
  不限角色 (注册过的 trader 即可),具体权限由后端 handler 校验
- 服务层: `subscribe` 校验 `trader.status == 'Active'`,`unsubscribe`
  校验 `subscription.follower_id == user.user_id`,`calculate-shares`
  校验 `trader.user_id == user.user_id`

**禁止清单**:
- 不要把 `trader_fee_pct` 默认值改成 0 (跟单交易员会失去激励)
- 不要在 fan-out 里用 `f64` (精度会丢,所有计算必须 `rust_decimal`)
- 不要把 fan-out 改成同步阻塞 (trader 下单延迟会随 follower 数量线性
  退化,fail/timeout 还会影响 trader 自己)
- 不要把 `(subscription_id, period_start, period_end)` 唯一约束去掉
  (它是结算 idempotency 的唯一保障)

