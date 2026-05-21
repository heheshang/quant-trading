// ===== Backtest Types — Aligned with Backend Schema =====

/** Backtest run status */
export type BacktestStatus = 'pending' | 'running' | 'completed' | 'failed'

/** Backend: BacktestRunRequest — POST /backtest body (nested structure) */
export interface BacktestRunRequest {
  strategy_id: string
  config: {
    symbol: string
    interval: string
    start_date: string
    end_date: string
    initial_capital: number
    fee_rate: number        // [0, 0.01] as decimal
    slippage_rate: number   // [0, 0.01] as decimal
  }
}

/** Backend: EquityPoint */
export interface EquityPoint {
  time: number             // i64 millisecond timestamp
  equity: number
  drawdown_pct: number
}

/** Backend: TradeRecord */
export interface TradeRecord {
  direction: 'long' | 'short'
  entry_time: string
  exit_time: string
  entry_price: number
  exit_price: number
  quantity: number
  pnl_usdt: number
  pnl_pct: number
  fee: number
  slippage: number
  exit_reason: 'take_profit' | 'stop_loss' | 'signal'
  holding_period_ms: number
}

/** Backend: BacktestResultResponse — GET /backtest/{id} response */
export interface BacktestResultResponse {
  id: string               // UUID string
  strategy_id: string
  status: BacktestStatus
  progress: number         // 0-100
  error: string | null
  config: {
    symbol: string
    interval: string
    start_date: string
    end_date: string
    initial_capital: number
    fee_rate: number
    slippage_rate: number
  }
  metrics: {
    total_return_pct: number
    annualized_return_pct: number
    sharpe_ratio: number
    sortino_ratio: number
    max_drawdown_pct: number
    calmar_ratio: number
    win_rate: number
    profit_factor: number | null   // null when INFINITY
    avg_win_pct: number
    avg_loss_pct: number
    total_trades: number
    total_fees: number
    total_slippage: number
    duration_ms: number
  }
  equity_curve: EquityPoint[]
  trades: TradeRecord[]
  created_at: string
}

/** @deprecated Use BacktestResultResponse */
export type BacktestResultDetail = BacktestResultResponse

/** Backend: BacktestSummary (item in paginated list) */
export interface BacktestSummary {
  id: string               // UUID string
  strategy_id: string
  symbol: string
  status: string
  total_return_pct: number
  sharpe_ratio: number
  created_at: string
}

/** Backend: PaginatedResponse<T> */
export interface PaginatedResponse<T> {
  items: T[]
  total: number
  page: number
  size: number
}

/** @deprecated Use BacktestSummary */
export type BacktestHistoryItem = BacktestSummary

/**
 * Flat params emitted by BacktestConfigForm and consumed by api/backtest.ts:runBacktest().
 * The API function transforms this to nested BacktestRunRequest format.
 */
export interface BacktestParams {
  strategy_id: string
  symbol: string
  interval: string
  start_date: string
  end_date: string
  initial_capital: number
  fee_rate: number
  slippage_rate: number
  strategy_params?: Record<string, unknown>
}

/** @deprecated Use TradeRecord */
export type BacktestTrade = TradeRecord
