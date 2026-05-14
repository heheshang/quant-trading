import { describe, it, expect, vi, beforeEach } from 'vitest'
import * as portfolioApi from '@/api/portfolio'
import type { PortfolioSummary, PaginatedPositions, PortfolioPerformance, EquityCurve } from '@/types/portfolio'

// Mock API module
vi.mock('@/api/portfolio', () => ({
  getPortfolioSummary: vi.fn(),
  listPortfolioPositions: vi.fn(),
  getPortfolioPerformance: vi.fn(),
  getEquityCurve: vi.fn(),
}))

const mockGetPortfolioSummary = vi.mocked(portfolioApi.getPortfolioSummary)
const mockListPortfolioPositions = vi.mocked(portfolioApi.listPortfolioPositions)
const mockGetPortfolioPerformance = vi.mocked(portfolioApi.getPortfolioPerformance)
const mockGetEquityCurve = vi.mocked(portfolioApi.getEquityCurve)

// Must import composables AFTER vi.mock
import { usePortfolioSummary } from '@/composables/usePortfolioSummary'
import { usePortfolioPositions } from '@/composables/usePortfolioPositions'
import { usePortfolioPerformance } from '@/composables/usePortfolioPerformance'
import { useEquityCurve } from '@/composables/useEquityCurve'

describe('usePortfolioSummary', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('initial state is null/empty', () => {
    const { summary, loading, error } = usePortfolioSummary()
    expect(summary.value).toBeNull()
    expect(loading.value).toBe(false)
    expect(error.value).toBeNull()
  })

  it('fetches and stores summary', async () => {
    const mockSummary: PortfolioSummary = {
      total_equity: '100000.00',
      daily_pnl: '1234.56',
      daily_pnl_rate: '1.25',
      cumulative_pnl: '15000.00',
      cumulative_pnl_rate: '17.65',
      total_positions: 5,
      updated_at: '2026-05-14T00:00:00Z',
    }
    mockGetPortfolioSummary.mockResolvedValueOnce(mockSummary)

    const { summary, loading, error, fetch } = usePortfolioSummary()
    const promise = fetch()
    expect(loading.value).toBe(true)

    await promise
    expect(loading.value).toBe(false)
    expect(summary.value).toEqual(mockSummary)
    expect(error.value).toBeNull()
  })

  it('handles fetch error', async () => {
    mockGetPortfolioSummary.mockRejectedValueOnce(new Error('API error'))

    const { summary, loading, error, fetch } = usePortfolioSummary()
    await fetch()

    expect(loading.value).toBe(false)
    expect(summary.value).toBeNull()
    expect(error.value).toBe('API error')
  })

  it('applyWsUpdate merges data', async () => {
    const mockSummary: PortfolioSummary = {
      total_equity: '100000.00',
      daily_pnl: '0',
      daily_pnl_rate: '0',
      cumulative_pnl: '0',
      cumulative_pnl_rate: '0',
      total_positions: 0,
      updated_at: '2026-05-14T00:00:00Z',
    }
    mockGetPortfolioSummary.mockResolvedValueOnce(mockSummary)

    const { summary, fetch, applyWsUpdate } = usePortfolioSummary()
    await fetch()

    applyWsUpdate({ total_equity: '105000.00', daily_pnl: '5000.00' })
    expect(summary.value?.total_equity).toBe('105000.00')
    expect(summary.value?.daily_pnl).toBe('5000.00')
  })

  it('passes userId to API', async () => {
    mockGetPortfolioSummary.mockResolvedValueOnce({} as any)

    const { fetch } = usePortfolioSummary()
    await fetch('user-123')

    expect(mockGetPortfolioSummary).toHaveBeenCalledWith('user-123')
  })
})

