// ===== Portfolio Types — Aligned with ADR-009 B1-B4 Contract =====
// B1: snake_case field names (matches backend API)
// B2: UUID → string
// B3: Option<T> → T | null
// B4: Decimal → string (avoids floating-point precision issues)

/** Portfolio summary — GET /api/v1/portfolio/summary */
export interface PortfolioSummary {
  total_equity: string        // "100000.00"
  daily_pnl: string           // "1234.56"
  daily_pnl_rate: string      // "1.25" (%)
  cumulative_pnl: string      // "15000.00"
  cumulative_pnl_rate: string // "17.65" (%)
  total_positions: number
  updated_at: string          // ISO8601
}

/** Position side */
export type PositionSide = 'long' | 'short'

/** Single position — item in GET /api/v1/portfolio/positions */
export interface PortfolioPosition {
  symbol: string
  side: PositionSide
  quantity: string
  avg_price: string
  current_price: string
  unrealized_pnl: string
  unrealized_pnl_rate: string  // "1.25" (%)
}

/** Paginated positions response */
export interface PaginatedPositions {
  items: PortfolioPosition[]
  total: number
  page: number
  size: number
}

/** Strategy performance — item in GET /api/v1/portfolio/performance */
export interface StrategyPerformance {
  strategy_id: string
  strategy_name: string
  total_pnl: string
  total_pnl_rate: string
  max_drawdown: string
  trade_count: number
  win_rate: string  // "62.50" (%)
}

/** Portfolio performance response */
export interface PortfolioPerformance {
  strategies: StrategyPerformance[]
  max_drawdown: string    // overall max drawdown
  sharpe_ratio: string    // overall sharpe ratio
  win_rate: string        // overall win rate
}

/** Single equity curve data point */
export interface EquityCurvePoint {
  timestamp: string
  equity: string
}

/** Equity curve response — GET /api/v1/portfolio/equity_curve */
export interface EquityCurve {
  points: EquityCurvePoint[]
}

/** Granularity options for equity curve */
export type EquityGranularity = 'hour' | 'day' | 'week'

/** Query params for positions list */
export interface PortfolioPositionsQuery {
  symbol?: string
  side?: PositionSide
  page?: number
  size?: number
  user_id?: string
}

/** Query params for equity curve */
export interface EquityCurveQuery {
  start_date?: string
  end_date?: string
  granularity?: EquityGranularity
  user_id?: string
}
