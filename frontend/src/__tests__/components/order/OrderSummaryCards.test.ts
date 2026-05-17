import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import OrderSummaryCards from '@/components/order/OrderSummaryCards.vue'
import type { PaperAccount } from '@/types/order'

const router = createRouter({
  history: createWebHistory(),
  routes: [{ path: '/', component: { template: '<div />' } }],
})

/** Create a mock PaperAccount */
function mockAccount(overrides?: Partial<PaperAccount>): PaperAccount {
  return {
    user_id: '1',
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
  return mount(OrderSummaryCards, {
    props: {
      account: null,
      ...props,
    },
    global: {
      plugins: [ElementPlus, router],
    },
  })
}

describe('OrderSummaryCards', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should render four summary cards', () => {
    const wrapper = createWrapper()
    const cards = wrapper.findAll('.summary-card')
    expect(cards).toHaveLength(4)
  })

  it('should display active orders count', () => {
    const wrapper = createWrapper({ activeOrdersCount: 7 })
    expect(wrapper.text()).toContain('7')
  })

  it('should display today fills count', () => {
    const wrapper = createWrapper({ todayFillsCount: 12 })
    expect(wrapper.text()).toContain('12')
  })

  it('should display frozen balance from account', () => {
    const wrapper = createWrapper({ account: mockAccount() })
    expect(wrapper.text()).toContain('4,950.00')
  })

  it('should display available balance from account', () => {
    const wrapper = createWrapper({ account: mockAccount() })
    expect(wrapper.text()).toContain('55,050.00')
  })

  it('should display equity in available balance sub-text', () => {
    const wrapper = createWrapper({ account: mockAccount() })
    expect(wrapper.text()).toContain('60,050.00')
  })

  it('should show loading skeleton when loading prop is true', () => {
    const wrapper = createWrapper({ loading: true })
    expect(wrapper.findAll('.skeleton-line').length).toBeGreaterThan(0)
  })

  it('should not show values when loading', () => {
    const wrapper = createWrapper({ loading: true, activeOrdersCount: 5 })
    // Values should be hidden behind v-if="loading"
    const valueEls = wrapper.findAll('.summary-card__value')
    // No value elements should be rendered
    expect(valueEls.length).toBe(0)
  })

  it('should display frozen percent of equity', () => {
    const wrapper = createWrapper({ account: mockAccount() })
    // 4950 / 60050 * 100 ≈ 8.2
    expect(wrapper.text()).toContain('8.2')
  })

  it('should display margin frozen sub-text when provided', () => {
    const wrapper = createWrapper({ marginFrozen: '4950.00' })
    expect(wrapper.text()).toContain('4,950.00')
  })

  it('should handle zero balance gracefully', () => {
    const wrapper = createWrapper({
      account: mockAccount({ balance: '0', frozen_balance: '0', equity: '0' }),
    })
    expect(wrapper.text()).toContain('0.00')
  })

  it('should handle null account gracefully', () => {
    const wrapper = createWrapper({ account: null })
    expect(wrapper.text()).toContain('0.00')
  })

  it('should apply --filled class to today fills value', () => {
    const wrapper = createWrapper({ todayFillsCount: 5 })
    const filledValue = wrapper.find('.summary-card__value--filled')
    expect(filledValue.exists()).toBe(true)
  })

  it('should apply --frozen class to frozen balance value', () => {
    const wrapper = createWrapper({ account: mockAccount() })
    const frozenValue = wrapper.find('.summary-card__value--frozen')
    expect(frozenValue.exists()).toBe(true)
  })
})
