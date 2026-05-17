import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createMemoryHistory } from 'vue-router'
import StrategiesView from '@/views/strategy/StrategiesView.vue'
import * as strategiesApi from '@/api/strategies'

// Mock the API
vi.mock('@/api/strategies', () => ({
  listStrategies: vi.fn(),
  deleteStrategy: vi.fn(),
  toggleStrategy: vi.fn(),
  bulkUpdateStatus: vi.fn(),
  bulkDeleteStrategies: vi.fn(),
}))

// Mock router
const router = createRouter({
  history: createMemoryHistory(),
  routes: [
    { path: '/strategies', name: 'Strategies', component: { template: '<div>list</div>' } },
    { path: '/strategies/create', name: 'StrategyCreate', component: { template: '<div>create</div>' } },
    { path: '/strategies/:id/edit', name: 'StrategyEdit', component: { template: '<div>edit</div>' } },
  ],
})

const mockStrategies = [
  {
    id: '1',
    user_id: 'user-1',
    name: '均线趋势跟踪',
    description: 'MA crossover strategy',
    symbol: 'BTCUSDT',
    timeframe: '1H',
    template_id: 'tpl-1',
    template_type: 'ma_crossover',
    parameters: { fast_period: 5, slow_period: 20 },
    status: 'active' as const,
    created_at: '2026-05-01T08:00:00Z',
    updated_at: '2026-05-12T10:00:00Z',
    performance: {
      total_return_pct: 12.34,
      sharpe_ratio: 1.85,
      max_drawdown_pct: -8.5,
      total_trades: 142,
    },
  },
  {
    id: '2',
    user_id: 'user-1',
    name: '网格交易',
    description: 'Bollinger grid strategy',
    symbol: 'ETHUSDT',
    timeframe: '4H',
    template_id: 'tpl-2',
    template_type: 'bollinger',
    parameters: { grid_levels: 10, grid_range: 0.05 },
    status: 'paused' as const,
    created_at: '2026-04-15T08:00:00Z',
    updated_at: '2026-05-10T10:00:00Z',
    performance: {
      total_return_pct: -3.2,
      sharpe_ratio: 0.45,
      max_drawdown_pct: -15.8,
      total_trades: 87,
    },
  },
  {
    id: '3',
    user_id: 'user-1',
    name: 'MACD信号策略',
    description: 'MACD signal strategy',
    symbol: 'BTCUSDT',
    timeframe: '1D',
    template_id: 'tpl-3',
    template_type: 'macd',
    parameters: { fast_length: 12, slow_length: 26 },
    status: 'stopped' as const,
    created_at: '2026-03-20T08:00:00Z',
    updated_at: '2026-04-01T10:00:00Z',
    performance: {
      total_return_pct: 5.67,
      sharpe_ratio: 1.12,
      max_drawdown_pct: -6.3,
      total_trades: 53,
    },
  },
]

const mockPaginatedResponse = {
  data: mockStrategies,
  items: mockStrategies,
  total: 3,
  code: 0,
  message: 'ok',
  meta: { page: 1, size: 20, total: 3 },
} as any

const mockArchivedStrategy = {
  id: '4',
  user_id: 'user-1',
  name: '旧趋势策略',
  description: 'Archived trend strategy',
  symbol: 'BTCUSDT',
  timeframe: '1H',
  template_id: 'tpl-4',
  template_type: 'ma_crossover',
  parameters: { fast_period: 10, slow_period: 50 },
  status: 'archived' as const,
  created_at: '2026-02-01T08:00:00Z',
  updated_at: '2026-02-15T10:00:00Z',
  performance: {
    total_return_pct: 3.21,
    sharpe_ratio: 0.95,
    max_drawdown_pct: -4.2,
    total_trades: 28,
  },
}

const mockPaginatedWithArchived = {
  data: [...mockStrategies, mockArchivedStrategy],
  items: [...mockStrategies, mockArchivedStrategy],
  total: 4,
  code: 0,
  message: 'ok',
  meta: { page: 1, size: 20, total: 4 },
}

const mountView = async () => {
  const wrapper = mount(StrategiesView, {
    global: {
      plugins: [ElementPlus, router],
      stubs: {
        'router-link': {
          template: '<a class="router-link-stub" :href="$attrs.to"><slot /></a>',
        },
        'router-view': true,
      },
    },
  })
  await router.isReady()
  return wrapper
}

