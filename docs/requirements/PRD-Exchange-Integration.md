---
status: WIP
date: 20260601
---

# PRD-Exchange-Integration-20260601 — 交易所直连接口

## 1. 功能概述

Exchange Integration 模块为用户提供交易所（Binance）直连 REST API 访问能力，包括连通性测试、账户余额查询、下单、撤单和频率限制查询。所有请求经过后端 JWT 认证，并通过 HMAC 签名透传到交易所。

**目标用户**：量化交易员（实盘交易）

---

## 2. 用户故事

- **US-E1**：作为交易员，我希望测试与 Binance 的连通性（延迟/状态），以便确认网络正常
- **US-E2**：作为交易员，我希望查看我在 Binance 的账户余额（所有币种），以便决定是否需要充值
- **US-E3**：作为交易员，我希望通过平台直接下单/撤单，而不是登录交易所网站，以便提高交易效率
- **US-E4**：作为交易员，我希望了解当前频率限制使用情况，避免触发交易所限流

---

## 3. API 端点

### 3.1 GET /api/v1/exchange/ping

**描述**：测试与 Binance 的连通性，返回 Binance 服务器时间

**认证**：JWT

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": {
    "server_time": 1717200000000,
    "status": "ok"
  },
  "message": "success"
}
```

---

### 3.2 GET /api/v1/exchange/account

**描述**：获取 Binance 账户余额（所有币种）

**认证**：JWT + HMAC（需用户已配置 API Key）

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": {
    "balances": [
      {
        "asset": "USDT",
        "free": "10000.00000000",
        "locked": "500.00000000"
      },
      {
        "asset": "BTC",
        "free": "0.12345678",
        "locked": "0.00000000"
      }
    ]
  },
  "message": "success"
}
```

---

### 3.3 POST /api/v1/exchange/order

**描述**：在 Binance 下单（限价单/市价单）

**认证**：JWT + HMAC

**请求体**：
```json
{
  "symbol": "BTCUSDT",
  "side": "BUY",
  "type": "LIMIT",
  "quantity": "0.01",
  "price": "60000.00",
  "time_in_force": "GTC"
}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| symbol | string | 是 | 交易对，如 `BTCUSDT` |
| side | string | 是 | `BUY` 或 `SELL` |
| type | string | 是 | `LIMIT` 或 `MARKET` |
| quantity | string | 否 | 数量（市价单可省略） |
| price | string | 否 | 价格（限价单必填） |
| time_in_force | string | 否 | `GTC`/`IOC`/`FOK`（限价单有效） |
| quote_order_qty | string | 否 | 报价金额（，市价买单用） |

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": {
    "order_id": "123456789",
    "symbol": "BTCUSDT",
    "side": "BUY",
    "order_type": "LIMIT",
    "status": "NEW",
    "executed_qty": "0.00000000",
    "fills": []
  },
  "message": "success"
}
```

---

### 3.4 DELETE /api/v1/exchange/order/{orderId}

**描述**：撤销指定订单

**认证**：JWT + HMAC

**请求体**：
```json
{
  "symbol": "BTCUSDT"
}
```

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": {
    "order_id": "123456789",
    "status": "CANCELLED"
  },
  "message": "success"
}
```

---

### 3.5 GET /api/v1/exchange/rate-limit

**描述**：查询频率限制状态（本地计数器 + Binance API 限制）

**认证**：JWT + HMAC

**响应** `200 OK`：
```json
{
  "code": 0,
  "data": {
    "user_id": "uuid",
    "local_remaining": 45,
    "local_reset_at_ms": 1717200060000,
    "binance_limit": {
      "rate_limit_type": "ORDERS",
      "interval": "SECOND",
      "interval_num": 1,
      "limit": 120,
      "count": 12
    }
  },
  "message": "success"
}
```

---

## 4. 数据库模型

### 4.1 表：`exchange_api_keys`

| 字段 | 类型 | 说明 |
|------|------|------|
| id | UUID | 主键 |
| user_id | UUID | 用户ID（唯一） |
| exchange | VARCHAR(20) | 交易所标识：`binance` |
| api_key | VARCHAR(64) | API 公钥（加密存储） |
| api_secret_enc | VARCHAR(128) | API 密钥（加密存储） |
| permissions | VARCHAR(50) | 权限：`trade+withdraw` |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

> 密钥使用 AES-256-GCM 加密，密钥存储在环境变量或 Vault 中。

---

## 5. 业务流程

```
客户端请求 → JWT 验证 → 查 exchange_api_keys（获取密钥）
                                    │
                    ├─→ HMAC 签名请求 → Binance API
                    │                       │
                    │               ←─ 响应 ─┤
                    │                       │
                    └─←─ 响应 ←────────────┘
```

**频率限制**：
- 本地滑动窗口计数器：每用户 60 请求/秒
- Binance API 限制：通过 `/rate-limit` 端点查询

---

## 6. 边界条件

- 用户未配置 API Key 时，返回 `401 Unauthorized`
- Binance 返回错误码时，透传错误信息
- 网络超时不阻塞其他用户（独立 tokio task）
- HMAC 签名失败返回 `403 Forbidden`

---

## 7. 验收标准（Gherkin 格式）

```gherkin
Feature: 交易所直连接口

  Scenario: 测试连通性
    Given 用户已登录且已配置 API Key
    When 用户调用 GET /exchange/ping
    Then 返回 Binance 服务器时间和状态 "ok"
    And 响应时间 < 500ms

  Scenario: 查询账户余额
    Given 用户已登录且已配置 Binance API Key
    When 用户调用 GET /exchange/account
    Then 返回所有币种余额（asset/free/locked）
    And 未配置的币种不显示

  Scenario: 下限价单
    Given 用户已登录且已配置 API Key
    When 用户提交 POST /exchange/order（symbol=BTCUSDT, side=BUY, type=LIMIT）
    Then 返回 order_id 和状态 NEW
    And 订单实际下到 Binance

  Scenario: 撤销订单
    Given 用户在 Binance 有一笔挂单
    When 用户调用 DELETE /exchange/order/{id}
    Then 订单被撤销
    And 返回状态 CANCELLED

  Scenario: 查询频率限制
    Given 用户已登录
    When 用户调用 GET /exchange/rate-limit
    Then 返回本地剩余次数和 Binance 限制信息

  Scenario: 未配置 API Key 时拒绝访问
    Given 用户已登录但未配置交易所 API Key
    When 用户调用 GET /exchange/account
    Then 返回 401 错误
    And 提示 "请先配置交易所 API Key"
```
