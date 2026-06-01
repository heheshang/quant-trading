# PRD: 行情模块 — 深度数据与实时Ticker

> 版本: v1.0
> 状态: Draft
> 作者: PM
> 关联: ADR-002 (WebSocket实时推送), ADR-005 (市场数据管道), data-model.md, PRD-kline-management.md

---

## 1. 背景

主 PRD 中 US-05（深度数据与盘口）和 US-06（Ticker 实时行情）定义了行情模块的核心实时展示功能。当前系统状态如下：

### 当前状态 (2026-05-13)

- **前端类型定义已存在**：`Ticker` 接口（symbol/price/change/change_percent/volume/high/low）、`Depth` 接口（bids/asks/timestamp）
- **API 层占位已存在**：`market.ts` 定义了 `getTickers()` 和 `getDepth(symbol)` 函数
- **后端 handler 为空**：`handlers/market.rs` 文件不存在，无 REST 端点实现
- **WebSocket 骨架已搭建**：`handlers/ws.rs` 实现了 JWT 鉴权 + echo 模式，但无订阅/推送逻辑
- **数据模型提及 `ticker_snapshots` 表**：data-model.md 行情域列出了该表，但无 DDL 定义
- **Redis 缓存 Key 已设计**：`ticker:{symbol}` (HASH, TTL=5s)、`depth:{symbol}` (STRING JSON, TTL=1s)
- **Redis Pub/Sub 频道已设计**：`market:ticker:{symbol}`、`market:depth:{symbol}`
- **MarketView.vue 为占位页**：显示 "Market data coming soon"

### 为何这个功能重要

深度数据和 Ticker 是交易决策的两个核心输入：

- **深度数据**揭示买卖压力和流动性分布，是挂单策略、大单冲击分析的基础
- **Ticker** 提供市场概览，是监控、选币、快速决策的第一入口
- 两者均需 **实时推送**（ADR-002 要求 < 500ms），是整个系统"活"的标志
- 策略引擎（`strategy_engine.rs`）依赖 Ticker 数据触发信号
- 风控管理器（`risk_manager.rs`）依赖实时价格计算浮动盈亏

没有实时深度和 Ticker，用户无法做出交易决策，策略无法运行。

---

## 2. 目标

| 目标 | 指标 | 当前状态 |
|------|------|---------|
| Ticker REST 查询 | GET /api/v1/market/tickers 返回所有交易对 Ticker，P99 < 200ms | API 函数已定义，后端未实现 |
| Ticker 单品查询 | GET /api/v1/market/ticker?symbol=X 返回单个交易对 Ticker | 未实现 |
| 深度数据 REST 查询 | GET /api/v1/market/depth?symbol=X 返回 N 档盘口，P99 < 100ms | API 函数已定义，后端未实现 |
| WebSocket Ticker 推送 | 订阅 ticker 频道后推送延迟 < 500ms | WS echo 模式，无订阅/推送 |
| WebSocket 深度推送 | 订阅 depth 频道后推送延迟 < 500ms | 未实现 |
| 深度档位配置 | 支持 5/10/20/50 档位切换 | 未实现 |
| Ticker 价格闪烁 | 价格变化时 UI 闪烁反馈，0.5s 恢复 | 未实现 |
| 深度图可视化 | 横轴价格、纵轴累计量的面积图 | 未实现 |
| Redis 缓存命中率 | Ticker ≥ 95%，Depth ≥ 90% | 缓存 Key 已设计，逻辑未实现 |
| 连接稳定性 | 1000 并发 WS 连接，断线重连 ≤ 5s | WS 基础连接已实现 |

### 非目标 (Non-Goals)

- ❌ 历史深度数据存储和回放（P2+，后续阶段）
- ❌ 历史Ticker快照查询（P2+，ticker_snapshots 表后续实现）
- ❌ 多交易所深度聚合（P2+，需 ADR-005 多 connector）
- ❌ Ticker 报警通知系统（P2+）
- ❌ 深度数据导出（P2+）
- ❌ 自选币组合管理（P1，独立 PRD）

---

## 3. 用户角色与权限

| 角色 | Ticker 权限 | 深度权限 | 说明 |
|------|-----------|---------|------|
| **trader** (普通用户) | 查看所有交易对 Ticker | 查看深度数据（默认10档） | 默认角色 |
| **pro-trader** (专业交易员) | 同 trader | 深度数据支持 50 档 | 需管理员开通 |
| **admin** (管理员) | 同 trader | 同 pro-trader | 运维/审计 |

