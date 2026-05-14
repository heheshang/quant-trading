import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import OrderCreateDialog from '@/components/order/OrderCreateDialog.vue'
import type { SymbolConfig, PaperAccount } from '@/types/order'

const router = createRouter({
  history: createWebHistory(),
  routes: [{ path: '/', component: { template: '<div />' } }],
})

vi.mock('@/api/order', () => ({
  createOrder: vi.fn().mockResolvedValue({
    order_id: '999',
    symbol: 'BTC/USDT',
    side: 'buy',
    order_type: 'limit',
    price: '49500.00',
    quantity: '0.1000',
    filled_quantity: '0',
    avg_fill_price: null,
    status: 'pending',
    mode: 'paper',
    fee: '0',
    time_in_force: 'GTC',
    created_at: '2026-05-14T06:00:00Z',
    updated_at: '2026-05-14T06:00:00Z',
  }),
}))

function mockSymbolConfig(overrides?: Partial<SymbolConfig>): SymbolConfig {
  return {
    symbol: 'BTC/USDT',
    base_currency: 'BTC',
    quote_currency: 'USDT',
    price_precision: 2,
    quantity_precision: 4,
    min_quantity: '0.001',
    max_quantity: '1000',
    min_notional: '10',
    fee_rate: '0.001',
    enabled: true,
    ...overrides,
  }
}

function mockAccount(overrides?: Partial<PaperAccount>): PaperAccount {
  return {
    user_id: 1,
    balance: '55050.00',
    frozen_balance: '4950.00',
    initial_balance: '100000.00',
    total_pnl: '-40450.00',
    equity: '60050.00',
    positions_count: 3,
    active_orders_count: 5,
    ...overrides,
  }
}

function createWrapper(props = {}) {
  return mount(OrderCreateDialog, {
    props: {
      modelValue: true,
      symbolConfigs: [mockSymbolConfig()],
      account: mockAccount(),
      availablePositionQty: '0.3000',
      bestBid: '49400.00',
      bestAsk: '49500.00',
      mode: 'paper',
      ...props,
    },
    attachTo: document.body,
    global: {
      plugins: [ElementPlus, router],
    },
  })
}

