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

// Re-export from market.ts (ADR D1/D2 aligned types)
export type {
  Ticker,
  DepthLevel,
  Depth,
  WsMessageType,
  WsMessage,
  WsSubscribe,
  WsPong,
  WsStatus,
  SymbolMetadata,
} from './market'

export { SYMBOL_NAMES, formatSymbol } from './market'

// ===== Strategy Types =====

/** Strategy summary for list/dashboard display (ADR D1: symbol/timeframe) */
export interface StrategySummary {
  id: string
  name: string
  description: string
  symbol: string      // ADR D1
  timeframe: string   // ADR D1
  template_id?: string
  pnl: number
  sharpe: number
  status: 'active' | 'paused' | 'stopped' | 'draft' | 'archived',
  template_type?: string
  param_summary?: string
  created_at: string
  updated_at: string
}

/** Full strategy detail from API (ADR D1: symbol/timeframe required) */
export interface StrategyFull {
  id: string
  user_id: string
  name: string
  description: string
  symbol: string         // ADR D1: 必填交易对，如 BTCUSDT
  timeframe: string      // ADR D1: 必填时间周期，如 1H
  strategy_type?: StrategyType  // T4.5: 策略类型
  template_id: string    // ADR D6: UUID 引用
  template_type: string
  parameters: Record<string, any>
  risk_config?: {
    max_position: number
    stop_loss: number
    stop_profit: number
  }
  // 绩效指标（来自最新回测，非实时交易）
  performance?: {
    total_return_pct?: number    // 收益率 %
    sharpe_ratio?: number         // 夏普率
    max_drawdown_pct?: number    // 最大回撤 %
    total_trades?: number        // 交易次数
  }
  status: 'active' | 'paused' | 'stopped' | 'draft' | 'archived'
  created_at: string
  updated_at: string
  strategy_code?: string  // T4.5: 上传的策略代码文件路径
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

/** Strategy template (aligned with backend TemplateInfo, ADR D6) */
export interface StrategyTemplate {
  id: string
  name: string
  description: string
  category: string
  default_parameters: Record<string, any>
  parameter_schema: StrategyParamDef[]
  // T4.5: strategy_type derived from category for CreateStrategyPayload
  strategy_type?: StrategyType
  // Marketplace fields (ADR D7)
  author?: string
  rating?: number
  usage_count?: number
  is_official?: boolean
}

/** Strategy type options (ADR D1) */
export type StrategyType = 'trend_following' | 'mean_reversion' | 'grid_trading' | 'arbitrage' | 'custom'

/** Create strategy payload (ADR D1: symbol/timeframe/strategy_type required, D6: template_id UUID) */
export interface CreateStrategyPayload {
  name: string
  description?: string     // T4.5: 策略描述，最多500字符
  symbol: string            // ADR D1 必填
  timeframe: string        // ADR D1 必填
  strategy_type: StrategyType  // ADR D1 必填
  template_id?: string     // ADR D6: UUID，兼容旧 template_type 字段
  template_type?: string   // 兼容旧版
  parameters: Record<string, any>
  strategy_code?: string   // T4.5: 可选 .py/.js 策略代码文件路径
}

/** Update strategy payload (ADR D3: symbol/timeframe immutable after create, D6: risk_config) */
export interface UpdateStrategyPayload {
  name?: string
  parameters?: Record<string, any>
  risk_config?: {
    max_position: number
    stop_loss: number
    stop_profit: number
  }
  strategy_code?: string   // T4.5: 可选 .py/.js 策略代码文件路径
}

/** Strategy paginated query params (ADR D2) */
export interface StrategyQueryParams {
  status?: string
  page?: number
  size?: number
  search?: string       // ADR D2: 搜索策略名称
  sort_by?: 'created_at' | 'name'  // ADR D2: 排序
  sort_order?: 'asc' | 'desc'     // ADR D2: 升序/降序
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

// Re-export from order.ts (ADR D6 aligned types)
export type {
  OrderSide,
  OrderType,
  OrderStatus,
  TradeMode,
  TimeInForce,
  Order as AdrOrder,
  CreateOrderRequest,
  OrderQueryParams,
  OrderListResponse,
  CancelOrderResponse,
  CancelAllResponse,
  PaperAccount,
  SymbolConfig,
  TradeWsMessageType,
  TradeWsMessage,
  Position as AdrPosition,
} from './order'

export {
  getOrderStatusType,
  getOrderStatusText,
  getOrderSideText,
} from './order'
