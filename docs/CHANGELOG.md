# Changelog

> Quant Trading Backend — 所有重要变更均记录在此文件中.

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/), 版本号遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/).

---

## [0.9.0] — 2026-05-21

### 新增

#### Phase 4 风控模块

- **F2: 单笔亏损限制**
  - `risk_manager.rs` 实现真实单笔亏损率计算
  - 公式：`|order_price - stop_loss_price| * quantity / equity`
  - 集成到 `order.rs` 下单流程

- **F6: 策略状态管理**
  - 新增 `StrategyStateManager` 服务
  - 断线监控后台任务（30s 检测 Binance WS 连接状态）
  - 断线时自动暂停策略

- **F8: 紧急全平后端增强**
  - 新增错误码 `RF-004`（RiskPaused 拒绝新订单）
  - 平仓前检查资产余额

- **F14: 紧急全平前端**
  - 新增 `RiskDashboardView.vue` 页面
  - 二次确认对话框 + 逐笔订单盈亏展示
  - `useRiskDashboard.ts` Composable

- **F15: 断线状态指示器**
  - 实时轮询 `/api/v1/risk/connection-status`（10s 间隔）
  - 三级颜色圆点 + 脉冲动画 + 策略暂停标签

### 新增 ADR

- ADR-015: ATR 追踪止损
- ADR-016: 策略状态控制
- ADR-017: 风控仪表盘前端

### 测试

- 后端单元测试：254（+6 风控测试）
- 前端单元测试：708

---

## [0.7.0] — 2026-05-14

### 新增

#### Portfolio 组合权益模块

- **组合权益 API** (`/api/v1/portfolio`)
  - `GET /api/v1/portfolio/summary` — 获取组合权益汇总（总资产/当日盈亏/累计盈亏/持仓数）
  - `GET /api/v1/portfolio/positions` — 获取持仓列表（分页/按交易对筛选）
  - `GET /api/v1/portfolio/performance` — 获取多策略绩效对比数据
  - `GET /api/v1/portfolio/equity_curve` — 获取权益曲线时序数据（支持粒度/日期范围）

- **组合权益汇总** (`/portfolio/summary`)
  - 从 `paper_accounts` 聚合总资产和累计盈亏
  - 从当日已成交订单计算当日盈亏
  - 支持普通用户查看自己、管理员查看任意用户

- **持仓列表** (`/portfolio/positions`)
  - 分页查询，支持按 `symbol` 筛选
  - 返回持仓方向（long/short）、数量、均价、现价、浮动盈亏
  - 分页参数 `page`（默认 1）和 `size`（默认 20，范围 1–100）

- **策略绩效对比** (`/portfolio/performance`)
  - 从 `backtest_results.metrics` 提取策略绩效指标
  - 统计各策略已成交订单数（`trade_count`）
  - 返回策略级别的总盈亏、盈亏率、最大回撤、胜率

- **权益曲线** (`/portfolio/equity_curve`)
  - 从 `portfolio_equity_history` 查询时序数据
  - 支持日期范围筛选（`start_date`/`end_date`，格式 `YYYY-MM-DD`）
  - 支持 `hour`/`day` 粒度聚合（同粒度取最后一条快照）

- **权益快照服务**
  - `record_equity_snapshot` 函数：写入权益快照到 `portfolio_equity_history`

#### 权限模型

- 统一访问控制：普通用户仅可查看自己，管理员可通过 `user_id` 查看他人
- `check_access` 辅助函数：越权访问返回 `40301` 错误

#### 数据库模型

| 表 | 说明 |
|---|------|
| `portfolio_equity_history` | 权益历史快照（`user_id` + `equity` + `timestamp`） |

### 已知限制

| 编号 | 限制 | 计划 |
|------|------|------|
| BA-03 | `/performance` 缺少组合整体 `max_drawdown`/`sharpe_ratio`/`win_rate` 指标 | v0.8.0 补充 |
| BA-04 | `week` 粒度聚合逻辑与 `day` 相同 | v0.8.0 补充 |
| BA-05 | `/positions` 不支持 `side` 筛选参数 | v0.8.0 补充 |
| BA-08 | `current_price` 使用 `avg_entry_price` 占位 | 接入行情服务后修复 |

---

## [0.4.0] — 2026-05-13

### 新增

