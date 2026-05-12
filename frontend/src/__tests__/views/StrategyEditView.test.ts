import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory, useRoute } from 'vue-router'
import StrategyEditView from '@/views/strategy/StrategyEditView.vue'
import * as strategiesApi from '@/api/strategies'

// Mock route with params
vi.mock('vue-router', async () => {
  const actual = await vi.importActual('vue-router')
  return {
    ...actual,
    useRoute: vi.fn(),
  }
})

vi.mock('@/api/strategies', () => ({
  getStrategy: vi.fn(),
  listTemplates: vi.fn(),
  updateStrategy: vi.fn(),
  createStrategy: vi.fn(),
}))

const mockRoute = {
  params: { id: '1' },
  query: {},
  path: '/strategies/1/edit',
  name: 'StrategyEdit',
  fullPath: '/strategies/1/edit',
}

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/strategies', name: 'Strategies', component: { template: '<div>list</div>' } },
    { path: '/strategies/:id/edit', name: 'StrategyEdit', component: { template: '<div>edit</div>' } },
  ],
})

const mockStrategy = {
  id: '1',
  name: '均线趋势跟踪',
  user_id: 'user-1',
  description: '基于MA均线交叉的趋势跟踪策略',
  template_type: 'trend_following',
  parameters: { fast_period: 5, slow_period: 20, use_ema: true, signal: 'cross' },
  status: 'active' as const,
  created_at: '2026-05-01T08:00:00Z',
  updated_at: '2026-05-12T10:00:00Z',
}

const mockTemplates = [
  {
    id: 'trend_following',
    name: '趋势跟踪',
    description: '基于移动平均线交叉的趋势跟踪策略',
    category: '趋势',
    default_parameters: { fast_period: 5, slow_period: 20, use_ema: true, signal: 'cross' },
    parameter_schema: [
      { name: 'fast_period', label: '快线周期', type: 'integer' as const, default: 5, min: 2, max: 50 },
      { name: 'slow_period', label: '慢线周期', type: 'integer' as const, default: 20, min: 5, max: 200 },
      { name: 'use_ema', label: '使用EMA', type: 'boolean' as const, default: true },
      { name: 'signal', label: '信号确认', type: 'select' as const, default: 'cross', options: ['cross'] },
    ],
  },
]

const mountView = async () => {
  vi.mocked(useRoute).mockReturnValue(mockRoute as any)

  const wrapper = mount(StrategyEditView, {
    global: {
      plugins: [ElementPlus, router],
      stubs: {
        'router-link': { template: '<a class="router-link-stub"><slot /></a>' },
        'router-view': true,
      },
    },
  })
  await router.isReady()
  await flushPromises()
  return wrapper
}

describe('StrategyEditView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the edit strategy page header', async () => {
    vi.mocked(strategiesApi.getStrategy).mockResolvedValue(mockStrategy)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)

    const wrapper = await mountView()

    // The StrategyCreateView shows "编辑策略" when strategyId prop is set
    expect(wrapper.find('.page-title').exists()).toBe(true)
  })

  it('loads existing strategy data from API', async () => {
    vi.mocked(strategiesApi.getStrategy).mockResolvedValue(mockStrategy)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)

    await mountView()

    expect(strategiesApi.getStrategy).toHaveBeenCalledWith('1')
  })

  it('prefills form fields with existing strategy data', async () => {
    vi.mocked(strategiesApi.getStrategy).mockResolvedValue(mockStrategy)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)

    const wrapper = await mountView()

    // The name field shows the strategy name
    // StrategyCreateView gets strategyId prop, loads data, and fills name
    expect(wrapper.text()).toContain('均线趋势跟踪')
  })

  it('preselects the correct template based on strategy data', async () => {
    vi.mocked(strategiesApi.getStrategy).mockResolvedValue(mockStrategy)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)

    const wrapper = await mountView()
    await flushPromises()

    // The template for trend_following should be selected (has is-selected class)
    expect(wrapper.text()).toContain('趋势跟踪')
    expect(wrapper.text()).toContain('快线周期')
  })

  it('calls updateStrategy on save', async () => {
    vi.mocked(strategiesApi.getStrategy).mockResolvedValue(mockStrategy)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    vi.mocked(strategiesApi.updateStrategy).mockResolvedValue(mockStrategy)

    const wrapper = await mountView()
    await flushPromises()

    const saveBtn = wrapper.find('.save-btn')
    expect(saveBtn.exists()).toBe(true)
    await saveBtn.trigger('click')
    await flushPromises()

    expect(strategiesApi.updateStrategy).toHaveBeenCalledWith(
      '1',
      expect.objectContaining({
        name: '均线趋势跟踪',
      })
    )
  })

  it('shows loading state while fetching strategy data', async () => {
    vi.mocked(strategiesApi.getStrategy).mockImplementation(
      () => new Promise(() => {})
    )
    vi.mocked(strategiesApi.listTemplates).mockImplementation(
      () => new Promise(() => {})
    )

    const wrapper = await mountView()

    // Should show skeleton initially
    expect(wrapper.find('.el-skeleton').exists() || wrapper.find('.skeleton-form').exists()).toBe(true)
  })

  it('shows error when strategy data fails to load', async () => {
    vi.mocked(strategiesApi.getStrategy).mockRejectedValue(new Error('Not found'))
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)

    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('加载失败')
  })

  it('renders parameter fields from the selected template', async () => {
    vi.mocked(strategiesApi.getStrategy).mockResolvedValue(mockStrategy)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)

    const wrapper = await mountView()
    await flushPromises()

    // The template params should be rendered
    expect(wrapper.text()).toContain('快线周期')
    expect(wrapper.text()).toContain('慢线周期')
    expect(wrapper.text()).toContain('使用EMA')
    expect(wrapper.text()).toContain('信号确认')
  })
})
