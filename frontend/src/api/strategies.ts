import client from './client'
import type {
  StrategyFull,
  StrategyTemplate,
  CreateStrategyPayload,
  UpdateStrategyPayload,
} from '@/types'

export function listStrategies(params?: { status?: string; page?: number; size?: number }): Promise<StrategyFull[]> {
  return client.get('/strategies', { params })
}

export function getStrategy(id: number): Promise<StrategyFull> {
  return client.get(`/strategies/${id}`)
}

export function createStrategy(data: CreateStrategyPayload): Promise<StrategyFull> {
  return client.post('/strategies', data)
}

export function updateStrategy(id: number, data: UpdateStrategyPayload): Promise<StrategyFull> {
  return client.put(`/strategies/${id}`, data)
}

export function deleteStrategy(id: number): Promise<void> {
  return client.delete(`/strategies/${id}`)
}

export function listTemplates(): Promise<StrategyTemplate[]> {
  return client.get('/strategies/templates')
}

export function toggleStrategy(id: number, status: 'active' | 'paused'): Promise<StrategyFull> {
  return client.post(`/strategies/${id}/status`, { status })
}