describe('OrderCreateDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ===== Computed properties (test via vm) =====
  it('should compute enabledSymbols from symbolConfigs', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.enabledSymbols.length).toBe(1)
    expect(vm.enabledSymbols[0].symbol).toBe('BTC/USDT')
  })

  it('should exclude disabled symbols', () => {
    const wrapper = createWrapper({
      symbolConfigs: [
        mockSymbolConfig({ enabled: true }),
        mockSymbolConfig({ symbol: 'SOL/USDT', enabled: false }),
      ],
    })
    const vm = wrapper.vm as any
    expect(vm.enabledSymbols.length).toBe(1)
  })

  it('should compute baseCurrency and quoteCurrency', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'BTC/USDT'
    expect(vm.baseCurrency).toBe('BTC')
    expect(vm.quoteCurrency).toBe('USDT')
  })

  it('should compute estimatedAmount for limit orders', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'BTC/USDT'
    vm.form.side = 'buy'
    vm.form.order_type = 'limit'
    vm.form.price = '49500.00'
    vm.form.quantity = '0.1000'
    expect(vm.estimatedAmount).toBeTruthy()
  })

  it('should compute estimatedAmount for market orders using best ask', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'BTC/USDT'
    vm.form.side = 'buy'
    vm.form.order_type = 'market'
    vm.form.quantity = '0.1000'
    expect(vm.estimatedAmount).toBeTruthy()
  })

  it('should return empty estimatedAmount when quantity is empty', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.quantity = ''
    expect(vm.estimatedAmount).toBe('')
  })

  it('should compute estimatedFee', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'BTC/USDT'
    vm.form.side = 'buy'
    vm.form.order_type = 'limit'
    vm.form.price = '49500.00'
    vm.form.quantity = '0.1000'
    expect(vm.estimatedFee).toBeTruthy()
  })

  it('should compute estimatedFreeze for buy limit orders', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'BTC/USDT'
    vm.form.side = 'buy'
    vm.form.order_type = 'limit'
    vm.form.price = '49500.00'
    vm.form.quantity = '0.1000'
    expect(vm.estimatedFreeze).toBeTruthy()
  })

  it('should not compute estimatedFreeze for sell orders', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.side = 'sell'
    expect(vm.estimatedFreeze).toBe('')
  })

  it('should compute priceDeviation from market price', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.order_type = 'limit'
    vm.form.side = 'buy'
    vm.form.price = '60000.00'
    expect(vm.priceDeviation).toBeGreaterThan(10)
  })

  it('should return 0 priceDeviation when no price set', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.order_type = 'limit'
    vm.form.price = ''
    expect(vm.priceDeviation).toBe(0)
  })

  it('should detect insufficient balance for buy orders', () => {
    const wrapper = createWrapper({ account: mockAccount({ balance: '100.00' }) })
    const vm = wrapper.vm as any
    vm.form.side = 'buy'
    vm.form.order_type = 'limit'
    vm.form.price = '49500.00'
    vm.form.quantity = '0.1000'
    expect(vm.isInsufficient).toBe(true)
  })

  it('should detect sufficient balance for buy orders', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.side = 'buy'
    vm.form.order_type = 'limit'
    vm.form.price = '49500.00'
    vm.form.quantity = '0.1000'
    expect(vm.isInsufficient).toBe(false)
  })

  it('should detect insufficient position for sell', () => {
    const wrapper = createWrapper({ availablePositionQty: '0.01' })
    const vm = wrapper.vm as any
    vm.form.side = 'sell'
    vm.form.quantity = '0.1000'
    expect(vm.isInsufficient).toBe(true)
  })

  it('should detect sufficient position for sell', () => {
    const wrapper = createWrapper({ availablePositionQty: '0.5000' })
    const vm = wrapper.vm as any
    vm.form.side = 'sell'
    vm.form.quantity = '0.1000'
    expect(vm.isInsufficient).toBe(false)
  })

  it('should disable submit when form is empty', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.canSubmit).toBe(false)
  })

  it('should enable submit when form is valid', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'BTC/USDT'
    vm.form.side = 'buy'
    vm.form.order_type = 'limit'
    vm.form.price = '49500.00'
    vm.form.quantity = '0.1000'
    expect(vm.canSubmit).toBe(true)
  })

  it('should disable submit when submitting', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'BTC/USDT'
    vm.form.side = 'buy'
    vm.form.order_type = 'limit'
    vm.form.price = '49500.00'
    vm.form.quantity = '0.1000'
    vm.submitting = true
    expect(vm.canSubmit).toBe(false)
  })

  it('should set quantity by percentage', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'BTC/USDT'
    vm.form.side = 'buy'
    vm.form.order_type = 'limit'
    vm.form.price = '49500.00'
    vm.setQuantityByPercent(50)
    expect(vm.form.quantity).toBeTruthy()
    expect(parseFloat(vm.form.quantity)).toBeGreaterThan(0)
  })

  it('should call createOrder and emit order-created on confirm submit', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'BTC/USDT'
    vm.form.side = 'buy'
    vm.form.order_type = 'limit'
    vm.form.price = '49500.00'
    vm.form.quantity = '0.1000'
    await flushPromises()
    await vm.onConfirmSubmit()
    await flushPromises()
    expect(wrapper.emitted('order-created')).toBeDefined()
  })

  it('should expose open() method', () => {
    const wrapper = createWrapper()
    expect(typeof (wrapper.vm as any).open).toBe('function')
  })

  it('should format money values correctly', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatMoney('55050.00')).toContain('55,050')
    expect(vm.formatMoney(null)).toBe('0.00')
    expect(vm.formatMoney(undefined)).toBe('0.00')
  })

  it('should clear price when switching to market order', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.order_type = 'limit'
    vm.form.price = '49500.00'
    // Switch to market
    vm.form.order_type = 'market'
    await flushPromises()
    expect(vm.form.price).toBe('')
  })

  it('should compute activePct as 0 without quantity', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.quantity = ''
    expect(vm.activePct).toBe(0)
  })

  it('should compute currentSymbolConfig', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'BTC/USDT'
    expect(vm.currentSymbolConfig).toBeTruthy()
    expect(vm.currentSymbolConfig.symbol).toBe('BTC/USDT')
  })

  it('should return undefined currentSymbolConfig for unknown symbol', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.symbol = 'UNKNOWN/USDT'
    expect(vm.currentSymbolConfig).toBeUndefined()
  })
})
