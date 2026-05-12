# ADR-002: WebSocket 实现实时行情推送

| 字段 | 值 |
|------|------|
| **ID** | ADR-002 |
| **状态** | 已批准 |
| **日期** | 2026-05-12 |
| **决策者** | Tech Lead |
| **影响范围** | 实时通信、前端架构、行情模块、网络层 |

## 背景

行情数据（K线、Ticker、深度）需要在 <500ms 内推送到所有连接的客户端。PRD 要求：
- K线实时更新（每 1 分钟闭盘推送新线）
- Ticker 价格闪烁效果
- 深度盘口平滑更新
- 支持 ≥ 1000 同时在线连接
- 网络断线后自动重连并补发增量数据

## 技术选项

### 选项 A: WebSocket (tokio-tungstenite) ✅ 选定

**工作原理：**
- 客户端建立持久 TCP 连接
- 服务端推送增量数据（仅发送变更，不发送全量）
- 断线重连 + 指数退避（1s→2s→4s→8s→max 30s）

**优势：**
- 双向通信，延迟最低（毫秒级）
- 标准协议，浏览器原生支持 WebSocket API
- 每个客户端独立连接，可单独控制推送频率
- Rust tokio-tungstenite 成熟稳定，与 Axum 深度集成
- 支持通过 nginx 代理升级，兼容现有网关层

**劣势：**
- 长连接占用服务器资源（但 tokio 基于 epoll，10k 连接无压力）
- 无内置自动重连逻辑，需客户端实现
- 无内建消息确认机制

### 选项 B: Server-Sent Events (SSE)

**优势：** 简单（HTTP 长连接），浏览器原生 EventSource API，自动重连
**劣势：** 单向通信（仅服务→客户端），不支持客户端发消息，并发连接数受 HTTP/1.1 限制（每个域 6-8 个）

### 选项 C: 短轮询 (HTTP Polling)

**优势：** 实现最简单，无长连接
**劣势：** 延迟高（无法 <500ms），浪费带宽（大量空响应），服务器负载高

### 选项 D: Socket.IO

**优势：** 自动降级（WebSocket→轮询），自动重连，事件命名空间
**劣势：** 额外协议开销，服务端需 Node.js 生态，与 Rust 集成复杂

## 架构设计

```
┌─────────────┐     WS(/api/v1/market/ws)     ┌──────────────────┐
│  Vue 3 SPA  │←────────────────────────────│ Rust Axum WS Hub │
│  WS Client  │    token query param         │ (tokio broadcast)│
└─────────────┘                              └────────┬─────────┘
                                                       │
                                              ┌────────▼─────────┐
                                              │  Market Data      │
                                              │  Collector Daemon │
                                              │  (es exchange)    │
                                              └──────────────────┘
```

**连接协议：** `wss://host/api/v1/market/ws?token=<jwt_access_token>`

**消息格式：**

```json
// 服务端推送
{
  "type": "kline" | "ticker" | "depth" | "strategy_status" | "heartbeat",
  "symbol": "BTC/USDT",
  "data": { /* 实际数据 */ },
  "ts": 1715500000000
}

// 客户端订阅
{
  "action": "subscribe" | "unsubscribe",
  "channels": ["kline:BTC/USDT:1m", "ticker:ETH/USDT"]
}
```

## 决策

采用 **WebSocket (tokio-tungstenite + Axum)** 实现实时行情推送。

1. 连接鉴权：通过 WebSocket query param 传递 JWT token
2. 客户端订阅模式：客户端发送 subscribe/unsubscribe 消息控制频道
3. 服务端 broadcast：每个频道独立 broadcast 通道
4. 心跳：服务端每 15s 发送 heartbeat，客户端 30s 无响应则断开
5. 断线重连：客户端指数退避 1s→2s→4s→...→30s
6. 增量补发：重连成功后，服务端补发断线期间的 K 线缓存

## 预期后果

**正面：**
- <500ms 延迟满足 PRD 要求
- tokio 异步模型可支撑 10k+ 并发连接
- 带宽利用最优（只推送变化数据）
- 客户端订阅模式灵活扩展

**负面：**
- 客户端需实现重连逻辑（已纳入 PRD 边界情况）
- WebSocket 调试不如 REST 直观
- 需要一个额外的 Redis Pub/Sub 层来跨实例广播

## 关联决策

- ADR-005（市场数据管道）
- PRD 6.1 网络异常处理
