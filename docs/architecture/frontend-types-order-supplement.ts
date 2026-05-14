// frontend/src/types/order.ts — 补充缺失类型修复
//
// 需合并到 /home/ssk/workspace/quant-trading/frontend/src/types/order.ts
// 对照: ADR-010, Contract-Review-Order B2 (CRITICAL fixes)

// ═══════════════════════════════════════════════════
// CRITICAL 修复: UUID 类型修正 (B2 审查 3个 CRITICAL)
// ═══════════════════════════════════════════════════
//
// 以下类型定义中 user_id / id 字段类型为 `number`，
// 但后端返回 UUID 字符串，必须修正为 `string`

/** Paper account (GET /api/v1/account)
 *  FIX: user_id 从 number 改为 string (UUID)
 */
export interface PaperAccount {
  user_id: string          // FIX: was `number`, backend returns UUID string
  balance: string
  frozen_balance: string
  initial_balance: string
  total_pnl: string
  equity: string
  positions_count: number
  active_orders_count: number
}

/** Position (ADR D7: weighted average avg_entry_price)
 *  FIX: id / user_id 从 number 改为 string (UUID)
 */
export interface Position {
  id: string               // FIX: was `number`, backend returns UUID string
  user_id: string          // FIX: was `number`, backend returns UUID string
  symbol: string
  side: 'long' | 'short'
  quantity: string
  available_quantity: string
  avg_entry_price: string
  unrealized_pnl: string
  realized_pnl: string
  mode: 'paper' | 'live'
  created_at: string
  updated_at: string
}

// ═══════════════════════════════════════════════════
// MEDIUM: 补充 P1 扩展类型 (B1/B4 审查)
// ═══════════════════════════════════════════════════

/** Extended OrderType for P1 stop orders */
export type OrderTypeExtended = 'limit' | 'market' | 'stop' | 'stop_limit'

/** Extended CreateOrderRequest with P1 fields
 *  PRD §6.2: stop_price, strategy_id, mode
 */
export interface CreateOrderRequestExtended {
  symbol: string
  side: 'buy' | 'sell'
  order_type: OrderTypeExtended
  price?: string
  quantity: string
  stop_price?: string       // P1: 止损单触发价
  strategy_id?: string      // P1: 策略信号关联
  mode?: 'paper' | 'live'   // P1: 交易模式 (默认 paper)
  time_in_force?: 'GTC' | 'IOC' | 'FOK'
}

/** Extended Order with P1 fields */
export interface OrderExtended extends Order {
  strategy_id: string | null   // P1: 策略关联
  expire_at: string | null     // P1: 过期时间
}

/** Risk rule (P1)
 *  PRD: US-TE-06
 *  ADR: D8
 */
export interface RiskRule {
  id: string
  rule_type: 'max_position' | 'max_daily_loss' | 'max_single_loss' | 'max_position_concentration'
  value: string
  enabled: boolean
  scope: 'global' | 'user'
}

// ═══════════════════════════════════════════════════
// MEDIUM: 修正 WS 消息类型 (B4 审查)
// ═══════════════════════════════════════════════════
//
// 原有 TradeWsMessageType 中 'trade' 与 PRD 推送字段 'fill' 不一致
// 统一为 PRD 定义

/** Trade WS message types (aligned with PRD §6.3) */
export type TradeWsMessageType = 'order_update' | 'fill' | 'position_update' | 'risk_alert' | 'subscribed' | 'error'
// FIX: was 'trade', now 'fill' to match PRD

/** WS fill message data */
export interface WsFillData {
  trade_id: string
  order_id: string
  symbol: string
  side: 'buy' | 'sell'
  price: string
  amount: string
  fee: string
  is_maker: boolean
}

/** WS order update data */
export interface WsOrderUpdateData {
  order_id: string
  symbol: string
  side: 'buy' | 'sell'
  status: 'pending' | 'partial_filled' | 'filled' | 'cancelled' | 'expired' | 'rejected'
  price: string
  amount: string
  filled: string
  updated_at: string
}

/** WS position update data */
export interface WsPositionUpdateData {
  symbol: string
  quantity: string
  avg_price: string
  unrealized_pnl: string
}

/** WS risk alert data */
export interface WsRiskAlertData {
  level: 'warning' | 'blocked'
  rule: string
  message: string
  action: 'notify' | 'block'
}

/** Typed WS message */
export interface TypedTradeWsMessage {
  type: TradeWsMessageType
  data: WsOrderUpdateData | WsFillData | WsPositionUpdateData | WsRiskAlertData | Record<string, unknown>
  ts: number
}