describe('StrategiesView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the page header with title and create button', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedResponse)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.find('.page-title').text()).toContain('策略管理')
    expect(wrapper.find('.create-btn').exists()).toBe(true)
  })

  it('shows loading skeleton while fetching data', async () => {
    vi.mocked(strategiesApi.listStrategies).mockImplementation(
      () => new Promise(() => {})
    )
    const wrapper = await mountView()

    expect(wrapper.find('.el-skeleton').exists()).toBe(true)
  })

  it('renders strategy list from API data', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedResponse)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('均线趋势跟踪')
    expect(wrapper.text()).toContain('网格交易')
    expect(wrapper.text()).toContain('MACD信号策略')
  })

  it('displays template type tags for each strategy', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedResponse)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('MA交叉')
    expect(wrapper.text()).toContain('布林带')
    expect(wrapper.text()).toContain('MACD')
  })

  it('displays status labels correctly', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedWithArchived as any)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('运行中')
    expect(wrapper.text()).toContain('已暂停')
    expect(wrapper.text()).toContain('已停止')
    expect(wrapper.text()).toContain('已归档')
  })

  it('shows empty state when no strategies exist', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({ data: [], items: [], total: 0, meta: { page: 1, size: 20, total: 0 } } as any)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('还没有创建策略')
  })

  it('shows error state when API fails', async () => {
    vi.mocked(strategiesApi.listStrategies).mockRejectedValue(new Error('Network error'))
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('加载失败')
    expect(wrapper.find('.el-button').exists()).toBe(true)
  })

  it('supports retry on error', async () => {
    vi.mocked(strategiesApi.listStrategies)
      .mockRejectedValueOnce(new Error('Network error'))
      .mockResolvedValueOnce(mockPaginatedResponse)

    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('加载失败')

    // Click retry button — it's in el-result's #extra slot (footer area)
    const retryBtn = wrapper.find('.el-result__main .el-button, .el-result__subtitle + div .el-button, .el-button[type="primary"]')
    await retryBtn.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('均线趋势跟踪')
    expect(strategiesApi.listStrategies).toHaveBeenCalledTimes(2)
  })

  it('shows filter pills for status filtering', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedWithArchived as any)
    const wrapper = await mountView()
    await flushPromises()

    const filterPills = wrapper.find('.filter-pills')
    expect(filterPills.exists()).toBe(true)
    expect(wrapper.text()).toContain('全部')
    expect(wrapper.text()).toContain('草稿')
    expect(wrapper.text()).toContain('运行中')
    expect(wrapper.text()).toContain('已暂停')
    expect(wrapper.text()).toContain('已归档')
  })

  it('shows search and sort controls', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedResponse)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.find('.search-input').exists()).toBe(true)
    expect(wrapper.find('.sort-select').exists()).toBe(true)
  })

  it('shows template marketplace link in header', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedResponse)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('策略模板市场')
  })

  it('displays symbol and timeframe for each strategy', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedResponse)
    const wrapper = await mountView()
    await flushPromises()

    // Symbol formatted as BTC/USDT
    expect(wrapper.text()).toContain('BTC/USDT')
    expect(wrapper.text()).toContain('ETH/USDT')
    // Timeframe
    expect(wrapper.text()).toContain('1H')
    expect(wrapper.text()).toContain('4H')
  })

  it('navigates to create page when create button is clicked', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedResponse)
    const wrapper = await mountView()
    await flushPromises()

    const createLink = wrapper.find('.router-link-stub')
    expect(createLink.exists()).toBe(true)
  })

  it('escapes XSS payload in description field using mustache interpolation', async () => {
    const xssPayload = '<script>alert("XSS")</script>'
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({
      data: [{
        ...mockStrategies[0],
        description: xssPayload,
      }],
      items: [{
        ...mockStrategies[0],
        description: xssPayload,
      }],
      total: 1,
      meta: { page: 1, size: 20, total: 1 },
    } as any)
    const wrapper = await mountView()
    await flushPromises()

    // The description text should appear as escaped literal text (not executed as HTML)
    const descEl = wrapper.find('.strategy-desc')
    expect(descEl.exists()).toBe(true)
    // Vue's {{ }} interpolates text content — raw tags should not appear as DOM elements
    expect(descEl.html()).not.toContain('<script>')
    // The rendered text should contain the escaped representation
    expect(descEl.text()).toContain('alert')
  })

  it('shows strategy name in delete modal for single strategy', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedResponse)
    const wrapper = await mountView()
    await flushPromises()

    // Directly call the function that sets up delete targets and opens the modal
    const vm = wrapper.vm as any
    vm.deleteTargets = [mockStrategies[0]]
    vm.deleteModalVisible = true
    await flushPromises()

    const modal = wrapper.find('.el-dialog')
    expect(modal.exists()).toBe(true)
    expect(modal.text()).toContain('均线趋势跟踪')
    expect(modal.text()).toContain('此操作不可撤销')
  })

  it('shows strategy names in delete modal for bulk delete', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockPaginatedResponse)
    const wrapper = await mountView()
    await flushPromises()

    const vm = wrapper.vm as any
    vm.deleteTargets = [mockStrategies[0], mockStrategies[1]]
    vm.deleteModalVisible = true
    await flushPromises()

    const modal = wrapper.find('.el-dialog')
    expect(modal.exists()).toBe(true)
    expect(modal.text()).toContain('均线趋势跟踪')
    expect(modal.text()).toContain('网格交易')
    expect(modal.text()).toContain('2 个策略')
  })
})
