## [Unreleased]

### 新增

- **Copy Trading 跟单系统上线 (§6-2 商业化)**
  - **后端 9 端点** (`backend/src/handlers/copy_trading.rs`):
    `GET /traders`, `GET /traders/{id}`, `POST /register`,
    `POST /subscribe`, `POST /unsubscribe`, `GET /my-subscriptions`,
    `GET /my-trader`, `GET /trades`, `GET /profit-shares`,
    `POST /calculate-shares`,全部带 `utoipa::path` + `ToSchema` 注解
  - **后端 OpenAPI 闭环**: `backend/src/lib.rs` 的 `ApiDoc.paths()`
    跟 `components(schemas())` 都注册了 10 copy-trading path + 16 schema
    (4 个请求体 + 6 个响应 wrapper + 6 个 service view);重生
    `docs/openapi.json` 后含 10 个 `/api/v1/copy-trading/...` path
  - **后端 fan-out 触发**: `backend/src/handlers/order.rs` 在订单
    提交路径调 `services::copy_trading::spawn_on_trader_order` 异步
    任务 (tokio spawn,trader 下单延迟不受影响)
  - **前端 store** (`frontend/src/stores/copyTrading.ts`): Pinia store,
    封装 list / get / register / subscribe / unsubscribe /
    my-subscriptions / my-trades / my-profit-shares / my-trader /
    calculate-shares,所有 wire 走 typedApi (openapi-fetch)
  - **前端 4 view** (`frontend/src/views/copy/`):
    - `CopyTraderListView.vue` — 交易员列表 (display_name / monthly_pnl /
      win_rate / follower_count / status) + "Register as Trader" 弹窗
    - `CopyTraderDetailView.vue` — 交易员详情 (4 KPI + bio) + 3 tab
      (Overview / Subscribers / Performance) + Subscribe 弹窗
      (ratio / max_position_size / max_loss_per_day)
    - `MySubscriptionsView.vue` — 我的跟单 (订阅卡 + 取消按钮) +
      "Recent copied trades" 表格
    - `CopyTraderDashboardView.vue` — 交易员面板 (我的 profile 卡 +
      Subscribers 表 + "Calculate shares" 弹窗)
  - **路由 + 侧边栏**: 4 路由挂到 MainLayout (`/copy`, `/copy/traders/:id`,
    `/copy/my`, `/copy/dashboard`),`AppSidebar.vue` 加 "跟单交易" 入口
  - **单元测试** (`frontend/src/__tests__/stores/copyTrading.test.ts`):
    2 case (listTraders 调 API + 状态; subscribe 调 API + 自动刷新
    mySubscriptions),用 `vi.hoisted` mock typedApi
  - **类型生成**: `frontend/src/types/api-generated.ts` 重生,含
    42 处 copy-trading 类型引用 (paths + operations + components)
  - **文档**: `docs/copy-trading.md` 新增 (491 行, 8 章节),含
    fan-out 算法 + 数字例子 (3 个 follower 不同 ratio) +
    结算算法 + 数字例子 (Carol 月度 P&L +250 / trader_share 50 /
    follower_share 200) + 10 端点 curl 例子 + 跟单人 / 交易员
    流程 + 风险说明 (fan-out / 结算 / 费用 / 性能 / 审计)
  - **CLAUDE.md**: 新增 §6-2 Copy Trading 章节 (两段算法 + 权限 +
    OpenAPI 闭环 + 禁止清单)
  - **CHANGELOG**: 本条目

- **PAMM 资管系统上线 (§6-1 商业化)**
  - **后端 wiring** (`backend/src/main.rs`): 把 `pamm.rs::router_user` 跟
    `pamm::router_manager` 接进全局 app。User 路由只挂 `auth_middleware`,
    manager 路由先 `auth_middleware` 再 `require_admin_middleware` (跟
    P3-4 audit_log 同款模板)
  - **后端 router split** (`backend/src/handlers/pamm.rs`): 9 端点拆成
    `router_user()` (7 端点,任意已登录用户) 跟 `router_manager()`
    (2 端点,distribute / liquidate),unit test 同步改
  - **前端 store** (`frontend/src/stores/pamm.ts`): Pinia store,封装
    list / get / create / subscribe / redeem / distribute / liquidate /
    my-investments / load-fund-investments,所有 wire 走 typedApi (openapi-fetch)
  - **前端 4 view** (`frontend/src/views/pamm/`):
    - `PammFundListView.vue` — 基金列表,manager 可见"开新基金"按钮,
      弹窗式创建表单 (name / base_currency / mgmt&perf fee / HWM 开关)
    - `PammFundDetailView.vue` — 基金详情 (4 KPI + 投资人表) + Subscribe
      / Redeem / Distribute / Liquidate 4 个操作弹窗
    - `PammMyInvestmentsView.vue` — 我的投资 (含 initial / current /
      P&L 绝对值 + 百分比)
    - `PammManagerDashboardView.vue` — 经理面板 (我管理的基金列表 +
      投资人快速入口)
  - **路由 + 侧边栏**: 4 路由挂到 MainLayout,`/pamm/manager` 走
    `roles: ['admin']` 守卫;`AppSidebar.vue` 加 "PAMM 基金" 入口
  - **auth store 补 `userId`**: `frontend/src/stores/auth.ts` 新增
    `userId` computed,前端 manager 判定 (fund.manager_id == me) 需要
  - **文档**: `docs/pamm.md` 新增 (10K+ 字符),含算法推导 + 完整数字
    例子 (NAV 100万 → 120万,perf_fee 4万,投资人分成 7.99/4.79/3.20 万)
    + 9 端点 curl 例子 + 投资人/经理流程 + 风险说明
  - **CLAUDE.md**: 新增 §6-1 PAMM 章节 (算法 + 权限 + wire 注意事项)
  - **CHANGELOG**: 本条目

