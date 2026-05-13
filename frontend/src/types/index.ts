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

/** Strategy summary for list/dashboard display */
export interface StrategySummary {
  id: string
  name: string
  description: string
  pnl: number
  sharpe: number
  status: 'active' | 'paused' | 'stopped'
  template_type?: string
  param_summary?: string
  created_at: string
  updated_at: string
}

/** Full strategy detail from API */
export interface StrategyFull {
  id: string
  user_id: string
  name: string
  description: string
  template_type: string
  parameters: Record<string, any>
  status: 'active' | 'paused' | 'stopped' | 'draft'
  created_at: string
  updated_at: string
}

/** Strategy parameter definition schema (aligned with backend) */
export interface StrategyParamDef {
  name: string
  label: string
  type: 'integer' | 'float' | 'select' | 'boolean' | 'string'
  default?: any
  min?: number
  max?: number
  options?: string[]
  description?: string
}

/** Strategy template (aligned with backend TemplateInfo) */
export interface StrategyTemplate {
  id: string
  name: string
  description: string
  category: string
  default_parameters: Record<string, any>
  parameter_schema: StrategyParamDef[]
}

/** Create strategy payload */
export interface CreateStrategyPayload {
  name: string
  template_type: string
  parameters: Record<string, any>
}

/** Update strategy payload */
export interface UpdateStrategyPayload {
  name?: string
  parameters?: Record<string, any>
}

/** Strategy paginated query params */
export interface StrategyQueryParams {
  status?: string
  page?: number
  size?: number
}

export interface BacktestParams {
  strategy_id: string
  symbol: string
  timeframe?: string
  start_date: string
  end_date: string
  initial_capital: number
  fee_rate?: number
  slippage?: number
  strategy_params?: Record<string, any>
}

export interface BacktestResult {
  id: number
  strategy_id: string
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
