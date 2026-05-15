import { describe, it, expect } from 'vitest'
import type {
  BacktestStatus,
  BacktestRunRequest,
  EquityPoint,
  TradeRecord,
  BacktestResultResponse,
  BacktestSummary,
  PaginatedResponse,
  BacktestParams,
  BacktestResultDetail,
  BacktestHistoryItem,
  BacktestTrade,
} from '@/types/backtest'

describe('backtest types', () => {
  describe('BacktestStatus', () => {
    it('accepts valid status values', () => {
      const statuses: BacktestStatus[] = ['pending', 'running', 'completed', 'failed']
      expect(statuses).toHaveLength(4)
      expect(statuses).toContain('pending')
      expect(statuses).toContain('running')
      expect(statuses).toContain('completed')
      expect(statuses).toContain('failed')
    })
  })

  describe('BacktestRunRequest', () => {
    it('has nested config structure', () => {
      const req: BacktestRunRequest = {
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
      }
      expect(req.strategy_id).toBe('str-1')
      expect(req.config.symbol).toBe('BTC/USDT')
      expect(req.config.initial_capital).toBe(100000)
      expect(req.config.fee_rate).toBe(0.001)
      expect(req.config.slippage_rate).toBe(0.001)
    })
  })

  describe('EquityPoint', () => {
    it('has time, equity, drawdown_pct fields', () => {
      const point: EquityPoint = {
        time: Date.parse('2024-01-01'),
        equity: 105000,
        drawdown_pct: -2.1,
      }
      expect(point.time).toBeTypeOf('number')
      expect(point.equity).toBe(105000)
      expect(point.drawdown_pct).toBe(-2.1)
    })
  })

  describe('TradeRecord', () => {
    it('has all required fields', () => {
      const trade: TradeRecord = {
        direction: 'long',
        entry_time: '2024-01-15T09:30:00Z',
        exit_time: '2024-01-20T14:00:00Z',
        entry_price: 42000,
        exit_price: 43500,
        quantity: 0.5,
        pnl_usdt: 750,
        pnl_pct: 3.57,
        fee: 21,
        slippage: 5,
        exit_reason: 'take_profit',
        holding_period_ms: 160200000,
      }
      expect(trade.direction).toBe('long')
      expect(trade.exit_reason).toBe('take_profit')
      expect(trade.pnl_usdt).toBe(750)
      expect(trade.quantity).toBe(0.5)
    })

    it('accepts short direction', () => {
      const trade: TradeRecord = {
        direction: 'short',
        entry_time: '2024-02-01T10:00:00Z',
        exit_time: '2024-02-05T16:00:00Z',
        entry_price: 44000,
        exit_price: 42800,
        quantity: 0.3,
        pnl_usdt: 360,
        pnl_pct: 2.73,
        fee: 13.2,
        slippage: 3.6,
        exit_reason: 'signal',
        holding_period_ms: 100800000,
      }
      expect(trade.direction).toBe('short')
      expect(trade.exit_reason).toBe('signal')
    })

    it('accepts stop_loss exit reason', () => {
      const trade: TradeRecord = {
        direction: 'long',
        entry_time: '2024-03-10T08:00:00Z',
        exit_time: '2024-03-15T12:00:00Z',
        entry_price: 45000,
        exit_price: 43800,
        quantity: 0.4,
        pnl_usdt: -480,
        pnl_pct: -2.67,
        fee: 17.76,
        slippage: 4.32,
        exit_reason: 'stop_loss',
        holding_period_ms: 115200000,
      }
      expect(trade.exit_reason).toBe('stop_loss')
      expect(trade.pnl_usdt).toBeLessThan(0)
    })
  })

  describe('BacktestResultResponse', () => {
    it('has all required fields with nested structure', () => {
      const result: BacktestResultResponse = {
        id: 'uuid-1',
        strategy_id: 'str-1',
        status: 'completed',
        progress: 100,
        error: null,
        config: {
          symbol: 'BTC/USDT',
          interval: '1h',
          start_date: '2024-01-01',
          end_date: '2024-12-31',
          initial_capital: 100000,
          fee_rate: 0.001,
          slippage_rate: 0.001,
        },
        metrics: {
          total_return_pct: 25.3,
          annualized_return_pct: 18.5,
          sharpe_ratio: 2.1,
          sortino_ratio: 1.8,
          max_drawdown_pct: -12.5,
          calmar_ratio: 1.48,
          win_rate: 65.8,
          profit_factor: 2.3,
          avg_win_pct: 3.2,
          avg_loss_pct: -1.8,
          total_trades: 96,
          total_fees: 240,
          total_slippage: 160,
          duration_ms: 86400000,
        },
        equity_curve: [],
        trades: [],
        created_at: '2024-01-01T00:00:00Z',
      }
      expect(result.id).toBe('uuid-1')
      expect(result.status).toBe('completed')
      expect(result.config.symbol).toBe('BTC/USDT')
      expect(result.metrics.sharpe_ratio).toBe(2.1)
      expect(result.metrics.profit_factor).toBe(2.3)
      expect(result.error).toBeNull()
    })

    it('accepts null profit_factor for INFINITY case', () => {
      const result: BacktestResultResponse = {
        id: 'uuid-2',
        strategy_id: 'str-1',
        status: 'completed',
        progress: 100,
        error: null,
        config: {
          symbol: 'BTC/USDT',
          interval: '1h',
          start_date: '2024-01-01',
          end_date: '2024-12-31',
          initial_capital: 100000,
          fee_rate: 0.001,
          slippage_rate: 0.001,
        },
        metrics: {
          total_return_pct: 25.3,
          annualized_return_pct: 18.5,
          sharpe_ratio: 2.1,
          sortino_ratio: 1.8,
          max_drawdown_pct: -12.5,
          calmar_ratio: 1.48,
          win_rate: 65.8,
          profit_factor: null,
          avg_win_pct: 3.2,
          avg_loss_pct: -1.8,
          total_trades: 96,
          total_fees: 240,
          total_slippage: 160,
          duration_ms: 86400000,
        },
        equity_curve: [],
        trades: [],
        created_at: '2024-01-01T00:00:00Z',
      }
      expect(result.metrics.profit_factor).toBeNull()
    })
  })

  describe('BacktestSummary', () => {
    it('has required fields for list display', () => {
      const summary: BacktestSummary = {
        id: 'uuid-1',
        strategy_id: 'str-1',
        symbol: 'BTC/USDT',
        status: 'completed',
        total_return_pct: 25.3,
        sharpe_ratio: 2.1,
        created_at: '2024-01-01T00:00:00Z',
      }
      expect(summary.id).toBe('uuid-1')
      expect(summary.symbol).toBe('BTC/USDT')
    })
  })

  describe('PaginatedResponse', () => {
    it('has items, total, page, size', () => {
      const page: PaginatedResponse<BacktestSummary> = {
        items: [
          { id: 'uuid-1', strategy_id: 'str-1', symbol: 'BTC/USDT', status: 'completed', total_return_pct: 25.3, sharpe_ratio: 2.1, created_at: '2024-01-01T00:00:00Z' },
        ],
        total: 1,
        page: 1,
        size: 10,
      }
      expect(page.items).toHaveLength(1)
      expect(page.total).toBe(1)
      expect(page.page).toBe(1)
      expect(page.size).toBe(10)
    })

    it('is generic and works with other types', () => {
      const page: PaginatedResponse<string> = {
        items: ['a', 'b'],
        total: 2,
        page: 1,
        size: 10,
      }
      expect(page.items).toHaveLength(2)
    })
  })

  describe('BacktestParams', () => {
    it('has flat structure for form emission', () => {
      const params: BacktestParams = {
        strategy_id: 'str-1',
        symbol: 'BTC/USDT',
        interval: '1h',
        start_date: '2024-01-01',
        end_date: '2024-12-31',
        initial_capital: 100000,
        fee_rate: 0.001,
        slippage_rate: 0.001,
      }
      expect(params.strategy_id).toBe('str-1')
      expect(params.symbol).toBe('BTC/USDT')
      expect(params.interval).toBe('1h')
      // No nested config — flat structure
      expect((params as any).config).toBeUndefined()
    })

    it('accepts optional strategy_params', () => {
      const params: BacktestParams = {
        strategy_id: 'str-1',
        symbol: 'BTC/USDT',
        interval: '1h',
        start_date: '2024-01-01',
        end_date: '2024-12-31',
        initial_capital: 100000,
        fee_rate: 0.001,
        slippage_rate: 0.001,
        strategy_params: { fast_period: 10, slow_period: 30 },
      }
      expect(params.strategy_params).toEqual({ fast_period: 10, slow_period: 30 })
    })
  })

  describe('deprecated type aliases', () => {
    it('BacktestResultDetail is alias for BacktestResultResponse', () => {
      const detail: BacktestResultDetail = {
        id: 'uuid-1',
        strategy_id: 'str-1',
        status: 'completed',
        progress: 100,
        error: null,
        config: {
          symbol: 'BTC/USDT',
          interval: '1h',
          start_date: '2024-01-01',
          end_date: '2024-12-31',
          initial_capital: 100000,
          fee_rate: 0.001,
          slippage_rate: 0.001,
        },
        metrics: {
          total_return_pct: 25.3,
          annualized_return_pct: 18.5,
          sharpe_ratio: 2.1,
          sortino_ratio: 1.8,
          max_drawdown_pct: -12.5,
          calmar_ratio: 1.48,
          win_rate: 65.8,
          profit_factor: 2.3,
          avg_win_pct: 3.2,
          avg_loss_pct: -1.8,
          total_trades: 96,
          total_fees: 240,
          total_slippage: 160,
          duration_ms: 86400000,
        },
        equity_curve: [],
        trades: [],
        created_at: '2024-01-01T00:00:00Z',
      }
      // BacktestResultDetail should have same shape as BacktestResultResponse
      expect(detail.id).toBe('uuid-1')
      expect(detail.config.symbol).toBe('BTC/USDT')
    })

    it('BacktestHistoryItem is alias for BacktestSummary', () => {
      const item: BacktestHistoryItem = {
        id: 'uuid-1',
        strategy_id: 'str-1',
        symbol: 'BTC/USDT',
        status: 'completed',
        total_return_pct: 25.3,
        sharpe_ratio: 2.1,
        created_at: '2024-01-01T00:00:00Z',
      }
      expect(item.id).toBe('uuid-1')
    })

    it('BacktestTrade is alias for TradeRecord', () => {
      const trade: BacktestTrade = {
        direction: 'long',
        entry_time: '2024-01-15T09:30:00Z',
        exit_time: '2024-01-20T14:00:00Z',
        entry_price: 42000,
        exit_price: 43500,
        quantity: 0.5,
        pnl_usdt: 750,
        pnl_pct: 3.57,
        fee: 21,
        slippage: 5,
        exit_reason: 'take_profit',
        holding_period_ms: 160200000,
      }
      expect(trade.direction).toBe('long')
    })
  })
})