> 注意：行情数据为公共数据，无 user_id 隔离要求。但 WebSocket 连接需 JWT 鉴权（ADR-002），限流按 user_id 计。

---

## 4. 用户故事 (User Stories)

### US-MD-01: Ticker 列表 REST 查询 (P0)

> **As a** 交易者
> **I want to** 通过 REST API 获取所有交易对的 Ticker 数据
> **So that** 我可以在页面加载时快速看到市场全貌

**验收条件：**

```gherkin
Feature: Ticker 列表 REST 查询

  Background:
    Given 用户已登录
      And Redis 中存在多个交易对的 Ticker 缓存

  Scenario: 获取所有交易对 Ticker
    When GET /api/v1/market/tickers
    Then 返回 200 状态码
      And 返回所有交易对的 Ticker 数组
      And 每条 Ticker 包含: symbol, price, change, change_percent, volume, high, low, bid, ask, timestamp
      And 响应时间 P99 < 200ms

  Scenario: 获取单个交易对 Ticker
    When GET /api/v1/market/ticker?symbol=BTCUSDT
    Then 返回 200 状态码
      And 返回 BTCUSDT 的 Ticker 数据
      And 包含 bid 和 ask 字段

  Scenario: 交易对不存在
    When GET /api/v1/market/ticker?symbol=INVALID99
    Then 返回 404 状态码
      And 提示 "交易对不存在"

  Scenario: Redis 缓存命中
    Given Redis 中存在 symbol=BTCUSDT 的 Ticker 缓存
    When GET /api/v1/market/ticker?symbol=BTCUSDT
    Then 直接从 Redis 返回数据
      And 不查询数据库

  Scenario: Redis 缓存未命中
    Given Redis 中不存在 symbol=NEWUSDT 的 Ticker 缓存
    When GET /api/v1/market/ticker?symbol=NEWUSDT
    Then 从 Collector 获取最新数据
      And 写入 Redis 缓存（TTL=5s）
      And 返回数据
```

---

### US-MD-02: 深度数据 REST 查询 (P0)

> **As a** 交易者
> **I want to** 通过 REST API 获取指定交易对的深度数据
> **So that** 我可以分析买卖盘口和流动性

**验收条件：**

```gherkin
Feature: 深度数据 REST 查询

  Background:
    Given 用户已登录
      And Redis 中存在 symbol=BTCUSDT 的深度缓存

  Scenario: 获取默认档位深度
    When GET /api/v1/market/depth?symbol=BTCUSDT
    Then 返回 200 状态码
      And 返回 10 档买盘和 10 档卖盘
      And 每档包含: price (价格), quantity (数量), total (累计量)
      And 买盘按价格降序排列
      And 卖盘按价格升序排列
      And 包含 timestamp 字段
      And 响应时间 P99 < 100ms

  Scenario: 指定档位数量
    When GET /api/v1/market/depth?symbol=BTCUSDT&levels=20
    Then 返回 20 档买盘和 20 档卖盘

  Scenario: pro-trader 请求 50 档
    Given 当前用户角色为 pro-trader
    When GET /api/v1/market/depth?symbol=BTCUSDT&levels=50
    Then 返回 50 档买盘和 50 档卖盘

  Scenario: trader 请求超过 20 档
    Given 当前用户角色为 trader
    When GET /api/v1/market/depth?symbol=BTCUSDT&levels=50
    Then 返回 403 状态码
      And 提示 "当前角色仅支持 20 档深度，升级至 pro-trader 可查看 50 档"

  Scenario: 无效档位参数
    When GET /api/v1/market/depth?symbol=BTCUSDT&levels=0
    Then 返回 400 状态码
      And 提示 "levels 参数必须在 5/10/20/50 中选择"

  Scenario: Redis 缓存命中
    Given Redis 中存在 depth:BTCUSDT 缓存
    When GET /api/v1/market/depth?symbol=BTCUSDT
    Then 直接从 Redis 返回数据
```

---

### US-MD-03: WebSocket Ticker 实时推送 (P0)

> **As a** 交易者
> **I want to** 通过 WebSocket 实时接收 Ticker 数据
> **So that** 我可以实时监控价格变化

**验收条件：**

