# 架构文档

- Version: 1.0.0
- Date: 2026-05-20
- Author: ssk

## 概述

量化交易系统后端采用 **Rust + Axum + SeaORM** 技术栈，前端采用 **Vue 3 + TypeScript + Vite**。

## 后端架构

```
src/
├── handlers/     # API 处理器 (17个)
├── services/     # 业务逻辑层
├── db/          # SeaORM 数据模型
├── models/      # 请求/响应数据模型
├── middleware/  # 认证、日志中间件
└── services/    # 核心服务
    ├── exchange/     # 交易所连接器 (Binance)
    ├── indicator.rs  # 技术指标 (KDJ)
    ├── matching_engine.rs
    ├── backtest.rs
    └── risk_manager.rs
```

## 技术选型

| 组件 | 技术 | 说明 |
|------|------|------|
| Web 框架 | Axum 0.7 | 异步、类型安全 |
| ORM | SeaORM | 异步 SQLx 封装 |
| 数据库 | PostgreSQL 16 | 主数据存储 |
| 缓存 | Redis 7 | 会话、实时数据 |
| 实时通信 | WebSocket | 市场数据推送 |
| 交易所 | Binance | 实盘接口 |

## 核心模块

### 市场数据流水线 (ADR-005)
- Binance WebSocket 实时行情
- K线聚合 (1m/5m/15m/1h/4h/1d)
- KDJ 指标计算

### 订单管理 (ADR-010)
- 限价单/市价单/止损单
- 仓位管理
- 风险规则引擎

### 回测引擎 (ADR-004/007)
- 历史 K线重放
- 模拟订单执行
- 性能指标计算

## ADR 索引

| 编号 | 标题 |
|------|------|
| ADR-001 | Rust + Axum 后端架构 |
| ADR-002 | WebSocket 实时推送 |
| ADR-003 | 策略引擎沙箱 |
| ADR-004 | 回测引擎 |
| ADR-005 | 市场数据流水线 |
| ADR-006 | Binance WS Connector |
| ADR-009 | Portfolio 模块 |
| ADR-010 | 订单管理 |
| ADR-012 | API 签名认证 |
| ADR-013 | 风险管理器 |
| ADR-014 | 应急风控 |

详见 `docs/architecture/` 目录。