#### 策略管理模块

- **策略 CRUD API** (`/api/v1/strategies`)
  - `GET /api/v1/strategies` — 获取当前用户策略列表（分页/状态筛选/名称搜索）
  - `POST /api/v1/strategies` — 创建新策略（基于模板，自动校验参数）
  - `GET /api/v1/strategies/{id}` — 获取单个策略详情
  - `PUT /api/v1/strategies/{id}` — 更新策略名称/参数（状态限制）
  - `DELETE /api/v1/strategies/{id}` — 删除策略（仅 draft/stopped）

- **策略状态机**
  - draft → active → paused → stopped 完整状态流转
  - 活跃策略上限检查（每用户最多 5 个 active 策略）
  - SELECT FOR UPDATE 事务保护状态切换原子性

- **策略模板引擎** (`/api/v1/strategies/templates`)
  - 10 个内置策略模板：双均线交叉、三均线、MACD、一目均衡表、布林带、RSI、均值回归、肯特纳通道、ATR 止损、双布林带
  - `GET /api/v1/strategies/templates` — 列出所有模板（含参数定义）
  - 参数类型支持：integer(min/max)、float(min/max)、select(options)、boolean
  - 参数校验：必填性、类型、范围、未知参数检测

- **导入/导出 API**
  - `GET /api/v1/strategies/{id}/export` — 导出策略为 JSON
  - `POST /api/v1/strategies/{id}/import` — 从 JSON 导入策略（自动重命名冲突）

#### 错误码扩展

| code | 含义 |
|------|------|
| 40401 | 策略不存在 |
| 40402 | 模板不存在 |
| 40403 | 导入时模板不存在 |
| 40901 | 策略名称重复 |
| 40902 | 模板正被使用 |
| 42201 | 非法状态转换 |
| 42901 | 活跃策略数超过上限（5个） |

#### 数据库模型

| 表 | 说明 |
|---|------|
| `user_strategies` | 用户策略实例（UUID 主键，关联 user + template） |
| `strategy_templates` | 策略模板（内置 10 个 + 预留自定义模板） |

---

## [0.3.0] — 2026-05-13

### 新增

#### 回测引擎模块

- **回测 API** (`/api/v1/backtest`)
  - `POST /api/v1/backtest` — 提交回测任务（异步，Semaphore 并发控制 ≤5）
  - `GET /api/v1/backtest/{id}` — 获取回测完整结果
  - `GET /api/v1/backtest/{id}/trades` — 获取交易明细
  - `GET /api/v1/backtest/{id}/equity` — 获取权益曲线（采样至 2000 点）
  - `GET /api/v1/backtest/history` — 回测历史列表（分页）
  - `DELETE /api/v1/backtest/{id}` — 删除回测记录
  - `POST /api/v1/backtest/{id}/cancel` — 取消运行中的回测

- **回测引擎架构**
  - 全内存向量化计算，O(n) 单次遍历 K 线
  - 实时进度跟踪（AtomicU32，每 ~10% 更新）
  - 可取消（CancellationToken，每 100 根 K 线检查）
  - 双向交易：做多（long）+ 做空（short）
  - 手续费与滑点模拟
  - 爆仓检测（权益归零自动终止）

- **绩效指标**
  - 年化收益率、最大回撤、夏普比率、索提诺比率、胜率、盈亏比
  - 总交易次数、持仓时长分布

- **合成 K 线**
  - 支持 1m/5m/15m/30m/1h/2h/4h/1d/1w 任意周期组合

---

## [0.2.0] — 2026-05-12

### 新增

#### 策略沙箱模块

- **模板引擎** (`backend/src/templates/`)
  - `StrategyTemplate` trait：每个模板实现 `generate_signal()` 方法
  - 10 个预定义策略模板（MADelta、MeanReversion、Cont Crack 等）
  - `TemplateRegistry` 全局注册表，`LazyLock` 线程安全初始化

- **参数校验** (`validate_params`)
  - 必填参数存在性检查
  - 参数类型匹配（Integer/Float/Select/Boolean）
  - 数值范围校验（min/max）
  - 未知参数拒绝

---

## [0.1.0] — 2026-05-12

### 新增

#### 认证系统