```gherkin
Feature: WebSocket Ticker 实时推送

  Background:
    Given 用户已通过 JWT 鉴权建立 WebSocket 连接
      And 连接地址为 wss://host/api/v1/ws?token=***

  Scenario: 订阅 Ticker 频道
    When 客户端发送 {"action":"subscribe","channels":["ticker:BTCUSDT"]}
    Then 服务端返回 {"type":"subscribed","channel":"ticker:BTCUSDT"}
      And 后续 BTCUSDT 的 Ticker 更新通过 WS 推送

  Scenario: Ticker 推送格式
    Given 已订阅 ticker:BTCUSDT
    When BTCUSDT 价格发生变化
    Then 服务端推送:
      """
      {
        "type": "ticker",
        "symbol": "BTCUSDT",
        "data": {
          "price": 50500.0,
          "change": 150.0,
          "change_percent": 0.30,
          "volume": 123456.7,
          "high": 51000.0,
          "low": 49000.0,
          "bid": 50499.0,
          "ask": 50501.0,
          "timestamp": 1715500000000
        },
        "ts": 1715500000000
      }
      """
      And 推送延迟 < 500ms

  Scenario: 取消订阅
    Given 已订阅 ticker:BTCUSDT
    When 客户端发送 {"action":"unsubscribe","channels":["ticker:BTCUSDT"]}
    Then 服务端返回 {"type":"unsubscribed","channel":"ticker:BTCUSDT"}
      And 不再推送 BTCUSDT Ticker 数据

  Scenario: 订阅多个交易对
    When 客户端发送 {"action":"subscribe","channels":["ticker:BTCUSDT","ticker:ETHUSDT"]}
    Then 服务端返回订阅确认
      And 两个交易对的 Ticker 更新均推送

  Scenario: 推送频率限制
    Given 已订阅 ticker:BTCUSDT
    When 1 秒内 BTCUSDT Ticker 更新 10 次
    Then 服务端最多推送 4 次（250ms 合并窗口）
      And 每次推送包含最新状态

  Scenario: 未鉴权连接
    When WebSocket 连接不携带 token 参数
    Then 连接被拒绝
      And 返回 401 错误
```

---

### US-MD-04: WebSocket 深度数据实时推送 (P0)

> **As a** 交易者
> **I want to** 通过 WebSocket 实时接收深度数据变化
> **So that** 我可以实时监控盘口变化

**验收条件：**

```gherkin
Feature: WebSocket 深度数据实时推送

  Background:
    Given 用户已通过 JWT 鉴权建立 WebSocket 连接

  Scenario: 订阅深度频道
    When 客户端发送 {"action":"subscribe","channels":["depth:BTCUSDT"]}
    Then 服务端返回 {"type":"subscribed","channel":"depth:BTCUSDT"}
      And 立即推送当前完整深度快照
      And 后续深度变化增量推送

  Scenario: 首次推送完整快照
    Given 已订阅 depth:BTCUSDT
    When 订阅成功后
    Then 服务端推送完整深度数据:
      """
      {
        "type": "depth",
        "symbol": "BTCUSDT",
        "data": {
          "bids": [[50499.0, 1.5], [50498.0, 2.3], ...],
          "asks": [[50501.0, 1.2], [50502.0, 1.8], ...],
          "timestamp": 1715500000000
        },
        "ts": 1715500000000
      }
      """

  Scenario: 增量深度更新
    Given 已订阅 depth:BTCUSDT 且已收到完整快照
    When 盘口发生变化
    Then 服务端推送增量更新:
      """
      {
        "type": "depth_update",
        "symbol": "BTCUSDT",
        "data": {
          "bids": [[50499.0, 1.8]],
          "asks": [[50503.0, 0.5]],
          "timestamp": 1715500000001
        },
        "ts": 1715500000001
      }
      """
      And 客户端本地合并增量到快照

  Scenario: 深度推送频率限制
    Given 已订阅 depth:BTCUSDT
    When 100ms 内盘口变化 20 次
    Then 服务端最多推送 2 次（100ms 合并窗口）
      And 每次推送包含合并后的最新增量

  Scenario: 取消订阅深度
    Given 已订阅 depth:BTCUSDT
    When 客户端发送 {"action":"unsubscribe","channels":["depth:BTCUSDT"]}
    Then 服务端返回取消确认
      And 停止推送
```

---

### US-MD-05: Ticker 面板前端展示 (P0)

> **As a** 交易者
> **I want to** 在行情页面看到所有交易对的 Ticker 概览
> **So that** 我可以快速掌握市场概况

**验收条件：**

