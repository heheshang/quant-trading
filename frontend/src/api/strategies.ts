import client from './client'
import type { Strategy, BacktestParams, BacktestResult } from '@/types'

export function listStrategies(): Promise<Strategy[]> {
  return client.get('/strategies')
}

export function getStrategy(id: number): Promise<Strategy> {
  return client.get(`/strategies/${id}`)
}

export function createStrategy(data: Partial<Strategy>): Promise<Strategy> {
  return client.post('/strategies', data)
}

export function updateStrategy(id: number, data: Partial<Strategy>): Promise<Strategy> {
  return client.put(`/strategies/${id}`, data)
}

export function deleteStrategy(id: number): Promise<void> {
  return client.delete(`/strategies/${id}`)
}

export function runBacktest(params: BacktestParams): Promise<BacktestResult> {
  return client.post('/strategies/backtest', params)
}

export function getBacktestResult(id: number): Promise<BacktestResult> {
  return client.get(`/strategies/backtest/${id}`)
}