- **多周期 K 线联动 (P2-3)**
  - 新增 `frontend/src/components/charts/MultiTimeframeChart.vue`:
    主图（用户选择周期）+ 副图（1h/4h/1d 切换）的上下叠放布局，副图
    X 轴自动跟随主图 pan/zoom
  - X 轴同步通过 `window` 自定义事件 `kline-visible-range-change`
    解耦，`KlineChart.vue` 在 `initChart` 内部 dispatch，
    `MultiTimeframeChart.vue` 监听并 `setVisibleRange`
  - `MultiTimeframeChart` 暴露 `addSubBar(bar)`，父组件可在 WS 推送
    匹配当前副图周期时透传实盘 bar
  - `frontend/src/views/trade/TradingView.vue` 接入：
    用 `MultiTimeframeChart` 替换原来的 `KlineChart`，
    默认主图 1m / 副图 1h，所有指标（MA/EMA/MACD/KDJ/RSI/Bollinger/ATR/Stoch）
    仍由主图内部 `KlineChart` 承载
  - 单元测试 `frontend/src/__tests__/components/MultiTimeframeChart.test.ts`：
    7 个 case（双图渲染、副图周期切换、双 X 轴同步、addSubBar 路径）
  - E2E `e2e-tests/market_data.spec.ts`：新增 test 07 验证
    `GET /api/v1/kline/query` 在 `1m` 和 `1h` 两个周期都能返回 200/404
    （不出现 500），证明多周期读取路径完整

### 修复

- `frontend/src/components/charts/MultiTimeframeChart.vue`：
  `subVolumeSeries` 的 `scaleMargins` 误放到了 series options 上（应放
  在 price scale options），导致 `vue-tsc` 类型检查失败
- `frontend/src/components/charts/KlineChart.vue`：
  原 `initChart` 缺少 `subscribeVisibleTimeRangeChange`，
  导致 `MultiTimeframeChart` 的副图 X 轴同步代码永远收不到事件
  （修复后多周期联动才真正生效）

## [0.8.0] — 2026-05-21

### 新增

- **CI 覆盖率门禁 (T6)**
  - `cargo-llvm-cov` 集成，生成 LCOV 报告
  - 新增 `coverage-gate` job，阈值：整体 70% / handlers 90% / services 75% / db 70%

- **文档补全 (T7)**
  - 新增 `CONTRIBUTING.md`：环境要求、分支管理、Commit 规范
  - 新增 `docs/architecture/README.md`：架构概览、ADR 索引
  - 新增 `docs/api/README.md`：API 分组、文档索引

### 修复

- `docs/phase2/T9-Release-Readiness-Checklist.md`：TODO checkbox 修复

## [0.7.0] — 2026-05-15

### 新增

#### 交易执行模块

- **交易执行主页面** (`frontend/src/views/trade/TradingView.vue`)
  - 三栏布局：K线图区（flex:2）+ 下单表单 + 委托管理Tab
  - 响应式：≤1200px 右侧面板320px，≤900px 切换两栏
  - WebSocket模拟 + 实时买卖盘口显示
  - 设计token：深色背景 #08090a + 主色 #7170ff

- **组件更新**
  - `OrderList.vue`：买入/卖出颜色区分，空状态骨架屏
  - `OrderSummaryCards.vue`：去除骨架屏CSS动画
  - `PositionPanel.vue`：去除骨架屏CSS动画

- **API契约修复**
  - `closePosition`：`:id` → `:symbol`（路由 `/positions/:symbol/close`）
  - `Position.id/user_id`：number → string（UUID）
  - `PaperAccount.user_id`：number → string（UUID）

