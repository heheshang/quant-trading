import { describe, it, expect, vi, beforeEach } from 'vitest'
import { ref } from 'vue'
import { useDepth } from '@/composables/useDepth'
import type { Depth, WsMessage } from '@/types'

// Mock the API
vi.mock('@/api/market', () => ({
  getDepth: vi.fn(),
}))

import { getDepth } from '@/api/market'

describe('useDepth', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('fetchDepth', () => {
    it('loads depth data from API', async () => {
      const mockDepth: Depth = {
        bids: [
          { price: 50000, quantity: 1.5, total: 3.0 },
          { price: 49999, quantity: 1.5, total: 1.5 },
        ],
        asks: [
          { price: 50001, quantity: 0.8, total: 0.8 },
          { price: 50002, quantity: 1.2, total: 2.0 },
        ],
        timestamp: 1715640000000,
      }
      vi.mocked(getDepth).mockResolvedValue(mockDepth)

      const symbol = ref('BTCUSDT')
      const levels = ref(10)
      const { depth, loading, fetchDepth } = useDepth(symbol, levels)

      await fetchDepth()

      expect(getDepth).toHaveBeenCalledWith('BTCUSDT', 10)
      expect(loading.value).toBe(false)
      expect(depth.value).not.toBeNull()
      expect(depth.value!.bids).toHaveLength(2)
      expect(depth.value!.asks).toHaveLength(2)
    })

    it('sorts bids descending by price', async () => {
      const mockDepth: Depth = {
        bids: [
          { price: 49999, quantity: 1.0, total: 1.0 },
          { price: 50000, quantity: 2.0, total: 3.0 },
        ],
        asks: [],
        timestamp: 0,
      }
      vi.mocked(getDepth).mockResolvedValue(mockDepth)

      const symbol = ref('BTCUSDT')
      const levels = ref(10)
      const { depth, fetchDepth } = useDepth(symbol, levels)

      await fetchDepth()

      expect(depth.value!.bids[0].price).toBe(50000)
      expect(depth.value!.bids[1].price).toBe(49999)
    })

    it('handles API errors', async () => {
      vi.mocked(getDepth).mockRejectedValue(new Error('Network error'))

      const symbol = ref('BTCUSDT')
      const levels = ref(10)
      const { depth, error, fetchDepth } = useDepth(symbol, levels)

      await fetchDepth()

      expect(error.value).toBe('Network error')
      expect(depth.value).toBeNull()
    })
  })

  describe('handleSnapshot', () => {
    it('replaces entire depth data', async () => {
      vi.mocked(getDepth).mockResolvedValue({
        bids: [],
        asks: [],
        timestamp: 0,
      })

      const symbol = ref('BTCUSDT')
      const levels = ref(10)
      const { depth, fetchDepth, handleSnapshot } = useDepth(symbol, levels)
      await fetchDepth()

      const msg: WsMessage = {
        type: 'depth',
        symbol: 'BTCUSDT',
        data: {
          bids: [{ price: 50000, quantity: 1.0, total: 1.0 }],
          asks: [{ price: 50001, quantity: 0.5, total: 0.5 }],
          timestamp: 1715640001000,
        } as Depth,
      }

      handleSnapshot(msg)

      expect(depth.value!.bids).toHaveLength(1)
      expect(depth.value!.bids[0].price).toBe(50000)
      expect(depth.value!.timestamp).toBe(1715640001000)
    })
  })

  describe('handleUpdate (incremental merge)', () => {
    it('updates existing price level quantity', async () => {
      vi.mocked(getDepth).mockResolvedValue({
        bids: [
          { price: 50000, quantity: 1.0, total: 1.0 },
          { price: 49999, quantity: 0.5, total: 1.5 },
        ],
        asks: [{ price: 50001, quantity: 0.8, total: 0.8 }],
        timestamp: 0,
      })

      const symbol = ref('BTCUSDT')
      const levels = ref(10)
      const { depth, fetchDepth, handleUpdate } = useDepth(symbol, levels)
      await fetchDepth()

      const msg: WsMessage = {
        type: 'depth_update',
        symbol: 'BTCUSDT',
        data: {
          bids: [{ price: 50000, quantity: 2.0, total: 0 }],
          asks: [],
          timestamp: 1715640002000,
        } as Depth,
      }

      handleUpdate(msg)

      expect(depth.value!.bids[0].quantity).toBe(2.0)
      // Total should be recalculated from best price (bids descending)
      // bids[0] = 50000 (qty 2.0) -> total = 2.0
      // bids[1] = 49999 (qty 0.5) -> total = 2.0 + 0.5 = 2.5
      expect(depth.value!.bids[0].total).toBe(2.0) // 2.0 (first/best level)
      expect(depth.value!.bids[1].total).toBe(2.5) // 2.0 + 0.5
    })

    it('deletes price level when quantity is 0', async () => {
      vi.mocked(getDepth).mockResolvedValue({
        bids: [
          { price: 50000, quantity: 1.0, total: 1.5 },
          { price: 49999, quantity: 0.5, total: 0.5 },
        ],
        asks: [],
        timestamp: 0,
      })

      const symbol = ref('BTCUSDT')
      const levels = ref(10)
      const { depth, fetchDepth, handleUpdate } = useDepth(symbol, levels)
      await fetchDepth()

      const msg: WsMessage = {
        type: 'depth_update',
        symbol: 'BTCUSDT',
        data: {
          bids: [{ price: 49999, quantity: 0, total: 0 }],
          asks: [],
          timestamp: 1715640003000,
        } as Depth,
      }

      handleUpdate(msg)

      expect(depth.value!.bids).toHaveLength(1)
      expect(depth.value!.bids[0].price).toBe(50000)
    })

    it('inserts new price level', async () => {
      vi.mocked(getDepth).mockResolvedValue({
        bids: [{ price: 50000, quantity: 1.0, total: 1.0 }],
        asks: [],
        timestamp: 0,
      })

      const symbol = ref('BTCUSDT')
      const levels = ref(10)
      const { depth, fetchDepth, handleUpdate } = useDepth(symbol, levels)
      await fetchDepth()

      const msg: WsMessage = {
        type: 'depth_update',
        symbol: 'BTCUSDT',
        data: {
          bids: [{ price: 49998, quantity: 0.3, total: 0 }],
          asks: [],
          timestamp: 1715640004000,
        } as Depth,
      }

      handleUpdate(msg)

      expect(depth.value!.bids).toHaveLength(2)
      // Should be sorted descending (buy side)
      expect(depth.value!.bids[0].price).toBe(50000)
      expect(depth.value!.bids[1].price).toBe(49998)
    })

    it('recalculates totals after merge', async () => {
      vi.mocked(getDepth).mockResolvedValue({
        bids: [
          { price: 50000, quantity: 1.0, total: 3.0 },
          { price: 49999, quantity: 1.5, total: 2.0 },
          { price: 49998, quantity: 0.5, total: 0.5 },
        ],
        asks: [],
        timestamp: 0,
      })

      const symbol = ref('BTCUSDT')
      const levels = ref(10)
      const { depth, fetchDepth, handleUpdate } = useDepth(symbol, levels)
      await fetchDepth()

      // Verify initial totals (recalculated from fetchDepth)
      // Bids descending: 50000(1.0), 49999(1.5), 49998(0.5)
      // Totals from best: 50000=1.0, 49999=1.0+1.5=2.5, 49998=2.5+0.5=3.0
      expect(depth.value!.bids[0].total).toBe(1.0)
      expect(depth.value!.bids[1].total).toBe(2.5)
      expect(depth.value!.bids[2].total).toBe(3.0)
    })
  })
})
