import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock the client module
vi.mock('@/api/client', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
  },
}))

import client from '@/api/client'
import {
  getRiskRules,
  updateRiskRules,
  getRiskLogs,
  emergencyClose,
  pauseTrading,
  resumeTrading,
  manualRiskCheck,
  getConnectionStatus,
} from '@/api/risk'

const mockClient = client as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
  put: ReturnType<typeof vi.fn>
  delete: ReturnType<typeof vi.fn>
}

describe('risk API', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ─── P0-F2: 资金风控规则 ────────────────────────────────────────

  describe('getRiskRules', () => {
    it('should GET /risk/rules', async () => {
      const mockRules = {
        id: 1,
        user_id: 1,
        daily_loss_limit: '1000.00',
        daily_loss_auto_close: true,
        single_trade_loss_ratio: '0.0500',
        max_drawdown_ratio: '0.2000',
        drawdown_auto_close: false,
        stop_loss_type: 'atr',
        atr_period: 14,
        atr_multiplier: '2.00',
        is_active: true,
        created_at: '2026-05-01T00:00:00Z',
        updated_at: '2026-05-15T00:00:00Z',
      }
      mockClient.get.mockResolvedValue({ data: mockRules })

      const result = await getRiskRules()

      expect(mockClient.get).toHaveBeenCalledWith('/risk/rules')
      expect(result).toEqual({ data: mockRules })
    })
  })

  describe('updateRiskRules', () => {
    it('should PUT /risk/rules with all fields', async () => {
      mockClient.put.mockResolvedValue({ data: { success: true } })

      const payload = {
        daily_loss_limit: '500.00',
        daily_loss_auto_close: true,
        single_trade_loss_ratio: '0.0300',
        max_drawdown_ratio: '0.1500',
        drawdown_auto_close: true,
        stop_loss_type: 'atr',
        atr_period: 14,
        atr_multiplier: '2.50',
        is_active: true,
      }
      const result = await updateRiskRules(payload)

      expect(mockClient.put).toHaveBeenCalledWith('/risk/rules', payload)
      expect(result).toEqual({ data: { success: true } })
    })

    it('should PUT /risk/rules with minimal fields', async () => {
      mockClient.put.mockResolvedValue({ data: { success: true } })

      const payload = {
        daily_loss_limit: '0',
        daily_loss_auto_close: false,
        single_trade_loss_ratio: '0.05',
        max_drawdown_ratio: '0.2',
        drawdown_auto_close: false,
        stop_loss_type: 'fixed',
        atr_period: null,
        atr_multiplier: null,
        is_active: false,
      }
      await updateRiskRules(payload)

      expect(mockClient.put).toHaveBeenCalledWith('/risk/rules', payload)
    })
  })

  describe('getRiskLogs', () => {
    it('should GET /risk/logs without params', async () => {
      const mockResponse = {
        data: [
          {
            id: 1,
            user_id: 1,
            triggered_rule: 'daily_loss_limit',
            severity: 'high',
            action: 'close_position',
            details: 'Daily loss 1200 exceeded limit 1000',
            snapshot: null,
            created_at: '2026-05-15T10:00:00Z',
          },
        ],
        total: 1,
        page: 1,
        size: 10,
      }
      mockClient.get.mockResolvedValue(mockResponse)

      const result = await getRiskLogs()

      expect(mockClient.get).toHaveBeenCalledWith('/risk/logs', { params: undefined })
      expect(result).toEqual(mockResponse)
    })

    it('should GET /risk/logs with pagination params', async () => {
      mockClient.get.mockResolvedValue({ data: [], total: 0, page: 2, size: 5 })

      await getRiskLogs({ page: 2, page_size: 5 })

      expect(mockClient.get).toHaveBeenCalledWith('/risk/logs', {
        params: { page: 2, page_size: 5 },
      })
    })
  })

  // ─── P0-F3: 应急操作 ────────────────────────────────────────────

  describe('emergencyClose', () => {
    it('should POST /risk/emergency-close with empty body', async () => {
      const mockResult = {
        data: {
          success: true,
          closed_positions: 3,
          total_pnl: '-150.50',
          message: 'All positions closed',
          details: [],
        },
      }
      mockClient.post.mockResolvedValue(mockResult)

      const result = await emergencyClose()

      expect(mockClient.post).toHaveBeenCalledWith('/risk/emergency-close', {})
      expect(result).toEqual(mockResult)
    })
  })

  describe('pauseTrading', () => {
    it('should POST /risk/pause with empty body', async () => {
      mockClient.post.mockResolvedValue({ data: { paused: true, reason: 'Manual pause' } })

      const result = await pauseTrading()

      expect(mockClient.post).toHaveBeenCalledWith('/risk/pause', {})
      expect(result.data.paused).toBe(true)
    })
  })

  describe('resumeTrading', () => {
    it('should POST /risk/resume with empty body', async () => {
      mockClient.post.mockResolvedValue({ data: { paused: false, reason: 'Manual resume' } })

      const result = await resumeTrading()

      expect(mockClient.post).toHaveBeenCalledWith('/risk/resume', {})
      expect(result.data.paused).toBe(false)
    })
  })

  describe('manualRiskCheck', () => {
    it('should POST /risk/check with empty body', async () => {
      mockClient.post.mockResolvedValue({ data: { violations_found: 0 } })

      const result = await manualRiskCheck()

      expect(mockClient.post).toHaveBeenCalledWith('/risk/check', {})
      expect(result).toEqual({ data: { violations_found: 0 } })
    })
  })

  describe('getConnectionStatus', () => {
    it('should GET /risk/connection-status', async () => {
      const mockStatus = {
        data: {
          exchange_connected: true,
          disconnect_elapsed_secs: 0,
          strategy_paused: false,
        },
      }
      mockClient.get.mockResolvedValue(mockStatus)

      const result = await getConnectionStatus()

      expect(mockClient.get).toHaveBeenCalledWith('/risk/connection-status')
      expect(result).toEqual(mockStatus)
    })

    it('should return disconnected status when WebSocket is down', async () => {
      const mockStatus = {
        data: {
          exchange_connected: false,
          disconnect_elapsed_secs: 65,
          strategy_paused: true,
        },
      }
      mockClient.get.mockResolvedValue(mockStatus)

      const result = await getConnectionStatus()

      expect(mockClient.get).toHaveBeenCalledWith('/risk/connection-status')
      expect(result.data.exchange_connected).toBe(false)
      expect(result.data.strategy_paused).toBe(true)
    })
  })
})