- **用户注册** (`POST /api/v1/auth/register`)
  - 支持用户名 (3–32 字符)、邮箱、密码 (≥8 字符) 注册
  - 自动分配 `user` 角色
  - 注册成功即返回 JWT token 对 (access + refresh)

- **用户登录** (`POST /api/v1/auth/login`)
  - 支持用户名或邮箱两种方式登录
  - bcrypt 密码验证
  - 更新 `last_login_at` 时间戳
  - 生成 JWT token 对 + 创建 session 记录

- **Token 刷新** (`POST /api/v1/auth/refresh`)
  - access token 默认 15 分钟有效期
  - refresh token 默认 7 天有效期, 支持轮转 (刷新时生成新 token 对, 撤销旧 session)

- **用户登出** (`POST /api/v1/auth/logout`)
  - 撤销当前用户所有活跃 session

#### 用户管理

- **获取当前用户信息** (`GET /api/v1/auth/me`, `GET /api/v1/users/me`)
- **更新个人资料** (`POST /api/v1/users/me`)
  - 支持更新 display_name、email (唯一性校验)、avatar_url
- **修改密码** (`POST /api/v1/users/me/password`)
  - 需验证旧密码, 新密码 ≥ 8 字符
- **用户列表** (`GET /api/v1/users`)
  - 分页查询, 默认每页 20 条
- **管理员更新用户** (`POST /api/v1/users/{id}`)
  - 支持更新 display_name、email、avatar_url、is_active、role_id
- **管理员删除用户** (`DELETE /api/v1/users/{id}`)

#### 角色与权限 (RBAC)

- **角色列表** (`GET /api/v1/roles`)
  - 提供 admin (管理员) 和 user (普通用户) 两个内置角色
  - 角色自动种子化 (首次启动时)
- JWT 中间件 (`middleware/auth.rs`)
  - Bearer Token 校验 + token 类型检查
  - 解析子: user_id、username、role、jti
- 预留角色鉴权中间件 `require_role` (`middleware/auth.rs`)
- 预留 `permissions` 与 `role_permissions` 数据表及 SeaORM 实体

#### 基础设施

- **健康检查** (`GET /api/v1/health`)
  - 返回服务状态、版本号、当前时间戳
- **WebSocket** (`GET /api/v1/ws`)
  - HTTP 升级至 WebSocket, 当前实现为 echo 模式
  - 预留 broadcast channel 架构 (`WsManager`)
- **统一响应格式**
  - 成功: `{"code": 0, "data": {...}, "message": "success"}`
  - 错误: `{"code": NNNNN, "message": "..."}` (含语义化的错误码)
- **CORS**: 全开放 (开发环境), 可通过 `CORS_ORIGINS` 环境变量限制
- **日志**: JSON 格式结构化日志, 通过 `LOG_LEVEL` 控制级别
- **数据库**: 启动时自动建表 + 种子化默认角色

#### 数据库模型

| 表 | 说明 |
|---|------|
| `users` | 用户 (UUID 主键, 关联 role) |
| `roles` | 角色 (admin + user) |
| `permissions` | 权限定义 (resource + action) |
| `role_permissions` | 多对多: 角色↔权限 |
| `user_sessions` | 用户会话 (refresh token hash 存储) |

#### 错误码体系

| code | 含义 |
|------|------|
| 0 | 成功 |
| 40001 | 请求参数错误 |
| 40002 | 校验失败 |
| 40101 | 认证失败 / 密码错误 |
| 40102 | Token 过期 |
| 40103 | Token 无效 |
| 40301 | 权限不足 / 账号禁用 |
| 40401 | 资源不存在 |
| 40901 | 资源冲突 |
| 42901 | 请求频率超限 |
| 50001 | 内部错误 |
| 50002 | 数据库错误 |

### 技术栈

- Rust 2021 edition, Axum 0.8, Tokio, SeaORM 1.x (PostgreSQL), JWT (jsonwebtoken 9.x), bcrypt 0.16, tower-http 0.6, tracing, serde

### 项目结构

```
22 source files, ~1,866 lines of Rust code
```

### 已知限制

- WebSocket 当前仅实现 echo, 尚未接入行情数据广播
- `permissions` 和 `role_permissions` 表已建, 但尚无管理 API 和运行时权限校验
- Redis 依赖已声明但未使用
- 暂无单元测试 (`dev-dependencies` 中已声明 rstest / axum-test / tokio-test)
