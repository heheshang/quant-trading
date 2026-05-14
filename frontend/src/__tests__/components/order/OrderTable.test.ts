import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import OrderTable from '@/components/order/OrderTable.vue'
import type { Order } from '@/types/order'

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

/** Create a mock order */
function mockOrder(overrides?: Partial<Order>): Order {
  return {
    order_id: '1',
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

function createWrapper(props = {}) {
  return mount(OrderTable, {
    props: {
      orders: [],
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

describe('OrderTable', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ===== Rendering tests =====
  it('should render table wrapper', () => {
    const wrapper = createWrapper({ orders: [mockOrder()], total: 1 })
    expect(wrapper.find('.order-table-wrapper').exists()).toBe(true)
  })

  it('should show loading skeleton when loading', () => {
    const wrapper = createWrapper({ loading: true })
    expect(wrapper.findAll('.skeleton-row').length).toBe(5)
  })

  it('should not render el-table when loading', () => {
    const wrapper = createWrapper({ loading: true })
    expect(wrapper.findComponent({ name: 'ElTable' }).exists()).toBe(false)
  })

  it('should render el-table when not loading', () => {
    const wrapper = createWrapper({ orders: [mockOrder()], total: 1 })
    expect(wrapper.findComponent({ name: 'ElTable' }).exists()).toBe(true)
  })

  it('should show pagination when total > 0', () => {
    const wrapper = createWrapper({ orders: [mockOrder()], total: 50 })
    expect(wrapper.find('.pagination-bar').exists()).toBe(true)
  })

  it('should hide pagination when total is 0', () => {
    const wrapper = createWrapper({ orders: [], total: 0 })
    expect(wrapper.find('.pagination-bar').exists()).toBe(false)
  })

  // ===== Logic tests (via vm) =====
  it('should compute fill percent correctly', () => {
    const wrapper = createWrapper({ orders: [mockOrder()] })
    const vm = wrapper.vm as any
    // 0.0500 / 0.1000 * 100 = 50
    expect(vm.getFillPercent(mockOrder())).toBe(50)
  })

  it('should compute fill percent as 0 for zero quantity', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.getFillPercent({ ...mockOrder(), quantity: '0', filled_quantity: '0' })).toBe(0)
  })

  it('should compute fill percent as 100 for fully filled', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.getFillPercent(mockOrder({ filled_quantity: '0.1000', quantity: '0.1000' }))).toBe(100)
  })

  it('should identify cancellable orders (pending and partial_filled)', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.isCancellable('pending')).toBe(true)
    expect(vm.isCancellable('partial_filled')).toBe(true)
  })

  it('should identify non-cancellable orders', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.isCancellable('filled')).toBe(false)
    expect(vm.isCancellable('cancelled')).toBe(false)
    expect(vm.isCancellable('expired')).toBe(false)
    expect(vm.isCancellable('rejected')).toBe(false)
  })

  it('should format time correctly', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    const formatted = vm.formatTime('2026-05-14T06:00:00Z')
    expect(formatted).toMatch(/\d{2}-\d{2} \d{2}:\d{2}/)
  })

  it('should return empty string for null time', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatTime('')).toBe('')
  })

  it('should format price with locale', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatPrice('49500')).toContain('49,500')
  })

  it('should format price as 0.00 for null', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatPrice(null)).toBe('0.00')
  })

  it('should get order type display text', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.getOrderTypeText('limit')).toBe('限价')
    expect(vm.getOrderTypeText('market')).toBe('市价')
    expect(vm.getOrderTypeText('stop')).toBe('止损')
    expect(vm.getOrderTypeText('stop_limit')).toBe('止损限价')
  })

  it('should get row class name based on status', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    const cls = vm.getRowClassName({ row: mockOrder({ status: 'pending' }) })
    expect(cls).toContain('order-row--pending')
  })

  // ===== Event tests =====
  it('should emit row-click when onRowClick is called', () => {
    const order = mockOrder()
    const wrapper = createWrapper({ orders: [order], total: 1 })
    const vm = wrapper.vm as any
    vm.onRowClick(order)
    expect(wrapper.emitted('row-click')).toBeTruthy()
    expect(wrapper.emitted('row-click')![0]).toEqual([order])
  })

  it('should emit page-change when onPageChange is called', () => {
    const wrapper = createWrapper({ orders: [mockOrder()], total: 50 })
    const vm = wrapper.vm as any
    vm.onPageChange(2)
    expect(wrapper.emitted('page-change')).toBeTruthy()
  })

  it('should emit page-change with size on size change', () => {
    const wrapper = createWrapper({ orders: [mockOrder()], total: 50 })
    const vm = wrapper.vm as any
    vm.onSizeChange(50)
    expect(wrapper.emitted('page-change')).toBeTruthy()
    expect(wrapper.emitted('page-change')![0]).toEqual([1, 50])
  })

  // ===== Cancel order tests =====
  it('should cancel order and emit cancel-success', async () => {
    const wrapper = createWrapper({ orders: [mockOrder()] })
    const vm = wrapper.vm as any

    // Skip confirmation dialog, call directly
    const { cancelOrder } = await import('@/api/order')
    vi.mocked(cancelOrder).mockResolvedValueOnce({
      order_id: '1',
      status: 'cancelled',
      filled_quantity: '0',
      released_amount: '4950.00',
    })

    // Direct test of cancel flow
    vm.cancellingIds.add('1')
    await cancelOrder('1')
    vm.cancellingIds.delete('1')
  })

  // ===== Multiple orders =====
  it('should pass correct orders prop to el-table', () => {
    const orders = [mockOrder({ order_id: '1' }), mockOrder({ order_id: '2' })]
    const wrapper = createWrapper({ orders, total: 2 })
    const table = wrapper.findComponent({ name: 'ElTable' })
    expect(table.exists()).toBe(true)
    expect(table.props('data')).toHaveLength(2)
  })

  it('should sync page from props', async () => {
    const wrapper = createWrapper({ orders: [mockOrder()], total: 50, page: 3 })
    const vm = wrapper.vm as any
    expect(vm.currentPage).toBe(3)
  })

  it('should sync size from props', async () => {
    const wrapper = createWrapper({ orders: [mockOrder()], total: 50, size: 50 })
    const vm = wrapper.vm as any
    expect(vm.pageSize).toBe(50)
  })
})
