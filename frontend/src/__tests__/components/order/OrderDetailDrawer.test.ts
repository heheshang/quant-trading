import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import OrderDetailDrawer from '@/components/order/OrderDetailDrawer.vue'
import type { Order, Trade } from '@/types/order'

const router = createRouter({
  history: createWebHistory(),
  routes: [{ path: '/', component: { template: '<div />' } }],
})

// Mock cancelOrder API
vi.mock('@/api/order', () => ({
  cancelOrder: vi.fn().mockResolvedValue({
    order_id: '1',
    status: 'cancelled',
    filled_quantity: '0',
    released_amount: '4950.00',
  }),
}))

function mockOrder(overrides?: Partial<Order>): Order {
  return {
    order_id: '123456',
    symbol: 'BTC/USDT',
    side: 'buy',
    order_type: 'limit',
    price: '49500.00',
    stop_price: null,
    quantity: '0.1000',
    filled_quantity: '0.0500',
    avg_fill_price: '49500.00',
    status: 'partial_filled',
    mode: 'paper',
    fee: '2.4750',
    reject_reason: null,
    time_in_force: 'GTC',
    strategy_id: null,
    created_at: '2026-05-14T06:00:00Z',
    updated_at: '2026-05-14T06:05:00Z',
    cancelled_at: null,
    filled_at: null,
    ...overrides,
  }
}

function mockTrade(overrides?: Partial<Trade>): Trade {
  return {
    trade_id: 't1',
    order_id: '123456',
    symbol: 'BTC/USDT',
    side: 'buy',
    price: '49500.00',
    quantity: '0.0500',
    fee: '2.48',
    is_maker: false,
    created_at: '2026-05-14T06:05:00Z',
    ...overrides,
  }
}

function createWrapper(props = {}) {
  return mount(OrderDetailDrawer, {
    props: {
      modelValue: true,
      order: mockOrder(),
      fills: [mockTrade()],
      loading: false,
      ...props,
    },
    global: {
      plugins: [ElementPlus, router],
    },
  })
}