```gherkin
Feature: Ticker 面板前端展示

  Background:
    Given 用户已登录并进入 /market 页面

  Scenario: Ticker 列表加载
    When 页面首次加载
    Then 调用 GET /api/v1/market/tickers 获取初始数据
      And 显示 Ticker 列表表格
      And 每行包含: 交易对、最新价、24h涨跌幅、24h最高/最低、24h成交量
      And 涨跌幅为正显示绿色，为负显示红色

  Scenario: Ticker 实时更新
    Given 页面已加载
    When 建立 WebSocket 连接并订阅所有已展示的 Ticker 频道
    Then 价格变化时实时更新表格数据
      And 无需刷新页面

  Scenario: 价格闪烁效果
    Given Ticker 面板已打开
    When 最新价上涨
    Then 价格数字绿色闪烁
      And 0.5s 后恢复正常颜色
    When 最新价下跌
    Then 价格数字红色闪烁
      And 0.5s 后恢复正常颜色

  Scenario: 按涨跌幅排序
    When 用户点击"24h涨跌幅"列头
    Then 列表按涨跌幅降序排列
    When 再次点击
    Then 列表按涨跌幅升序排列

  Scenario: 搜索交易对
    When 用户在搜索框输入 "BTC"
    Then 列表仅显示包含 "BTC" 的交易对
      And 实时过滤（debounce 300ms）

  Scenario: WebSocket 断线重连
    When WebSocket 连接断开
    Then 前端自动重连（指数退避 1s→2s→4s→...→30s）
      And 重连期间显示"连接中断"提示
      And 重连成功后重新订阅所有频道
      And Ticker 数据恢复实时更新
```

---

### US-MD-06: 深度盘口前端展示 (P0)

> **As a** 交易者
> **I want to** 在行情页面查看深度盘口和深度图
> **So that** 我可以判断买卖压力和流动性

**验收条件：**

```gherkin
Feature: 深度盘口前端展示

  Background:
    Given 用户已登录并进入 /market 页面
      And 已选择交易对 "BTCUSDT"

  Scenario: 加载深度数据
    When 点击"深度"标签
    Then 调用 GET /api/v1/market/depth?symbol=BTCUSDT 获取初始数据
      And 显示买盘列表（右侧/绿色）：买一至买十
      And 显示卖盘列表（左侧/红色）：卖一至卖十
      And 每行显示: 价格、数量、累计量
      And 累计量从最优价开始累加

  Scenario: 深度实时更新
    Given 深度面板已打开
    When 建立 WebSocket 连接并订阅 depth:BTCUSDT
    Then 盘口变化时实时更新
      And 深度数据平滑过渡（无闪烁/跳动）

  Scenario: 深度图可视化
    Given 深度数据已加载
    Then 显示深度面积图（ECharts）
      And 横轴为价格
      And 纵轴为累计量
      And 买盘为绿色面积，卖盘为红色面积
      And 中间为最新成交价分隔线

  Scenario: 档位切换
    When 用户选择档位下拉框切换为 "20档"
    Then 重新请求 GET /api/v1/market/depth?symbol=BTCUSDT&levels=20
      And 更新盘口和深度图

  Scenario: 深度图悬浮交互
    When 鼠标悬浮在深度图上
    Then 显示 tooltip: 价格、累计量、买卖方向
      And 对应价格行高亮

  Scenario: 盘口数据精度
    Given BTCUSDT 最小价格变动为 0.01
    Then 盘口价格显示精度为 2 位小数
      And 数量显示精度为 3 位小数
      And 累计量显示精度为 3 位小数
```

---

### US-MD-07: 心跳与连接管理 (P1)

> **As a** 系统
> **I want to** 维护 WebSocket 连接健康状态
> **So that** 及时发现断线并自动恢复

**验收条件：**

```gherkin
Feature: WebSocket 心跳与连接管理

  Scenario: 服务端心跳
    Given WebSocket 连接已建立
    When 每 15 秒
    Then 服务端发送 {"type":"heartbeat","ts":1715500000000}

  Scenario: 客户端超时断开
    Given WebSocket 连接已建立
    When 服务端 30 秒未收到客户端任何消息（含 pong）
    Then 服务端主动断开连接
      And 释放资源

  Scenario: 客户端心跳响应
    Given WebSocket 连接已建立
    When 客户端收到 heartbeat 消息
    Then 客户端发送 {"type":"pong"} 响应

  Scenario: 连接数限制
    Given 单个用户
    When 同一用户建立超过 5 个 WebSocket 连接
    Then 最早建立的连接被服务端断开
      And 返回 {"type":"kick","reason":"max_connections_exceeded"}
```

---

### US-MD-08: Ticker 快照持久化 (P1)

> **As a** 系统
> **I want to** 定期将 Ticker 数据持久化到数据库
> **So that** 支持历史 Ticker 数据查询和分析

