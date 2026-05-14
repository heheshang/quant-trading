// frontend/src/api/order.ts — 补充缺失的 API 函数
//
// 需合并到 /home/ssk/workspace/quant-trading/frontend/src/api/order.ts
// 对照: ADR-010 D6, PRD §6.1-6.2, Contract-Review-Order B2

import client from './client'
import type {
  Order,
  CreateOrderRequest,
  OrderQueryParams,
  OrderListResponse,
  CancelOrderResponse,
  CancelAllResponse,
  PaperAccount,
  SymbolConfig,
  Position,
  RiskRule,
} from '@/types/order'

// ═══════════════════════════════════════════════════
// 现有函数 (已实现，无需修改)
// ═══════════════════════════════════════════════════

/** Create order: POST /api/v1/orders */
export function createOrder(data: CreateOrderRequest): Promise<Order> {
  return client.post('/orders', data)
}

/** Get orders: GET /api/v1/orders */
export function getOrders(params?: OrderQueryParams): Promise<OrderListResponse> {
  return client.get('/orders', { params })
}

/** Get order detail: GET /api/v1/orders/:id */
export function getOrder(id: string): Promise<Order> {
  return client.get(`/orders/${id}`)
}

/** Cancel order: POST /api/v1/orders/:id/cancel */
export function cancelOrder(id: string): Promise<CancelOrderResponse> {
  return client.post(`/orders/${id}/cancel`)
}

/** Cancel all orders: POST /api/v1/orders/cancel-all */
export function cancelAllOrders(params?: { symbol?: string; side?: string }): Promise<CancelAllResponse> {
  return client.post('/orders/cancel-all', params)
}

/** Get paper account: GET /api/v1/account */
export function getAccount(): Promise<PaperAccount> {
  return client.get('/account')
}

/** Get symbol configs: GET /api/v1/symbols */
export function getSymbols(): Promise<{ items: SymbolConfig[] }> {
  return client.get('/symbols')
}

/** Get positions: GET /api/v1/positions */
export function getPositions(): Promise<Position[]> {
  return client.get('/positions')
}

// ═══════════════════════════════════════════════════
// 新增函数 (缺失，需补充)
// ═══════════════════════════════════════════════════

/** Close position: POST /api/v1/positions/:symbol/close
 *  PRD: US-TE-05 (平仓操作)
 *  ADR: D6 (路由), D3 (保证金解冻)
 *
 *  注意: 原有 closePosition(id: number) 传的是数字 ID，
 *  PRD/ADR 定义路径参数为 symbol (交易对)，已修正
 */
export function closePosition(
  symbol: string,
  data?: { quantity?: string; type?: 'market' | 'limit' }
): Promise<Order> {
  return client.post(`/positions/${symbol}/close`, data)
}

/** Init paper account: POST /api/v1/account/init
 *  PRD: US-TE-09 (模拟账户初始化)
 *  P1 优先级
 */
export function initAccount(): Promise<PaperAccount> {
  return client.post('/account/init')
}

/** Get risk rules: GET /api/v1/trade/risk/rules
 *  PRD: US-TE-06 (风控规则可配置)
 *  ADR: D8 (风控前置)
 *  P1 优先级
 */
export function getRiskRules(): Promise<{ items: RiskRule[] }> {
  return client.get('/trade/risk/rules')
}

/** Update risk rule: PUT /api/v1/trade/risk/rules/:id
 *  PRD: US-TE-06 (风控规则可配置)
 *  ADR: D8 (风控前置)
 *  P1 优先级, admin only
 */
export function updateRiskRule(
  id: string,
  data: { value?: string; enabled?: boolean }
): Promise<RiskRule> {
  return client.put(`/trade/risk/rules/${id}`, data)
}

/** Get trades (fills): GET /api/v1/trades
 *  PRD: US-TE-04 (成交记录)
 *  注意: 原 trades.ts 已废弃，统一使用 order.ts
 */
export function getTrades(params?: {
  symbol?: string
  side?: 'buy' | 'sell'
  start_date?: string
  end_date?: string
  page?: number
  size?: number
}): Promise<{
  items: Array<{
    trade_id: string
    order_id: string
    symbol: string
    side: 'buy' | 'sell'
    price: string
    quantity: string
    fee: string
    is_maker: boolean
    created_at: string
  }>
  total: number
  page: number
  size: number
}> {
  return client.get('/trades', { params })
}
