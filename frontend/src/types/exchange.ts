// Types for Exchange API (P0-F1 API Signing)
// API Contract: backend/src/handlers/exchange.rs
// Snake_case (Rust) → camelCase (TypeScript)

export interface ApiResponse<T> {
  code: number
  data: T
  msg: string
}

// ─── Response Types ─────────────────────────────────────────────

export interface ExchangePingResponse {
  serverTime: number   // i64 → number
  status: string
}

export interface ExchangeBalance {
  asset: string
  free: string
  locked: string
}

export interface ExchangeAccountResponse {
  balances: ExchangeBalance[]
}

export interface ExchangeFill {
  price: string
  qty: string
  commission: string
}

export interface ExchangeOrderResponse {
  orderId: string      // order_id → orderId
  symbol: string
  side: string
  orderType: string    // order_type → orderType
  status: string
  executedQty: string  // executed_qty → executedQty
  fills: ExchangeFill[]
}

export interface ExchangeCancelResponse {
  orderId: string
  status: string
}

export interface BinanceLimitInfo {
  rateLimitType: string  // rate_limit_type → rateLimitType
  interval: string
  intervalNum: number     // i32 → number
  limit: number          // i64 → number
  count: number           // i64 → number
}

export interface ExchangeRateLimitResponse {
  userId: string
  localRemaining: number  // u64 → number
  localResetAtMs: number  // i64 → number
  binanceLimit: BinanceLimitInfo
}

// ─── Request Types ─────────────────────────────────────────────

export interface CreateExchangeOrderRequest {
  symbol: string
  side: 'BUY' | 'SELL'
  type: 'LIMIT' | 'MARKET' | 'STOP_LOSS' | 'STOP_LOSS_LIMIT' | 'TAKE_PROFIT' | 'TAKE_PROFIT_LIMIT' | 'LIMIT_MAKER'
  quantity?: string
  price?: string
  timeInForce?: 'GTC' | 'IOC' | 'FOK'
}

export interface CancelExchangeOrderRequest {
  symbol: string
}
