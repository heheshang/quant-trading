# ADR: 交易前端 WebSocket 补全

> ADR-ID: ADR-011  
> 标题：交易前端 WebSocket 实时行情接入  
> 状态：Draft  
> 日期：2026-05-18  
> 决策者：Tech-Lead  
> 关联：PRD-trade-execution.md, ADR-010 (order management), ADR-005 (market data pipeline)

---

## 背景

交易前端 `TradingView.vue` 当前使用 `setInterval` 模拟价格行情（wsTimer），需替换为真实 WebSocket 连接，订阅后端实时行情推送。

- 现有 `/ws` 端点已实现（`handlers/ws.rs`），支持 `market:ticker` / `market:depth` / `market:kline` channel
- 缺口：`TradingView.vue` 的 wsTimer 模拟未连接真实 WS
- ADR-010 D5（TradeWsHub）尚未实现，本次仅接入现有 market data WS

---

## 决策

### D1: WS 连接方案
- 复用现有 `/ws` 端点（`handlers/ws.rs`），不新建独立 `/ws/trade` 路由
- 前端按需订阅 `market:ticker` channel 获取 bestBid/bestAsk
- 连接参数：?symbol=BTCUSDT&channels=ticker,depth,kline

### D2: 订阅协议
- 客户端连接后发送 JSON 订阅消息：`{"action":"subscribe","channels":["market:ticker","market:depth"],"symbol":"BTCUSDT"}`
- 服务端解析并注册 channel 过滤器，只推送相关消息
- 已有 HubMessage 支持 ticker/depth/kline，按 symbol 路由即可

### D3: 前端 WS 封装
- 新建 `frontend/src/api/ws.ts` WebSocket 封装类
- 自动重连（指数退避，最多 5 次）
- 连接状态通过 Pinia store 共享

### D4: 行情数据流
```
后端 WS Hub (HubMessage::Ticker/Depth)
  → /ws 端点广播
  → 前端 WebSocket 连接
  → ws.ts 解析 channel + data
  → TradingView reactive 更新 bestBid/bestAsk
```

### D5: 不实现（本次范围外）
- TradeWsHub（ADR-010 D5）— 交易成交推送，独立 ticket
- 策略信号触发下单自动化 — PRD-strategy-sandbox-mvp.md 覆盖

---

## 影响

| 组件 | 变更 |
|------|------|
| `frontend/src/api/ws.ts` | 新建 WebSocket 封装 |
| `frontend/src/stores/trading.ts` | 新建或扩展 Pinia store |
| `TradingView.vue` | 替换 wsTimer 为真实 WS |
| `handlers/ws.rs` | 解析 subscribe 消息并过滤 channel |
| `services/exchange/ws_hub.rs` | 已有 ticker/depth/kline，无需修改 |

---

## 参考

- ADR-005 Market Data Pipeline
- ADR-010 Order Management (D5: TradeWsHub — 范围外)
- `handlers/ws.rs` 现有实现
- `services/exchange/ws_hub.rs` HubMessage 类型定义