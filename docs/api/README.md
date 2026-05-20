# API 文档

- Version: 1.0.0
- Date: 2026-05-20
- Author: ssk

## 概述

后端 API 基于 Axum 0.7 构建，RESTful 风格，认证方式为 API Key + HMAC 签名。

## 基础信息

- 基础路径: `/api/v1`
- 认证: `X-API-Key` + `X-Signature` + `X-Timestamp` 请求头
- 内容类型: `application/json`

## 核心 API 分组

| 前缀 | 说明 |
|------|------|
| `/api/v1/auth` | 认证 (登录/登出) |
| `/api/v1/market` | 市场数据 (行情/K线) |
| `/api/v1/portfolio` | 账户资产组合 |
| `/api/v1/orders` | 订单管理 |
| `/api/v1/strategies` | 策略管理 |
| `/api/v1/backtest` | 回测引擎 |
| `/api/v1/risk` | 风险规则 |
| `/api/v1/trigger` | 条件单/止盈止损 |
| `/api/v1/alerts` | 告警配置 |
| `/api/v1/export` | 数据导出 (CSV/Excel) |

## 详细 API 文档

| 文档 | 路径 |
|------|------|
| 交易执行 API | `docs/api/TradingExecution_API.md` |
| 账户资产 API | `docs/api/Portfolio_API.md` |
| K线管理 API | `docs/api/KlineManagement_API.md` |
| 回测 API | `docs/api/backtest-api.md` |

## WebSocket

- 路径: `/ws`
- 认证: URL Query 参数 `?token=xxx`
- 频道: `market:ticker:{symbol}`, `market:kline:{symbol}:{interval}`, `trade:{user_id}`

## 错误码

| 错误码 | 说明 |
|--------|------|
| 400 | 请求参数错误 |
| 401 | 认证失败 |
| 403 | 无权限 |
| 404 | 资源不存在 |
| 429 | 请求频率超限 |
| 500 | 服务器内部错误 |