- **设计文档** (`docs/design/`)
  - `Design_TradingExecution.md`：1100行完整规格（US-TE-01~10全覆盖）
  - `TradingExecution_Checklist.md`：180项UI/UX走查清单
  - `prototype_trading_execution.html`：57KB交互原型

- **后端MVP完整实现** (ADR-010)
  - `close_position` handler：`POST /positions/:symbol/close`
  - 风控前置：交易对校验、余额检查、卖出持仓检查、保证金冻结
  - 11个API端点全部就绪

## [0.5.0] — 2026-05-13
## [0.6.0] — 2026-05-14

### 新增

#### 订单管理模块

- **订单管理前端页面** (`frontend/src/views/order/OrderManagementView.vue`)
  - 订单列表页（状态筛选/时间范围/交易对搜索）
  - 订单创建对话框（市价单/限价单）
  - 订单详情侧滑栏
  - 持仓面板
  - 成交记录 Tabs
  - 批量撤单功能

- **订单管理 API** (`backend/src/handlers/order.rs`)
  - `POST /api/v1/orders` — 创建委托（市价/限价/止损）
  - `GET /api/v1/orders` — 查询委托列表（分页+多条件筛选）
  - `GET /api/v1/orders/:id` — 委托详情
  - `POST /api/v1/orders/:id/cancel` — 撤单
  - `POST /api/v1/orders/cancel-all` — 批量撤单
  - `GET /api/v1/trades` — 成交记录
  - `GET /api/v1/positions` — 持仓列表
  - `GET /api/v1/account` — 账户信息
  - `GET /api/v1/symbols` — 交易对配置
  - 风控前置（D8）、保证金冻结（D3）、PG行锁（D2）



### 新增

#### K 线数据管理模块

- **K 线数据查询 API** (`/api/v1/kline/query`)
  - `GET /api/v1/kline/query` — 分页查询 K 线数据（symbol / interval / 时间范围）
  - `GET /api/v1/kline/latest` — 获取指定交易对 + 周期的最新一根 K 线
  - 支持 8 种周期：`1m` / `5m` / `15m` / `30m` / `1h` / `4h` / `1d` / `1w`
  - 强制 `user_id` 过滤（JWT），多用户数据隔离
  - Redis Read-Through 缓存（1m 周期 5min TTL，其他 1h TTL）
  - 时间范围超过 10 年返回 `400`
  - `gap_detected` 标记（连续超过 8h 无数据）

- **K 线数据导入 API** (`/api/v1/kline/import`)
  - `POST /api/v1/kline/import` — 批量导入（CSV / API / 交易所直采）
  - 支持拖拽上传 CSV 文件（FormData）和 JSON 数组导入
  - 分片处理：5000 条/批次，`COPY FROM STDIN` 并行写入
  - UPSERT 策略：`ON CONFLICT (user_id, symbol, interval, open_time) DO NOTHING`
  - 性能目标：10 万条 CSV 数据 < 30s
  - 文件大小限制：100 MB（超过返回 `413`）
  - `GET /api/v1/kline/import-history` — 导入历史记录（分页）

- **K 线数据导出 API** (`/api/v1/kline/export`)
  - `GET /api/v1/kline/export` — 导出为 CSV 或 JSON
  - 流式响应，支持大数据量导出
  - 可选字段导出（`fields` 参数）
  - 文件名格式：`kline_{symbol}_{interval}_{start}_{end}.{csv|json}`

- **K 线数据删除 API** (`/api/v1/kline`)
  - `DELETE /api/v1/kline` — 删除指定范围的 K 线数据（不可逆）

- **数据质量检测 API** (`/api/v1/kline/quality`)
  - `GET /api/v1/kline/quality` — 数据质量检测报告
  - 5 项检测规则：缺尖（>2×interval 无数据）、异常值（偏离 5 日均值 ±15%）、重复、零成交量、价格反向
  - 覆盖率计算：`valid_rows / total_rows`

- **数据清洗 API** (`/api/v1/kline/clean`)
  - `POST /api/v1/kline/clean` — 自动清洗（缺尖线性插值、重复去重、异常标记）
  - 清洗前自动创建 `kline_backup` 快照（7 天过期）
  - 异常数据标记 `suspicious` / `corrupted`，不自动删除
  - 返回 `backup_id` 用于回滚（P2 阶段实现）

#### 存储与缓存

- **TimescaleDB 分区存储**
  - `kline_data` 超表按 `open_time` 分区
  - `compress_segmentby` 按 `(user_id, symbol, interval)` 压缩
  - `drop_chunks_policy` 自动过期：1m/5m → 30 天，15m/30m/1h → 1 年，4h/1d/1w → 永久

