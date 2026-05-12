import type { Trade } from './trade'

export interface Strategy {
  id: number
  name: string
  description: string
  parameters: Record<string, any>
  status: 'active' | 'paused' | 'stopped'
  created_at: string
  updated_at: string
}

export interface BacktestParams {
  strategy_id: number
  symbol: string
  start_date: string
  end_date: string
  initial_capital: number
}

export interface BacktestResult {
  id: number
  strategy_id: number
  total_return: number
  sharpe_ratio: number
  max_drawdown: number
  total_trades: number
  win_rate: number
  trades: Trade[]
}
