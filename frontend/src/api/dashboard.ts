import client from './client'
import type { DashboardStats, PnLPoint } from '@/types'

export function getDashboardStats(): Promise<DashboardStats> {
  return client.get('/dashboard/stats')
}

export function getPnLHistory(range?: string): Promise<PnLPoint[]> {
  return client.get('/dashboard/pnl', { params: { range } })
}
