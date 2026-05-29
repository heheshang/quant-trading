# Quant Trading 量化交易系统

> 基于 Rust + Axum + PostgreSQL 的量化交易后端系统

## 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 前端框架 | Vue 3 + TypeScript | 组合式 API + Pinia 状态管理 |
| UI 组件 | Element Plus | 金融风格组件库 |
| 图表 | lightweight-charts | TradingView K 线图表 |
| 后端 | Rust + Axum 0.8 | 异步 Web 框架，原生 WebSocket |
| 数据库 | PostgreSQL 16 + SeaORM | 关系型数据库 + ORM |
| 缓存 | Redis | WebSocket 会话 / 实时行情 |
| 认证 | JWT (jsonwebtoken) | 双 Token（access + refresh） |
| 部署 | Docker Compose | 前后端容器化 |

## 项目结构

```
quant-trading/
├── backend/                  # Rust 后端（Axum）
│   └── src/
│       ├── main.rs           # 入口 + 路由注册
│       ├── lib.rs            # 模块导出
│       ├── config.rs         # 配置管理
│       ├── db/               # SeaORM 实体层
│       ├── handlers/         # HTTP 处理器（认证/策略/交易/市场/风控）
│       ├── services/         # 业务逻辑层
│       ├── models/           # 请求/响应数据模型
│       ├── middleware/       # JWT 认证中间件
│       └── utils/            # 统一错误/响应
├── frontend/                 # Vue 3 前端
│   └── src/
│       ├── api/              # Axios HTTP 客户端
│       ├── assets/styles/    # 全局样式（CSS 变量系统）
│       ├── components/       # 公共组件（K 线图/订单表单/持仓面板）
│       ├── layouts/          # 主布局（侧边栏 + 顶栏）
│       ├── router/           # Vue Router 路由
│       ├── stores/           # Pinia 状态管理
│       ├── types/            # TypeScript 类型定义
│       └── views/            # 页面视图（11 个业务页面）
├── docs/                     # 文档
│   ├── architecture/         # 架构文档
│   ├── design/               # 设计文档（UI/UX 设计规范）
│   └── ...
└── docker-compose.yml        # Docker 编排（postgres/redis/backend/frontend）
```

## 快速开始

### 前置条件

- Rust 1.85+
- PostgreSQL 16+
- Docker & Docker Compose（可选）

### 配置

复制环境变量模板:

```bash
cp .env.example .env
```

编辑 `.env` 文件:

```env
DATABASE_URL=postgres://user:password@localhost:5432/quant_trading
JWT_SECRET=your-secret-key
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
```

### 启动数据库（Docker）

```bash
docker compose up -d db
```

### 运行后端

```bash
cd backend
cargo run
```

启动后访问 `http://localhost:3000/api/v1/health` 验证。

## 数据库模型

系统包含以下核心数据表:

| 表名 | 说明 |
|------|------|
| `users` | 用户账户（含角色 + 权限） |
| `strategies` | 交易策略（含参数/状态/模板类型） |
| `orders` | 委托订单 |
| `trades` | 成交记录 |
| `positions` | 持仓记录 |
| `kline_data` | K 线行情数据 |
| `backtest_results` | 回测结果 |

> 完整 ER 图见 [数据模型文档](./docs/architecture/data-model.md)。

## 模块: 策略管理

策略模块提供量化交易策略的完整生命周期管理。

### API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/v1/strategies/templates` | 获取策略模板列表 |
| POST | `/api/v1/strategies` | 创建策略 |
| GET | `/api/v1/strategies` | 查询用户策略列表 |
| GET | `/api/v1/strategies/{id}` | 获取策略详情 |
| PUT | `/api/v1/strategies/{id}` | 更新策略 |
| DELETE | `/api/v1/strategies/{id}` | 删除策略 |
| POST | `/api/v1/strategies/{id}/status` | 变更策略状态 |

### 策略模板

10 个预定义模板，覆盖 4 个类别:

| 类别 | 模板 |
|------|------|
| trend | MA Crossover, Triple MA, MACD, Ichimoku Cloud |
| mean_reversion | RSI, Mean Reversion |
| volatility | Bollinger Bands, Keltner Channels, ATR Stop Loss |
| composite | Double Bollinger Bands |

### 状态流转

```
draft → active → paused → stopped
```

> 详细文档见 [策略 API 文档](./docs/strategy-api.md) 和 [用户指南](./docs/strategy-user-guide.md)。

## 模块: 回测引擎

回测引擎提供策略在历史数据上的模拟运行，输出绩效指标、交易明细和权益曲线。

### API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/backtest` | 运行回测（异步） |
| GET | `/api/v1/backtest/{id}` | 获取回测完整结果 |
| GET | `/api/v1/backtest/{id}/trades` | 获取交易明细 |
| GET | `/api/v1/backtest/{id}/equity` | 获取权益曲线（采样至 2000 点） |
| GET | `/api/v1/backtest/history` | 回测历史列表（分页） |
| DELETE | `/api/v1/backtest/{id}` | 删除回测记录 |
| POST | `/api/v1/backtest/{id}/cancel` | 取消运行中回测 |

### 核心特性

- **异步执行**：Semaphore 信号量控制并发（上限 5）
- **双向交易**：支持做多 (long) 和做空 (short)
- **成本模拟**：手续费 + 滑点真实计算
- **实时进度**：AtomicU32 进度追踪 + CancellationToken 取消支持
- **14 项绩效指标**：Sharpe / Sortino / Calmar / 最大回撤 / 盈亏比等

### 状态流转

```
running → completed  (成功完成)
       → failed      (执行失败/被取消)
```

## 文档索引

| 文档 | 说明 |
|------|------|
| [认证 API](./docs/auth-api.md) | 注册/登录/登出/刷新 Token |
| [策略 API](./docs/strategy-api.md) | 策略管理 7 个端点完整说明 |
| [策略用户指南](./docs/strategy-user-guide.md) | 模板详解 + 参数配置最佳实践 |
| [回测引擎 API](./docs/backtest-api.md) | 回测 7 个端点完整说明 + curl 示例 + 错误码 |
| [回测使用说明](./docs/backtest-user-guide.md) | 回测流程 + 交易模拟 + 最佳实践 + FAQ |
| [绩效指标定义](./docs/backtest-metrics.md) | 14 项指标公式详解 + 参考标准 |
| [数据模型](./docs/architecture/data-model.md) | 数据库表结构 + ER 图 |
| [PRD](./docs/prd/PRD.md) | 产品需求文档 |
| [架构概览](./docs/architecture/architecture-overview.html) | 系统架构图 |

## 开发

运行测试:

```bash
cd backend
cargo test       # 单元测试
cargo clippy     # 代码检查
```

## 版本

当前版本: v1.0.0 — [CHANGELOG](./CHANGELOG.md)

### 版本历史

| 版本 | 日期 | 主要变更 |
|------|------|---------|
| v1.0.0 | 2026-05-29 | UI 全面重构（Financial Dashboard 风格 + Trust Blue + 铺满布局 + K 线图表优化） |
| v0.3.0 | 2026-05-13 | 新增回测引擎模块（异步回测 + 14 项绩效指标 + 双向交易） |
| v0.2.0 | 2026-05-13 | 新增策略管理模块（模板引擎 + CRUD + 状态流转） |
| v0.1.0 | 2026-05-11 | 初始版本：认证系统 + RBAC 权限 + 用户管理 |
