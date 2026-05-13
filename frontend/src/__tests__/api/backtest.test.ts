import { describe, it, expect, vi, beforeEach } from 'vitest'
import * as backtestApi from '@/api/backtest'
import client from '@/api/client'

vi.mock('@/api/client', () => ({
  default: {
    post: vi.fn(),
    get: vi.fn(),
    delete: vi.fn(),
  },
}))

describe('backtest API', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('runBacktest sends POST /backtest with nested { strategy_id, config } structure', async () => {
    const params = {
      strategy_id: 'str-1',
      symbol: 'BTC/USDT',
      interval: '1h',
      start_date: '2024-01-01',
      end_date: '2024-12-31',
      initial_capital: 100000,
      fee_rate: 0.001,
      slippage_rate: 0.001,
    }
    const mockResult = {
      id: 'uuid-1',
      strategy_id: 'str-1',
      status: 'completed' as const,
      progress: 100,
      error: null,
      config: { symbol: 'BTC/USDT', interval: '1h', start_date: '2024-01-01', end_date: '2024-12-31', initial_capital: 100000, fee_rate: 0.001, slippage_rate: 0.001 },
      metrics: { total_return_pct: 15.5, annualized_return_pct: 12.3, sharpe_ratio: 1.8, sortino_ratio: 2.1, max_drawdown_pct: -8.2, calmar_ratio: 1.2, win_rate: 62.5, profit_factor: null, avg_win_pct: 3.2, avg_loss_pct: -1.8, total_trades: 48, total_fees: 120, total_slippage: 80, duration_ms: 5000 },
      equity_curve: [],
      trades: [],
      created_at: '2024-01-01T00:00:00Z',
    }
    vi.mocked(client.post).mockResolvedValue(mockResult)

    const result = await backtestApi.runBacktest(params)
    // Verify the call sends nested { strategy_id, config } — backend expects nested structure
    expect(client.post).toHaveBeenCalledWith('/backtest', {
      strategy_id: 'str-1',
      config: {
        symbol: 'BTC/USDT',
        interval: '1h',
        start_date: '2024-01-01',
        end_date: '2024-12-31',
        initial_capital: 100000,
        fee_rate: 0.001,
        slippage_rate: 0.001,
      },
    })
    expect(result).toEqual(mockResult)
  })

  it('getBacktestResult sends GET /backtest/:id with string id', async () => {
    const mockResult = {
      id: 'uuid-123',
      strategy_id: 'str-1',
      status: 'completed' as const,
      progress: 100,
      error: null,
      config: { symbol: 'BTC/USDT', interval: '1h', start_date: '2024-01-01', end_date: '2024-12-31', initial_capital: 100000, fee_rate: 0.001, slippage_rate: 0.001 },
      metrics: { total_return_pct: 15.5, annualized_return_pct: 12.3, sharpe_ratio: 1.8, sortino_ratio: 2.1, max_drawdown_pct: -8.2, calmar_ratio: 1.2, win_rate: 62.5, profit_factor: 2.1, avg_win_pct: 3.2, avg_loss_pct: -1.8, total_trades: 48, total_fees: 120, total_slippage: 80, duration_ms: 5000 },
      equity_curve: [],
      trades: [],
      created_at: '2024-01-01T00:00:00Z',
    }
    vi.mocked(client.get).mockResolvedValue(mockResult)

    const result = await backtestApi.getBacktestResult('uuid-123')
    expect(client.get).toHaveBeenCalledWith('/backtest/uuid-123')
    expect(result).toEqual(mockResult)
  })

  it('listBacktestHistory sends GET /backtest/history and returns PaginatedResponse', async () => {
    const mockPage = {
      items: [
        { id: 'uuid-1', strategy_id: 'str-1', symbol: 'BTC/USDT', status: 'completed', total_return_pct: 10, sharpe_ratio: 1.5, created_at: '2024-01-01T00:00:00Z' },
      ],
      total: 1,
      page: 1,
      size: 10,
    }
    vi.mocked(client.get).mockResolvedValue(mockPage)

    const result = await backtestApi.listBacktestHistory({ strategy_id: 'str-1', page: 1, size: 10 })
    expect(client.get).toHaveBeenCalledWith('/backtest/history', { params: { strategy_id: 'str-1', page: 1, size: 10 } })
    expect(result.items).toHaveLength(1)
    expect(result.total).toBe(1)
    expect(result).toEqual(mockPage)
  })

  it('deleteBacktestResult sends DELETE /backtest/:id with string id', async () => {
    vi.mocked(client.delete).mockResolvedValue(undefined)

    await backtestApi.deleteBacktestResult('uuid-123')
    expect(client.delete).toHaveBeenCalledWith('/backtest/uuid-123')
  })
})