import client from './client'
import type {
  PortfolioSummary,
  PaginatedPositions,
  PortfolioPositionsQuery,
  PortfolioPerformance,
  EquityCurve,
  EquityCurveQuery,
} from '@/types/portfolio'

/** GET /api/v1/portfolio/summary */
export function getPortfolioSummary(userId?: string): Promise<PortfolioSummary> {
  return client.get('/portfolio/summary', { params: { user_id: userId } })
}

/** GET /api/v1/portfolio/positions */
export function listPortfolioPositions(params?: PortfolioPositionsQuery): Promise<PaginatedPositions> {
  return client.get('/portfolio/positions', { params })
}

/** GET /api/v1/portfolio/performance */
export function getPortfolioPerformance(userId?: string): Promise<PortfolioPerformance> {
  return client.get('/portfolio/performance', { params: { user_id: userId } })
}

/** GET /api/v1/portfolio/equity_curve */
export function getEquityCurve(params?: EquityCurveQuery): Promise<EquityCurve> {
  return client.get('/portfolio/equity_curve', { params })
}