**验收条件：**

```gherkin
Feature: Ticker 快照持久化

  Scenario: 定时快照写入
    Given 系统 Collector 正在运行
    When 每 1 分钟
    Then 系统将所有交易对的 Ticker 快照写入 ticker_snapshots 表
      And 包含: symbol, price, change, change_percent, volume, high, low, bid, ask, timestamp

  Scenario: 快照数据保留
    Given ticker_snapshots 表数据
    When 数据超过 90 天
    Then 自动删除过期数据

  Scenario: 历史快照查询
    When GET /api/v1/market/ticker/history?symbol=BTCUSDT&start=2024-01-01&end=2024-01-31
    Then 返回该时间范围内的 Ticker 快照
      And 支持分页
```

---

## 5. 数据模型

### 5.1 新增 ticker_snapshots 表

```sql
-- Ticker 快照表（定期采集，支持历史查询）
CREATE TABLE ticker_snapshots (
    id          BIGSERIAL       PRIMARY KEY,
    symbol      VARCHAR(50)     NOT NULL,
    price       DECIMAL(20,8)   NOT NULL,
    change      DECIMAL(20,8)   NOT NULL DEFAULT 0,
    change_percent DECIMAL(10,4) NOT NULL DEFAULT 0,
    volume      DECIMAL(20,8)   NOT NULL DEFAULT 0,
    high        DECIMAL(20,8)   NOT NULL DEFAULT 0,
    low         DECIMAL(20,8)   NOT NULL DEFAULT 0,
    bid         DECIMAL(20,8)   NOT NULL DEFAULT 0,
    ask         DECIMAL(20,8)   NOT NULL DEFAULT 0,
    timestamp   BIGINT          NOT NULL,
    created_at  TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- 索引
CREATE INDEX idx_ticker_snap_symbol_ts ON ticker_snapshots(symbol, timestamp DESC);
CREATE INDEX idx_ticker_snap_created_at ON ticker_snapshots(created_at DESC);

-- 分区（按月，自动管理）
-- CREATE TABLE ticker_snapshots_2026_01 PARTITION OF ticker_snapshots
--   FOR VALUES FROM ('2026-01-01') TO ('2026-02-01');
```

### 5.2 Redis 数据结构扩展

在 data-model.md 已有设计基础上补充：

| Key | 类型 | TTL | 说明 |
|-----|------|-----|------|
| `ticker:{symbol}` | HASH | 5s | 最新 Ticker（字段: price/change/change_percent/volume/high/low/bid/ask/timestamp） |
| `depth:{symbol}` | STRING(JSON) | 1s | 最新深度快照（完整 bids/asks 数组） |
| `ws:sessions:{user_id}` | SET | — | 用户 WebSocket 连接 ID（已有，用于连接数限制） |
| `ws:subs:{conn_id}` | SET | 连接有效 | 单连接订阅频道列表 |

### 5.3 前端类型扩展

```typescript
// 扩展 Ticker 接口（新增 bid/ask/timestamp）
export interface Ticker {
  symbol: string
  price: number
  change: number
  change_percent: number
  volume: number
  high: number
  low: number
  bid: number        // 新增：买一价
  ask: number        // 新增：卖一价
  timestamp: number  // 新增：数据时间戳
}

// 扩展 Depth 接口（新增累计量）
export interface DepthLevel {
  price: number
  quantity: number
  total: number      // 累计量
}

export interface Depth {
  bids: DepthLevel[]
  asks: DepthLevel[]
  timestamp: number
}

// WebSocket 消息类型
export interface WsMessage {
  type: 'ticker' | 'depth' | 'depth_update' | 'heartbeat' | 'subscribed' | 'unsubscribed' | 'pong' | 'kick'
  symbol?: string
  data?: any
  ts?: number
}

export interface WsSubscribe {
  action: 'subscribe' | 'unsubscribe'
  channels: string[]  // e.g. ['ticker:BTCUSDT', 'depth:ETHUSDT']
}
```

---

## 6. API 端点设计

| 方法 | 路径 | 描述 | 优先级 |
|------|------|------|--------|
| GET | /api/v1/market/tickers | 获取所有交易对 Ticker | P0 |
| GET | /api/v1/market/ticker | 获取单个交易对 Ticker | P0 |
| GET | /api/v1/market/depth | 获取深度数据 | P0 |
| GET | /api/v1/market/ticker/history | Ticker 历史快照查询 | P1 |
| WS | /api/v1/ws | WebSocket 实时推送 | P0 |

