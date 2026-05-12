# Quant Trading 量化交易系统

> 基于 Rust + Axum + PostgreSQL 的量化交易后端系统

## 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 运行环境 | Rust 2021 | 高性能系统级语言 |
| Web 框架 | Axum 0.8 | 异步 Web 框架，原生支持 WebSocket |
| 数据库 | PostgreSQL 16 + SeaORM | 关系型数据库 + ORM |
| 认证 | JWT (jsonwebtoken) | 双 Token 机制（access + refresh） |
| 密码 | bcrypt | 密码哈希存储 |
| 序列化 | Serde + serde_json | JSON 序列化/反序列化 |

## 项目结构

```
quant-trading/
├── backend/                  # Rust 后端
│   └── src/
│       ├── main.rs           # 入口 + 路由注册
│       ├── lib.rs            # 模块导出
│       ├── config.rs         # 配置管理
│       ├── db/               # 数据库层（SeaORM 实体 + 迁移）
│       │   ├── mod.rs
│       │   ├── strategy.rs   # 策略表实体
│       │   ├── user.rs       # 用户表实体
│       │   ├── permission.rs # 权限表实体
│       │   └── ...
│       ├── handlers/         # HTTP 处理器
│       │   ├── mod.rs
│       │   ├── auth.rs       # 认证处理器
│       │   ├── strategy.rs   # 策略 CRUD 处理器
│       │   ├── users.rs      # 用户管理处理器
│       │   └── ws.rs         # WebSocket 处理器
│       ├── services/         # 业务逻辑层
│       │   ├── mod.rs
│       │   ├── auth.rs       # 认证服务
│       │   └── strategy.rs   # 策略服务（模板引擎 + CRUD）
│       ├── models/           # 数据模型
│       │   ├── mod.rs
│       │   └── schemas.rs    # 请求/响应 Schema
│       ├── middleware/       # 中间件
│       │   ├── mod.rs
│       │   └── auth.rs       # JWT 认证中间件
│       └── utils/            # 工具函数
│           ├── mod.rs
│           ├── error.rs      # 统一错误处理
│           └── response.rs   # 统一响应格式
├── frontend/                 # 前端（Vue/React）
├── docs/                     # 文档
│   ├── architecture/         # 架构文档
│   ├── design/               # 设计文档
│   ├── prd/                  # 产品需求文档
│   ├── auth-api.md           # 认证 API 文档
│   └── strategy-api.md       # 策略管理 API 文档
└── docker-compose.yml        # Docker 编排
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

## 文档索引

| 文档 | 说明 |
|------|------|
| [认证 API](./docs/auth-api.md) | 注册/登录/登出/刷新 Token |
| [策略 API](./docs/strategy-api.md) | 策略管理 7 个端点完整说明 |
| [策略用户指南](./docs/strategy-user-guide.md) | 模板详解 + 参数配置最佳实践 |
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

当前版本: v0.2.0 — [CHANGELOG](./CHANGELOG.md)

### 版本历史

| 版本 | 日期 | 主要变更 |
|------|------|---------|
| v0.2.0 | 2026-05-13 | 新增策略管理模块（模板引擎 + CRUD + 状态流转） |
| v0.1.0 | 2026-05-11 | 初始版本：认证系统 + RBAC 权限 + 用户管理 |
