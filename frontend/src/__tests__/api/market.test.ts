import { describe, it, expect, vi, beforeEach } from 'vitest'
import { getTickers, getTicker, getDepth, getKline } from '@/api/market'
import client from '@/api/client'

// Mock the client
vi.mock('@/api/client', () => ({
  default: {
    get: vi.fn(),
  },
}))

describe('market API', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('getTickers', () => {
    it('calls GET /market/tickers', async () => {
      const mockData = [
        { symbol: 'BTCUSDT', price: 50000, change: 150, change_percent: 0.3, volume: 123456, high: 51000, low: 49000, bid: 49999, ask: 50001, timestamp: 1715640000000 },
      ]
      vi.mocked(client.get).mockResolvedValue(mockData)

      const result = await getTickers()
      expect(client.get).toHaveBeenCalledWith('/market/tickers')
      expect(result).toEqual(mockData)
    })
  })

  describe('getTicker', () => {
    it('calls GET /market/ticker with symbol param', async () => {
      const mockData = { symbol: 'BTCUSDT', price: 50000 }
      vi.mocked(client.get).mockResolvedValue(mockData)

      const result = await getTicker('BTCUSDT')
      expect(client.get).toHaveBeenCalledWith('/market/ticker', { params: { symbol: 'BTCUSDT' } })
      expect(result).toEqual(mockData)
    })
  })

  describe('getDepth', () => {
    it('calls GET /market/depth with symbol and levels params (CR3)', async () => {
      const mockData = {
        bids: [{ price: 50000, quantity: 1.5, total: 1.5 }],
        asks: [{ price: 50001, quantity: 0.8, total: 0.8 }],
        timestamp: 1715640000000,
      }
      vi.mocked(client.get).mockResolvedValue(mockData)

      const result = await getDepth('BTCUSDT', 10)
      expect(client.get).toHaveBeenCalledWith('/market/depth', { params: { symbol: 'BTCUSDT', levels: 10 } })
      expect(result).toEqual(mockData)
    })

    it('defaults levels to 10', async () => {
      vi.mocked(client.get).mockResolvedValue({ bids: [], asks: [], timestamp: 0 })

      await getDepth('BTCUSDT')
      expect(client.get).toHaveBeenCalledWith('/market/depth', { params: { symbol: 'BTCUSDT', levels: 10 } })
    })
  })

  describe('getKline', () => {
    it('calls GET /market/kline with symbol and interval', async () => {
      vi.mocked(client.get).mockResolvedValue([])

      await getKline('BTCUSDT', '1d')
      expect(client.get).toHaveBeenCalledWith('/market/kline', { params: { symbol: 'BTCUSDT', interval: '1d' } })
    })
  })
})