### 6.1 API 详细设计

#### GET /api/v1/market/tickers

**查询参数：** 无

**响应：**
```json
{
  "code": 0,
  "data": [
    {
      "symbol": "BTCUSDT",
      "price": 50500.0,
      "change": 150.0,
      "change_percent": 0.30,
      "volume": 123456.7,
      "high": 51000.0,
      "low": 49000.0,
      "bid": 50499.0,
      "ask": 50501.0,
      "timestamp": 1715500000000
    }
  ],
  "message": "success"
}
```

#### GET /api/v1/market/ticker

**查询参数：**
- `symbol` (必填): 交易对

**响应：**
```json
{
  "code": 0,
  "data": {
    "symbol": "BTCUSDT",
    "price": 50500.0,
    "change": 150.0,
    "change_percent": 0.30,
    "volume": 123456.7,
    "high": 51000.0,
    "low": 49000.0,
    "bid": 50499.0,
    "ask": 50501.0,
    "timestamp": 1715500000000
  },
  "message": "success"
}
```

#### GET /api/v1/market/depth

**查询参数：**
- `symbol` (必填): 交易对
- `levels` (可选, 默认10, 可选 5/10/20/50): 档位数量

**响应：**
```json
{
  "code": 0,
  "data": {
    "bids": [
      {"price": 50499.0, "quantity": 1.5, "total": 1.5},
      {"price": 50498.0, "quantity": 2.3, "total": 3.8}
    ],
    "asks": [
      {"price": 50501.0, "quantity": 1.2, "total": 1.2},
      {"price": 50502.0, "quantity": 1.8, "total": 3.0}
    ],
    "timestamp": 1715500000000
  },
  "message": "success"
}
```

#### GET /api/v1/market/ticker/history

**查询参数：**
- `symbol` (必填): 交易对
- `start` (必填): 起始时间戳（毫秒）
- `end` (必填): 结束时间戳（毫秒）
- `page` (可选, 默认1): 页码
- `page_size` (可选, 默认100, 最大1000): 每页条数

**响应：**
```json
{
  "code": 0,
  "data": [
    {
      "symbol": "BTCUSDT",
      "price": 50500.0,
      "change": 150.0,
      "change_percent": 0.30,
      "volume": 123456.7,
      "high": 51000.0,
      "low": 49000.0,
      "bid": 50499.0,
      "ask": 50501.0,
      "timestamp": 1715500000000
    }
  ],
  "meta": {
    "total": 43200,
    "page": 1,
    "page_size": 100
  },
  "message": "success"
}
```

---

## 7. 前端页面设计

### 7.1 页面路由

| 路径 | 组件 | 说明 |
|------|------|------|
| /market | MarketView.vue | 行情首页（重构占位页） |
| /market/ticker | TickerListView.vue | Ticker 列表面板 |
| /market/depth | DepthView.vue | 深度盘口面板 |

### 7.2 页面布局

```
/market 页面布局（Element Plus Tabs）
├── Tab1: Ticker 概览
│   ├── 搜索栏: 交易对搜索（el-input + debounce 300ms）
│   ├── Ticker 列表表格 (el-table)
│   │   ├── 列: 交易对 | 最新价 | 24h涨跌幅 | 24h最高 | 24h最低 | 24h成交量
│   │   ├── 排序: 点击列头排序（涨跌幅默认降序）
│   │   └── 闪烁: 价格变化时 CSS transition + 0.5s 背景色
│   └── 连接状态指示器: 🟢 已连接 / 🔴 连接中断
│
├── Tab2: 深度盘口
│   ├── 交易对选择器: el-select
│   ├── 档位选择器: el-radio-group (5/10/20/50)
│   ├── 盘口列表 (左右分栏)
│   │   ├── 左侧: 卖盘（红色，价格升序）
│   │   ├── 中间: 最新成交价 + 涨跌幅
│   │   └── 右侧: 买盘（绿色，价格降序）
│   └── 深度图 (ECharts 面积图)
│       ├── X轴: 价格
│       ├── Y轴: 累计量
│       └── 交互: 悬浮 tooltip + 行高亮
│
└── Tab3: K线图表 (已有 PRD-kline-management)
```

### 7.3 组件清单

