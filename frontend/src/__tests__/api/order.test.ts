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
  createOrder,
  getOrders,
  getOrder,
  cancelOrder,
  cancelAllOrders,
  getAccount,
  getSymbols,
  getPositions,
  closePosition,
} from '@/api/order'
import type { Order, OrderListResponse, PaperAccount, SymbolConfig, Position } from '@/types/order'

const mockClient = client as unknown as {
  get: ReturnType<typeof vi.fn>
  post: ReturnType<typeof vi.fn>
  put: ReturnType<typeof vi.fn>
  delete: ReturnType<typeof vi.fn>
}

describe('order API', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('createOrder', () => {
    it('should POST /orders with order data', async () => {
      const mockOrder: Order = {
        order_id: '123456',
        symbol: 'BTC/USDT',
        side: 'buy',
        order_type: 'limit',
        price: '49500.00',
        quantity: '0.1000',
        filled_quantity: '0.0000',
        avg_fill_price: null,
        status: 'pending',
        mode: 'paper',
        fee: '0',
        reject_reason: null,
        time_in_force: 'GTC',
        created_at: '2026-05-14T06:00:00Z',
        updated_at: '2026-05-14T06:00:00Z',
        cancelled_at: null,
        filled_at: null,
      }
      mockClient.post.mockResolvedValue(mockOrder)

      const result = await createOrder({
        symbol: 'BTC/USDT',
        side: 'buy',
        order_type: 'limit',
        price: '49500.00',
        quantity: '0.1000',
        time_in_force: 'GTC',
      })

      expect(mockClient.post).toHaveBeenCalledWith('/orders', {
        symbol: 'BTC/USDT',
        side: 'buy',
        order_type: 'limit',
        price: '49500.00',
        quantity: '0.1000',
        time_in_force: 'GTC',
      })
      expect(result).toEqual(mockOrder)
    })

    it('should omit price for market orders', async () => {
      mockClient.post.mockResolvedValue({})

      await createOrder({
        symbol: 'BTC/USDT',
        side: 'buy',
        order_type: 'market',
        quantity: '0.1000',
      })

      expect(mockClient.post).toHaveBeenCalledWith('/orders', {
        symbol: 'BTC/USDT',
        side: 'buy',
        order_type: 'market',
        quantity: '0.1000',
      })
    })
  })

  describe('getOrders', () => {
    it('should GET /orders without params', async () => {
      const mockResponse: OrderListResponse = {
        items: [],
        total: 0,
        page: 1,
        size: 20,
      }
      mockClient.get.mockResolvedValue(mockResponse)

      const result = await getOrders()

      expect(mockClient.get).toHaveBeenCalledWith('/orders', { params: undefined })
      expect(result).toEqual(mockResponse)
    })

    it('should GET /orders with query params', async () => {
      mockClient.get.mockResolvedValue({ items: [], total: 0, page: 1, size: 20 })

      await getOrders({ status: 'active', symbol: 'BTC/USDT', page: 1, size: 20 })

      expect(mockClient.get).toHaveBeenCalledWith('/orders', {
        params: { status: 'active', symbol: 'BTC/USDT', page: 1, size: 20 },
      })
    })
  })

  describe('getOrder', () => {
    it('should GET /orders/:id', async () => {
      mockClient.get.mockResolvedValue({})

      await getOrder('123456')

      expect(mockClient.get).toHaveBeenCalledWith('/orders/123456')
    })
  })

  describe('cancelOrder', () => {
    it('should POST /orders/:id/cancel', async () => {
      mockClient.post.mockResolvedValue({})

      await cancelOrder('123456')

      expect(mockClient.post).toHaveBeenCalledWith('/orders/123456/cancel')
    })
  })

  describe('cancelAllOrders', () => {
    it('should POST /orders/cancel-all with filters', async () => {
      mockClient.post.mockResolvedValue({})

      await cancelAllOrders({ symbol: 'BTC/USDT' })

      expect(mockClient.post).toHaveBeenCalledWith('/orders/cancel-all', { symbol: 'BTC/USDT' })
    })

    it('should POST /orders/cancel-all without filters', async () => {
      mockClient.post.mockResolvedValue({})

      await cancelAllOrders()

      expect(mockClient.post).toHaveBeenCalledWith('/orders/cancel-all', undefined)
    })
  })

  describe('getAccount', () => {
    it('should GET /account', async () => {
      const mockAccount: PaperAccount = {
        user_id: 1,
        balance: '55000.00',
        frozen_balance: '4950.00',
        initial_balance: '100000.00',
        total_pnl: '-40450.00',
        equity: '60050.00',
        positions_count: 3,
        active_orders_count: 5,
      }
      mockClient.get.mockResolvedValue(mockAccount)

      const result = await getAccount()

      expect(mockClient.get).toHaveBeenCalledWith('/account')
      expect(result).toEqual(mockAccount)
    })
  })

  describe('getSymbols', () => {
    it('should GET /symbols', async () => {
      const mockSymbols: { items: SymbolConfig[] } = {
        items: [{
          symbol: 'BTC/USDT',
          base_currency: 'BTC',
          quote_currency: 'USDT',
          price_precision: 2,
          quantity_precision: 4,
          min_quantity: '0.0010',
          max_quantity: '1000.0000',
          min_notional: '10.00',
          fee_rate: '0.001000',
          enabled: true,
        }],
      }
      mockClient.get.mockResolvedValue(mockSymbols)

      const result = await getSymbols()

      expect(mockClient.get).toHaveBeenCalledWith('/symbols')
      expect(result).toEqual(mockSymbols)
    })
  })

  describe('getPositions', () => {
    it('should GET /positions', async () => {
      mockClient.get.mockResolvedValue([])

      await getPositions()

      expect(mockClient.get).toHaveBeenCalledWith('/positions')
    })
  })

  describe('closePosition', () => {
    it('should POST /positions/:id/close', async () => {
      mockClient.post.mockResolvedValue(undefined)

      await closePosition(42)

      expect(mockClient.post).toHaveBeenCalledWith('/positions/42/close')
    })
  })
})
