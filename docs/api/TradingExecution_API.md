# 交易执行 API 文档

> 版本: v0.6.0 | 基础路径: `/api/v1` | 认证: Bearer Token (JWT)

---

## 目录

- [API 总览](#api-总览)
- [1. POST /orders — 创建委托](#1-post-orders--创建委托)
- [2. GET /orders — 委托列表](#2-get-orders--委托列表)
- [3. GET /orders/{id} — 委托详情](#3-get-ordersid--委托详情)
- [4. DELETE /orders/{id} — 撤单](#4-delete-ordersid--撤单)
- [数据模型](#数据模型)
- [WebSocket 推送协议](#websocket-推送协议)
- [错误码](#错误码)

---

## API 总览

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| POST | `/api/v1/orders` | 创建委托（限价/市价） | 必需 |
| GET | `/api/v1/orders` | 获取当前用户委托列表（分页/筛选） | 必需 |
| GET | `/api/v1/orders/{id}` | 获取单个委托详情 | 必需 |
| DELETE | `/api/v1/orders/{id}` | 撤销委托（仅 pending / partial_filled） | 必需 |

---

## 统一响应格式

### 成功响应

```json
{
  "code": 0,
  "data": { ... },
  "message": "success"
}
```

### 分页响应

```json
{
  "code": 0,
  "data": {
    "items": [...],
    "page": 1,
    "size": 20,
    "total": 42
  },
  "message": "success"
}
```

### 错误响应

```json
{
  "code": 40001,
  "message": "账户余额不足"
}
```

---

## 1. POST /orders — 创建委托

创建一个新的委托订单。限价单将挂单等待撮合；市价单将立即按市场最优价成交。

### 请求

```
POST /api/v1/orders
Content-Type: application/json
```

### 请求体

```json
{
  "symbol": "BTC/USDT",
  "side": "buy",
  "order_type": "limit",
  "price": "49500.00",
  "quantity": "0.1000",
  "time_in_force": "GTC"
}
```

### 请求参数说明

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `symbol` | string | 是 | 交易对，如 `BTC/USDT`、`ETH/USDT` |
| `side` | string | 是 | 方向: `buy`（买入）/ `sell`（卖出） |
| `order_type` | string | 是 | 委托类型: `limit`（限价）/ `market`（市价） |
| `price` | string | 条件必填 | 限价单必填，Decimal 字符串；市价单忽略此字段（传 `null` 或省略） |
| `quantity` | string | 是 | 委托数量，Decimal 字符串，精度受交易对 `quantity_precision` 约束 |
| `time_in_force` | string | 是 | 有效期: `GTC`（一直有效）/ `IOC`（立即成交或取消）/ `FOK`（全部成交或取消） |

### 验证规则

| 规则 | 说明 |
|------|------|
| `symbol` 合法 | 必须为系统支持的交易对 |
| `side` 枚举 | 仅允许 `buy` / `sell` |
| `order_type` 枚举 | 仅允许 `limit` / `market` |
| `price` 必填性 | 限价单时 `price` 必填且 > 0 |
| `price` 精度 | 小数位数 ≤ 交易对的 `price_precision` |
| `quantity` 正值 | 必须 > 0 |
| `quantity` 精度 | 小数位数 ≤ 交易对的 `quantity_precision` |
| `quantity` 范围 | ≥ `min_quantity` 且 ≤ `max_quantity` |
| `time_in_force` 枚举 | 仅允许 `GTC` / `IOC` / `FOK` |
| 余额充足 | 买入: `price × quantity ≤ account.balance` |
| 持仓充足 | 卖出: `quantity ≤ position.available_quantity` |
| 最小金额 | `price × quantity ≥ min_notional` |
| 未成交委托上限 | 当前用户未成交委托数不得超过上限（默认 50） |

### 响应体（201 Created）

```json
{
  "order_id": "1234567890",
  "symbol": "BTC/USDT",
  "side": "buy",
  "order_type": "limit",
  "price": "49500.00",
  "quantity": "0.1000",
  "filled_quantity": "0.0000",
  "avg_fill_price": null,
  "status": "pending",
  "mode": "paper",
  "fee": "0.0000",
  "time_in_force": "GTC",
  "created_at": "2026-05-14T06:00:00Z",
  "updated_at": "2026-05-14T06:00:00Z"
}
```

### 行为说明

| 场景 | 行为 |
|------|------|
| 限价单 | 冻结保证金 → 写入 orders 表 status=pending → 进入撮合队列 → WS 推送 order_update |
| 市价单 | 冻结保证金 → 写入 orders 表 status=pending → 立即撮合 → 成交后 WS 推送 order_update + trade |
| 余额不足 | 返回 40001 错误，不创建委托 |
| 持仓不足 | 返回 40002 错误，不创建委托 |
| 交易对不支持 | 返回 40003 错误 |
| 未成交委托达上限 | 返回 42901 错误 |

### curl 示例

```bash
# 限价买入
curl -X POST "http://localhost:8080/api/v1/orders" \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTC/USDT",
    "side": "buy",
    "order_type": "limit",
    "price": "49500.00",
    "quantity": "0.1000",
    "time_in_force": "GTC"
  }'

# 市价卖出
curl -X POST "http://localhost:8080/api/v1/orders" \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTC/USDT",
    "side": "sell",
    "order_type": "market",
    "quantity": "0.0500",
    "time_in_force": "IOC"
  }'
```

### 错误码

| code | 场景 |
|------|------|
| 40001 | 账户余额不足 |
| 40002 | 可用持仓不足 |
| 40003 | 交易对不支持 |
| 40004 | 数量无效（零/负/超精度/超范围） |
| 40005 | 价格无效（零/负/超精度） |
| 40006 | 低于最小下单量 |
| 40007 | 低于最小下单金额 |
| 40301 | 权限不足（如无实盘权限） |
| 42901 | 未成交委托达上限 |

---

## 2. GET /orders — 委托列表

获取当前用户的委托列表，支持分页和多条件筛选。

### 请求

```
GET /api/v1/orders?page=1&size=20&status=pending&symbol=BTC/USDT&side=buy
```

### 查询参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `page` | integer | 否 | `1` | 页码，从 1 开始 |
| `size` | integer | 否 | `20` | 每页条数，范围 1~100 |
| `status` | string | 否 | — | 按状态筛选: `pending` / `partial_filled` / `filled` / `cancelled` / `expired` / `rejected` |
| `symbol` | string | 否 | — | 按交易对筛选，精确匹配 |
| `side` | string | 否 | — | 按方向筛选: `buy` / `sell` |
| `order_type` | string | 否 | — | 按类型筛选: `limit` / `market` |
| `start_time` | string | 否 | — | 起始时间（ISO 8601），筛选 `created_at` |
| `end_time` | string | 否 | — | 结束时间（ISO 8601），筛选 `created_at` |

### 响应体

```json
{
  "items": [
    {
      "order_id": "1234567890",
      "symbol": "BTC/USDT",
      "side": "buy",
      "order_type": "limit",
      "price": "49500.00",
      "quantity": "0.1000",
      "filled_quantity": "0.0500",
      "avg_fill_price": "49500.00",
      "status": "partial_filled",
      "mode": "paper",
      "fee": "2.4750",
      "time_in_force": "GTC",
      "created_at": "2026-05-14T06:00:00Z",
      "updated_at": "2026-05-14T06:05:00Z"
    }
  ],
  "page": 1,
  "size": 20,
  "total": 3
}
```

### 委托状态说明

| 状态 | 说明 | 可进行的操作 |
|------|------|-------------|
| `pending` | 待成交，委托已提交等待撮合 | 撤单 |
| `partial_filled` | 部分成交 | 撤单（撤销剩余数量） |
| `filled` | 已完全成交 | — |
| `cancelled` | 已撤销 | — |
| `expired` | 已过期（GTC 到期或 IOC/FOK 未成交） | — |
| `rejected` | 已拒绝（风控拒绝） | — |

### 排序

默认按 `created_at` 降序（最新委托在前）。

### curl 示例

```bash
# 获取所有委托（第一页）
curl -X GET "http://localhost:8080/api/v1/orders?page=1&size=20" \
  -H "Authorization: Bearer <token>"

# 筛选未成交委托
curl -X GET "http://localhost:8080/api/v1/orders?status=pending&status=partial_filled" \
  -H "Authorization: Bearer <token>"

# 按交易对和时间范围筛选
curl -X GET "http://localhost:8080/api/v1/orders?symbol=BTC/USDT&start_time=2026-05-14T00:00:00Z&end_time=2026-05-14T23:59:59Z" \
  -H "Authorization: Bearer <token>"
```

### 错误码

| code | 场景 |
|------|------|
| 40001 | 参数类型错误（如 page 非数字） |

---

## 3. GET /orders/{id} — 委托详情

获取单个委托的完整信息，包括所有字段。

### 请求

```
GET /api/v1/orders/1234567890
```

### 路径参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 委托 ID（order_id） |

### 响应体

```json
{
  "order_id": "1234567890",
  "symbol": "BTC/USDT",
  "side": "buy",
  "order_type": "limit",
  "price": "49500.00",
  "quantity": "0.1000",
  "filled_quantity": "0.0500",
  "avg_fill_price": "49500.00",
  "status": "partial_filled",
  "mode": "paper",
  "fee": "2.4750",
  "time_in_force": "GTC",
  "created_at": "2026-05-14T06:00:00Z",
  "updated_at": "2026-05-14T06:05:00Z"
}
```

### curl 示例

```bash
curl -X GET "http://localhost:8080/api/v1/orders/1234567890" \
  -H "Authorization: Bearer <token>"
```

### 错误码

| code | 场景 |
|------|------|
| 40401 | 委托不存在 |
| 40301 | 无权访问（委托属于其他用户） |

---

## 4. DELETE /orders/{id} — 撤单

撤销指定委托。仅 `pending` 和 `partial_filled` 状态的委托可以撤销。

### 请求

```
DELETE /api/v1/orders/1234567890
```

### 路径参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | string | 委托 ID（order_id） |

### 响应体（200 OK）

```json
{
  "order_id": "1234567890",
  "status": "cancelled",
  "released_amount": "4950.00",
  "message": "委托已撤销，冻结保证金已释放"
}
```

### 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `order_id` | string | 被撤销的委托 ID |
| `status` | string | 撤销后的状态，固定为 `cancelled` |
| `released_amount` | string | 释放的冻结保证金金额（Decimal 字符串） |
| `message` | string | 操作结果描述 |

### 状态限制

| 当前状态 | 是否可撤销 |
|----------|------------|
| `pending` | ✅ 允许撤销，释放全部冻结保证金 |
| `partial_filled` | ✅ 允许撤销剩余未成交部分，释放剩余冻结保证金 |
| `filled` | ❌ 已完全成交，返回 40901 |
| `cancelled` | ❌ 已撤销，返回 40901 |
| `expired` | ❌ 已过期，返回 40901 |
| `rejected` | ❌ 已拒绝，返回 40901 |

### 行为说明

| 场景 | 行为 |
|------|------|
| 正常撤单 | PG 行锁 `SELECT FOR UPDATE` → 检查状态 → 从内存订单簿移除 → 释放冻结保证金 → 更新 status = cancelled → WS 推送 order_update |
| 并发冲突 | 委托在撤单请求到达时已成交，返回 40901 |
| 部分成交后撤单 | 撤销剩余未成交数量，释放剩余冻结保证金，已成交部分不受影响 |

### curl 示例

```bash
curl -X DELETE "http://localhost:8080/api/v1/orders/1234567890" \
  -H "Authorization: Bearer <token>"
```

### 错误码

| code | 场景 |
|------|------|
| 40401 | 委托不存在 |
| 40301 | 无权操作（委托属于其他用户） |
| 40901 | 委托状态不允许撤销（已成交/已撤销/已过期/已拒绝） |

---

## 数据模型

### Order（委托）

| 字段 | 类型 | 说明 |
|------|------|------|
| `order_id` | string | 委托唯一标识，i64 序列化为 string |
| `symbol` | string | 交易对，如 `BTC/USDT` |
| `side` | string | 方向: `buy` / `sell` |
| `order_type` | string | 委托类型: `limit` / `market` |
| `price` | string \| null | 限价单为 Decimal 字符串，市价单为 `null` |
| `quantity` | string | 委托数量，Decimal 字符串 |
| `filled_quantity` | string | 已成交数量，Decimal 字符串 |
| `avg_fill_price` | string \| null | 成交均价，Decimal 字符串，未成交时为 `null` |
| `status` | string | 状态: `pending` / `partial_filled` / `filled` / `cancelled` / `expired` / `rejected` |
| `mode` | string | 交易模式: `paper`（模拟）/ `live`（实盘） |
| `fee` | string | 手续费，Decimal 字符串 |
| `time_in_force` | string | 有效期: `GTC` / `IOC` / `FOK` |
| `created_at` | string | 创建时间，ISO 8601 |
| `updated_at` | string | 最后更新时间，ISO 8601 |

### CreateOrderRequest（创建委托请求）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `symbol` | string | 是 | 交易对 |
| `side` | string | 是 | 方向: `buy` / `sell` |
| `order_type` | string | 是 | 委托类型: `limit` / `market` |
| `price` | string | 条件必填 | 限价单必填，Decimal 字符串 |
| `quantity` | string | 是 | 委托数量，Decimal 字符串 |
| `time_in_force` | string | 是 | 有效期: `GTC` / `IOC` / `FOK` |

### Position（持仓）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | string | 持仓唯一标识 |
| `symbol` | string | 交易对 |
| `side` | string | 方向: `long`（多头）/ `short`（空头） |
| `quantity` | string | 持仓数量，Decimal 字符串 |
| `available_quantity` | string | 可用数量（扣除冻结），Decimal 字符串 |
| `avg_entry_price` | string | 开仓加权均价，Decimal 字符串 |
| `unrealized_pnl` | string | 未实现盈亏，Decimal 字符串 |
| `realized_pnl` | string | 已实现盈亏，Decimal 字符串 |
| `mode` | string | 交易模式: `paper` / `live` |
| `created_at` | string | 创建时间，ISO 8601 |
| `updated_at` | string | 最后更新时间，ISO 8601 |

### Account（账户）

| 字段 | 类型 | 说明 |
|------|------|------|
| `user_id` | string | 用户 ID |
| `balance` | string | 可用余额，Decimal 字符串 |
| `frozen_balance` | string | 冻结余额，Decimal 字符串 |
| `initial_balance` | string | 初始资金，Decimal 字符串 |
| `total_pnl` | string | 累计盈亏，Decimal 字符串 |
| `equity` | string | 权益 = balance + frozen_balance + unrealized_pnl，Decimal 字符串 |
| `positions_count` | integer | 持仓数量 |
| `active_orders_count` | integer | 活跃委托数量 |

### Trade（成交）

| 字段 | 类型 | 说明 |
|------|------|------|
| `trade_id` | string | 成交唯一标识 |
| `order_id` | string | 关联委托 ID |
| `symbol` | string | 交易对 |
| `side` | string | 方向: `buy` / `sell` |
| `price` | string | 成交价格，Decimal 字符串 |
| `quantity` | string | 成交数量，Decimal 字符串 |
| `fee` | string | 手续费，Decimal 字符串 |
| `is_maker` | boolean | 是否为挂单方 |
| `created_at` | string | 成交时间，ISO 8601 |

---

## WebSocket 推送协议

交易执行模块通过 WebSocket 实时推送委托状态变更和成交信息。

### 连接

```
ws://localhost:8080/ws/trade?token=<jwt_token>
```

### 频道订阅

连接建立后，客户端发送订阅消息：

```json
{
  "action": "subscribe",
  "channels": ["order:all", "position:all"]
}
```

| 频道 | 格式 | 推送内容 |
|------|------|---------|
| `order:all` | `order:all` | 用户所有委托状态变更 |
| `position:all` | `position:all` | 用户持仓变更 |

### 订阅确认

```json
{
  "type": "subscribed",
  "channels": ["order:all", "position:all"]
}
```

### 消息类型

#### order_update — 委托状态变更

```json
{
  "type": "order_update",
  "data": {
    "order_id": "1234567890",
    "symbol": "BTC/USDT",
    "side": "buy",
    "order_type": "limit",
    "price": "49500.00",
    "quantity": "0.1000",
    "filled_quantity": "0.0500",
    "status": "partial_filled",
    "updated_at": 1715649000000
  },
  "ts": 1715649000000
}
```

#### trade — 成交通知

```json
{
  "type": "trade",
  "data": {
    "trade_id": "9876543210",
    "order_id": "1234567890",
    "symbol": "BTC/USDT",
    "side": "buy",
    "price": "49500.00",
    "quantity": "0.0500",
    "fee": "1.2375",
    "timestamp": 1715649000000
  },
  "ts": 1715649000000
}
```

#### position_update — 持仓变更

```json
{
  "type": "position_update",
  "data": {
    "symbol": "BTC/USDT",
    "side": "long",
    "quantity": "0.0500",
    "avg_entry_price": "49500.00",
    "unrealized_pnl": "25.00"
  },
  "ts": 1715649000000
}
```

#### error — 错误消息

```json
{
  "type": "error",
  "code": 40101,
  "message": "认证失败"
}
```

### 取消订阅

```json
{
  "action": "unsubscribe",
  "channels": ["position:all"]
}
```

### 重连策略

- 指数退避: 1s → 2s → 4s → 8s → 16s → 30s（封顶）
- 重连成功后自动恢复订阅
- 补发断线期间状态变更（后端支持）
- 前端幂等去重（基于 `order_id` + `status` + `updated_at`）

### 降级方案

WS 不可用时，前端自动降级为 REST 轮询：

| 数据 | 轮询接口 | 间隔 |
|------|---------|------|
| 委托列表 | `GET /api/v1/orders?status=pending&status=partial_filled` | 5s |
| 持仓列表 | `GET /api/v1/positions` | 10s |
| 账户信息 | `GET /api/v1/account` | 10s |

---

## 错误码

### 委托相关错误码

| code | 含义 | 说明 |
|------|------|------|
| 40001 | 账户余额不足 | 买入时可用余额不足以支付委托金额 |
| 40002 | 可用持仓不足 | 卖出时可用持仓不足以完成委托 |
| 40003 | 交易对不支持 | 指定的 symbol 不在系统支持的交易对列表中 |
| 40004 | 数量无效 | 数量为零/负/超精度/超范围 |
| 40005 | 价格无效 | 价格为零/负/超精度 |
| 40006 | 低于最小下单量 | 委托数量低于交易对的 `min_quantity` |
| 40007 | 低于最小下单金额 | 委托金额低于交易对的 `min_notional` |
| 40301 | 权限不足 | 无实盘交易权限等 |
| 40401 | 委托不存在 | 指定 ID 的委托不存在 |
| 40901 | 状态冲突 | 委托状态不允许当前操作（如已成交无法撤单） |
| 42901 | 未成交委托达上限 | 用户未成交委托数超过限制（默认 50） |

### 通用错误码

| code | 含义 |
|------|------|
| 0 | 成功 |
| 40101 | 认证失败 |
| 40102 | Token 过期 |
| 40103 | Token 无效 |
| 50001 | 内部错误 |
| 50002 | 数据库错误 |
| 50301 | 行情数据不可用 |
