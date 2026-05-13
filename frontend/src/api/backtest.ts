import client from './client'
import type {
  BacktestParams,
  BacktestRunRequest,
  BacktestResultResponse,
  BacktestSummary,
  PaginatedResponse,
} from '@/types/backtest'

/**
 * Submit a backtest run.
 * Transforms flat BacktestParams to nested BacktestRunRequest format.
 * strategy_params is omitted — backend gets strategy config from the strategy model.
 */
export function runBacktest(params: BacktestParams): Promise<BacktestResultResponse> {
  const { strategy_id, strategy_params, ...config } = params
  const requestBody: BacktestRunRequest = {
    strategy_id,
    config,
  }
  return client.post('/backtest', requestBody)
}

/**
 * Get backtest result by ID.
 * Also used for polling — the response includes a `status` field.
 */
export function getBacktestResult(id: string): Promise<BacktestResultResponse> {
  return client.get(`/backtest/${id}`)
}

/**
 * Cancel a running backtest.
 */
export function cancelBacktest(id: string): Promise<void> {
  return client.post(`/backtest/${id}/cancel`)
}

/**
 * List backtest history with pagination.
 * Returns PaginatedResponse<BacktestSummary>.
 */
export function listBacktestHistory(params?: {
  strategy_id?: string
  page?: number
  size?: number
}): Promise<PaginatedResponse<BacktestSummary>> {
  return client.get('/backtest/history', { params })
}

/**
 * Delete a backtest result.
 * ID is a UUID string.
 */
export function deleteBacktestResult(id: string): Promise<void> {
  return client.delete(`/backtest/${id}`)
}
