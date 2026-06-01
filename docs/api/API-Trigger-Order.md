# 条件单 API 文档

> 版本: v1.0.0 | 基础路径: `/api/v1` | 认证: Bearer Token (JWT)

---

## 目录

- [API 总览](#api-总览)
- [1. POST /trigger-orders/stop-loss — 创建止损单](#1-post-trigger-ordersstop-loss--创建止损单)
- [2. POST /trigger-orders/take-profit — 创建止盈单](#2-post-trigger-orderstake-profit--创建止盈单)
- [3. POST /trigger-orders/oco — 创建 OCO 单](#3-post-trigger-ordersoco--创建-oco-单)
- [4. POST /trigger-orders/twap — 创建 TWAP 单](#4-post-trigger-orderstwap--创建-twap-单)
- [5. GET /trigger-orders — 条件单列表](#5-get-trigger-orders--条件单列表)
- [6. GET /trigger-orders/{id} — 获取条件单](#6-get-trigger-ordersid--获取条件单)
- [7. DELETE /trigger-orders/{id} — 取消条件单](#7-delete-trigger-ordersid--取消条件单)
- [数据模型](#数据模型)
- [错误码](#错误码)

---

## API 总览

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| POST | `/api/v1/trigger-orders/stop-loss` | 创建止损单 | 必需 |
| POST | `/api/v1/trigger-orders/take-profit` | 创建止盈单 | 必需 |
| POST | `/api/v1/trigger-orders/oco` | 创建 OCO 单（止损+止盈组合） | 必需 |
| POST | `/api/v1/trigger-orders/twap` | 创建 TWAP 单（时间加权平均） | 必需 |
| GET | `/api/v1/trigger-orders` | 条件单列表 | 必需 |
| GET | `/api/v1/trigger-orders/{id}` | 获取单个条件单 | 必需 |
| DELETE | `/api/v1/trigger-orders/{id}` | 取消条件单 | 必需 |

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
  title: 条件单 API
  version: 1.0.0
paths:
  /api/v1/trigger-orders/stop-loss:
    post:
      tags:
        - Trigger Orders
      summary: 创建止损单
      security:
        - BearerAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateStopLossRequest'
      responses:
        '201':
          description: 创建成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TriggerOrderResponse'
        '400':
          description: 请求参数错误
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/trigger-orders/take-profit:
    post:
      tags:
        - Trigger Orders
      summary: 创建止盈单
      security:
        - BearerAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateTakeProfitRequest'
      responses:
        '201':
          description: 创建成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TriggerOrderResponse'
        '400':
          description: 请求参数错误
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/trigger-orders/oco:
    post:
      tags:
        - Trigger Orders
      summary: 创建 OCO 单（One-Cancels-Other）
      security:
        - BearerAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateOcoRequest'
      responses:
        '201':
          description: 创建成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/OcoPairResponse'
        '400':
          description: 请求参数错误
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/trigger-orders/twap:
    post:
      tags:
        - Trigger Orders
      summary: 创建 TWAP 单（Time-Weighted Average Price）
      security:
        - BearerAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateTwapRequest'
      responses:
        '201':
          description: 创建成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TriggerOrderResponse'
        '400':
          description: 请求参数错误
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/trigger-orders:
    get:
      tags:
        - Trigger Orders
      summary: 条件单列表
      security:
        - BearerAuth: []
      parameters:
        - name: status
          in: query
          required: false
          schema:
            type: string
          description: 按状态过滤
        - name: symbol
          in: query
          required: false
          schema:
            type: string
          description: 按交易对过滤
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/TriggerOrderResponse'
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/trigger-orders/{id}:
    get:
      tags:
        - Trigger Orders
      summary: 获取单个条件单
      security:
        - BearerAuth: []
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
            format: uuid
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/TriggerOrderResponse'
        '404':
          description: 条件单不存在
        '401':
          $ref: '#/components/responses/Unauthorized'
    delete:
      tags:
        - Trigger Orders
      summary: 取消条件单
      security:
        - BearerAuth: []
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
            format: uuid
      requestBody:
        required: false
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CancelTriggerOrderRequest'
      responses:
        '204':
          description: 取消成功
        '404':
          description: 条件单不存在
        '401':
          $ref: '#/components/responses/Unauthorized'
components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
  schemas:
    CreateStopLossRequest:
      type: object
      required:
        - positionId
        - symbol
        - triggerPrice
        - quantity
      properties:
        positionId:
          type: string
          format: uuid
          description: 持仓 ID
        symbol:
          type: string
          description: 交易对符号
        triggerPrice:
          type: number
          description: 触发价格
        basePrice:
          type: number
          nullable: true
          description: 基准价格
        quantity:
          type: number
          description: 数量
    CreateTakeProfitRequest:
      type: object
      required:
        - positionId
        - symbol
        - triggerPrice
        - quantity
      properties:
        positionId:
          type: string
          format: uuid
          description: 持仓 ID
        symbol:
          type: string
          description: 交易对符号
        triggerPrice:
          type: number
          description: 触发价格
        basePrice:
          type: number
          nullable: true
          description: 基准价格
        quantity:
          type: number
          description: 数量
    CreateOcoRequest:
      type: object
      required:
        - positionId
        - symbol
        - stopLossPrice
        - takeProfitPrice
        - quantity
      properties:
        positionId:
          type: string
          format: uuid
          description: 持仓 ID
        symbol:
          type: string
          description: 交易对符号
        stopLossPrice:
          type: number
          description: 止损触发价格
        takeProfitPrice:
          type: number
          description: 止盈触发价格
        basePrice:
          type: number
          nullable: true
          description: 基准价格
        quantity:
          type: number
          description: 数量
    CreateTwapRequest:
      type: object
      required:
        - symbol
        - side
        - quantity
        - sliceQuantity
        - intervalSecs
        - durationSecs
      properties:
        symbol:
          type: string
          description: 交易对符号
        side:
          type: string
          enum: [buy, sell]
          description: 交易方向
        quantity:
          type: number
          description: 总数量
        sliceQuantity:
          type: number
          description: 每份数量
        intervalSecs:
          type: integer
          format: int32
          description: 间隔秒数
        durationSecs:
          type: integer
          format: int32
          description: 总持续秒数
    TriggerOrderResponse:
      type: object
      properties:
        id:
          type: string
          format: uuid
        userId:
          type: string
          format: uuid
        positionId:
          type: string
          format: uuid
          nullable: true
        symbol:
          type: string
        triggerType:
          type: string
          enum: [stop_loss, take_profit, oco, twap]
        status:
          type: string
        triggerDirection:
          type: string
        triggerPrice:
          type: number
        triggerPriceUpper:
          type: number
          nullable: true
        triggerPriceLower:
          type: number
          nullable: true
        basePrice:
          type: number
          nullable: true
        side:
          type: string
        quantity:
          type: number
        filledQuantity:
          type: number
        avgFillPrice:
          type: number
          nullable: true
        ocoPairId:
          type: string
          format: uuid
          nullable: true
        triggeredOrderId:
          type: string
          format: uuid
          nullable: true
        triggerReason:
          type: string
          nullable: true
        triggeredAt:
          type: string
          format: date-time
          nullable: true
        createdAt:
          type: string
          format: date-time
        updatedAt:
          type: string
          format: date-time
        twapSliceQuantity:
          type: number
        twapIntervalSecs:
          type: integer
          format: int32
        twapExecutedSlices:
          type: integer
          format: int32
        twapMaxSlices:
          type: integer
          format: int32
    OcoPairResponse:
      type: object
      properties:
        stopLoss:
          $ref: '#/components/schemas/TriggerOrderResponse'
        takeProfit:
          $ref: '#/components/schemas/TriggerOrderResponse'
    CancelTriggerOrderRequest:
      type: object
      properties:
        reason:
          type: string
          description: 取消原因
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

## 1. POST /trigger-orders/stop-loss — 创建止损单

为指定持仓创建止损单。当市场价格触发止损价时，自动执行平仓。

### curl 示例

```bash
curl -X POST "http://localhost:8080/api/v1/trigger-orders/stop-loss" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "positionId": "550e8400-e29b-41d4-a716-446655440000",
    "symbol": "BTCUSDT",
    "triggerPrice": 65000.00,
    "basePrice": 67000.00,
    "quantity": 0.5
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `positionId` | UUID | 是 | 持仓 ID |
| `symbol` | string | 是 | 交易对符号 |
| `triggerPrice` | number | 是 | 触发价格 |
| `basePrice` | number | 否 | 基准价格（用于计算偏离） |
| `quantity` | number | 是 | 数量 |

### 响应示例

```json
{
  "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "userId": "123e4567-e89b-12d3-a456-426614174000",
  "positionId": "550e8400-e29b-41d4-a716-446655440000",
  "symbol": "BTCUSDT",
  "triggerType": "stop_loss",
  "status": "pending",
  "triggerDirection": "below",
  "triggerPrice": 65000.0,
  "triggerPriceUpper": null,
  "triggerPriceLower": null,
  "basePrice": 67000.0,
  "side": "sell",
  "quantity": 0.5,
  "filledQuantity": 0.0,
  "avgFillPrice": null,
  "ocoPairId": null,
  "triggeredOrderId": null,
  "triggerReason": null,
  "triggeredAt": null,
  "createdAt": "2026-06-01T10:00:00Z",
  "updatedAt": "2026-06-01T10:00:00Z",
  "twapSliceQuantity": 0.0,
  "twapIntervalSecs": 0,
  "twapExecutedSlices": 0,
  "twapMaxSlices": 0
}
```

---

## 2. POST /trigger-orders/take-profit — 创建止盈单

为指定持仓创建止盈单。当市场价格触发止盈价时，自动执行平仓。

### curl 示例

```bash
curl -X POST "http://localhost:8080/api/v1/trigger-orders/take-profit" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "positionId": "550e8400-e29b-41d4-a716-446655440000",
    "symbol": "BTCUSDT",
    "triggerPrice": 70000.00,
    "basePrice": 67000.00,
    "quantity": 0.5
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `positionId` | UUID | 是 | 持仓 ID |
| `symbol` | string | 是 | 交易对符号 |
| `triggerPrice` | number | 是 | 触发价格 |
| `basePrice` | number | 否 | 基准价格 |
| `quantity` | number | 是 | 数量 |

### 响应示例

响应格式同止损单，`triggerType` 为 `take_profit`。

---

## 3. POST /trigger-orders/oco — 创建 OCO 单

创建 OCO（One-Cancels-Other）单，同时设置止损单和止盈单。当其中一个触发时，另一个自动取消。

### curl 示例

```bash
curl -X POST "http://localhost:8080/api/v1/trigger-orders/oco" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "positionId": "550e8400-e29b-41d4-a716-446655440000",
    "symbol": "BTCUSDT",
    "stopLossPrice": 65000.00,
    "takeProfitPrice": 70000.00,
    "basePrice": 67000.00,
    "quantity": 0.5
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `positionId` | UUID | 是 | 持仓 ID |
| `symbol` | string | 是 | 交易对符号 |
| `stopLossPrice` | number | 是 | 止损触发价格 |
| `takeProfitPrice` | number | 是 | 止盈触发价格 |
| `basePrice` | number | 否 | 基准价格 |
| `quantity` | number | 是 | 数量 |

### 响应示例

```json
{
  "stopLoss": {
    "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
    "userId": "123e4567-e89b-12d3-a456-426614174000",
    "positionId": "550e8400-e29b-41d4-a716-446655440000",
    "symbol": "BTCUSDT",
    "triggerType": "stop_loss",
    "status": "pending",
    "triggerDirection": "below",
    "triggerPrice": 65000.0,
    "triggerPriceUpper": null,
    "triggerPriceLower": null,
    "basePrice": 67000.0,
    "side": "sell",
    "quantity": 0.5,
    "filledQuantity": 0.0,
    "avgFillPrice": null,
    "ocoPairId": "a87ff679-a2f3-451d-a000-0e02b2c3d479",
    "triggeredOrderId": null,
    "triggerReason": null,
    "triggeredAt": null,
    "createdAt": "2026-06-01T10:00:00Z",
    "updatedAt": "2026-06-01T10:00:00Z",
    "twapSliceQuantity": 0.0,
    "twapIntervalSecs": 0,
    "twapExecutedSlices": 0,
    "twapMaxSlices": 0
  },
  "takeProfit": {
    "id": "b47ac10b-58cc-4372-a567-0e02b2c3d479",
    "userId": "123e4567-e89b-12d3-a456-426614174000",
    "positionId": "550e8400-e29b-41d4-a716-446655440000",
    "symbol": "BTCUSDT",
    "triggerType": "take_profit",
    "status": "pending",
    "triggerDirection": "above",
    "triggerPrice": 70000.0,
    "triggerPriceUpper": null,
    "triggerPriceLower": null,
    "basePrice": 67000.0,
    "side": "sell",
    "quantity": 0.5,
    "filledQuantity": 0.0,
    "avgFillPrice": null,
    "ocoPairId": "a87ff679-a2f3-451d-a000-0e02b2c3d479",
    "triggeredOrderId": null,
    "triggerReason": null,
    "triggeredAt": null,
    "createdAt": "2026-06-01T10:00:00Z",
    "updatedAt": "2026-06-01T10:00:00Z",
    "twapSliceQuantity": 0.0,
    "twapIntervalSecs": 0,
    "twapExecutedSlices": 0,
    "twapMaxSlices": 0
  }
}
```

---

## 4. POST /trigger-orders/twap — 创建 TWAP 单

创建 TWAP（Time-Weighted Average Price）单，在指定时间周期内分批执行订单。

### curl 示例

```bash
curl -X POST "http://localhost:8080/api/v1/trigger-orders/twap" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTCUSDT",
    "side": "buy",
    "quantity": 10.0,
    "sliceQuantity": 1.0,
    "intervalSecs": 60,
    "durationSecs": 600
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `symbol` | string | 是 | 交易对符号 |
| `side` | string | 是 | 交易方向：`buy` 或 `sell` |
| `quantity` | number | 是 | 总数量 |
| `sliceQuantity` | number | 是 | 每份数量 |
| `intervalSecs` | integer | 是 | 每份间隔秒数 |
| `durationSecs` | integer | 是 | 总持续秒数 |

### 响应示例

```json
{
  "id": "c47ac10b-58cc-4372-a567-0e02b2c3d479",
  "userId": "123e4567-e89b-12d3-a456-426614174000",
  "positionId": null,
  "symbol": "BTCUSDT",
  "triggerType": "twap",
  "status": "pending",
  "triggerDirection": "above",
  "triggerPrice": 0.0,
  "triggerPriceUpper": null,
  "triggerPriceLower": null,
  "basePrice": null,
  "side": "buy",
  "quantity": 10.0,
  "filledQuantity": 0.0,
  "avgFillPrice": null,
  "ocoPairId": null,
  "triggeredOrderId": null,
  "triggerReason": null,
  "triggeredAt": null,
  "createdAt": "2026-06-01T10:00:00Z",
  "updatedAt": "2026-06-01T10:00:00Z",
  "twapSliceQuantity": 1.0,
  "twapIntervalSecs": 60,
  "twapExecutedSlices": 0,
  "twapMaxSlices": 10
}
```

---

## 5. GET /trigger-orders — 条件单列表

获取当前用户的条件单列表。

### Query 参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `status` | string | 否 | 按状态过滤（`pending` / `triggered` / `cancelled` / `expired`） |
| `symbol` | string | 否 | 按交易对过滤 |

### curl 示例

```bash
# 获取所有条件单
curl -X GET "http://localhost:8080/api/v1/trigger-orders" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"

# 获取待触发状态的止损单
curl -X GET "http://localhost:8080/api/v1/trigger-orders?status=pending&symbol=BTCUSDT" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
[
  {
    "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
    "userId": "123e4567-e89b-12d3-a456-426614174000",
    "positionId": "550e8400-e29b-41d4-a716-446655440000",
    "symbol": "BTCUSDT",
    "triggerType": "stop_loss",
    "status": "pending",
    "triggerDirection": "below",
    "triggerPrice": 65000.0,
    "triggerPriceUpper": null,
    "triggerPriceLower": null,
    "basePrice": 67000.0,
    "side": "sell",
    "quantity": 0.5,
    "filledQuantity": 0.0,
    "avgFillPrice": null,
    "ocoPairId": null,
    "triggeredOrderId": null,
    "triggerReason": null,
    "triggeredAt": null,
    "createdAt": "2026-06-01T10:00:00Z",
    "updatedAt": "2026-06-01T10:00:00Z",
    "twapSliceQuantity": 0.0,
    "twapIntervalSecs": 0,
    "twapExecutedSlices": 0,
    "twapMaxSlices": 0
  }
]
```

---

## 6. GET /trigger-orders/{id} — 获取条件单

获取指定条件单的详细信息。

### 路径参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | UUID | 是 | 条件单 ID |

### curl 示例

```bash
curl -X GET "http://localhost:8080/api/v1/trigger-orders/f47ac10b-58cc-4372-a567-0e02b2c3d479" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
{
  "id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "userId": "123e4567-e89b-12d3-a456-426614174000",
  "positionId": "550e8400-e29b-41d4-a716-446655440000",
  "symbol": "BTCUSDT",
  "triggerType": "stop_loss",
  "status": "pending",
  "triggerDirection": "below",
  "triggerPrice": 65000.0,
  "triggerPriceUpper": null,
  "triggerPriceLower": null,
  "basePrice": 67000.0,
  "side": "sell",
  "quantity": 0.5,
  "filledQuantity": 0.0,
  "avgFillPrice": null,
  "ocoPairId": null,
  "triggeredOrderId": null,
  "triggerReason": null,
  "triggeredAt": null,
  "createdAt": "2026-06-01T10:00:00Z",
  "updatedAt": "2026-06-01T10:00:00Z",
  "twapSliceQuantity": 0.0,
  "twapIntervalSecs": 0,
  "twapExecutedSlices": 0,
  "twapMaxSlices": 0
}
```

---

## 7. DELETE /trigger-orders/{id} — 取消条件单

取消指定的待触发条件单。

### 路径参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | UUID | 是 | 条件单 ID |

### curl 示例

```bash
curl -X DELETE "http://localhost:8080/api/v1/trigger-orders/f47ac10b-58cc-4372-a567-0e02b2c3d479" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "手动取消"
  }'
```

### 请求体（可选）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `reason` | string | 否 | 取消原因 |

### 响应

返回 `204 No Content` 表示取消成功。

---

## 数据模型

### TriggerOrderResponse

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 条件单 ID |
| `userId` | UUID | 用户 ID |
| `positionId` | UUID | 关联持仓 ID（TWAP 可能为 null） |
| `symbol` | String | 交易对符号 |
| `triggerType` | String | 类型：`stop_loss` / `take_profit` / `oco` / `twap` |
| `status` | String | 状态：`pending` / `triggered` / `cancelled` / `expired` |
| `triggerDirection` | String | 触发方向：`above` / `below` |
| `triggerPrice` | f64 | 触发价格 |
| `triggerPriceUpper` | Option<f64> | 价格上限（OCO 止盈） |
| `triggerPriceLower` | Option<f64> | 价格下限（OCO 止损） |
| `basePrice` | Option<f64> | 基准价格 |
| `side` | String | 交易方向：`buy` / `sell` |
| `quantity` | f64 | 数量 |
| `filledQuantity` | f64 | 已成交数量 |
| `avgFillPrice` | Option<f64> | 平均成交价 |
| `ocoPairId` | Option<UUID> | OCO 对 ID |
| `triggeredOrderId` | Option<UUID> | 触发后生成的订单 ID |
| `triggerReason` | Option<String> | 触发原因 |
| `triggeredAt` | Option<String> | 触发时间（RFC3339） |
| `createdAt` | String | 创建时间（RFC3339） |
| `updatedAt` | String | 更新时间（RFC3339） |
| `twapSliceQuantity` | f64 | TWAP 每份数量 |
| `twapIntervalSecs` | i32 | TWAP 间隔秒数 |
| `twapExecutedSlices` | i32 | TWAP 已执行份数 |
| `twapMaxSlices` | i32 | TWAP 总份数 |

### OcoPairResponse

| 字段 | 类型 | 说明 |
|------|------|------|
| `stopLoss` | TriggerOrderResponse | 止损单 |
| `takeProfit` | TriggerOrderResponse | 止盈单 |

---

## 错误码

| HTTP 状态码 | code | message | 说明 |
|-------------|------|---------|------|
| 400 | 400 | Bad Request | 请求参数错误 |
| 401 | 401 | Unauthorized | Token 无效或已过期 |
| 404 | 404 | Not Found | 条件单不存在 |
| 500 | 500 | Internal server error | 服务器内部错误 |

---

## 备注

- 所有响应遵循统一格式：`{ "code": 0, "data": T, "message": "success" }`
- `code !== 0` 时表示请求失败
- 删除接口返回 `204 No Content`（无响应体）
- OCO 单会创建两个关联的条件单，共用同一个 `ocoPairId`
- TWAP 单会按设定的间隔分批执行，直到完成全部数量或达到总时长
