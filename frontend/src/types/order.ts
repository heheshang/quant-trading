/** Order side (ADR D6) */
export type OrderSide = 'buy' | 'sell'

/** Order type (ADR D6/D9: MVP supports limit + market; stop/stop_limit reserved) */
export type OrderType = 'limit' | 'market' | 'stop' | 'stop_limit'

/** Order status - ADR aligned (D2: PG row lock state machine) */
export type OrderStatus = 'pending' | 'partial_filled' | 'filled' | 'cancelled' | 'expired' | 'rejected'

/** Trade mode */
export type TradeMode = 'paper' | 'live'

/** Time in force */
export type TimeInForce = 'GTC' | 'IOC' | 'FOK'

/** Order response from API (ADR D6: /api/v1/orders, field names: order_id, order_type, price as string|null) */
export interface Order {
  order_id: string
  symbol: string
  side: OrderSide
  order_type: OrderType
  price: string | null
  stop_price: string | null
  quantity: string
  filled_quantity: string
  avg_fill_price: string | null
  status: OrderStatus
  mode: TradeMode
  fee: string
  reject_reason: string | null
  time_in_force: TimeInForce
  strategy_id: string | null
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

/** Paper account (GET /api/v1/account) */
export interface PaperAccount {
  user_id: string
  balance: string
  frozen_balance: string
  initial_balance: string
  total_pnl: string
  equity: string
  positions_count: number
  active_orders_count: number
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

/** Trade WS message types (ADR D5: independent TradeWsHub) */
export type TradeWsMessageType = 'order_update' | 'trade' | 'position_update' | 'subscribed' | 'error'

export interface TradeWsMessage {
  type: TradeWsMessageType
  data: Record<string, any>
  ts: number
}

/** Position (ADR D7: weighted average avg_entry_price) */
export interface Position {
  id: string
  user_id: string
  symbol: string
  side: 'long' | 'short'
  quantity: string
  available_quantity: string
  avg_entry_price: string
  unrealized_pnl: string
  realized_pnl: string
  mode: TradeMode
  created_at: string
  updated_at: string
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

/** Trade record from API (ADR D4: /api/v1/trades) */
export interface Trade {
  trade_id: string
  order_id: string
  symbol: string
  side: OrderSide
  price: string
  quantity: string
  fee: string
  is_maker: boolean
  created_at: string
}

/** Trade query params (GET /api/v1/trades) */
export interface TradeQueryParams {
  symbol?: string
  side?: OrderSide
  start_date?: string
  end_date?: string
  page?: number
  size?: number
}

/** Paginated trade list response */
export interface TradeListResponse {
  items: Trade[]
  total: number
  page: number
  size: number
}

/** Order side display text */
export function getOrderSideText(side: OrderSide): string {
  return side === 'buy' ? '买入' : '卖出'
}
