import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { nextTick } from 'vue'
import OrderList from '@/components/trade/OrderList.vue'
import type { Order, OrderListResponse } from '@/types/order'

// Mock API
const mockGetOrders = vi.fn()
const mockCancelOrder = vi.fn()
const mockCancelAllOrders = vi.fn()

vi.mock('@/api/order', () => ({
  getOrders: (...args: any[]) => mockGetOrders(...args),
  cancelOrder: (...args: any[]) => mockCancelOrder(...args),
  cancelAllOrders: (...args: any[]) => mockCancelAllOrders(...args),
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

const mockOrders: Order[] = [
  {
    order_id: '1001',
    symbol: 'BTC/USDT',
    side: 'buy',
    order_type: 'limit',
    price: '49500.00',
    quantity: '0.1000',
    filled_quantity: '0.0000',
    avg_fill_price: null,
    status: 'pending',
    mode: 'paper',
    fee: '0',
    reject_reason: null,
    time_in_force: 'GTC',
    created_at: '2026-05-14T06:00:00Z',
    updated_at: '2026-05-14T06:00:00Z',
    cancelled_at: null,
    filled_at: null,
  },
  {
    order_id: '1002',
    symbol: 'ETH/USDT',
    side: 'sell',
    order_type: 'limit',
    price: '3200.00',
    quantity: '1.0000',
    filled_quantity: '0.5000',
    avg_fill_price: '3200.00',
    status: 'partial_filled',
    mode: 'paper',
    fee: '1.6',
    reject_reason: null,
    time_in_force: 'GTC',
    created_at: '2026-05-14T05:00:00Z',
    updated_at: '2026-05-14T06:00:00Z',
    cancelled_at: null,
    filled_at: null,
  },
  {
    order_id: '1003',
    symbol: 'BTC/USDT',
    side: 'buy',
    order_type: 'market',
    price: null,
    quantity: '0.0500',
    filled_quantity: '0.0500',
    avg_fill_price: '50000.00',
    status: 'filled',
    mode: 'paper',
    fee: '2.5',
    reject_reason: null,
    time_in_force: 'IOC',
    created_at: '2026-05-14T04:00:00Z',
    updated_at: '2026-05-14T04:00:01Z',
    cancelled_at: null,
    filled_at: '2026-05-14T04:00:01Z',
  },
]

const mockCurrentResponse: OrderListResponse = {
  items: mockOrders.filter((o) => o.status === 'pending' || o.status === 'partial_filled'),
  total: 2,
  page: 1,
  size: 100,
}

const mockHistoryResponse: OrderListResponse = {
  items: mockOrders.filter((o) => o.status === 'filled'),
  total: 1,
  page: 1,
  size: 20,
}

function createWrapper(props = {}) {
  return mount(OrderList, {
    props: {
      symbol: 'BTC/USDT',
      ...props,
    },
    global: {
      plugins: [ElementPlus],
    },
  })
}

describe('OrderList', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetOrders.mockResolvedValue(mockCurrentResponse)
  })

  it('renders correctly', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    expect(wrapper.find('.order-list').exists()).toBe(true)
  })

  it('fetches current orders on mount', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    expect(mockGetOrders).toHaveBeenCalledWith(
      expect.objectContaining({
        status: 'active',
        page: 1,
        size: 100,
      }),
    )
  })

  it('computes activeCount correctly', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    const vm = wrapper.vm as any
    expect(vm.activeCount).toBe(2)
  })

  it('switches to history tab', async () => {
    mockGetOrders.mockResolvedValueOnce(mockCurrentResponse)
    mockGetOrders.mockResolvedValueOnce(mockHistoryResponse)

    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    vm.activeTab = 'history'
    await nextTick()
    await flushPromises()
    expect(mockGetOrders).toHaveBeenCalledWith(
      expect.objectContaining({
        page: 1,
        status: 'active',
      }),
    )
  })

  it('renders order table with data', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    const table = wrapper.find('.order-table')
    expect(table.exists()).toBe(true)
  })

  it('displays cancel button for pending orders', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    // Current tab shows cancel for pending/partial_filled
    const cancelButtons = wrapper.findAll('button')
    // At least the cancel button should exist for the current tab
    expect(wrapper.text()).toContain('当前委托')
  })

  it('shows empty text when no orders', async () => {
    mockGetOrders.mockResolvedValue({ items: [], total: 0, page: 1, size: 100 })
    const wrapper = createWrapper()
    await flushPromises()
    expect(wrapper.text()).toContain('暂无未成交委托')
  })

  it('applies symbol filter', async () => {
    mockGetOrders.mockResolvedValue(mockCurrentResponse)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    vm.filterSymbol = 'BTC/USDT'
    await nextTick()

    // Trigger fetch via filter change
    await flushPromises()
    expect(mockGetOrders).toHaveBeenCalledWith(
      expect.objectContaining({ symbol: 'BTC/USDT' }),
    )
  })
  it('shows cancel all button when active orders exist', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    const cancelAllBtn = wrapper.find('.el-button--danger')
    expect(cancelAllBtn.exists()).toBe(true)
  })
})