| 组件 | 说明 |
|------|------|
| `MarketView.vue` | 主容器，Tab 导航（重构占位页） |
| `TickerListView.vue` | Ticker 列表面板 |
| `TickerTable.vue` | Ticker 表格（排序、闪烁） |
| `TickerSearchBar.vue` | 交易对搜索框 |
| `ConnectionStatus.vue` | WebSocket 连接状态指示器 |
| `DepthView.vue` | 深度面板主容器 |
| `OrderBookTable.vue` | 买卖盘口表格 |
| `DepthChart.vue` | 深度面积图（ECharts） |
| `DepthLevelSelector.vue` | 档位选择器 |

### 7.4 Composables

| Composable | 说明 |
|-----------|------|
| `useMarketWs.ts` | WebSocket 连接管理（连接/重连/心跳/订阅） |
| `useTicker.ts` | Ticker 数据管理（REST + WS 更新合并） |
| `useDepth.ts` | 深度数据管理（快照 + 增量合并） |
| `usePriceFlash.ts` | 价格闪烁动画（CSS class toggle + setTimeout 500ms） |

---

## 8. 后端设计

### 8.1 模块结构

```
handlers/
  └── market.rs           # REST API handler (tickers/ticker/depth/ticker-history)

services/
  └── market_data.rs      # 行情数据服务
      - get_tickers()          # 从 Redis 获取所有 Ticker
      - get_ticker(symbol)     # 从 Redis 获取单个 Ticker
      - get_depth(symbol, levels)  # 从 Redis 获取深度
      - get_ticker_history(symbol, start, end) # 从 DB 获取历史
      - snapshot_tickers()     # 定时快照写入 DB

handlers/
  └── ws.rs               # WebSocket handler（扩展现有）
      - ws_handler()           # JWT 鉴权 + 升级
      - handle_socket()        # 订阅/推送循环
      - subscribe_channels()   # 频道订阅管理
      - unsubscribe_channels() # 频道取消
      - broadcast_ticker()     # Ticker 推送
      - broadcast_depth()      # 深度推送

db/
  └── ticker_snapshot.rs  # SeaORM 模型
      - create_snapshot()
      - query_snapshots()
```

### 8.2 WebSocket Hub 设计

扩展现有 `WsManager`，实现频道订阅/推送：

```rust
// handlers/ws.rs 扩展
pub struct WsManager {
    pub tx: broadcast::Sender<String>,          // 全局广播
    pub subscriptions: DashMap<String, HashSet<String>>,  // conn_id -> channels
    pub user_connections: DashMap<String, Vec<String>>,   // user_id -> [conn_ids]
}

// 订阅协议
// Client → Server: {"action":"subscribe","channels":["ticker:BTCUSDT"]}
// Server → Client: {"type":"subscribed","channel":"ticker:BTCUSDT"}

// 推送协议
// Server → Client: {"type":"ticker","symbol":"BTCUSDT","data":{...},"ts":...}
// Server → Client: {"type":"depth","symbol":"BTCUSDT","data":{...},"ts":...}
// Server → Client: {"type":"depth_update","symbol":"BTCUSDT","data":{...},"ts":...}
```

### 8.3 推送频率控制

| 频道类型 | 合并窗口 | 最大推送频率 | 说明 |
|---------|---------|-------------|------|
| ticker:{symbol} | 250ms | 4次/s | 合并窗口内多次更新仅推送最新 |
| depth:{symbol} | 100ms | 10次/s | 盘口变化频繁，需更高频率 |
| heartbeat | 15s | — | 固定频率 |

### 8.4 Collector 数据流集成

Collector Daemon（ADR-005）负责从交易所获取数据，写入 Redis 并通过 Pub/Sub 通知 WS Hub：

```
[Exchange WS] → [Collector: 解析+规范化]
    ↓
    ├→ Redis HSET ticker:BTCUSDT (price/change/...)
    ├→ Redis SET depth:BTCUSDT (JSON string)
    └→ Redis PUBLISH market:ticker:BTCUSDT / market:depth:BTCUSDT
         ↓
    [WS Hub: 订阅 Redis Pub/Sub]
         ↓
    [合并窗口 → 推送到已订阅客户端]
```

---

## 9. 边界情况