- **Redis 热数据缓存**
  - Key 格式：`kline:{symbol}:{timeframe}:{ts}`
  - 仅缓存最近 24h 数据（内存保护）
  - 导入/清洗后主动 invalidate 对应 key prefix

#### 错误码扩展

| code | 含义 |
|------|------|
| 40003 | 数据校验失败（high < low / volume < 0） |
| 40401 | K 线数据不存在 |
| 41301 | 请求体过大（CSV 超 100 MB） |
| 50301 | Redis 缓存不可用（降级为直接查库） |

#### 数据库模型

| 表 | 说明 |
|----|------|
| `kline_data` | K 线数据主表（UUID 主键，复合唯一键 `(user_id, symbol, interval, open_time)`，TimescaleDB 超表） |
| `kline_import_log` | 导入日志（记录每次导入的统计信息） |
| `kline_quality_report` | 质量检测报告（存储历史报告） |
| `kline_backup` | 清洗前备份快照（7 天自动过期） |

#### 回测引擎 P0 阻塞项解除

- 实现 `db/kline.rs::load_klines()`，签名与 PRD section 8.2 一致，供 `backtest_engine.rs` 直接调用

---

# Changelog

## v0.3.0 (2026-05-13)

### 新增: 回测引擎模块

**回测引擎核心:**
- 全内存向量化回测引擎（O(n) 单次遍历 K 线数据）
- 异步执行：Semaphore 并发控制（上限 5 个同时运行）
- CancellationToken 取消支持（每 100 根 K 线检查）
- AtomicU32 实时进度追踪（每 ~10% 更新）
- 双向交易模拟：做多 (long) + 做空 (short)
- 真实成本模拟：手续费率 + 滑点率
- 爆仓检测：权益归零自动终止
- 合成 K 线数据生成（无真实数据时用于演示/测试）
- 权益曲线均匀采样（上限 2000 点，保留首尾）

**绩效指标 (14 项):**
- 总收益率、年化收益率、最大回撤
- Sharpe 比率、Sortino 比率、Calmar 比率
- 胜率、总交易次数、盈亏比
- 平均盈利、平均亏损、平均交易收益
- 总手续费、总滑点成本

**回测 API 端点 (7 个):**
- `POST /api/v1/backtest` — 运行回测（异步）
- `GET /api/v1/backtest/{id}` — 获取回测完整结果
- `GET /api/v1/backtest/{id}/trades` — 获取交易明细
- `GET /api/v1/backtest/{id}/equity` — 获取权益曲线
- `GET /api/v1/backtest/history` — 回测历史列表（分页）
- `DELETE /api/v1/backtest/{id}` — 删除回测记录
- `POST /api/v1/backtest/{id}/cancel` — 取消运行中回测

**文档:**
- 回测引擎 API 文档（7 个端点完整说明 + curl 示例 + 错误码）
- 回测引擎使用说明（流程 + 交易模拟 + 最佳实践 + FAQ）
- 绩效指标定义（14 项指标公式详解 + 参考标准）
- README 回测模块说明更新

## v0.2.0 (2026-05-13)

### 新增: 策略管理模块

**策略模板引擎:**
- 10 个预定义策略模板，覆盖趋势/均值回归/波动率/复合四大类别
- 统一的 `StrategyTemplate` trait，支持模板注册与按 ID 查询
- 每个模板包含独立参数 schema（`ParameterDef`: 名/类型/范围/默认值）
- 模板级参数校验（边界检查 + 业务约束如 `fast < slow`）
- MA Crossover、Triple MA、MACD、Bollinger Bands、RSI、Keltner Channels、ATR Stop Loss、Mean Reversion、Ichimoku Cloud、Double Bollinger

**策略 CRUD 端点 (7 个):**
- `GET /api/v1/strategies/templates` — 策略模板列表
- `POST /api/v1/strategies` — 创建策略（基于模板 + 参数校验）
- `GET /api/v1/strategies` — 分页查询用户策略列表
- `GET /api/v1/strategies/{id}` — 获取策略详情
- `PUT /api/v1/strategies/{id}` — 更新策略名称/参数
- `DELETE /api/v1/strategies/{id}` — 删除策略
- `POST /api/v1/strategies/{id}/status` — 策略状态流转

**状态机:**
- 四态流转: draft → active → paused → stopped
- 结构化校验拦截非法转换
- 所有端点绑定额外的所有权校验（`user_id` 匹配）

**文档:**
- 策略管理 API 文档（7 个端点完整说明 + curl 示例 + 错误码）
- 策略用户指南（模板详解 + 参数配置最佳实践 + 常见问题）
- README 策略模块说明更新
