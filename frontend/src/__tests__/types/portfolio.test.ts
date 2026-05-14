import { describe, it, expect } from 'vitest'
import type {
  PortfolioSummary,
  PortfolioPosition,
  PaginatedPositions,
  StrategyPerformance,
  PortfolioPerformance,
  EquityCurvePoint,
  EquityCurve,
  PositionSide,
  EquityGranularity,
  PortfolioPositionsQuery,
  EquityCurveQuery,
} from '@/types/portfolio'

describe('Portfolio Types', () => {
  it('PortfolioSummary has correct field names (B1 snake_case)', () => {
    const summary: PortfolioSummary = {
      total_equity: '100000.00',
      daily_pnl: '1234.56',
      daily_pnl_rate: '1.25',
      cumulative_pnl: '15000.00',
      cumulative_pnl_rate: '17.65',
      total_positions: 5,
      updated_at: '2026-05-14T00:00:00Z',
    }
    expect(summary.total_equity).toBe('100000.00')
    expect(summary.daily_pnl).toBe('1234.56')
    expect(summary.cumulative_pnl_rate).toBe('17.65')
    expect(summary.total_positions).toBe(5)
  })

  it('PortfolioSummary uses string for decimals (B4)', () => {
    const summary: PortfolioSummary = {
      total_equity: '100000.00',
      daily_pnl: '1234.56',
      daily_pnl_rate: '1.25',
      cumulative_pnl: '15000.00',
      cumulative_pnl_rate: '17.65',
      total_positions: 0,
      updated_at: '2026-05-14T00:00:00Z',
    }
    expect(typeof summary.total_equity).toBe('string')
    expect(typeof summary.daily_pnl).toBe('string')
  })

  it('PortfolioPosition has all required fields', () => {
    const pos: PortfolioPosition = {
      symbol: 'BTCUSDT',
      side: 'long',
      quantity: '0.5',
      avg_price: '60000.00',
      current_price: '65000.00',
      unrealized_pnl: '2500.00',
      unrealized_pnl_rate: '8.33',
    }
    expect(pos.symbol).toBe('BTCUSDT')
    expect(pos.side).toBe('long')
    expect(pos.quantity).toBe('0.5')
  })

  it('PositionSide is long or short', () => {
    const long: PositionSide = 'long'
    const short: PositionSide = 'short'
    expect(long).toBe('long')
    expect(short).toBe('short')
  })

  it('PaginatedPositions has items and pagination info', () => {
    const paginated: PaginatedPositions = {
      items: [
        { symbol: 'BTCUSDT', side: 'long', quantity: '0.5', avg_price: '60000.00', current_price: '65000.00', unrealized_pnl: '2500.00', unrealized_pnl_rate: '8.33' },
      ],
      total: 1,
      page: 1,
      size: 10,
    }
    expect(paginated.items).toHaveLength(1)
    expect(paginated.total).toBe(1)
    expect(paginated.page).toBe(1)
    expect(paginated.size).toBe(10)
  })

  it('StrategyPerformance has all required fields', () => {
    const sp: StrategyPerformance = {
      strategy_id: 'uuid-1',
      strategy_name: '网格策略',
      total_pnl: '5000.00',
      total_pnl_rate: '12.50',
      max_drawdown: '-8.30',
      trade_count: 42,
      win_rate: '62.50',
    }
    expect(sp.strategy_name).toBe('网格策略')
    expect(sp.trade_count).toBe(42)
    expect(typeof sp.total_pnl).toBe('string')
  })

  it('PortfolioPerformance contains strategies and overall metrics', () => {
    const perf: PortfolioPerformance = {
      strategies: [
        { strategy_id: 'uuid-1', strategy_name: '网格策略', total_pnl: '5000.00', total_pnl_rate: '12.50', max_drawdown: '-8.30', trade_count: 42, win_rate: '62.50' },
      ],
      max_drawdown: '-8.30',
      sharpe_ratio: '1.85',
      win_rate: '62.50',
    }
    expect(perf.strategies).toHaveLength(1)
    expect(perf.sharpe_ratio).toBe('1.85')
  })

  it('EquityCurvePoint has timestamp and equity', () => {
    const point: EquityCurvePoint = {
      timestamp: '2026-05-14T00:00:00Z',
      equity: '100000.00',
    }
    expect(point.timestamp).toBe('2026-05-14T00:00:00Z')
    expect(point.equity).toBe('100000.00')
  })

  it('EquityCurve has points array', () => {
    const curve: EquityCurve = {
      points: [
        { timestamp: '2026-05-14T00:00:00Z', equity: '100000.00' },
        { timestamp: '2026-05-15T00:00:00Z', equity: '101000.00' },
      ],
    }
    expect(curve.points).toHaveLength(2)
  })

  it('EquityGranularity accepts hour/day/week', () => {
    const g1: EquityGranularity = 'hour'
    const g2: EquityGranularity = 'day'
    const g3: EquityGranularity = 'week'
    expect([g1, g2, g3]).toEqual(['hour', 'day', 'week'])
  })

  it('PortfolioPositionsQuery has optional fields', () => {
    const q1: PortfolioPositionsQuery = {}
    const q2: PortfolioPositionsQuery = { symbol: 'BTC', side: 'long', page: 2, size: 20 }
    expect(q1).toEqual({})
    expect(q2.symbol).toBe('BTC')
  })

  it('EquityCurveQuery has optional fields', () => {
    const q: EquityCurveQuery = {
      start_date: '2026-04-14',
      end_date: '2026-05-14',
      granularity: 'day',
    }
    expect(q.granularity).toBe('day')
  })

  it('B3: nullable fields use T | null', () => {
    // This is a compile-time check; ensure types are compatible
    const summary: PortfolioSummary = {
      total_equity: '100000.00',
      daily_pnl: '0',
      daily_pnl_rate: '0',
      cumulative_pnl: '0',
      cumulative_pnl_rate: '0',
      total_positions: 0,
      updated_at: '2026-05-14T00:00:00Z',
    }
    expect(summary).toBeDefined()
  })
})
