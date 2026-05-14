import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import TradeRecordTab from '@/components/order/TradeRecordTab.vue'
import type { Trade } from '@/types/order'

const router = createRouter({
  history: createWebHistory(),
  routes: [{ path: '/', component: { template: '<div />' } }],
})

function mockTrade(overrides?: Partial<Trade>): Trade {
  return {
    trade_id: 't1',
    order_id: 'o1',
    symbol: 'BTC/USDT',
    side: 'buy',
    price: '49500.00',
    quantity: '0.0500',
    fee: '2.48',
    is_maker: false,
    created_at: '2026-05-14T06:05:12Z',
    ...overrides,
  }
}

function createWrapper(props = {}) {
  return mount(TradeRecordTab, {
    props: {
      trades: [],
      total: 0,
      page: 1,
      size: 20,
      loading: false,
      ...props,
    },
    global: {
      plugins: [ElementPlus, router],
    },
  })
}

describe('TradeRecordTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ===== Rendering =====
  it('should render filter bar', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.trade-filter').exists()).toBe(true)
  })

  it('should show loading skeleton when loading', () => {
    const wrapper = createWrapper({ loading: true })
    expect(wrapper.find('.skeleton-container').exists()).toBe(true)
  })

  it('should not show table when loading', () => {
    const wrapper = createWrapper({ loading: true })
    expect(wrapper.findComponent({ name: 'ElTable' }).exists()).toBe(false)
  })

  it('should render el-table when not loading', () => {
    const wrapper = createWrapper({ trades: [mockTrade()], total: 1 })
    expect(wrapper.findComponent({ name: 'ElTable' }).exists()).toBe(true)
  })

  it('should show pagination when total > 0', () => {
    const wrapper = createWrapper({ trades: [mockTrade()], total: 50 })
    expect(wrapper.find('.pagination-bar').exists()).toBe(true)
  })

  it('should hide pagination when total is 0', () => {
    const wrapper = createWrapper({ trades: [], total: 0 })
    expect(wrapper.find('.pagination-bar').exists()).toBe(false)
  })

  // ===== Formatter tests =====
  it('should format time for today as HH:mm:ss', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    const now = new Date()
    const todayIso = now.toISOString()
    const formatted = vm.formatTime(todayIso)
    expect(formatted).toMatch(/\d{2}:\d{2}:\d{2}/)
  })

  it('should format time for past dates as MM-DD HH:mm', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    const past = new Date('2026-05-01T10:30:00Z')
    const formatted = vm.formatTime(past.toISOString())
    expect(formatted).toMatch(/\d{2}-\d{2} \d{2}:\d{2}/)
  })

  it('should return empty string for empty time input', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatTime('')).toBe('')
  })

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

  // ===== Notional calculation =====
  it('should compute notional value (price * quantity)', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    const notional = vm.calcNotional(mockTrade())
    // 49500 * 0.05 = 2475
    expect(notional).toBe('2,475.00')
  })

  it('should handle NaN in notional calculation', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    const trade = { ...mockTrade(), price: 'invalid', quantity: '0.05' }
    expect(vm.calcNotional(trade)).toBe('0.00')
  })

  // ===== Events =====
  it('should emit row-click event', async () => {
    const trade = mockTrade()
    const wrapper = createWrapper({ trades: [trade], total: 1 })
    const vm = wrapper.vm as any
    vm.onRowClick(trade)
    expect(wrapper.emitted('row-click')).toBeTruthy()
    expect(wrapper.emitted('row-click')![0]).toEqual([trade])
  })

  it('should emit filter-change on filter change', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.onFilterChange()
    expect(wrapper.emitted('filter-change')).toBeDefined()
  })

  it('should emit page-change on page change', async () => {
    const wrapper = createWrapper({ trades: [mockTrade()], total: 50 })
    const vm = wrapper.vm as any
    vm.onPageChange(2)
    expect(wrapper.emitted('page-change')).toBeTruthy()
    expect(wrapper.emitted('page-change')![0]).toEqual([2, 20])
  })

  it('should emit page-change with size=1 on size change', async () => {
    const wrapper = createWrapper({ trades: [mockTrade()], total: 50 })
    const vm = wrapper.vm as any
    vm.onSizeChange(50)
    expect(wrapper.emitted('page-change')).toBeTruthy()
    expect(wrapper.emitted('page-change')![0]).toEqual([1, 50])
  })

  // ===== Props sync =====
  it('should sync page from props', async () => {
    const wrapper = createWrapper({ trades: [mockTrade()], total: 50, page: 3 })
    const vm = wrapper.vm as any
    expect(vm.currentPage).toBe(3)
  })

  it('should sync size from props', async () => {
    const wrapper = createWrapper({ trades: [mockTrade()], total: 50, size: 50 })
    const vm = wrapper.vm as any
    expect(vm.pageSize).toBe(50)
  })

  // ===== Table data =====
  it('should pass trades to el-table data prop', () => {
    const trades = [mockTrade(), mockTrade({ trade_id: 't2' })]
    const wrapper = createWrapper({ trades, total: 2 })
    const table = wrapper.findComponent({ name: 'ElTable' })
    expect(table.props('data')).toHaveLength(2)
  })

  // ===== Debounce =====
  it('should debounce symbol input', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vi.useFakeTimers()
    vm.onSymbolInput()
    // Should not emit immediately
    expect(wrapper.emitted('filter-change')).toBeUndefined()
    vi.advanceTimersByTime(300)
    expect(wrapper.emitted('filter-change')).toBeDefined()
    vi.useRealTimers()
  })
})
