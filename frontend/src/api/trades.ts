/** @deprecated Use ./order.ts for ADR-aligned API routes (/api/v1/orders) */
import client from './client'
import type { Order, Position, Portfolio, Trade } from '@/types'

/** @deprecated Use getOrders from ./order */
export function listOrders(): Promise<Order[]> {
  return client.get('/orders')
}

/** @deprecated Use createOrder from ./order */
export function createOrder(data: Partial<Order>): Promise<Order> {
  return client.post('/orders', data)
}

/** @deprecated Use cancelOrder from ./order */
export function cancelOrder(id: number): Promise<void> {
  return client.post(`/orders/${id}/cancel`)
}

/** @deprecated Use getPositions from ./order */
export function listPositions(): Promise<Position[]> {
  return client.get('/positions')
}

/** @deprecated Use closePosition from ./order */
export function closePosition(id: number): Promise<void> {
  return client.post(`/positions/${id}/close`)
}

/** @deprecated Use getAccount from ./order */
export function getPortfolio(): Promise<Portfolio> {
  return client.get('/account')
}

/** @deprecated Use getOrders with status=filled from ./order */
export function listTrades(): Promise<Trade[]> {
  return client.get('/trades')
}