| 场景 | 处理方式 |
|------|---------|
| WebSocket 连接断开 | 客户端指数退避重连（1s→2s→4s→...→30s），重连后重新订阅 |
| Redis 不可用 | Ticker/Depth REST 降级为从 Collector 内部缓存返回；WS 推送暂停，显示"数据延迟" |
| Collector 宕机 | Ticker/Depth 数据停留在 Redis 缓存的最后状态（TTL 过期后返回 503） |
| 交易所 WS 断开 | ADR-005 降级策略：退避重连 + REST 拉取补充 |
| 交易所 API 限流 (429) | 使用 Redis 缓存数据，前端显示"数据延迟"提示 |
| 单用户超过 5 个 WS 连接 | 踢出最早连接，返回 kick 消息 |
| 深度数据为空（新交易对） | REST 返回空 bids/asks 数组，WS 不推送 |
| Ticker 价格异常（偏离过大） | 标记 `stale: true`，前端显示"数据待确认" |
| 大量客户端同时订阅同一频道 | 使用 broadcast channel，O(1) 推送（非 per-client 循环） |
| 深度增量合并时客户端快照过旧 | 服务端检测 seq_id 差距过大，重发完整快照 |
| 并发 REST 请求同一 symbol | Redis 缓存天然去重，无并发问题 |
| ticker_snapshots 写入失败 | 记录日志，不影响实时推送功能 |

---

## 10. 非功能性需求

| 需求 | 指标 |
|------|------|
| Ticker REST 延迟 | P99 < 200ms |
| Depth REST 延迟 | P99 < 100ms |
| WS 推送延迟 | Exchange→Client < 500ms |
| 并发 WS 连接 | ≥ 1000 |
| 单用户 WS 连接上限 | 5 |
| 推送频率（Ticker） | ≤ 4次/s/频道 |
| 推送频率（Depth） | ≤ 10次/s/频道 |
| Redis 缓存命中率 | Ticker ≥ 95%, Depth ≥ 90% |
| 断线重连时间 | ≤ 5s |
| Ticker 快照持久化 | 每 1 分钟一次 |
| 快照数据保留 | 90 天 |
| 心跳间隔 | 服务端 15s，客户端超时 30s |
| 可观测性 | WS 连接数、推送频率、Redis 命中率接入 Prometheus |

---

## 11. 优先级矩阵

| User Story | 功能 | P0 | P1 | P2 |
|------------|------|----|----|-----|
| US-MD-01 | Ticker 列表 REST 查询 | ✅ | | |
| US-MD-02 | 深度数据 REST 查询 | ✅ | | |
| US-MD-03 | WebSocket Ticker 推送 | ✅ | | |
| US-MD-04 | WebSocket 深度推送 | ✅ | | |
| US-MD-05 | Ticker 面板前端展示 | ✅ | | |
| US-MD-06 | 深度盘口前端展示 | ✅ | | |
| US-MD-07 | 心跳与连接管理 | | ✅ | |
| US-MD-08 | Ticker 快照持久化 | | ✅ | |

**P0 关键路径：**
`US-MD-01` (Ticker REST) + `US-MD-02` (Depth REST) → 前端可展示基础数据
`US-MD-03` (WS Ticker) + `US-MD-04` (WS Depth) → 前端可实时更新
`US-MD-05` + `US-MD-06` → 用户可交互

**依赖关系：**
- US-MD-03/04 依赖 ADR-005 Collector 提供数据源
- US-MD-05/06 依赖 US-MD-01/02（初始加载）和 US-MD-03/04（实时更新）
- US-MD-08 依赖 US-MD-01（数据来源）

---

## 12. 实施建议（分阶段）

### 阶段一（当前 PRD 核心，P0 功能）：

1. **后端 REST API**
   - 创建 `handlers/market.rs`，实现 Ticker 和 Depth REST 端点
   - 实现 `services/market_data.rs`，从 Redis 读取 Ticker/Depth 缓存
   - Redis 缓存未命中时降级为 Collector 内部 HashMap

2. **WebSocket 订阅/推送**
   - 扩展 `handlers/ws.rs`，实现频道订阅管理
   - 实现 WsHub 订阅 Redis Pub/Sub 频道
   - 实现合并窗口（Ticker 250ms, Depth 100ms）

3. **前端 Ticker 面板**
   - 重构 `MarketView.vue`，实现 Tab 导航
   - 实现 `TickerListView.vue` + `TickerTable.vue`（排序、闪烁）
   - 实现 `useMarketWs.ts` composable（连接、重连、订阅）

4. **前端深度面板**
   - 实现 `DepthView.vue` + `OrderBookTable.vue`
   - 实现 `DepthChart.vue`（ECharts 面积图）
   - 实现 `useDepth.ts` composable（快照+增量合并）

### 阶段二（P1 功能）：

1. 心跳与连接管理（US-MD-07）
2. Ticker 快照持久化 + 历史查询（US-MD-08）
3. 连接数限制与限流

### 阶段三（P2 功能）：

1. 历史深度数据存储
2. 多交易所深度聚合
3. Ticker 报警通知
