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
} from '@/types/order'

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

/** Close position: POST /api/v1/positions/:id/close */
export function closePosition(id: number): Promise<void> {
  return client.post(`/positions/${id}/close`)
}
