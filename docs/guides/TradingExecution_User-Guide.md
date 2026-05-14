# 交易执行使用指南

> 本文档介绍交易执行模块的完整操作流程，包括下单、委托管理、撤单、持仓管理以及 WebSocket 实时推送的使用方法。

---

## 目录

- [概述](#概述)
- [核心概念](#核心概念)
- [下单流程](#下单流程)
- [委托管理](#委托管理)
- [撤单操作](#撤单操作)
- [持仓管理](#持仓管理)
- [WebSocket 实时推送](#websocket-实时推送)
- [风控校验说明](#风控校验说明)
- [前端集成指南](#前端集成指南)
- [常见问题](#常见问题)

---

## 概述

交易执行模块负责量化交易系统的核心交易功能，通过本模块，用户可以：

- 提交限价单和市价单委托
- 管理委托状态（查看、筛选、撤销）
- 查看持仓信息和浮动盈亏
- 通过 WebSocket 接收实时委托状态变更和成交通知

---

## 核心概念

### 委托类型

| 类型 | 说明 | 成交方式 |
|------|------|---------|
| 限价单 (`limit`) | 指定价格的委托 | 挂单等待，当市场价格达到指定价格时撮合成交 |
| 市价单 (`market`) | 按市场最优价成交 | 立即按当前买一/卖一价撮合，实际成交价可能有偏差 |

### 有效期

| 类型 | 说明 |
|------|------|
| `GTC` (Good Till Cancel) | 一直有效，直到手动撤销或系统过期 |
| `IOC` (Immediate Or Cancel) | 立即成交，未成交部分自动取消 |
| `FOK` (Fill Or Kill) | 全部成交或全部取消，不允许部分成交 |

### 委托状态

| 状态 | 说明 | 颜色标识 |
|------|------|---------|
| `pending` | 待成交 | 黄色 ⏱ |
| `partial_filled` | 部分成交 | 蓝色 ⟳ |
| `filled` | 已成交 | 绿色 ✓ |
| `cancelled` | 已撤销 | 灰色 ✕ |
| `expired` | 已过期 | 灰色 ⏰ |
| `rejected` | 已拒绝 | 红色 ✕ |

### 状态转换图

```
  pending ──▶ partial_filled ──▶ filled
    │                │
    │                ▼
    └───────▶ cancelled
    │
    ▼
  expired / rejected
```

### 交易模式

| 模式 | 说明 | 标识 |
|------|------|------|
| `paper` | 模拟交易，使用虚拟资金 | 🧪 蓝色标签 |
| `live` | 实盘交易，涉及真实资金 | 🔴 红色标签 |

---

## 下单流程

### 操作流程

1. **选择交易对**
   在交易页面顶部选择交易对（如 `BTC/USDT`），切换后表单自动重置。

2. **选择方向**
   点击「买入」或「卖出」切换方向：
   - 买入：显示可用余额，以卖一价作为默认价格
   - 卖出：显示可用持仓，以买一价作为默认价格

3. **选择委托类型**
   点击「限价」或「市价」切换类型：
   - 限价：需要输入委托价格
   - 市价：价格输入框隐藏，按当前市场最优价成交

4. **输入价格**（限价单）
   - 在价格输入框中填入委托价格
   - 精度不得超过交易对的 `price_precision`
   - 支持上下箭头步进调整

5. **输入数量**
   - 直接输入数量，或使用百分比滑块（25% / 50% / 75% / 100%）
   - 买入 25% = `可用余额 × 0.25 / 委托价格`（向下取整）
   - 卖出 25% = `可用持仓 × 0.25`（向下取整）

6. **确认信息**
   查看账户信息区域和预估金额：
   - 可用余额 / 可用持仓
   - 预估金额 = 价格 × 数量
   - 预估手续费
   - 合计冻结金额

7. **提交委托**
   点击「提交委托」按钮，弹出确认对话框，确认后提交。

8. **等待结果**
   - 成功：Toast 提示「委托已提交」，委托出现在当前委托列表
   - 失败：风控错误提示，显示具体错误原因

### 限价单示例

```
交易对:    BTC/USDT
方向:      买入
类型:      限价
价格:      49,500.00 USDT
数量:      0.1000 BTC
有效期:    GTC
预估金额:  ≈ 4,950.00 USDT
手续费(≈): ≈ 4.95 USDT
合计冻结:  ≈ 4,954.95 USDT
```

### 市价单示例

```
交易对:    BTC/USDT
方向:      卖出
类型:      市价
数量:      0.0500 BTC
有效期:    IOC
预估金额:  ≈ 2,500.00 USDT (以当前价估算)
```

> **注意**: 市价单将按市场最优价成交，实际成交价可能有偏差。市价单不支持设置价格。

### 价格偏离警告

当委托价格偏离当前市价超过 10% 时，系统弹出黄色警告；偏离超过 100% 时弹出强提示，需二次确认后方可提交。

---

## 委托管理

### 当前委托

交易页面右侧的「当前委托」标签页显示所有未成交和部分成交的委托。

**表格列**:

| 列 | 说明 |
|----|------|
| 时间 | 委托创建时间 `HH:mm:ss` |
| 交易对 | 如 `BTC/USDT` |
| 方向 | 买入（绿色）/ 卖出（红色） |
| 类型 | 限 / 市 |
| 价格 | 委托价格 |
| 数量 | 委托数量 |
| 已成交 | 已成交数量（部分成交时蓝色显示） |
| 状态 | 状态 Pill 标签 |
| 操作 | 撤单按钮 |

**排序**: 默认按 `created_at` 降序（最新委托在前）。

**实时更新**: 通过 WebSocket 推送实时更新，委托状态变化时行背景闪烁 300ms。

### 历史委托

「历史委托」标签页显示所有已成交、已撤销、已过期和已拒绝的委托。

**筛选器**:
- 日期范围: 日期选择器
- 状态: 多选下拉
- 交易对: 搜索输入框
- 方向: 全部 / 买入 / 卖出

**分页**: 每页 20 条，支持翻页。

**详情查看**: 点击委托行打开详情抽屉，显示完整委托信息（含成交明细）。

### 委托详情

委托详情抽屉包含以下信息:

| 字段 | 说明 |
|------|------|
| 交易对 | 如 `BTC/USDT` |
| 方向 | 买入 / 卖出 |
| 类型 | 限价 / 市价 |
| 价格 | 委托价格（市价单显示实际成交价） |
| 数量 | 委托数量 |
| 已成交 | 已成交数量 |
| 成交均价 | 加权平均成交价 |
| 手续费 | 总手续费 |
| 有效期 | GTC / IOC / FOK |
| 创建时间 | ISO 8601 格式 |
| 更新时间 | ISO 8601 格式 |
| 状态 | 当前状态 |
| 成交明细 | 每笔成交的价格、数量、手续费 |

---

## 撤单操作

### 可撤销状态

| 状态 | 是否可撤销 |
|------|------------|
| `pending` | ✅ 允许，释放全部冻结保证金 |
| `partial_filled` | ✅ 允许，撤销剩余未成交部分，释放剩余保证金 |
| `filled` | ❌ 已完全成交 |
| `cancelled` | ❌ 已撤销 |
| `expired` | ❌ 已过期 |
| `rejected` | ❌ 已拒绝 |

### 单个撤单

1. 在当前委托列表中，点击委托行右侧的「撤单」按钮
2. 弹出确认对话框
3. 确认后按钮变为 loading 状态
4. 成功：Toast 提示「委托已撤销」，保证金释放，委托移至历史
5. 失败(409)：Toast 提示「委托已成交，无法撤销」

### 全部撤单

1. 点击当前委托列表底部的「全部撤单」按钮
2. 弹出确认：「确认撤销全部 N 个未成交委托？」
3. 确认后批量撤单
4. 成功：Toast 提示「成功撤销 M 个委托」
5. 部分失败：Toast 提示「成功撤销 M 个，N 个无法撤销」

### 撤单并发冲突

在高频交易场景下，撤单请求到达时委托可能已经成交。此时系统返回 40901 错误，前端自动刷新委托列表显示最新状态。

---

## 持仓管理

### 账户汇总

持仓页面顶部显示账户汇总信息:

| 字段 | 说明 |
|------|------|
| 可用余额 | 当前可用于下单的余额 |
| 冻结余额 | 被未成交委托冻结的保证金 |
| 权益 | = 可用余额 + 冻结余额 + 未实现盈亏 |
| 初始资金 | 账户创建时的初始金额 |
| 累计盈亏 | 所有已实现盈亏的总和（盈利绿色/亏损红色） |
| 持仓数 | 当前持仓数量 |
| 活跃委托 | 当前未成交委托数量 |

### 持仓列表

| 列 | 说明 |
|----|------|
| 交易对 | 如 `BTC/USDT` |
| 方向 | 多头（绿色）/ 空头（红色） |
| 持仓数量 | 总持仓量 |
| 可用数量 | 扣除冻结后的可用数量 |
| 开仓均价 | 加权平均开仓价格 |
| 当前价 | 实时市场价格（通过 Ticker WS 更新，变化时闪烁） |
| 浮动盈亏 | = (当前价 - 开仓均价) × 数量（多头） |
| 盈亏率 | = 浮动盈亏 / (开仓均价 × 数量) × 100% |
| 操作 | 平仓按钮 |

### 平仓操作

1. **全部平仓**: 生成市价反向单，数量等于持仓数量
2. **部分平仓**: 弹出数量输入对话框，输入后生成市价反向单

**平仓确认对话框**:
```
确认平仓 BTC/USDT 0.3 BTC？
将以市价卖出
[取消]  [确认平仓]
```

> **注意**: 平仓使用市价单，实际成交价可能有偏差。

---

## WebSocket 实时推送

### 连接与认证

```
ws://localhost:8080/ws/trade?token=<jwt_token>
```

使用 JWT Token 作为查询参数进行认证。连接建立后需订阅频道才能接收推送。

### 频道订阅

连接后发送订阅消息:

```json
{
  "action": "subscribe",
  "channels": ["order:all", "position:all"]
}
```

| 频道 | 推送内容 | 优先级 |
|------|---------|--------|
| `order:all` | 用户所有委托状态变更 | P0 |
| `position:all` | 用户持仓变更 | P1 |

### 消息处理

| 消息类型 | 前端处理 |
|---------|---------|
| `order_update` | 更新委托列表中对应委托行；若状态变为 filled/cancelled/expired，从当前委托移至历史 |
| `trade` | 成交通知，可弹出 Toast 提示；更新成交记录 |
| `position_update` | 更新持仓列表；更新浮动盈亏 |
| `subscribed` | 记录已订阅频道 |
| `error` | 显示错误 Toast |

### 重连策略

WS 断线后自动重连:

- 指数退避: 1s → 2s → 4s → 8s → 16s → 30s（封顶）
- 重连成功后自动恢复订阅
- 后端补发断线期间的状态变更
- 前端基于 `order_id` + `status` + `updated_at` 进行幂等去重

### 降级方案

WS 不可用时，前端自动降级为 REST 轮询:

| 数据 | 轮询接口 | 间隔 |
|------|---------|------|
| 委托列表 | `GET /api/v1/orders?status=pending&status=partial_filled` | 5s |
| 持仓列表 | `GET /api/v1/positions` | 10s |
| 账户信息 | `GET /api/v1/account` | 10s |

降级时界面显示 WS 状态指示器为灰色，并提示「交易推送断开，降级为轮询」。

---

## 风控校验说明

交易执行模块在前端和后端均实现了风控校验，确保交易安全。

### 前端实时校验

前端在用户输入时实时进行以下校验:

| 校验项 | 错误码 | 提示方式 |
|--------|--------|---------|
| 交易对不支持 | 40003 | 下拉不可选 |
| 数量无效(零/负/超精度) | 40004 | 输入框下方红色文字 |
| 价格无效(零/负/超精度) | 40005 | 输入框下方红色文字 |
| 低于最小下单量 | 40006 | 输入框下方红色文字 |
| 低于最小下单金额 | 40007 | 预估金额区域红色文字 |
| 余额不足 | 40001 | 账户信息红色 + 提交按钮禁用 |
| 持仓不足 | 40002 | 账户信息红色 + 提交按钮禁用 |
| 未成交委托达上限 | 42901 | 提交后弹出错误提示 |
| 权限不足(实盘) | 40301 | 模式选择受限 |

### 后端校验

后端在收到创建委托请求后，执行 RiskChecker 校验（8 项）。校验通过后冻结保证金并写入委托；校验失败返回对应错误码。

| HTTP 状态码 | 错误码 | 前端处理 |
|------------|--------|---------|
| 400 | 40001~40007 | 显示具体错误信息 |
| 401 | 40101 | 跳转登录页 |
| 403 | 40301 | Toast「权限不足」 |
| 404 | 40401 | Toast「委托不存在」+ 刷新列表 |
| 409 | 40901 | Toast「委托已成交，无法撤销」+ 刷新列表 |
| 429 | 42901 | Toast「未成交委托达上限」+ 引导撤单 |
| 503 | 50301 | Toast「行情数据不可用」+ 市价单禁用 |

---

## 前端集成指南

### Vue 3 + TypeScript 集成

#### 1. API 调用封装

```typescript
// api/trading.ts
import request from '@/utils/request'

export interface CreateOrderRequest {
  symbol: string
  side: 'buy' | 'sell'
  order_type: 'limit' | 'market'
  price?: string
  quantity: string
  time_in_force: 'GTC' | 'IOC' | 'FOK'
}

export interface Order {
  order_id: string
  symbol: string
  side: 'buy' | 'sell'
  order_type: 'limit' | 'market'
  price: string | null
  quantity: string
  filled_quantity: string
  avg_fill_price: string | null
  status: 'pending' | 'partial_filled' | 'filled' | 'cancelled' | 'expired' | 'rejected'
  mode: 'paper' | 'live'
  fee: string
  time_in_force: 'GTC' | 'IOC' | 'FOK'
  created_at: string
  updated_at: string
}

// 创建委托
export function createOrder(data: CreateOrderRequest) {
  return request.post<Order>('/api/v1/orders', data)
}

// 委托列表
export function listOrders(params: {
  page?: number
  size?: number
  status?: string
  symbol?: string
  side?: string
}) {
  return request.get<{ items: Order[]; total: number }>('/api/v1/orders', { params })
}

// 委托详情
export function getOrder(orderId: string) {
  return request.get<Order>(`/api/v1/orders/${orderId}`)
}

// 撤单
export function cancelOrder(orderId: string) {
  return request.delete(`/api/v1/orders/${orderId}`)
}
```

#### 2. WebSocket Composable

```typescript
// composables/useTradeWs.ts
import { ref, onUnmounted } from 'vue'

interface WsMessage {
  type: 'order_update' | 'trade' | 'position_update' | 'subscribed' | 'error'
  data?: any
  ts?: number
}

export function useTradeWs(token: string) {
  const status = ref<'connected' | 'connecting' | 'reconnecting' | 'disconnected'>('disconnected')
  const lastMessage = ref<WsMessage | null>(null)

  let ws: WebSocket | null = null
  let reconnectTimer: number | null = null
  let retryDelay = 1000

  function connect() {
    status.value = 'connecting'
    ws = new WebSocket(`ws://localhost:8080/ws/trade?token=${token}`)

    ws.onopen = () => {
      status.value = 'connected'
      retryDelay = 1000
      // 订阅频道
      ws?.send(JSON.stringify({
        action: 'subscribe',
        channels: ['order:all', 'position:all']
      }))
    }

    ws.onmessage = (event) => {
      const msg: WsMessage = JSON.parse(event.data)
      lastMessage.value = msg
    }

    ws.onclose = () => {
      status.value = 'disconnected'
      scheduleReconnect()
    }

    ws.onerror = () => {
      ws?.close()
    }
  }

  function scheduleReconnect() {
    status.value = 'reconnecting'
    reconnectTimer = window.setTimeout(() => {
      retryDelay = Math.min(retryDelay * 2, 30000)
      connect()
    }, retryDelay)
  }

  function disconnect() {
    if (reconnectTimer) clearTimeout(reconnectTimer)
    ws?.close()
    ws = null
    status.value = 'disconnected'
  }

  onUnmounted(() => disconnect())

  return { status, lastMessage, connect, disconnect }
}
```

#### 3. RBAC 权限

| 角色 | 权限 |
|------|------|
| `viewer` | 查看委托和持仓，不可下单/撤单 |
| `trader` | 模拟交易完整权限 |
| `trader:live` | 模拟 + 实盘交易权限 |

---

## 常见问题

### Q: 为什么提交委托时提示「账户余额不足」？

**A**: 买入委托需要冻结保证金（委托金额 + 预估手续费）。检查以下可能原因:

1. 可用余额不足以支付委托金额
   - 解决: 减少委托数量或先撤销其他委托释放保证金

2. 冻结余额占用过多
   - 解决: 撤销不需要的未成交委托，释放冻结保证金

3. 委托价格偏离较大
   - 解决: 调整委托价格至合理范围

### Q: 市价单的成交价为什么和预期不同？

**A**: 市价单按市场最优价成交，成交价取决于下单时的市场深度。如果市场深度较薄，大额市价单可能产生滑点。建议:

1. 大额委托使用限价单，控制成交价格
2. 关注深度盘口，评估市场流动性
3. 使用 IOC 有效期，避免挂单等待

### Q: 撤单时提示「委托已成交，无法撤销」怎么办？

**A**: 这通常发生在并发场景下——委托在撤单请求到达前已经成交。这是正常的竞态条件，系统已自动处理。刷新委托列表查看最新状态即可。

### Q: WebSocket 连接频繁断开怎么办？

**A**: 检查以下可能原因:

1. JWT Token 过期 — 重新登录获取新 Token
2. 网络不稳定 — 系统会自动重连（指数退避），无需手动处理
3. 长时间无活动 — 后端可能断开空闲连接，客户端会自动重连

系统在 WS 断开时自动降级为 REST 轮询，不会遗漏委托状态更新。

### Q: 部分成交的委托可以撤销吗？

**A**: 可以。部分成交的委托撤销后，已成交部分不受影响，仅撤销剩余未成交数量，并释放对应的冻结保证金。

### Q: GTC 委托会一直有效吗？

**A**: GTC (Good Till Cancel) 委托在以下情况下会结束:

1. 完全成交 — 状态变为 `filled`
2. 手动撤销 — 状态变为 `cancelled`
3. 系统过期 — 系统可能对长期未成交的委托设置过期时间，状态变为 `expired`

### Q: 如何区分模拟交易和实盘交易？

**A**: 交易页面顶部有交易模式标识:

- 🧪 蓝色标签 = 模拟交易 (`paper`)
- 🔴 红色标签 = 实盘交易 (`live`)

实盘交易涉及真实资金，系统会在确认对话框中显示红色警告。请确保在正确的模式下操作。

### Q: 未成交委托数量有上限吗？

**A**: 有。每个用户最多同时持有 50 个未成交委托（`pending` + `partial_filled`）。达到上限后提交新委托会返回 42901 错误。请及时撤销不需要的委托。
