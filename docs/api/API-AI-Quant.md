# AI 量化 API 文档

> 版本: v1.0.0 | 基础路径: `/api/v1` | 认证: Bearer Token (JWT)

---

## 目录

- [API 总览](#api-总览)
- [1. GET /ai/models — 列出 AI 模型](#1-get-aimodels--列出-ai-模型)
- [2. GET /ai/predictions/{symbol} — 获取 AI 预测](#2-get-aipredictionssymbol--获取-ai-预测)
- [3. POST /ai/backtest — AI 信号回测](#3-post-aibacktest--ai-信号回测)
- [数据模型](#数据模型)
- [错误码](#错误码)

---

## API 总览

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| GET | `/api/v1/ai/models` | 列出可用的 AI 模型 | 必需 |
| GET | `/api/v1/ai/predictions/{symbol}` | 获取指定交易对的 AI 预测 | 必需 |
| POST | `/api/v1/ai/backtest` | 使用 AI 信号进行回测 | 必需 |

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
  title: AI 量化 API
  version: 1.0.0
paths:
  /api/v1/ai/models:
    get:
      tags:
        - AI Quant
      summary: 列出可用的 AI 模型
      security:
        - BearerAuth: []
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ListModelsResponse'
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/ai/predictions/{symbol}:
    get:
      tags:
        - AI Quant
      summary: 获取指定交易对的 AI 预测
      security:
        - BearerAuth: []
      parameters:
        - name: symbol
          in: path
          required: true
          schema:
            type: string
          description: 交易对符号，如 BTCUSDT
        - name: interval
          in: query
          required: false
          schema:
            type: string
            default: 1h
          description: K 线周期
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PredictionResponse'
        '401':
          $ref: '#/components/responses/Unauthorized'
        '404':
          description: 未找到 K 线数据
  /api/v1/ai/backtest:
    post:
      tags:
        - AI Quant
      summary: 使用 AI 信号进行回测
      security:
        - BearerAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/BacktestRequest'
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/BacktestResponse'
        '400':
          description: 请求参数错误
        '401':
          $ref: '#/components/responses/Unauthorized'
components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
  schemas:
    ListModelsResponse:
      type: object
      properties:
        models:
          type: array
          items:
            $ref: '#/components/schemas/ModelInfo'
        currentModel:
          type: string
          description: 当前激活的模型版本
    ModelInfo:
      type: object
      properties:
        version:
          type: string
          description: 模型版本号
        name:
          type: string
          description: 模型名称
        accuracy:
          type: number
          description: 模型准确率
        status:
          type: string
          enum: [active, inactive]
    PredictionResponse:
      type: object
      properties:
        symbol:
          type: string
        interval:
          type: string
        prediction:
          $ref: '#/components/schemas/PredictionResult'
        ruleBasedSignal:
          $ref: '#/components/schemas/RuleSignalOutput'
        fusedSignal:
          $ref: '#/components/schemas/PredictionResult'
    PredictionResult:
      type: object
      properties:
        direction:
          type: string
          enum: [long, short, neutral]
          description: 预测方向
        confidence:
          type: number
          description: 置信度 (0-1)
        priceTarget:
          type: number
          nullable: true
        modelVersion:
          type: string
        generatedAt:
          type: string
          format: date-time
    RuleSignalOutput:
      type: object
      properties:
        direction:
          type: string
        confidence:
          type: number
    BacktestRequest:
      type: object
      required:
        - symbol
        - interval
        - startTime
        - endTime
        - initialBalance
      properties:
        symbol:
          type: string
          description: 交易对
        interval:
          type: string
          description: K 线周期
        startTime:
          type: string
          format: date-time
          description: 回测开始时间 (RFC3339)
        endTime:
          type: string
          format: date-time
          description: 回测结束时间 (RFC3339)
        initialBalance:
          type: number
          description: 初始资金
        modelVersion:
          type: string
          nullable: true
          description: 指定模型版本
    BacktestResponse:
      type: object
      properties:
        backtestId:
          type: string
        symbol:
          type: string
        interval:
          type: string
        period:
          $ref: '#/components/schemas/BacktestPeriod'
        results:
          $ref: '#/components/schemas/BacktestResults'
        modelVersion:
          type: string
    BacktestPeriod:
      type: object
      properties:
        start:
          type: string
          format: date-time
        end:
          type: string
          format: date-time
    BacktestResults:
      type: object
      properties:
        totalTrades:
          type: integer
        winningTrades:
          type: integer
        winRate:
          type: number
        totalPnl:
          type: number
        totalPnlPercent:
          type: number
        maxDrawdown:
          type: number
        sharpeRatio:
          type: number
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

## 1. GET /ai/models — 列出 AI 模型

列出所有可用的 AI 模型版本及当前激活的模型。

### curl 示例

```bash
curl -X GET "http://localhost:8080/api/v1/ai/models" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
{
  "models": [
    {
      "version": "v1.0",
      "name": "AI Model v1.0",
      "accuracy": 0.72,
      "status": "active"
    },
    {
      "version": "v2.0",
      "name": "AI Model v2.0",
      "accuracy": 0.75,
      "status": "inactive"
    }
  ],
  "currentModel": "v1.0"
}
```

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `models` | array | AI 模型列表 |
| `models[].version` | string | 模型版本号 |
| `models[].name` | string | 模型名称 |
| `models[].accuracy` | number | 模型准确率 (0-1) |
| `models[].status` | string | 状态：`active` 或 `inactive` |
| `currentModel` | string | 当前激活的模型版本 |

---

## 2. GET /ai/predictions/{symbol} — 获取 AI 预测

获取指定交易对的融合 AI 预测信号，包括 AI 模型预测、规则信号和融合信号。

### 路径参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `symbol` | string | 是 | 交易对符号，如 `BTCUSDT`、`ETHUSDT` |

### Query 参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `interval` | string | 否 | `1h` | K 线周期 |

### curl 示例

```bash
# 获取 BTCUSDT 的 AI 预测
curl -X GET "http://localhost:8080/api/v1/ai/predictions/BTCUSDT" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"

# 获取 BTCUSDT 4 小时周期的预测
curl -X GET "http://localhost:8080/api/v1/ai/predictions/BTCUSDT?interval=4h" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
{
  "symbol": "BTCUSDT",
  "interval": "1h",
  "prediction": {
    "direction": "long",
    "confidence": 0.78,
    "priceTarget": 67500.00,
    "modelVersion": "v1.0",
    "generatedAt": "2026-06-01T10:30:00Z"
  },
  "ruleBasedSignal": {
    "direction": "neutral",
    "confidence": 0.50
  },
  "fusedSignal": {
    "direction": "long",
    "confidence": 0.64,
    "priceTarget": null,
    "modelVersion": "v1.0",
    "generatedAt": "2026-06-01T10:30:00Z"
  }
}
```

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `symbol` | string | 交易对 |
| `interval` | string | K 线周期 |
| `prediction` | object | AI 模型预测结果 |
| `prediction.direction` | string | 预测方向：`long` / `short` / `neutral` |
| `prediction.confidence` | number | 置信度 (0-1) |
| `prediction.priceTarget` | number | 目标价格（可能为 null） |
| `prediction.modelVersion` | string | 使用的模型版本 |
| `prediction.generatedAt` | string | 生成时间（RFC3339） |
| `ruleBasedSignal` | object | 规则信号（可能为 null） |
| `fusedSignal` | object | 融合信号（AI + 规则） |

---

## 3. POST /ai/backtest — AI 信号回测

使用 AI 信号对指定交易对和时间范围进行回测。

### curl 示例

```bash
curl -X POST "http://localhost:8080/api/v1/ai/backtest" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTCUSDT",
    "interval": "1h",
    "startTime": "2024-01-01T00:00:00Z",
    "endTime": "2024-06-01T00:00:00Z",
    "initialBalance": 100000.0,
    "modelVersion": "v1.0"
  }'
```

### 请求体

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `symbol` | string | 是 | 交易对 |
| `interval` | string | 是 | K 线周期 |
| `startTime` | string | 是 | 回测开始时间（RFC3339 格式） |
| `endTime` | string | 是 | 回测结束时间（RFC3339 格式） |
| `initialBalance` | number | 是 | 初始资金（≥ 100） |
| `modelVersion` | string | 否 | 指定模型版本，默认为 `v1.0` |

### 验证规则

- `startTime` 必须早于 `endTime`
- 回测时间范围至少需要 100 根 K 线数据
- `initialBalance` 不得低于 100 USDT

### 响应示例

```json
{
  "backtestId": "bt_a1b2c3d4e5f6",
  "symbol": "BTCUSDT",
  "interval": "1h",
  "period": {
    "start": "2024-01-01T00:00:00Z",
    "end": "2024-06-01T00:00:00Z"
  },
  "results": {
    "totalTrades": 45,
    "winningTrades": 28,
    "winRate": 0.622,
    "totalPnl": 12500.00,
    "totalPnlPercent": 12.5,
    "maxDrawdown": 0.08,
    "sharpeRatio": 1.45
  },
  "modelVersion": "v1.0"
}
```

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `backtestId` | string | 回测任务 ID |
| `symbol` | string | 交易对 |
| `interval` | string | K 线周期 |
| `period.start` | string | 回测开始时间 |
| `period.end` | string | 回测结束时间 |
| `results.totalTrades` | integer | 总交易次数 |
| `results.winningTrades` | integer | 盈利交易次数 |
| `results.winRate` | number | 胜率 (0-1) |
| `results.totalPnl` | number | 总盈亏（USDT） |
| `results.totalPnlPercent` | number | 总盈亏百分比 |
| `results.maxDrawdown` | number | 最大回撤 (0-1) |
| `results.sharpeRatio` | number | 夏普比率 |
| `modelVersion` | string | 使用的模型版本 |

---

## 数据模型

### ModelInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| `version` | String | 模型版本号 |
| `name` | String | 模型名称 |
| `accuracy` | f64 | 准确率 (0-1) |
| `status` | String | 状态：`active` / `inactive` |

### PredictionResult

| 字段 | 类型 | 说明 |
|------|------|------|
| `direction` | String | 方向：`long` / `short` / `neutral` |
| `confidence` | f64 | 置信度 (0-1) |
| `priceTarget` | Option<f64> | 目标价格 |
| `modelVersion` | String | 模型版本 |
| `generatedAt` | String | 生成时间（RFC3339） |

### BacktestResults

| 字段 | 类型 | 说明 |
|------|------|------|
| `totalTrades` | i32 | 总交易次数 |
| `winningTrades` | i32 | 盈利交易次数 |
| `winRate` | f64 | 胜率 (0-1) |
| `totalPnl` | f64 | 总盈亏 |
| `totalPnlPercent` | f64 | 盈亏百分比 |
| `maxDrawdown` | f64 | 最大回撤 |
| `sharpeRatio` | f64 | 夏普比率 |

---

## 错误码

| HTTP 状态码 | code | message | 说明 |
|-------------|------|---------|------|
| 400 | 400 | Bad Request | 请求参数错误（数据不足、时间范围错误等） |
| 401 | 401 | Unauthorized | Token 无效或已过期 |
| 404 | 404 | Not Found | 未找到 K 线数据 |
| 500 | 500 | Internal server error | 服务器内部错误 |

---

## 备注

- 所有响应遵循统一格式：`{ "code": 0, "data": T, "message": "success" }`
- `code !== 0` 时表示请求失败
- AI 服务不可用时会返回降级信号（neutral direction, 0.5 confidence）
- K 线数据至少需要 10 根才能生成有效预测
- 回测至少需要 100 根 K 线数据
