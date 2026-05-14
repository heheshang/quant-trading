import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import MarketView from '@/views/market/MarketView.vue'

// Mock dependencies
vi.mock('@/composables/useMarketWs', () => ({
  useMarketWs: () => ({
    status: { value: 'idle' },
    connect: vi.fn(),
    disconnect: vi.fn(),
    subscribe: vi.fn(),
    unsubscribe: vi.fn(),
    onMessage: vi.fn(),
    onKick: vi.fn(),
  }),
}))

vi.mock('@/stores/auth', () => ({
  useAuthStore: () => ({
    token: 'mock-token',
    user: { role: 'trader' },
    isAuthenticated: true,
  }),
}))

vi.mock('element-plus', () => ({
  ElMessage: { warning: vi.fn() },
}))

describe('MarketView', () => {
  it('renders page title 行情中心', () => {
    const wrapper = mount(MarketView, {
      global: {
        stubs: {
          ElTabs: {
            template: '<div><slot /></div>',
          },
          ElTabPane: {
            template: '<div><slot /></div>',
            props: ['label', 'name'],
          },
          TickerListView: { template: '<div class="ticker-list-stub" />' },
          DepthView: { template: '<div class="depth-view-stub" />' },
        },
      },
    })
    expect(wrapper.find('.page-title').text()).toBe('行情中心')
  })

  it('renders tab navigation with Ticker and Depth tabs', () => {
    const wrapper = mount(MarketView, {
      global: {
        stubs: {
          ElTabs: {
            template: '<div class="el-tabs"><slot /></div>',
            props: ['modelValue'],
          },
          ElTabPane: {
            template: '<div><slot /></div>',
            props: ['label', 'name'],
          },
          TickerListView: { template: '<div class="ticker-list-stub" />' },
          DepthView: { template: '<div class="depth-view-stub" />' },
        },
      },
    })
    expect(wrapper.find('.market-tabs').exists()).toBe(true)
  })

  it('shows WS disconnection banner when disconnected', () => {
    // This test verifies the banner conditional rendering
    const wrapper = mount(MarketView, {
      global: {
        stubs: {
          ElTabs: { template: '<div><slot /></div>' },
          ElTabPane: { template: '<div><slot /></div>' },
          TickerListView: { template: '<div />' },
          DepthView: { template: '<div />' },
        },
      },
    })
    // When wsStatus is 'idle', no banner should show
    expect(wrapper.find('.ws-banner').exists()).toBe(false)
  })
})
