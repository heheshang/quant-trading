import { describe, it, expect, vi, beforeEach } from 'vitest'
import * as portfolioApi from '@/api/portfolio'
import type { PortfolioSummary, PaginatedPositions, PortfolioPerformance, EquityCurve } from '@/types/portfolio'

// Mock the API client
vi.mock('@/api/client', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
  },
}))

import client from '@/api/client'
const mockGet = client.get as ReturnType<typeof vi.fn>

describe('Portfolio API', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('getPortfolioSummary calls correct endpoint', async () => {
    const mockSummary: PortfolioSummary = {
      total_equity: '100000.00',
      daily_pnl: '1234.56',
      daily_pnl_rate: '1.25',
      cumulative_pnl: '15000.00',
      cumulative_pnl_rate: '17.65',
      total_positions: 5,
      updated_at: '2026-05-14T00:00:00Z',
    }
    mockGet.mockResolvedValueOnce(mockSummary)

    const result = await portfolioApi.getPortfolioSummary()
    expect(mockGet).toHaveBeenCalledWith('/portfolio/summary', { params: { user_id: undefined } })
    expect(result).toEqual(mockSummary)
  })

  it('getPortfolioSummary passes userId param', async () => {
    mockGet.mockResolvedValueOnce({})

    await portfolioApi.getPortfolioSummary('user-123')
    expect(mockGet).toHaveBeenCalledWith('/portfolio/summary', { params: { user_id: 'user-123' } })
  })

  it('listPortfolioPositions calls correct endpoint with params', async () => {
    const mockPositions: PaginatedPositions = {
      items: [],
      total: 0,
      page: 1,
      size: 10,
    }
    mockGet.mockResolvedValueOnce(mockPositions)

    const result = await portfolioApi.listPortfolioPositions({ page: 2, size: 20 })
    expect(mockGet).toHaveBeenCalledWith('/portfolio/positions', {
      params: { page: 2, size: 20 },
    })
    expect(result).toEqual(mockPositions)
  })

  it('listPortfolioPositions passes filter params', async () => {
    mockGet.mockResolvedValueOnce({ items: [], total: 0, page: 1, size: 10 })

    await portfolioApi.listPortfolioPositions({ symbol: 'BTC', side: 'long' })
    expect(mockGet).toHaveBeenCalledWith('/portfolio/positions', {
      params: { symbol: 'BTC', side: 'long' },
    })
  })

  it('getPortfolioPerformance calls correct endpoint', async () => {
    const mockPerf: PortfolioPerformance = {
      strategies: [],
      max_drawdown: '-5.00',
      sharpe_ratio: '1.50',
      win_rate: '55.00',
    }
    mockGet.mockResolvedValueOnce(mockPerf)

    const result = await portfolioApi.getPortfolioPerformance()
    expect(mockGet).toHaveBeenCalledWith('/portfolio/performance', { params: { user_id: undefined } })
    expect(result).toEqual(mockPerf)
  })

  it('getPortfolioPerformance passes userId', async () => {
    mockGet.mockResolvedValueOnce({})

    await portfolioApi.getPortfolioPerformance('user-456')
    expect(mockGet).toHaveBeenCalledWith('/portfolio/performance', { params: { user_id: 'user-456' } })
  })

  it('getEquityCurve calls correct endpoint with params', async () => {
    const mockCurve: EquityCurve = {
      points: [
        { timestamp: '2026-05-14T00:00:00Z', equity: '100000.00' },
      ],
    }
    mockGet.mockResolvedValueOnce(mockCurve)

    const result = await portfolioApi.getEquityCurve({
      start_date: '2026-04-14',
      end_date: '2026-05-14',
      granularity: 'day',
    })
    expect(mockGet).toHaveBeenCalledWith('/portfolio/equity_curve', {
      params: { start_date: '2026-04-14', end_date: '2026-05-14', granularity: 'day' },
    })
    expect(result).toEqual(mockCurve)
  })

  it('getEquityCurve works without params', async () => {
    mockGet.mockResolvedValueOnce({ points: [] })

    await portfolioApi.getEquityCurve()
    expect(mockGet).toHaveBeenCalledWith('/portfolio/equity_curve', { params: undefined })
  })

  it('handles API errors', async () => {
    mockGet.mockRejectedValueOnce(new Error('Network error'))

    await expect(portfolioApi.getPortfolioSummary()).rejects.toThrow('Network error')
  })
})
