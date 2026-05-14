import { describe, it, expect } from 'vitest'
import { formatSymbol, SYMBOL_NAMES } from '@/types/market'
import type { Ticker, DepthLevel, Depth, WsMessage } from '@/types/market'

describe('market types (CR1/CR2)', () => {
  describe('Ticker type', () => {
    it('includes bid, ask, timestamp fields (CR1)', () => {
      const ticker: Ticker = {
        symbol: 'BTCUSDT',
        price: 50000,
        change: 150,
        change_percent: 0.3,
        volume: 123456,
        high: 51000,
        low: 49000,
        bid: 49999,      // CR1
        ask: 50001,      // CR1
        timestamp: 1715640000000,  // CR1
      }
      expect(ticker.bid).toBe(49999)
      expect(ticker.ask).toBe(50001)
      expect(ticker.timestamp).toBe(1715640000000)
    })
  })

  describe('DepthLevel type (CR2)', () => {
    it('uses structured DepthLevel objects instead of tuples', () => {
      const level: DepthLevel = {
        price: 50000,
        quantity: 1.5,
        total: 1.5,
      }
      expect(level.price).toBe(50000)
      expect(level.quantity).toBe(1.5)
      expect(level.total).toBe(1.5)
    })
  })

  describe('Depth type (CR2)', () => {
    it('uses DepthLevel[] for bids and asks', () => {
      const depth: Depth = {
        bids: [{ price: 50000, quantity: 1.5, total: 1.5 }],
        asks: [{ price: 50001, quantity: 0.8, total: 0.8 }],
        timestamp: 1715640000000,
      }
      expect(depth.bids[0].price).toBe(50000)
      expect(depth.asks[0].quantity).toBe(0.8)
    })
  })

  describe('WsMessage type', () => {
    it('supports all WS message types', () => {
      const tickerMsg: WsMessage = { type: 'ticker', symbol: 'BTCUSDT', data: undefined }
      const depthMsg: WsMessage = { type: 'depth', symbol: 'BTCUSDT' }
      const updateMsg: WsMessage = { type: 'depth_update', symbol: 'BTCUSDT' }
      const heartbeat: WsMessage = { type: 'heartbeat' }
      const kick: WsMessage = { type: 'kick', reason: 'too many connections' }
      const error: WsMessage = { type: 'error', code: 40001, message: 'invalid channel' }

      expect(tickerMsg.type).toBe('ticker')
      expect(depthMsg.type).toBe('depth')
      expect(updateMsg.type).toBe('depth_update')
      expect(heartbeat.type).toBe('heartbeat')
      expect(kick.type).toBe('kick')
      expect(error.type).toBe('error')
    })
  })
})

describe('formatSymbol (D3)', () => {
  it('formats BTCUSDT to BTC/USDT', () => {
    expect(formatSymbol('BTCUSDT')).toBe('BTC/USDT')
  })

  it('formats ETHUSDT to ETH/USDT', () => {
    expect(formatSymbol('ETHUSDT')).toBe('ETH/USDT')
  })

  it('formats ETHBTC to ETH/BTC', () => {
    expect(formatSymbol('ETHBTC')).toBe('ETH/BTC')
  })

  it('returns unchanged if no known quote currency', () => {
    expect(formatSymbol('UNKNOWN')).toBe('UNKNOWN')
  })
})

describe('SYMBOL_NAMES', () => {
  it('contains BTC and ETH names', () => {
    expect(SYMBOL_NAMES.BTCUSDT).toBe('Bitcoin')
    expect(SYMBOL_NAMES.ETHUSDT).toBe('Ethereum')
  })
})
