# T3 技术设计评审 Checklist — 交易前端 WebSocket

> 功能名称：TradingFront-WS
> 日期：2026-05-18
> 评审人：Tech-Lead
> 状态：✅ 通过

---

## 交付物检查

### ✅ ADR-011-TradingFront-WS.md
- D1 WS 连接方案（复用 /ws，不新建路由）✅
- D2 订阅协议（action:subscribe/channels）✅
- D3 前端 ws.ts 封装（自动重连）✅
- D4 行情数据流（HubMessage → 广播）✅
- D5 范围外说明（TradeWsHub）✅
- ADR 格式完整（背景/决策/影响/参考）✅

### ✅ Design_TradingFront_WS.md
- 现状分析（已实现 + 缺口清单）✅
- WS 架构图（连接流程）✅
- WS 消息格式（subscribe/unsubscribe/ticker/depth）✅
- 后端变更：handlers/ws.rs subscribe 解析 ✅
- 前端 ws.ts 封装 + Pinia Store 方案 ✅
- TradingView.vue 改造（Before/After）✅
- API 数据流图 ✅
- 验收标准（5 项）✅
- 文件变更清单 ✅

### ✅ 前端组件现状
- `api/order.ts` ✅
- `api/trades.ts` ✅
- `types/order.ts` ✅
- `TradingView.vue` ✅（wsTimer 模拟待替换）
- `OrderForm.vue` / `OrderList.vue` ✅
- `PositionPanel.vue` / `TradeRecordTab.vue` ✅

### ✅ 后端组件现状
- `handlers/ws.rs` ✅（已有 ticker/depth/kline）
- `services/exchange/ws_hub.rs` ✅（HubMessage 已定义）
- `handlers/order.rs` ✅（完整 CRUD）

---

## 契约审查（B1-B4）

### B1: 前端 → 后端 API
| 端点 | 方法 | request | response |
|------|------|---------|----------|
| `/ws` | WS | subscribe JSON | HubMessage JSON |
| `/api/v1/orders` | POST | CreateOrderRequest | Order |
| `/api/v1/orders/:id/cancel` | POST | - | CancelOrderResponse |
| `/api/v1/orders/cancel-all` | POST | {symbol?, side?} | CancelAllResponse |

### B2: 后端 → 前端 WS
| channel | data fields |
|---------|------------|
| `market:ticker` | price, bid, ask, change, changePercent, volume, high, low |
| `market:depth` | bids[], asks[] |

### B3: 类型对齐
- `CreateOrderRequest` (frontend) ↔ `CreateOrderRequest` (backend handlers/order.rs) ✅
- `Order` ↔ `OrderResponse` ✅
- `OrderStatus` ↔ `OrderStatus` (sea_orm enum) ✅

### B4: 技术约束
- WebSocket 重连：指数退避（max 5 次）
- WS 消息延迟目标 < 100ms
- API P99 < 200ms（order/cancel）

---

## T3 结论

**✅ 通过** — 技术设计完整，契约清晰，可进入 T4 开发。

### T4 开发任务（分 P0/P1）

**P0（必须）：**
1. `frontend/src/api/ws.ts` — WebSocket 封装类
2. `frontend/src/stores/trading.ts` — Pinia store
3. `backend/src/handlers/ws.rs` — subscribe 消息解析
4. `TradingView.vue` — 替换 wsTimer 为真实 WS

**P1（如时间允许）：**
5. WS 连接状态 UI（断线重连提示）
6. Depth 深度图集成到 KlineChart