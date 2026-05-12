// ===== Common Types =====

export interface User {
  id: number
  username: string
  email: string
  role: 'admin' | 'trader' | 'observer'
  status: 'active' | 'disabled'
  createdAt: string
  lastLogin: string | null
}

export interface DashboardStats {
  totalPnl: number
  winRate: number
  sharpeRatio: number
  activePositions: number
}

export interface PnLPoint {
  date: string
  value: number
}

export interface Trade {
  id: number
  symbol: string
  side: 'buy' | 'sell'
  price: number
  quantity: number
  pnl: number
  time: string
}

export interface Strategy {
  id: number
  name: string
  description: string
  pnl: number
  sharpe: number
  status: 'running' | 'paused' | 'draft' | 'archived'
}

// ===== Auth Types =====

export interface LoginPayload {
  username: string
  password: string
  rememberMe?: boolean
}

export interface RegisterPayload {
  username: string
  email: string
  password: string
  confirmPassword: string
}

export interface AuthState {
  user: User | null
  token: string | null
  isAuthenticated: boolean
  loading: boolean
  error: string | null
}

// ===== API Types =====

export interface ApiResponse<T> {
  code: number
  data: T
  message: string
}

export interface PaginationMeta {
  page: number
  size: number
  total: number
}

export interface PaginatedResponse<T> extends ApiResponse<T> {
  meta: PaginationMeta
}

// ===== Market Types =====

export interface Kline {
  timestamp: number
  open: number
  high: number
  low: number
  close: number
  volume: number
}

export interface Ticker {
  symbol: string
  price: number
  change: number
  change_percent: number
  volume: number
  high: number
  low: number
}

export interface Depth {
  bids: [number, number][]
  asks: [number, number][]
  timestamp: number
}

// ===== Strategy Types =====

export interface StrategyFull {
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

// ===== Trade Types =====

export interface Order {
  id: number
  symbol: string
  side: 'buy' | 'sell'
  type: 'limit' | 'market' | 'stop'
  price: number
  quantity: number
  filled_quantity: number
  status: 'pending' | 'filled' | 'cancelled' | 'rejected'
  created_at: string
}

export interface Position {
  id: number
  symbol: string
  side: 'long' | 'short'
  quantity: number
  entry_price: number
  current_price: number
  pnl: number
  pnl_percent: number
  liquidation_price?: number
}

export interface Portfolio {
  total_equity: number
  available_balance: number
  frozen_balance: number
  total_pnl: number
  positions: Position[]
}
