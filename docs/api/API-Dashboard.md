# 仪表盘 API 文档

> 版本: v1.0.0 | 基础路径: `/api/v1` | 认证: Bearer Token (JWT)

---

## 目录

- [API 总览](#api-总览)
- [1. GET /dashboard/stats — 仪表盘统计](#1-get-dashboardstats--仪表盘统计)
- [2. GET /dashboard/pnl — PnL 时序数据](#2-get-dashboardpnl--pnl-时序数据)
- [数据模型](#数据模型)
- [错误码](#错误码)

---

## API 总览

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| GET | `/api/v1/dashboard/stats` | 获取仪表盘统计数据 | 必需 |
| GET | `/api/v1/dashboard/pnl` | 获取 PnL 时序数据（收益曲线） | 必需 |

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
  title: 仪表盘 API
  version: 1.0.0
paths:
  /api/v1/dashboard/stats:
    get:
      tags:
        - Dashboard
      summary: 获取仪表盘统计数据
      security:
        - BearerAuth: []
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/DashboardStats'
        '401':
          $ref: '#/components/responses/Unauthorized'
  /api/v1/dashboard/pnl:
    get:
      tags:
        - Dashboard
      summary: 获取 PnL 时序数据
      security:
        - BearerAuth: []
      parameters:
        - name: range
          in: query
          required: false
          schema:
            type: string
            enum: [7d, 30d, 90d]
            default: 30d
          description: 时间范围
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PnLHistory'
        '401':
          $ref: '#/components/responses/Unauthorized'
components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
  schemas:
    DashboardStats:
      type: object
      properties:
        totalPnl:
          type: number
          description: 总盈亏
        dailyPnl:
          type: number
          description: 今日盈亏
        winRate:
          type: number
          description: 胜率 (%)
        sharpeRatio:
          type: number
          description: 夏普比率
        activePositions:
          type: integer
          format: int64
          description: 活跃策略数
        totalTrades:
          type: integer
          format: int64
          description: 总交易次数
        balance:
          type: number
          description: 账户余额
    PnLHistory:
      type: object
      properties:
        points:
          type: array
          items:
            $ref: '#/components/schemas/PnLPoint'
    PnLPoint:
      type: object
      properties:
        timestamp:
          type: string
          format: date-time
        pnl:
          type: number
        equity:
          type: number
        value:
          type: number
          description: pnl 的别名，兼容前端
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

## 1. GET /dashboard/stats — 仪表盘统计

获取当前用户的仪表盘核心统计数据，包括总盈亏、胜率、交易次数等。

### curl 示例

```bash
# 获取仪表盘统计
curl -X GET "http://localhost:8080/api/v1/dashboard/stats" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
{
  "code": 0,
  "data": {
    "totalPnl": 15234.56,
    "dailyPnl": 1234.56,
    "winRate": 65.5,
    "sharpeRatio": 1.45,
    "activePositions": 5,
    "totalTrades": 150,
    "balance": 100000.00
  },
  "message": "success"
}
```

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `totalPnl` | number | 总盈亏（USDT） |
| `dailyPnl` | number | 今日盈亏（USDT） |
| `winRate` | number | 胜率百分比（0-100） |
| `sharpeRatio` | number | 夏普比率 |
| `activePositions` | integer | 活跃策略数 |
| `totalTrades` | integer | 历史总交易次数 |
| `balance` | number | 账户余额（USDT） |

---

## 2. GET /dashboard/pnl — PnL 时序数据

获取指定时间范围内的每日 PnL 和权益曲线数据。

### Query 参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `range` | string | 否 | `30d` | 时间范围：`7d` / `30d` / `90d` |

### curl 示例

```bash
# 获取最近 7 天的 PnL 数据
curl -X GET "http://localhost:8080/api/v1/dashboard/pnl?range=7d" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"

# 获取最近 30 天的 PnL 数据（默认）
curl -X GET "http://localhost:8080/api/v1/dashboard/pnl" \
  -H "Authorization: Bearer <your_token>" \
  -H "Content-Type: application/json"
```

### 响应示例

```json
{
  "code": 0,
  "data": {
    "points": [
      {
        "timestamp": "2026-05-25T00:00:00Z",
        "pnl": 500.25,
        "equity": 100500.25,
        "value": 500.25
      },
      {
        "timestamp": "2026-05-26T00:00:00Z",
        "pnl": 1200.50,
        "equity": 101700.75,
        "value": 1200.50
      },
      {
        "timestamp": "2026-05-27T00:00:00Z",
        "pnl": -300.00,
        "equity": 101400.75,
        "value": -300.00
      }
    ]
  },
  "message": "success"
}
```

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `points` | array | PnL 数据点数组 |
| `points[].timestamp` | string | 时间戳（RFC3339 格式） |
| `points[].pnl` | number | 当日盈亏（USDT） |
| `points[].equity` | number | 累计权益（USDT） |
| `points[].value` | number | pnl 的别名，用于前端兼容 |

---

## 数据模型

### DashboardStats

| 字段 | 类型 | 说明 |
|------|------|------|
| `totalPnl` | f64 | 汇总盈亏 |
| `dailyPnl` | f64 | 今日盈亏 |
| `winRate` | f64 | 胜率百分比 |
| `sharpeRatio` | f64 | 夏普比率 |
| `activePositions` | i64 | 活跃策略数 |
| `totalTrades` | i64 | 总交易次数 |
| `balance` | f64 | 账户余额 |

### PnLHistory

| 字段 | 类型 | 说明 |
|------|------|------|
| `points` | Vec<PnLPoint> | PnL 数据点列表 |

### PnLPoint

| 字段 | 类型 | 说明 |
|------|------|------|
| `timestamp` | String | 时间戳（RFC3339） |
| `pnl` | f64 | 当日盈亏 |
| `equity` | f64 | 累计权益 |
| `value` | f64 | pnl 别名 |

---

## 错误码

| HTTP 状态码 | code | message | 说明 |
|-------------|------|---------|------|
| 401 | 401 | Unauthorized | Token 无效或已过期 |
| 500 | 500 | Internal server error | 服务器内部错误 |
| 500 | — | Database error | 数据库查询失败 |

---

## 备注

- 所有响应遵循统一格式：`{ "code": 0, "data": T, "message": "success" }`
- `code !== 0` 时表示请求失败
- 时间范围默认为 30 天
- PnL 数据按天聚合，权益曲线为累计值