// ═══════════════════════════════════════════════════
// 补充: 交易 WS composable 接口 (PRD §7.5)
// ═══════════════════════════════════════════════════
//
// 需新建: frontend/src/composables/useTradeWs.ts
//
// ```typescript
// import { ref, onUnmounted } from 'vue'
// import type { Ref } from 'vue'
// import type {
//   TradeWsMessageType,
//   TypedTradeWsMessage,
//   WsOrderUpdateData,
//   WsFillData,
//   WsPositionUpdateData,
//   WsRiskAlertData,
// } from '@/types/order'
//
// type ConnectionStatus = 'connecting' | 'connected' | 'disconnected' | 'reconnecting'
//
// export function useTradeWs() {
//   const status: Ref<ConnectionStatus> = ref('disconnected')
//   let ws: WebSocket | null = null
//   let reconnectTimer: ReturnType<typeof setTimeout> | null = null
//   let retryDelay = 1000
//   const MAX_RETRY_DELAY = 30000
//   const callbacks = new Map<string, Set<(data: any) => void>>()
//
//   function connect(token: string) {
//     const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
//     const host = window.location.host
//     ws = new WebSocket(`${protocol}//${host}/api/v1/trade/ws?token=${token}`)
//     status.value = 'connecting'
//
//     ws.onopen = () => {
//       status.value = 'connected'
//       retryDelay = 1000 // reset
//     }
//
//     ws.onmessage = (event) => {
//       const msg: TypedTradeWsMessage = JSON.parse(event.data)
//       const cbs = callbacks.get(msg.type)
//       if (cbs) cbs.forEach(cb => cb(msg.data))
//     }
//
//     ws.onclose = () => {
//       status.value = 'reconnecting'
//       reconnectTimer = setTimeout(() => {
//         retryDelay = Math.min(retryDelay * 2, MAX_RETRY_DELAY)
//         connect(token)
//       }, retryDelay)
//     }
//   }
//
//   function subscribe(channels: TradeWsMessageType[]) {
//     if (ws?.readyState === WebSocket.OPEN) {
//       ws.send(JSON.stringify({ action: 'subscribe', channels }))
//     }
//   }
//
//   function onOrderUpdate(cb: (data: WsOrderUpdateData) => void) {
//     addCallback('order_update', cb)
//   }
//   function onFill(cb: (data: WsFillData) => void) {
//     addCallback('fill', cb)
//   }
//   function onPositionUpdate(cb: (data: WsPositionUpdateData) => void) {
//     addCallback('position_update', cb)
//   }
//   function onRiskAlert(cb: (data: WsRiskAlertData) => void) {
//     addCallback('risk_alert', cb)
//   }
//
//   function addCallback(type: string, cb: Function) {
//     if (!callbacks.has(type)) callbacks.set(type, new Set())
//     callbacks.get(type)!.add(cb as any)
//   }
//
//   function disconnect() {
//     if (reconnectTimer) clearTimeout(reconnectTimer)
//     ws?.close()
//     ws = null
//     status.value = 'disconnected'
//   }
//
//   onUnmounted(disconnect)
//
//   return {
//     status,
//     connect,
//     disconnect,
//     subscribe,
//     onOrderUpdate,
//     onFill,
//     onPositionUpdate,
//     onRiskAlert,
//   }
// }
// ```

// ═══════════════════════════════════════════════════
// 保留以下原有类型 (无需修改)
// ═══════════════════════════════════════════════════

/** Order side (ADR D6) */
export type OrderSide = 'buy' | 'sell'

/** Order type (ADR D6) */
export type OrderType = 'limit' | 'market'

/** Order status - ADR aligned (D2: PG row lock state machine) */
export type OrderStatus = 'pending' | 'partial_filled' | 'filled' | 'cancelled' | 'expired' | 'rejected'

/** Trade mode */
export type TradeMode = 'paper' | 'live'

/** Time in force */
export type TimeInForce = 'GTC' | 'IOC' | 'FOK'

/** Order response from API (ADR D6) */
export interface Order {
  order_id: string
  symbol: string
  side: OrderSide
  order_type: OrderType
  price: string | null
  quantity: string
  filled_quantity: string
  avg_fill_price: string | null
  status: OrderStatus
  mode: TradeMode
  fee: string
  reject_reason: string | null
  time_in_force: TimeInForce
  created_at: string
  updated_at: string
  cancelled_at: string | null
  filled_at: string | null
}

/** Create order request (ADR D6: POST /api/v1/orders) */
export interface CreateOrderRequest {
  symbol: string
  side: OrderSide
  order_type: OrderType
  price?: string
  quantity: string
  time_in_force?: TimeInForce
}

/** Order query params (GET /api/v1/orders) */
export interface OrderQueryParams {
  status?: OrderStatus | 'active'
  symbol?: string
  side?: OrderSide
  start_date?: string
  end_date?: string
  page?: number
  size?: number
}

/** Paginated order list response */
export interface OrderListResponse {
  items: Order[]
  total: number
  page: number
  size: number
}

/** Cancel order response */
export interface CancelOrderResponse {
  order_id: string
  status: OrderStatus
  filled_quantity: string
  released_amount: string
}

/** Cancel all orders response */
export interface CancelAllResponse {
  cancelled_count: number
  failed_count: number
  failed_orders: Array<{ order_id: string; reason: string }>
}

/** Symbol config (GET /api/v1/symbols) */
export interface SymbolConfig {
  symbol: string
  base_currency: string
  quote_currency: string
  price_precision: number
  quantity_precision: number
  min_quantity: string
  max_quantity: string
  min_notional: string
  fee_rate: string
  enabled: boolean
}

/** Trade WS message — deprecated, use TypedTradeWsMessage */
export type TradeWsMessageType = 'order_update' | 'trade' | 'position_update' | 'subscribed' | 'error'
export interface TradeWsMessage {
  type: TradeWsMessageType
  data: Record<string, any>
  ts: number
}

/** Order status tag type mapping for Element Plus */
export function getOrderStatusType(status: OrderStatus): 'warning' | 'primary' | 'success' | 'info' | 'danger' {
  const map: Record<OrderStatus, 'warning' | 'primary' | 'success' | 'info' | 'danger'> = {
    pending: 'warning',
    partial_filled: 'primary',
    filled: 'success',
    cancelled: 'info',
    expired: 'info',
    rejected: 'danger',
  }
  return map[status]
}

/** Order status display text */
export function getOrderStatusText(status: OrderStatus): string {
  const map: Record<OrderStatus, string> = {
    pending: '待成交',
    partial_filled: '部分成交',
    filled: '已成交',
    cancelled: '已撤销',
    expired: '已过期',
    rejected: '已拒绝',
  }
  return map[status]
}

/** Order side display text */
export function getOrderSideText(side: OrderSide): string {
  return side === 'buy' ? '买入' : '卖出'
}
