export interface DashboardStats {
  total_pnl: number
  daily_pnl: number
  win_rate: number
  sharpe_ratio: number
  total_trades: number
  active_positions: number
  balance: number
}

export interface PnLPoint {
  timestamp: string
  value: number
}