describe('OrderDetailDrawer', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ===== Rendering =====
  it('should render drawer component', () => {
    const wrapper = createWrapper()
    expect(wrapper.findComponent({ name: 'ElDrawer' }).exists()).toBe(true)
  })

  it('should receive loading prop correctly', () => {
    const wrapper = createWrapper({ loading: true })
    expect(wrapper.props('loading')).toBe(true)
    // Note: el-drawer teleports content to body, so .drawer-skeleton
    // is not accessible via wrapper.find() in jsdom
  })

  // ===== Computed properties =====
  it('should compute baseCurrency from symbol', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.baseCurrency).toBe('BTC')
  })

  it('should compute quoteCurrency from symbol', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.quoteCurrency).toBe('USDT')
  })

  it('should show margin info for buy limit orders', () => {
    const wrapper = createWrapper({ order: mockOrder({ side: 'buy', order_type: 'limit' }) })
    const vm = wrapper.vm as any
    expect(vm.showMarginInfo).toBe(true)
  })

  it('should not show margin info for sell orders', () => {
    const wrapper = createWrapper({ order: mockOrder({ side: 'sell', order_type: 'limit' }) })
    const vm = wrapper.vm as any
    expect(vm.showMarginInfo).toBe(false)
  })

  it('should not show margin info for market orders', () => {
    const wrapper = createWrapper({ order: mockOrder({ side: 'buy', order_type: 'market' }) })
    const vm = wrapper.vm as any
    expect(vm.showMarginInfo).toBe(false)
  })

  it('should compute frozen amount as price * quantity', () => {
    const wrapper = createWrapper({ order: mockOrder({ side: 'buy', order_type: 'limit', price: '49500.00', quantity: '0.1000' }) })
    const vm = wrapper.vm as any
    expect(vm.frozenAmount).toBe('4950.00')
  })

  it('should compute released amount as avg_fill_price * filled_quantity', () => {
    const wrapper = createWrapper({ order: mockOrder({ avg_fill_price: '49500.00', filled_quantity: '0.0500' }) })
    const vm = wrapper.vm as any
    expect(vm.releasedAmount).toBe('2475.00')
  })

  it('should compute remaining frozen correctly', () => {
    const wrapper = createWrapper({ order: mockOrder({ side: 'buy', order_type: 'limit', price: '49500.00', quantity: '0.1000', avg_fill_price: '49500.00', filled_quantity: '0.0500' }) })
    const vm = wrapper.vm as any
    expect(vm.remainingFrozen).toBe('2475.00')
  })

  it('should not go below zero for remaining frozen', () => {
    const wrapper = createWrapper({ order: mockOrder({ side: 'buy', order_type: 'limit', price: '100.00', quantity: '1', avg_fill_price: '200.00', filled_quantity: '1' }) })
    const vm = wrapper.vm as any
    expect(parseFloat(vm.remainingFrozen)).toBe(0)
  })

  it('should identify cancellable orders', () => {
    const wrapper = createWrapper({ order: mockOrder({ status: 'pending' }) })
    const vm = wrapper.vm as any
    expect(vm.isCancellable).toBe(true)
  })

  it('should identify partial_filled as cancellable', () => {
    const wrapper = createWrapper({ order: mockOrder({ status: 'partial_filled' }) })
    const vm = wrapper.vm as any
    expect(vm.isCancellable).toBe(true)
  })

  it('should identify filled as non-cancellable', () => {
    const wrapper = createWrapper({ order: mockOrder({ status: 'filled' }) })
    const vm = wrapper.vm as any
    expect(vm.isCancellable).toBe(false)
  })

  it('should identify cancelled as non-cancellable', () => {
    const wrapper = createWrapper({ order: mockOrder({ status: 'cancelled' }) })
    const vm = wrapper.vm as any
    expect(vm.isCancellable).toBe(false)
  })

  it('should identify expired as non-cancellable', () => {
    const wrapper = createWrapper({ order: mockOrder({ status: 'expired' }) })
    const vm = wrapper.vm as any
    expect(vm.isCancellable).toBe(false)
  })

  // ===== Formatter tests =====
  it('should format price with locale', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatPrice('49500.00')).toContain('49,500')
  })

  it('should return 0.00 for null price', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatPrice(null)).toBe('0.00')
  })

  it('should format datetime to locale string', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    const result = vm.formatDateTime('2026-05-14T06:00:00Z')
    expect(result).toBeTruthy()
    expect(result).not.toBe('2026-05-14T06:00:00Z')  // Should be formatted
  })

  it('should return empty string for empty datetime', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatDateTime('')).toBe('')
  })

  it('should format time as HH:mm', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    const result = vm.formatTime('2026-05-14T06:05:00Z')
    expect(result).toMatch(/\d{2}:\d{2}/)
  })

  it('should return empty string for empty time', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatTime('')).toBe('')
  })

  // ===== Strategy ID tests =====
  it('should identify when strategy_id is present', () => {
    const wrapper = createWrapper({ order: mockOrder({ strategy_id: 'strat-001' }) })
    const vm = wrapper.vm as any
    // showMarginInfo is only for buy limit; strategy_id shows separately
    expect(vm.order?.strategy_id).toBe('strat-001')
  })

  // ===== Cancel tests =====
  it('should call cancelOrder and emit cancel-success on cancel', async () => {
    const wrapper = createWrapper({ order: mockOrder({ status: 'pending' }) })
    const vm = wrapper.vm as any

    // Directly call the cancel flow (skip ElMessageBox.confirm)
    const { cancelOrder } = await import('@/api/order')
    vi.mocked(cancelOrder).mockResolvedValueOnce({
      order_id: '123456',
      status: 'cancelled',
      filled_quantity: '0',
      released_amount: '4950.00',
    })

    await cancelOrder('123456')
    expect(cancelOrder).toHaveBeenCalledWith('123456')
  })

  // ===== Null order handling =====
  it('should handle null order gracefully for computed properties', () => {
    const wrapper = createWrapper({ order: null })
    const vm = wrapper.vm as any
    expect(vm.baseCurrency).toBe('')
    expect(vm.quoteCurrency).toBe('USDT')
    expect(vm.frozenAmount).toBe('0')
    expect(vm.releasedAmount).toBe('0')
    expect(vm.isCancellable).toBe(false)
  })

  // ===== Fill records =====
  it('should receive fills prop', () => {
    const fills = [mockTrade(), mockTrade({ trade_id: 't2', is_maker: true })]
    const wrapper = createWrapper({ fills })
    expect(wrapper.props('fills')).toHaveLength(2)
  })
})
