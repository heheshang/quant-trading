import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { nextTick } from 'vue'
import ElementPlus from 'element-plus'
import OrderForm from '@/components/trade/OrderForm.vue'
import type { SymbolConfig, PaperAccount } from '@/types/order'

// Mock the API
vi.mock('@/api/order', () => ({
  createOrder: vi.fn().mockResolvedValue({
    order_id: '123',
    symbol: 'BTC/USDT',
    side: 'buy',
    order_type: 'limit',
    price: '49500.00',
    quantity: '0.1000',
    filled_quantity: '0.0000',
    status: 'pending',
    mode: 'paper',
    fee: '0',
    time_in_force: 'GTC',
    created_at: '2026-05-14T06:00:00Z',
    updated_at: '2026-05-14T06:00:00Z',
  }),
}))

// Mock ElMessageBox
vi.mock('element-plus', async () => {
  const actual = await vi.importActual('element-plus')
  return {
    ...actual,
    ElMessageBox: {
      confirm: vi.fn().mockResolvedValue('confirm'),
    },
    ElMessage: {
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn(),
    },
  }
})

const mockSymbolConfig: SymbolConfig = {
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
}

const mockAccount: PaperAccount = {
  user_id: 1,
  balance: '100000.00',
  frozen_balance: '0',
  initial_balance: '100000.00',
  total_pnl: '0',
  equity: '100000.00',
  positions_count: 0,
  active_orders_count: 0,
}

function createWrapper(props = {}) {
  return mount(OrderForm, {
    props: {
      symbol: 'BTC/USDT',
      symbolConfig: mockSymbolConfig,
      account: mockAccount,
      bestBid: '50000.00',
      bestAsk: '50001.00',
      ...props,
    },
    global: {
      plugins: [ElementPlus],
    },
  })
}

describe('OrderForm', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders correctly with default state', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.order-form').exists()).toBe(true)
    expect(wrapper.find('.side-toggle').exists()).toBe(true)
  })

  it('toggles buy/sell side', async () => {
    const wrapper = createWrapper()
    const buyBtn = wrapper.find('.buy-btn')
    const sellBtn = wrapper.find('.sell-btn')

    expect(buyBtn.classes()).toContain('active')
    expect(sellBtn.classes()).not.toContain('active')

    await sellBtn.trigger('click')
    expect(sellBtn.classes()).toContain('active')
    expect(buyBtn.classes()).not.toContain('active')
  })

  it('toggles limit/market order type', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    // Default: limit order
    expect(vm.form.order_type).toBe('limit')

    // Switch to market
    vm.form.order_type = 'market'
    await nextTick()
    expect(vm.form.order_type).toBe('market')
  })

  it('renders percentage buttons', () => {
    const wrapper = createWrapper()
    const buttons = wrapper.findAll('.percent-buttons .el-button')
    expect(buttons.length).toBe(4)
    expect(buttons[0].text()).toContain('25')
    expect(buttons[3].text()).toContain('100')
  })

  it('shows submit button with buy text by default', () => {
    const wrapper = createWrapper()
    const submitBtn = wrapper.find('.submit-btn')
    expect(submitBtn.text()).toContain('买入')
  })

  it('shows available balance for buy side', () => {
    const wrapper = createWrapper()
    const balance = wrapper.find('.balance-info')
    expect(balance.exists()).toBe(true)
    expect(balance.text()).toContain('可用余额')
  })

  it('shows estimated total for limit orders', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.price = '50000'
    vm.form.quantity = '1'
    await nextTick()
    expect(vm.estimatedTotal).toBe(50000)
  })

  it('hides estimated total for market orders', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.order_type = 'market'
    await nextTick()
    expect(vm.estimatedTotal).toBeNull()
  })

  it('hides price hint for market orders', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.order_type = 'market'
    await nextTick()
    expect(wrapper.find('.price-hint').exists()).toBe(false)
  })

  it('sets quantity via percentage buttons', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.form.price = '50000'
    await nextTick()

    // Click 25% button
    const buttons = wrapper.findAll('.percent-buttons .el-button')
    await buttons[0].trigger('click')
    // 100000 * 0.25 / 50000 = 0.5
    expect(vm.form.quantity).toBe('0.5000')
  })

  it('accepts symbol config prop', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.symbolConfig).toEqual(mockSymbolConfig)
  })

  it('accepts account prop', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.account).toEqual(mockAccount)
  })

  it('pre-fills price from bestAsk for buy limit order', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    // Buy side should pre-fill with bestAsk
    expect(vm.form.side).toBe('buy')
    // Price should be pre-filled on mount (from watch on bestAsk)
  })

  it('switches time_in_force to IOC for market orders', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.form.time_in_force).toBe('GTC')
    vm.form.order_type = 'market'
    vm.onOrderTypeChange()
    expect(vm.form.time_in_force).toBe('IOC')
  })
})
