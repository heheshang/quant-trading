# 套利 API 文档

> 版本: v1.0.0 | 基础路径: `/api/v1` | 认证: Bearer Token (JWT)

---

## 目录

- [API 总览](#api-总览)
- [1. POST /arbitrage/pairs — 创建套利对](#1-post-arbitragepairs--创建套利对)
- [2. GET /arbitrage/pairs — 列出套利对](#2-get-arbitragepairs--列出套利对)
- [3. GET /arbitrage/pairs/{pair_id} — 获取套利对](#3-get-arbitragepairspair_id--获取套利对)
- [4. PUT /arbitrage/pairs/{pair_id} — 更新套利对](#4-put-arbitragepairspair_id--更新套利对)
- [5. DELETE /arbitrage/pairs/{pair_id} — 删除套利对](#5-delete-arbitragepairspair_id--删除套利对)
- [6. GET /arbitrage/spread/{pair_id} — 计算价差](#6-get-arbitragespreadpair_id--计算价差)
- [7. GET /arbitrage/positions — 套利持仓列表](#7-get-arbitragepositions--套利持仓列表)
- [8. GET /arbitrage/signals — 信号列表](#8-get-arbitragesignals--信号列表)
- [数据模型](#数据模型)
- [错误码](#错误码)

---

## API 总览

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| POST | `/api/v1/arbitrage/pairs` | 创建套利对 | 必需 |
| GET | `/api/v1/arbitrage/pairs` | 列出所有活跃套利对 | 必需 |
| GET | `/api/v1/arbitrage/pairs/{pair_id}` | 获取单个套利对 | 必需 |
| PUT | `/api/v1/arbitrage/pairs/{pair_id}` | 更新套利对 | 必需 |
| DELETE | `/api/v1/arbitrage/pairs/{pair_id}` | 删除套利对（软删除） | 必需 |
| GET | `/api/v1/arbitrage/spread/{pair_id}` | 计算当前价差 | 必需 |
| GET | `/api/v1/arbitrage/positions` | 套利持仓列表 | 必需 |
| GET | `/api/v1/arbitrage/signals` | 信号列表 | 必需 |

---

## 认证方式

所有接口需要携带 JWT Bearer Token：

```
Authorization: Bearer <access_token>
```

Token 通过 `POST /api/v1/auth/login` 或 `POST /api/v1/auth/refresh` 获取。

---

## OpenAPI 3.0 定义

```yaml
openapi: 3.0.3
info:
  title: 套利 API
  version: 1.0.0
paths:
  /api/v1/arbitrage/pairs:
    post:
      tags:
        - Arbitrage
      summary: 创建套利对
      security:
        - BearerAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreatePairRequest'
      responses:
        '201':
          description: 创建成功
          content:
            application/json:
              schema:
                type: object
                properties:
                  id:
                    type: integer
                    format: int64
        '400':
          description: 请求参数错误
        '401':
          $ref: '#/components/responses/Unauthorized'
    get:
      tags:
        - Arbitrage
      summary: 列出所有活跃套利对
      security:
        - BearerAuth: []
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/PairListItem'
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/arbitrage/pairs/{pair_id}:
    get:
      tags:
        - Arbitrage
      summary: 获取单个套利对
      security:
        - BearerAuth: []
      parameters:
        - name: pair_id
          in: path
          required: true
          schema:
            type: integer
            format: int64
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PairListItem'
        '404':
          description: 套利对不存在
        '401':
          $ref: '#/components/responses/Unauthorized'
    put:
      tags:
        - Arbitrage
      summary: 更新套利对
      security:
        - BearerAuth: []
      parameters:
        - name: pair_id
          in: path
          required: true
          schema:
            type: integer
            format: int64
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/UpdatePairRequest'
      responses:
        '200':
          description: 成功
        '404':
          description: 套利对不存在
        '401':
          $ref: '#/components/responses/Unauthorized'
    delete:
      tags:
        - Arbitrage
      summary: 删除套利对（软删除）
      security:
        - BearerAuth: []
      parameters:
        - name: pair_id
          in: path
          required: true
          schema:
            type: integer
            format: int64
      responses:
        '200':
          description: 成功
        '404':
          description: 套利对不存在
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/arbitrage/spread/{pair_id}:
    get:
      tags:
        - Arbitrage
      summary: 计算当前价差
      security:
        - BearerAuth: []
      parameters:
        - name: pair_id
          in: path
          required: true
          schema:
            type: integer
            format: int64
        - name: price_a
          in: query
          required: false
          schema:
            type: number
          description: 交易对 A 当前价格
        - name: price_b
          in: query
          required: false
          schema:
            type: number
          description: 交易对 B 当前价格
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/SpreadData'
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/arbitrage/positions:
    get:
      tags:
        - Arbitrage
      summary: 套利持仓列表
      security:
        - BearerAuth: []
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                type: array
                items:
                  type: object
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/arbitrage/signals:
    get:
      tags:
        - Arbitrage
      summary: 信号列表
      security:
        - BearerAuth: []
      parameters:
        - name: pair_id
          in: query
          required: false
          schema:
            type: integer
            format: int64
          description: 按套利对 ID 过滤
        - name: limit
          in: query
          required: false
          schema:
            type: integer
            format: int64
            default: 50
          description: 返回条数限制
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                type: array
                items:
                  type: object
        '401':
          $ref: '#/components/responses/Unauthorized'
components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
  schemas:
    CreatePairRequest:
      type: object
      required:
        - pairType
        - symbolA
        - symbolB
        - spreadEntryThreshold
        - spreadExitThreshold
      properties:
        pairType:
          type: string
          description: 套利对类型
        symbolA:
          type: string
          description: 交易对 A 符号
        symbolB:
          type: string
          description: 交易对 B 符号
        exchange:
          type: string
          default: binance
          description: 交易所
        spreadEntryThreshold:
          type: number
          description: 入场价差阈值
        spreadExitThreshold:
          type: number
          description: 出场价差阈值
        maxPositionSize:
          type: number
          description: 最大持仓量
        calculationMode:
          type: string
          default: percentage
          description: 计算模式
        correlationThreshold:
          type: number
          description: 相关性阈值
        zScoreEntry:
          type: number
          description: Z-score 入场阈值
        zScoreExit:
          type: number
          description: Z-score 出场阈值
    UpdatePairRequest:
      type: object
      properties:
        pairType:
          type: string
        symbolA:
          type: string
        symbolB:
          type: string
        exchange:
          type: string
        status:
          type: string
        spreadEntryThreshold:
          type: number
        spreadExitThreshold:
          type: number
        maxPositionSize:
          type: number
        calculationMode:
          type: string
        correlationThreshold:
          type: number
        zScoreEntry:
          type: number
        zScoreExit:
          type: number
    PairListItem:
      type: object
      properties:
        id:
          type: integer
          format: int64
        pairType:
          type: string
        symbolA:
          type: string
        symbolB:
          type: string
        exchange:
          type: string
        status:
          type: string
        spreadEntryThreshold:
          type: number
        spreadExitThreshold:
          type: number
        maxPositionSize:
          type: number
        calculationMode:
          type: string
        correlationThreshold:
          type: number
          nullable: true
        zScoreEntry:
          type: number
          nullable: true
        zScoreExit:
          type: number
          nullable: true
        createdAt:
          type: string
          format: date-time
        updatedAt:
          type: string
          format: date-time
    SpreadData:
      type: object
      properties:
        pairId:
          type: integer
          format: int64
        spread:
          type: number
        spreadPct:
          type: number
        zScore:
          type: number
          nullable: true
        historicalMean:
          type: number
          nullable: true
        historicalStd:
          type: number
          nullable: true
        signal:
          type: string
          enum: [long, short, neutral]
        timestamp:
          type: string
          format: date-time
  responses:
    Unauthorized:
      description: 未授权
      content:
        application/json:
          schema:
            type: object
            properties:
              code:
                type: integer
                example: 401
              message:
                type: string
                example: Unauthorized
```

---

## 1. POST /arbitrage/pairs — 创建套利对

创建一个新的套利对。

### curl 示例

```bash
curl -X POST "http://localhost:8080/api/v1/arbitrage/pairs" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "pairType": "futures_spot",
    "symbolA": "BTC_USDT",
    "symbolB": "ETH_USDT",
    "exchange": "binance",
    "spreadEntryThreshold": 0.05,
    "spreadExitThreshold": 0.02,
    "maxPositionSize": 10000,
    "calculationMode": "percentage",
    "zScoreEntry": 2.0,
    "zScoreExit": 0.5
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `pairType` | string | 是 | — | 套利对类型 |
| `symbolA` | string | 是 | — | 交易对 A 符号 |
| `symbolB` | string | 是 | — | 交易对 B 符号 |
| `exchange` | string | 否 | `binance` | 交易所 |
| `spreadEntryThreshold` | number | 是 | — | 入场价差阈值 |
| `spreadExitThreshold` | number | 是 | — | 出场价差阈值 |
| `maxPositionSize` | number | 否 | `10000` | 最大持仓量 |
| `calculationMode` | string | 否 | `percentage` | 计算模式 |
| `correlationThreshold` | number | 否 | — | 相关性阈值 |
| `zScoreEntry` | number | 否 | — | Z-score 入场阈值 |
| `zScoreExit` | number | 否 | — | Z-score 出场阈值 |

### 响应示例

```json
{
  "id": 1
}
```

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | integer | 新创建的套利对 ID |

---

## 2. GET /arbitrage/pairs — 列出套利对

列出所有状态为 `active` 的套利对。

### curl 示例

```bash
curl -X GET "http://localhost:8080/api/v1/arbitrage/pairs" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
[
  {
    "id": 1,
    "pairType": "futures_spot",
    "symbolA": "BTC_USDT",
    "symbolB": "ETH_USDT",
    "exchange": "binance",
    "status": "active",
    "spreadEntryThreshold": "0.05",
    "spreadExitThreshold": "0.02",
    "maxPositionSize": "10000",
    "calculationMode": "percentage",
    "correlationThreshold": null,
    "zScoreEntry": "2.0",
    "zScoreExit": "0.5",
    "createdAt": "2026-05-01T10:00:00Z",
    "updatedAt": "2026-05-01T10:00:00Z"
  }
]
```

---

## 3. GET /arbitrage/pairs/{pair_id} — 获取套利对

获取指定套利对的详细信息。

### 路径参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `pair_id` | integer | 是 | 套利对 ID |

### curl 示例

```bash
curl -X GET "http://localhost:8080/api/v1/arbitrage/pairs/1" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
{
  "id": 1,
  "pairType": "futures_spot",
  "symbolA": "BTC_USDT",
  "symbolB": "ETH_USDT",
  "exchange": "binance",
  "status": "active",
  "spreadEntryThreshold": "0.05",
  "spreadExitThreshold": "0.02",
  "maxPositionSize": "10000",
  "calculationMode": "percentage",
  "correlationThreshold": null,
  "zScoreEntry": "2.0",
  "zScoreExit": "0.5",
  "createdAt": "2026-05-01T10:00:00Z",
  "updatedAt": "2026-05-01T10:00:00Z"
}
```

---

## 4. PUT /arbitrage/pairs/{pair_id} — 更新套利对

更新指定套利对的配置参数。

### 路径参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `pair_id` | integer | 是 | 套利对 ID |

### curl 示例

```bash
curl -X PUT "http://localhost:8080/api/v1/arbitrage/pairs/1" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "spreadEntryThreshold": 0.06,
    "spreadExitThreshold": 0.025,
    "status": "paused"
  }'
```

### 请求体

所有字段均为可选，仅更新提供的字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `pairType` | string | 套利对类型 |
| `symbolA` | string | 交易对 A 符号 |
| `symbolB` | string | 交易对 B 符号 |
| `exchange` | string | 交易所 |
| `status` | string | 状态：`active` / `paused` / `inactive` |
| `spreadEntryThreshold` | number | 入场价差阈值 |
| `spreadExitThreshold` | number | 出场价差阈值 |
| `maxPositionSize` | number | 最大持仓量 |
| `calculationMode` | string | 计算模式 |
| `correlationThreshold` | number | 相关性阈值 |
| `zScoreEntry` | number | Z-score 入场阈值 |
| `zScoreExit` | number | Z-score 出场阈值 |

### 响应示例

```json
{
  "success": true
}
```

---

## 5. DELETE /arbitrage/pairs/{pair_id} — 删除套利对

软删除套利对，将状态设置为 `inactive`。

### 路径参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `pair_id` | integer | 是 | 套利对 ID |

### curl 示例

```bash
curl -X DELETE "http://localhost:8080/api/v1/arbitrage/pairs/1" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
{
  "success": true
}
```

---

## 6. GET /arbitrage/spread/{pair_id} — 计算价差

计算指定套利对的当前价差及统计指标。

### 路径参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `pair_id` | integer | 是 | 套利对 ID |

### Query 参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `price_a` | number | 否 | 交易对 A 当前价格 |
| `price_b` | number | 否 | 交易对 B 当前价格 |

### curl 示例

```bash
curl -X GET "http://localhost:8080/api/v1/arbitrage/spread/1?price_a=67500&price_b=3450" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
{
  "pairId": 1,
  "spread": 0.015,
  "spreadPct": 1.5,
  "zScore": 1.2,
  "historicalMean": 0.01,
  "historicalStd": 0.005,
  "signal": "neutral",
  "timestamp": "2026-06-01T10:30:00Z"
}
```

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `pairId` | integer | 套利对 ID |
| `spread` | number | 当前价差 |
| `spreadPct` | number | 价差百分比 |
| `zScore` | number | Z-score 值 |
| `historicalMean` | number | 历史均值 |
| `historicalStd` | number | 历史标准差 |
| `signal` | string | 信号方向：`long` / `short` / `neutral` |
| `timestamp` | string | 计算时间 |

---

## 7. GET /arbitrage/positions — 套利持仓列表

列出所有状态为 `open` 的套利持仓。

### curl 示例

```bash
curl -X GET "http://localhost:8080/api/v1/arbitrage/positions" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
[
  {
    "id": 1,
    "pairId": 1,
    "direction": "long",
    "sizeA": "0.5",
    "sizeB": "10.0",
    "entrySpread": "0.048",
    "currentSpread": "0.052",
    "unrealizedPnl": "120.50",
    "status": "open",
    "openedAt": "2026-05-15T08:00:00Z",
    "closedAt": null
  }
]
```

---

## 8. GET /arbitrage/signals — 信号列表

获取最近的套利信号记录。

### Query 参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `pair_id` | integer | 否 | — | 按套利对 ID 过滤 |
| `limit` | integer | 否 | `50` | 返回条数限制 |

### curl 示例

```bash
# 获取最近 20 条信号
curl -X GET "http://localhost:8080/api/v1/arbitrage/signals?limit=20" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"

# 获取指定套利对的信号
curl -X GET "http://localhost:8080/api/v1/arbitrage/signals?pair_id=1&limit=10" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
[
  {
    "id": 1,
    "pairId": 1,
    "signalType": "entry",
    "spread": "0.055",
    "zScore": 2.1,
    "confidence": 0.85,
    "executed": true,
    "createdAt": "2026-05-20T14:30:00Z"
  },
  {
    "id": 2,
    "pairId": 1,
    "signalType": "exit",
    "spread": "0.025",
    "zScore": 0.4,
    "confidence": 0.72,
    "executed": false,
    "createdAt": "2026-05-21T09:15:00Z"
  }
]
```

---

## 数据模型

### PairListItem

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | i32 | 套利对 ID |
| `pairType` | String | 套利对类型 |
| `symbolA` | String | 交易对 A 符号 |
| `symbolB` | String | 交易对 B 符号 |
| `exchange` | String | 交易所 |
| `status` | String | 状态：`active` / `paused` / `inactive` |
| `spreadEntryThreshold` | Decimal | 入场价差阈值 |
| `spreadExitThreshold` | Decimal | 出场价差阈值 |
| `maxPositionSize` | Decimal | 最大持仓量 |
| `calculationMode` | String | 计算模式 |
| `correlationThreshold` | Option<Decimal> | 相关性阈值 |
| `zScoreEntry` | Option<Decimal> | Z-score 入场阈值 |
| `zScoreExit` | Option<Decimal> | Z-score 出场阈值 |
| `createdAt` | DateTime | 创建时间 |
| `updatedAt` | DateTime | 更新时间 |

### SpreadData

| 字段 | 类型 | 说明 |
|------|------|------|
| `pairId` | i32 | 套利对 ID |
| `spread` | Decimal | 当前价差 |
| `spreadPct` | Decimal | 价差百分比 |
| `zScore` | Option<Decimal> | Z-score 值 |
| `historicalMean` | Option<Decimal> | 历史均值 |
| `historicalStd` | Option<Decimal> | 历史标准差 |
| `signal` | SignalDirection | 信号方向 |
| `timestamp` | DateTime | 计算时间 |

---

## 错误码

| HTTP 状态码 | code | message | 说明 |
|-------------|------|---------|------|
| 400 | 400 | Bad Request | 请求参数错误 |
| 401 | 401 | Unauthorized | Token 无效或已过期 |
| 404 | 404 | Not Found | 套利对不存在 |
| 500 | 500 | Internal server error | 服务器内部错误 |

---

## 备注

- 所有响应遵循统一格式：`{ "code": 0, "data": T, "message": "success" }`
- `code !== 0` 时表示请求失败
- 删除套利对为软删除，仅将状态设置为 `inactive`
- 价差计算需要提供 `price_a` 和 `price_b` 参数
- 信号列表默认按创建时间倒序排列
