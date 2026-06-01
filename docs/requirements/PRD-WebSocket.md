---
status: WIP
date: 20260601
---

# PRD-WebSocket-20260601 — WebSocket 服务

## 1. 功能概述

WebSocket 服务为客户端提供实时双向数据推送通道，支持行情数据（ticker/depth/kline）、交易执行通知、回测进度推送等功能。客户端通过 JWT 认证订阅指定频道，服务端按需过滤推送。

**目标用户**：前端 Vue 客户端（实时行情监控）、交易员（订单执行通知）

---

## 2. 用户故事

- **US-WS1**：作为交易员，我希望打开行情页面时自动接收实时价格更新，无需刷新页面
- **US-WS2**：作为交易员，我希望下单后立即收到成交通知，无需轮询
- **US-WS3**：作为交易员，我希望能够订阅/取消订阅特定交易对的行情，以便节省带宽

---

## 3. API 端点

### 3.1 GET /api/v1/ws — WebSocket 升级

**描述**：升级 HTTP 连接为 WebSocket 连接

**认证**：JWT（通过 Query 参数 `?token=xxx`）

**Query 参数**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| token | string | 是 | JWT access token |

**升级成功**：返回 101 Switching Protocols，后续数据通过 WebSocket 传输

---

## 4. 消息格式

### 4.1 服务端 → 客户端消息

#### 行情 ticker
```json
{
  "type": "ticker",
  "symbol": "BTCUSDT",
  "data": {
    "price": "60000.00",
    "change": "123.45",
    "changePercent": "0.21",
    "volume": "12345.67",
    "high": "60100.00",
    "low": "59800.00",
    "bid": "59999.00",
    "ask": "60000.00"
  }
}
```

#### 订单簿 depth
```json
{
  "type": "depth",
  "symbol": "BTCUSDT",
  "data": {
    "bids": [["59999.00", "1.23"]],
    "asks": [["60000.00", "2.34"]],
    "timestamp": 1717200000000
  }
}
```

#### K线 kline
```json
{
  "type": "kline",
  "symbol": "BTCUSDT",
  "data": {
    "interval": "1m",
    "open": "59900.00",
    "high": "60000.00",
    "low": "59800.00",
    "close": "60000.00",
    "volume": "123.45",
    "timestamp": 1717200000000
  }
}
```

#### 交易执行通知 trade_executed
```json
{
  "type": "trade_executed",
  "symbol": "BTCUSDT",
  "data": {
    "order_id": "uuid",
    "side": "BUY",
    "filled_quantity": "0.5",
    "avg_fill_price": "60000.00",
    "is_fully_filled": true,
    "realized_pnl": "123.45"
  }
}
```

#### 回测进度 backtest_progress
```json
{
  "type": "backtest_progress",
  "symbol": "backtest-uuid",
  "data": {
    "progress": 45,
    "status": "running"
  }
}
```

#### 心跳 heartbeat
```json
{
  "type": "heartbeat",
  "ts": 1717200000000
}
```

### 4.2 客户端 → 服务端消息

#### 订阅 subscribe
```json
{
  "action": "subscribe",
  "channels": ["market:ticker:BTCUSDT", "market:depth:ETHUSDT"],
  "symbol": "BTCUSDT"
}
```

#### 取消订阅 unsubscribe
```json
{
  "action": "unsubscribe",
  "channels": ["market:ticker:BTCUSDT"],
  "symbol": null
}
```

#### 服务端响应
```json
{ "action": "subscribed", "status": "ok" }
```

---

## 5. 频道命名规范

| 频道 | 消息类型 | 说明 |
|------|---------|------|
| `market:ticker:{symbol}` | ticker | 单交易对 ticker |
| `market:depth:{symbol}` | depth | 单交易对订单簿 |
| `market:kline:{symbol}` | kline | 单交易对 K 线 |
| `trade:executed` | trade_executed | 个人成交通知（全量推送，按 user_id 过滤） |
| `backtest:progress:{id}` | backtest_progress | 回测进度（按 ID 精确匹配） |

---

## 6. 连接管理

### 6.1 连接建立流程

```
客户端 → GET /api/v1/ws?token=JWT
    │
    ├─→ 验证 JWT（token 合法性）
    ├─→ 解析 user_id
    ├─→ 建立 WsHub 订阅
    └─→ 进入消息循环
```

### 6.2 心跳机制

- 服务端每 30 秒无消息发送时，主动发送 `heartbeat`
- 客户端 60 秒无响应则断开连接

### 6.3 断线重连

- 客户端应实现自动重连（指数退避：1s → 2s → 4s → max 30s）

---

## 7. 业务流程

```
Binance WS Connector
    │
    ├─→ 接收 Binance WebSocket 原始消息
    ├─→ 规范化为 HubMessage
    └─→ 推送到 WsHub（broadcast）

WsHub（broadcast channel）
    │
    └─→ 所有连接的 WebSocket Handler
            │
            ├─→ 按订阅过滤（channels / symbols）
            ├─→ 按 user_id 过滤（trade_executed）
            └─→ 序列化后发送给客户端
```

---

## 8. 边界条件

- JWT token 无效或过期 → 拒绝升级，返回 401
- 订阅频道不存在 → 静默忽略，不报错
- 行情数据不可用时 → 不推送，不阻塞其他频道
- trade_executed 消息严格按 user_id 路由，不泄露给其他用户
- 并发连接数上限：每用户 3 个连接（超出则最早的连接被踢出）

---

## 9. 验收标准（Gherkin 格式）

```gherkin
Feature: WebSocket 实时通信

  Scenario: 客户端成功建立 WebSocket 连接
    Given 用户已登录并持有有效 JWT
    When 客户端连接 GET /api/v1/ws?token=xxx
    Then 连接升级成功
    And 客户端收到心跳消息

  Scenario: 订阅行情 ticker
    Given 客户端已连接
    When 客户端发送 subscribe（channels=["market:ticker:BTCUSDT"]）
    Then 服务端确认订阅成功
    And 后续 BTCUSDT ticker 更新推送到客户端

  Scenario: 取消订阅
    Given 客户端已订阅多个频道
    When 客户端发送 unsubscribe（channels=["market:ticker:BTCUSDT"]）
    Then BTCUSDT ticker 不再推送
    And 其他频道继续正常推送

  Scenario: trade_executed 消息隔离
    Given 用户 A 和用户 B 都连接了 WebSocket
    When 用户 A 成交
    Then 只有用户 A 收到 trade_executed 通知
    And 用户 B 不收到

  Scenario: 断线自动重连
    Given 客户端连接中
    When 网络中断导致断线
    Then 客户端自动重连（指数退避）
    And 订阅关系在重连后需重新建立

  Scenario: 心跳保持连接
    Given 客户端连接但无其他消息
    When 30 秒无消息
    Then 服务端发送 heartbeat
    And 客户端保持连接活跃

  Scenario: 多交易对订阅
    Given 客户端连接
    When 客户端订阅 symbol=BTCUSDT 和 symbol=ETHUSDT
    Then BTCUSDT 和 ETHUSDT 的行情都会推送
    And 其他交易对不推送
```
