# 策略管理 API 文档

> 版本: v0.2.0 | 基准路径: `/api/v1`

---

## 目录

- [1. 概述](#1-概述)
- [2. 通用约定](#2-通用约定)
  - [2.1 认证方式](#21-认证方式)
  - [2.2 响应格式](#22-响应格式)
  - [2.3 错误码](#23-错误码)
- [3. 策略模板 API](#3-策略模板-api)
  - [3.1 获取模板列表](#31-获取模板列表)
- [4. 策略 CRUD API](#4-策略-crud-api)
  - [4.1 创建策略](#41-创建策略)
  - [4.2 查询策略列表](#42-查询策略列表)
  - [4.3 获取策略详情](#43-获取策略详情)
  - [4.4 更新策略](#44-更新策略)
  - [4.5 删除策略](#45-删除策略)
- [5. 策略状态 API](#5-策略状态-api)
  - [5.1 变更策略状态](#51-变更策略状态)
- [6. 策略模板参考](#6-策略模板参考)

---

## 1. 概述

策略管理模块提供量化交易策略的完整生命周期管理，包含:

| 功能 | 说明 |
|------|------|
| **策略模板** | 10 个预定义策略模板，覆盖趋势、均值回归、波动率等类别 |
| **策略 CRUD** | 创建、查询、更新、删除策略 |
| **状态流转** | draft → active → paused → stopped 的标准化状态机 |
| **参数校验** | 每个模板拥有独立的参数 schema 与校验规则 |

所有策略端点（除 `/health` 和 WebSocket）均需 JWT 认证。

---

## 2. 通用约定

### 2.1 认证方式

使用 `Bearer Token` 方式在 HTTP 请求头中携带 JWT:

```http
Authorization: Bearer <access_token>
```

获取方式见 [认证 API 文档](./auth-api.md)。

### 2.2 响应格式

**成功响应:**

```json
{
  "code": 0,
  "data": { ... },
  "message": "success"
}
```

**错误响应:**

```json
{
  "code": 40401,
  "message": "Not found: Strategy not found"
}
```

### 2.3 错误码

| 错误码 | HTTP 状态码 | 说明 |
|--------|------------|------|
| 0 | 200 | 成功 |
| 40001 | 400 | 请求参数错误 |
| 40002 | 400 | 参数校验失败（如模板参数校验不通过） |
| 40101 | 401 | 无效凭证 |
| 40102 | 401 | Token 过期 |
| 40103 | 401 | Token 无效 |
| 40301 | 403 | 无权限 |
| 40401 | 404 | 资源不存在 |
| 40901 | 409 | 资源冲突 |
| 42901 | 429 | 请求频率超限 |
| 50001 | 500 | 服务器内部错误 |
| 50002 | 500 | 数据库错误 |

---

## 3. 策略模板 API

### 3.1 获取模板列表

获取所有可用的策略模板列表，包含默认参数和参数 schema。

```
GET /api/v1/strategies/templates
```

**请求头:**

```
Authorization: Bearer <access_token>
```

**响应示例:**

```json
{
  "code": 0,
  "data": [
    {
      "id": "ma_crossover",
      "name": "MA Crossover",
      "description": "Moving average crossover strategy using fast and slow MAs",
      "category": "trend",
      "default_parameters": { "fast_period": 10, "slow_period": 30 },
      "parameter_schema": [
        {
          "name": "fast_period",
          "param_type": "integer",
          "label": "Fast Period",
          "description": "Fast moving average period",
          "default": 10,
          "min": 5,
          "max": 50,
          "options": null
        },
        {
          "name": "slow_period",
          "param_type": "integer",
          "label": "Slow Period",
          "description": "Slow moving average period",
          "default": 30,
          "min": 10,
          "max": 200,
          "options": null
        }
      ]
    }
  ],
  "message": "success"
}
```

> 完整模板列表见 [6. 策略模板参考](#6-策略模板参考)。

**错误码:**

| 错误码 | 条件 |
|--------|------|
| 40101 | Token 缺失或无效 |

**curl 示例:**

```bash
curl -s -H "Authorization: Bearer <access_token>" \
  http://localhost:3000/api/v1/strategies/templates | jq .
```

---

## 4. 策略 CRUD API

### 4.1 创建策略

基于模板创建新策略，系统自动校验参数合法性。

```
POST /api/v1/strategies
```

**请求头:**

```
Authorization: Bearer <access_token>
Content-Type: application/json
```

**请求体:**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | 是 | 策略名称，1-100 字符 |
| `template_type` | string | 是 | 模板 ID（如 `ma_crossover`） |
| `parameters` | object | 是 | 策略参数，由具体模板的 schema 定义 |

**请求示例:**

```json
{
  "name": "My MA Crossover",
  "template_type": "ma_crossover",
  "parameters": {
    "fast_period": 10,
    "slow_period": 30
  }
}
```

**响应示例 (201):**

```json
{
  "code": 0,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": "a1b2c3d4-e29b-41d4-a716-446655440001",
    "name": "My MA Crossover",
    "template_type": "ma_crossover",
    "parameters": {
      "fast_period": 10,
      "slow_period": 30
    },
    "status": "draft",
    "created_at": "2026-05-13T00:00:00Z",
    "updated_at": "2026-05-13T00:00:00Z"
  },
  "message": "success"
}
```

**错误码:**

| 错误码 | 条件 |
|--------|------|
| 40002 | 参数校验失败（如 `fast_period` 超出范围、模板类型不存在） |
| 40001 | 策略名称为空或超过 100 字符 |

**curl 示例:**

```bash
curl -s -X POST \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"name":"My MA Crossover","template_type":"ma_crossover","parameters":{"fast_period":10,"slow_period":30}}' \
  http://localhost:3000/api/v1/strategies | jq .
```

---

### 4.2 查询策略列表

获取当前用户的所有策略，按更新时间降序排列。

```
GET /api/v1/strategies
```

**请求头:**

```
Authorization: Bearer <access_token>
```

**查询参数:**

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `page` | integer | 否 | 1 | 页码，从 1 开始 |
| `size` | integer | 否 | 20 | 每页数量，范围 1-100 |

**响应示例:**

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "user_id": "a1b2c3d4-e29b-41d4-a716-446655440001",
        "name": "My MA Crossover",
        "template_type": "ma_crossover",
        "parameters": { "fast_period": 10, "slow_period": 30 },
        "status": "draft",
        "created_at": "2026-05-13T00:00:00Z",
        "updated_at": "2026-05-13T00:00:00Z"
      }
    ],
    "total": 1,
    "page": 1,
    "size": 20
  },
  "message": "success"
}
```

**错误码:**

| 错误码 | 条件 |
|--------|------|
| 40101 | Token 缺失或无效 |

**curl 示例:**

```bash
curl -s -H "Authorization: Bearer <access_token>" \
  "http://localhost:3000/api/v1/strategies?page=1&size=20" | jq .
```

---

### 4.3 获取策略详情

```
GET /api/v1/strategies/{id}
```

**请求头:**

```
Authorization: Bearer <access_token>
```

**路径参数:**

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 策略 ID |

**响应示例:**

```json
{
  "code": 0,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": "a1b2c3d4-e29b-41d4-a716-446655440001",
    "name": "My MA Crossover",
    "template_type": "ma_crossover",
    "parameters": { "fast_period": 10, "slow_period": 30 },
    "status": "active",
    "created_at": "2026-05-13T00:00:00Z",
    "updated_at": "2026-05-13T00:00:00Z"
  },
  "message": "success"
}
```

**错误码:**

| 错误码 | 条件 |
|--------|------|
| 40401 | 策略不存在（或不属于当前用户） |
| 40101 | Token 缺失或无效 |

**curl 示例:**

```bash
curl -s -H "Authorization: Bearer <access_token>" \
  http://localhost:3000/api/v1/strategies/550e8400-e29b-41d4-a716-446655440000 | jq .
```

---

### 4.4 更新策略

更新策略的名称和/或参数。仅允许在 `draft` 和 `paused` 状态下修改参数。

```
PUT /api/v1/strategies/{id}
```

**请求头:**

```
Authorization: Bearer <access_token>
Content-Type: application/json
```

**路径参数:**

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 策略 ID |

**请求体:**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | 否 | 策略名称，1-100 字符 |
| `parameters` | object | 否 | 策略参数，由模板 schema 定义 |

> 至少提供 `name` 或 `parameters` 之一。更新参数时会重新校验参数合法性。

**请求示例:**

```json
{
  "name": "Updated Strategy Name",
  "parameters": {
    "fast_period": 12,
    "slow_period": 26
  }
}
```

**响应示例:**

```json
{
  "code": 0,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": "a1b2c3d4-e29b-41d4-a716-446655440001",
    "name": "Updated Strategy Name",
    "template_type": "ma_crossover",
    "parameters": { "fast_period": 12, "slow_period": 26 },
    "status": "draft",
    "created_at": "2026-05-13T00:00:00Z",
    "updated_at": "2026-05-13T00:00:00Z"
  },
  "message": "success"
}
```

**错误码:**

| 错误码 | 条件 |
|--------|------|
| 40401 | 策略不存在（或不属于当前用户） |
| 40001 | 策略名称为空或超过 100 字符 |
| 40002 | 参数校验失败 |

**curl 示例:**

```bash
curl -s -X PUT \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"name":"Updated Strategy","parameters":{"fast_period":12,"slow_period":26}}' \
  http://localhost:3000/api/v1/strategies/550e8400-e29b-41d4-a716-446655440000 | jq .
```

---

### 4.5 删除策略

```
DELETE /api/v1/strategies/{id}
```

**请求头:**

```
Authorization: Bearer <access_token>
```

**路径参数:**

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 策略 ID |

**响应示例:**

```json
{
  "code": 0,
  "data": { "message": "Strategy deleted successfully" },
  "message": "success"
}
```

**错误码:**

| 错误码 | 条件 |
|--------|------|
| 40401 | 策略不存在（或不属于当前用户） |
| 40101 | Token 缺失或无效 |

**curl 示例:**

```bash
curl -s -X DELETE \
  -H "Authorization: Bearer <access_token>" \
  http://localhost:3000/api/v1/strategies/550e8400-e29b-41d4-a716-446655440000 | jq .
```

---

## 5. 策略状态 API

### 5.1 变更策略状态

```
POST /api/v1/strategies/{id}/status
```

**请求头:**

```
Authorization: Bearer <access_token>
Content-Type: application/json
```

**路径参数:**

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 策略 ID |

**请求体:**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `status` | string | 是 | 目标状态，取值: `active` / `paused` / `stopped` |

**状态流转规则:**

```
  ┌───────┐
  │ draft │ ──→ active (启动策略)
  └───────┘
                ┌────────┐
    active ──→ │ paused │ ──→ active (恢复运行)
               └────┬───┘
                     │
                     ↓
                  ┌─────────┐
                  │ stopped │ (终止，不可恢复)
                  └─────────┘
```

| 当前状态 | 允许的目标状态 |
|----------|---------------|
| draft    | active        |
| active   | paused        |
| paused   | active, stopped |
| stopped  | (不可变更)     |

**请求示例:**

```json
{
  "status": "active"
}
```

**响应示例:**

```json
{
  "code": 0,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": "a1b2c3d4-e29b-41d4-a716-446655440001",
    "name": "My MA Crossover",
    "template_type": "ma_crossover",
    "parameters": { "fast_period": 10, "slow_period": 30 },
    "status": "active",
    "created_at": "2026-05-13T00:00:00Z",
    "updated_at": "2026-05-13T00:00:00Z"
  },
  "message": "success"
}
```

**错误码:**

| 错误码 | 条件 |
|--------|------|
| 40002 | 无效的状态转换（如从 draft 直接到 stopped） |
| 40401 | 策略不存在（或不属于当前用户） |

**curl 示例:**

```bash
curl -s -X POST \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{"status":"active"}' \
  http://localhost:3000/api/v1/strategies/550e8400-e29b-41d4-a716-446655440000/status | jq .
```

---

## 6. 策略模板参考

| ID | 名称 | 类别 | 参数 | 默认值 |
|----|------|------|------|--------|
| `ma_crossover` | MA Crossover | trend | `fast_period` (int, 5-50), `slow_period` (int, 10-200) | `{fast_period: 10, slow_period: 30}` |
| `triple_ma` | Triple MA | trend | `short` (int, 5-20), `medium` (int, 10-50), `long` (int, 20-200) | `{short: 9, medium: 21, long: 50}` |
| `macd` | MACD | trend | `fast` (int, 2-50), `slow` (int, 5-100), `signal` (int, 2-30) | `{fast: 12, slow: 26, signal: 9}` |
| `ichimoku` | Ichimoku Cloud | trend | `conversion` (int, 5-20), `base` (int, 10-60), `span` (int, 20-120), `displ` (int, 5-60) | `{conversion: 9, base: 26, span: 52, displ: 26}` |
| `rsi` | RSI | mean_reversion | `period` (int, 5-50), `overbought` (float, 65-90), `oversold` (float, 10-35) | `{period: 14, overbought: 75, oversold: 25}` |
| `mean_reversion` | Mean Reversion | mean_reversion | `period` (int, 5-100), `entry_std` (float, 1-3), `exit_std` (float, 0-1) | `{period: 20, entry_std: 2.0, exit_std: 0.5}` |
| `bollinger` | Bollinger Bands | volatility | `period` (int, 5-100), `std_dev` (float, 1-4) | `{period: 20, std_dev: 2.0}` |
| `keltner` | Keltner Channels | volatility | `period` (int, 5-100), `atr_multiplier` (float, 1-3) | `{period: 20, atr_multiplier: 1.5}` |
| `atr_stop` | ATR Stop Loss | volatility | `period` (int, 5-50), `multiplier` (float, 1-5) | `{period: 14, multiplier: 3.0}` |
| `double_bollinger` | Double Bollinger | composite | `period` (int, 5-100), `inner_std` (float, 1-2.5), `outer_std` (float, 2-4) | `{period: 20, inner_std: 1.5, outer_std: 2.5}` |

### 校验规则

| 模板 | 特殊约束 |
|------|---------|
| ma_crossover | `fast_period < slow_period` |
| triple_ma | `short < medium < long` |
| macd | `fast < slow` |
| rsi | `oversold < overbought` |
| mean_reversion | `exit_std < entry_std` |
| double_bollinger | `inner_std < outer_std` |

---

## 附录: API 端点汇总

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| GET | `/api/v1/strategies/templates` | 获取模板列表 | 是 |
| POST | `/api/v1/strategies` | 创建策略 | 是 |
| GET | `/api/v1/strategies` | 查询策略列表 | 是 |
| GET | `/api/v1/strategies/{id}` | 获取策略详情 | 是 |
| PUT | `/api/v1/strategies/{id}` | 更新策略 | 是 |
| DELETE | `/api/v1/strategies/{id}` | 删除策略 | 是 |
| POST | `/api/v1/strategies/{id}/status` | 变更策略状态 | 是 |