describe('usePortfolioPositions', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('initial state is null/empty', () => {
    const { positions, loading, error, filters, pagination } = usePortfolioPositions()
    expect(positions.value).toBeNull()
    expect(loading.value).toBe(false)
    expect(error.value).toBeNull()
    expect(filters.symbol).toBe('')
    expect(filters.side).toBe('')
    expect(pagination.page).toBe(1)
    expect(pagination.size).toBe(10)
  })

  it('fetches and stores positions', async () => {
    const mockPositions: PaginatedPositions = {
      items: [
        { symbol: 'BTCUSDT', side: 'long', quantity: '0.5', avg_price: '60000.00', current_price: '65000.00', unrealized_pnl: '2500.00', unrealized_pnl_rate: '8.33' },
      ],
      total: 1,
      page: 1,
      size: 10,
    }
    mockListPortfolioPositions.mockResolvedValueOnce(mockPositions)

    const { positions, loading, fetch } = usePortfolioPositions()
    await fetch()

    expect(loading.value).toBe(false)
    expect(positions.value).toEqual(mockPositions)
  })

  it('passes filters and pagination to API', async () => {
    mockListPortfolioPositions.mockResolvedValueOnce({ items: [], total: 0, page: 1, size: 10 })

    const { filters, pagination, fetch } = usePortfolioPositions()
    filters.symbol = 'BTC'
    filters.side = 'long'
    pagination.page = 2
    pagination.size = 20
    await fetch()

    expect(mockListPortfolioPositions).toHaveBeenCalledWith({
      symbol: 'BTC',
      side: 'long',
      page: 2,
      size: 20,
    })
  })

  it('handles fetch error', async () => {
    mockListPortfolioPositions.mockRejectedValueOnce(new Error('Server error'))

    const { error, fetch } = usePortfolioPositions()
    await fetch()

    expect(error.value).toBe('Server error')
  })

  it('applyWsUpdate updates existing position', async () => {
    const mockPositions: PaginatedPositions = {
      items: [
        { symbol: 'BTCUSDT', side: 'long', quantity: '0.5', avg_price: '60000.00', current_price: '65000.00', unrealized_pnl: '2500.00', unrealized_pnl_rate: '8.33' },
      ],
      total: 1,
      page: 1,
      size: 10,
    }
    mockListPortfolioPositions.mockResolvedValueOnce(mockPositions)

    const { positions, fetch, applyWsUpdate } = usePortfolioPositions()
    await fetch()

    const updatedPos = { symbol: 'BTCUSDT', side: 'long' as const, quantity: '0.6', avg_price: '60000.00', current_price: '66000.00', unrealized_pnl: '3600.00', unrealized_pnl_rate: '10.00' }
    applyWsUpdate(updatedPos)

    expect(positions.value?.items[0]).toEqual(updatedPos)
  })

  it('applyWsUpdate adds new position if not found', async () => {
    const mockPositions: PaginatedPositions = {
      items: [
        { symbol: 'BTCUSDT', side: 'long', quantity: '0.5', avg_price: '60000.00', current_price: '65000.00', unrealized_pnl: '2500.00', unrealized_pnl_rate: '8.33' },
      ],
      total: 1,
      page: 1,
      size: 10,
    }
    mockListPortfolioPositions.mockResolvedValueOnce(mockPositions)

    const { positions, fetch, applyWsUpdate } = usePortfolioPositions()
    await fetch()

    const newPos = { symbol: 'ETHUSDT', side: 'short' as const, quantity: '10', avg_price: '3200.00', current_price: '3000.00', unrealized_pnl: '2000.00', unrealized_pnl_rate: '6.25' }
    applyWsUpdate(newPos)

    expect(positions.value?.items).toHaveLength(2)
    expect(positions.value?.items[0].symbol).toBe('ETHUSDT')
    expect(positions.value?.total).toBe(2)
  })
})

describe('usePortfolioPerformance', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('initial state is null/empty', () => {
    const { performance, loading, error } = usePortfolioPerformance()
    expect(performance.value).toBeNull()
    expect(loading.value).toBe(false)
    expect(error.value).toBeNull()
  })

  it('fetches and stores performance', async () => {
    const mockPerf: PortfolioPerformance = {
      strategies: [
        { strategy_id: 'uuid-1', strategy_name: '网格策略', total_pnl: '5000.00', total_pnl_rate: '12.50', max_drawdown: '-8.30', trade_count: 42, win_rate: '62.50' },
      ],
      max_drawdown: '-8.30',
      sharpe_ratio: '1.85',
      win_rate: '62.50',
    }
    mockGetPortfolioPerformance.mockResolvedValueOnce(mockPerf)

    const { performance, loading, fetch } = usePortfolioPerformance()
    await fetch()

    expect(loading.value).toBe(false)
    expect(performance.value).toEqual(mockPerf)
  })

  it('handles fetch error', async () => {
    mockGetPortfolioPerformance.mockRejectedValueOnce(new Error('Failed'))

    const { error, fetch } = usePortfolioPerformance()
    await fetch()

    expect(error.value).toBe('Failed')
  })
})

describe('useEquityCurve', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('initial state has default granularity and dateRange', () => {
    const { curve, loading, error, granularity, dateRange } = useEquityCurve()
    expect(curve.value).toBeNull()
    expect(loading.value).toBe(false)
    expect(error.value).toBeNull()
    expect(granularity.value).toBe('day')
    expect(dateRange.value).toHaveLength(2)
  })

  it('fetches and stores equity curve', async () => {
    const mockCurve: EquityCurve = {
      points: [
        { timestamp: '2026-05-14T00:00:00Z', equity: '100000.00' },
        { timestamp: '2026-05-15T00:00:00Z', equity: '101000.00' },
      ],
    }
    mockGetEquityCurve.mockResolvedValueOnce(mockCurve)

    const { curve, loading, fetch } = useEquityCurve()
    await fetch()

    expect(loading.value).toBe(false)
    expect(curve.value).toEqual(mockCurve)
  })

  it('passes dateRange and granularity to API', async () => {
    mockGetEquityCurve.mockResolvedValueOnce({ points: [] })

    const { fetch, granularity, dateRange } = useEquityCurve()
    granularity.value = 'hour'
    await fetch()

    expect(mockGetEquityCurve).toHaveBeenCalledWith({
      start_date: dateRange.value[0],
      end_date: dateRange.value[1],
      granularity: 'hour',
    })
  })

  it('handles fetch error', async () => {
    mockGetEquityCurve.mockRejectedValueOnce(new Error('Curve error'))

    const { error, fetch } = useEquityCurve()
    await fetch()

    expect(error.value).toBe('Curve error')
  })
})
