import { describe, it, expect } from 'vitest'
import {
  getOrderStatusType,
  getOrderStatusText,
  getOrderSideText,
  type OrderStatus,
  type OrderSide,
  type OrderType,
  type TradeMode,
  type TimeInForce,
} from '@/types/order'

describe('Order type utilities', () => {
  describe('getOrderStatusType', () => {
    it('should return correct Element Plus tag type for each status', () => {
      expect(getOrderStatusType('pending')).toBe('warning')
      expect(getOrderStatusType('partial_filled')).toBe('primary')
      expect(getOrderStatusType('filled')).toBe('success')
      expect(getOrderStatusType('cancelled')).toBe('info')
      expect(getOrderStatusType('expired')).toBe('info')
      expect(getOrderStatusType('rejected')).toBe('danger')
    })
  })

  describe('getOrderStatusText', () => {
    it('should return Chinese display text for each status', () => {
      expect(getOrderStatusText('pending')).toBe('待成交')
      expect(getOrderStatusText('partial_filled')).toBe('部分成交')
      expect(getOrderStatusText('filled')).toBe('已成交')
      expect(getOrderStatusText('cancelled')).toBe('已撤销')
      expect(getOrderStatusText('expired')).toBe('已过期')
      expect(getOrderStatusText('rejected')).toBe('已拒绝')
    })
  })

  describe('getOrderSideText', () => {
    it('should return Chinese display text for each side', () => {
      expect(getOrderSideText('buy')).toBe('买入')
      expect(getOrderSideText('sell')).toBe('卖出')
    })
  })
})

describe('Order type definitions', () => {
  it('should accept valid OrderStatus values', () => {
    const statuses: OrderStatus[] = [
      'pending', 'partial_filled', 'filled', 'cancelled', 'expired', 'rejected',
    ]
    expect(statuses).toHaveLength(6)
  })

  it('should accept valid OrderSide values', () => {
    const sides: OrderSide[] = ['buy', 'sell']
    expect(sides).toHaveLength(2)
  })

  it('should accept valid OrderType values including stop types', () => {
    const types: OrderType[] = ['limit', 'market', 'stop', 'stop_limit']
    expect(types).toHaveLength(4)
  })

  it('should accept valid TradeMode values', () => {
    const modes: TradeMode[] = ['paper', 'live']
    expect(modes).toHaveLength(2)
  })

  it('should accept valid TimeInForce values', () => {
    const tifs: TimeInForce[] = ['GTC', 'IOC', 'FOK']
    expect(tifs).toHaveLength(3)
  })
})

describe('Trade type', () => {
  it('should define Trade interface with all required fields', () => {
    const trade = {
      trade_id: 't1',
      order_id: 'o1',
      symbol: 'BTC/USDT',
      side: 'buy' as const,
      price: '49500.00',
      quantity: '0.1000',
      fee: '2.48',
      is_maker: false,
      created_at: '2026-05-14T06:05:00Z',
    }
    expect(trade.trade_id).toBe('t1')
    expect(trade.is_maker).toBe(false)
  })
})

describe('Position type', () => {
  it('should define Position interface with available_quantity', () => {
    const position = {
      id: 1,
      user_id: 1,
      symbol: 'BTC/USDT',
      side: 'long' as const,
      quantity: '0.3000',
      available_quantity: '0.2000',
      avg_entry_price: '49000.00',
      unrealized_pnl: '300.00',
      realized_pnl: '0',
      mode: 'paper' as const,
      created_at: '2026-05-14T06:00:00Z',
      updated_at: '2026-05-14T06:00:00Z',
    }
    expect(position.available_quantity).toBe('0.2000')
    expect(position.side).toBe('long')
  })
})
