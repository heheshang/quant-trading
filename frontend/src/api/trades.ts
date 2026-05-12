import client from './client'
import type { Order, Position, Portfolio, Trade } from '@/types'

export function listOrders(): Promise<Order[]> {
  return client.get('/trades/orders')
}

export function createOrder(data: Partial<Order>): Promise<Order> {
  return client.post('/trades/orders', data)
}

export function cancelOrder(id: number): Promise<void> {
  return client.post(`/trades/orders/${id}/cancel`)
}

export function listPositions(): Promise<Position[]> {
  return client.get('/trades/positions')
}

export function closePosition(id: number): Promise<void> {
  return client.post(`/trades/positions/${id}/close`)
}

export function getPortfolio(): Promise<Portfolio> {
  return client.get('/trades/portfolio')
}

export function listTrades(): Promise<Trade[]> {
  return client.get('/trades/history')
}
