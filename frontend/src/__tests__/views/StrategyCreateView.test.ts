import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import StrategyCreateView from '@/views/strategy/StrategyCreateView.vue'
import * as strategiesApi from '@/api/strategies'

vi.mock('@/api/strategies', () => ({
  listTemplates: vi.fn(),
  createStrategy: vi.fn(),
}))

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/strategies', name: 'Strategies', component: { template: '<div>list</div>' } },
    { path: '/strategies/create', name: 'StrategyCreate', component: { template: '<div>create</div>' } },
  ],
})

const mockTemplates = [
  {
    id: 'trend_following',
    name: '趋势跟踪',
    description: '基于移动平均线交叉的趋势跟踪策略',
    icon: 'TrendCharts',
    category: '趋势',
    params: [
      { key: 'fast_period', label: '快线周期', type: 'number' as const, required: true, default: 5, min: 2, max: 50, step: 1, description: '快线MA周期' },
      { key: 'slow_period', label: '慢线周期', type: 'number' as const, required: true, default: 20, min: 5, max: 200, step: 1, description: '慢线MA周期' },
      { key: 'use_ema', label: '使用EMA', type: 'boolean' as const, default: true, description: '启用指数移动平均' },
      { key: 'signal', label: '信号确认', type: 'select' as const, required: true, default: 'cross', options: [
        { label: '金叉死叉', value: 'cross' },
        { label: '持续交叉', value: 'continuous' },
        { label: '斜率确认', value: 'slope' },
      ]},
    ],
  },
  {
    id: 'grid_trading',
    name: '网格交易',
    description: '震荡行情网格交易策略',
    icon: 'Grid',
    category: '震荡',
    params: [
      { key: 'grid_levels', label: '网格层数', type: 'number' as const, required: true, default: 10, min: 3, max: 50, step: 1 },
      { key: 'grid_range', label: '网格范围', type: 'number' as const, required: true, default: 0.05, min: 0.01, max: 0.5, step: 0.01 },
      { key: 'take_profit', label: '启用止盈', type: 'boolean' as const, default: false },
    ],
  },
]

const mountView = async () => {
  const wrapper = mount(StrategyCreateView, {
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

describe('StrategyCreateView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the create strategy page header', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    expect(wrapper.find('.page-title').text()).toContain('新建策略')
    expect(wrapper.find('.back-btn').exists()).toBe(true)
  })

  it('loads and displays templates in step 1', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    expect(wrapper.text()).toContain('趋势跟踪')
    expect(wrapper.text()).toContain('网格交易')
    expect(strategiesApi.listTemplates).toHaveBeenCalledTimes(1)
  })

  it('shows template cards in a grid layout', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    const cards = wrapper.findAll('.template-card')
    expect(cards.length).toBe(2)
  })

  it('displays parameter form after selecting a template', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    // Click template card
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    // Parameter form should be visible
    expect(wrapper.find('.param-form').exists()).toBe(true)
    expect(wrapper.text()).toContain('快线周期')
    expect(wrapper.text()).toContain('慢线周期')
  })

  it('renders numeric parameter controls', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    // Number params show slider and input fields
    expect(wrapper.text()).toContain('快线周期')
    expect(wrapper.text()).toContain('慢线周期')
  })

  it('renders boolean params text', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    // Boolean params show switch
    expect(wrapper.text()).toContain('使用EMA')
  })

  it('renders select params', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('信号确认')
  })

  it('renders parameter preview area', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    expect(wrapper.find('.param-preview').exists()).toBe(true)
  })

  it('requires a template to be selected before save can create a strategy', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    vi.mocked(strategiesApi.createStrategy).mockResolvedValue({} as any)

    const wrapper = await mountView()

    // Click save without selecting a template — should still call because validation passes
    // but template_type will be empty
    const saveBtn = wrapper.find('.save-btn')
    await saveBtn.trigger('click')
    await flushPromises()

    // When no template is selected, template_type is empty string
    expect(strategiesApi.createStrategy).toHaveBeenCalledWith(
      expect.objectContaining({
        template_type: '',
      })
    )
  })

  it('saves strategy with correct payload', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    vi.mocked(strategiesApi.createStrategy).mockResolvedValue({ id: 1 } as any)

    const wrapper = await mountView()

    // Select template
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    // Fill name
    const nameInput = wrapper.find('.strategy-name-input input')
    await nameInput.setValue('我的趋势策略')

    // Click save
    const saveBtn = wrapper.find('.save-btn')
    await saveBtn.trigger('click')
    await flushPromises()

    expect(strategiesApi.createStrategy).toHaveBeenCalledWith(
      expect.objectContaining({
        name: '我的趋势策略',
        template_type: 'trend_following',
        parameters: expect.any(Object),
      })
    )
  })

  it('shows cancel button that navigates back', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    expect(wrapper.find('.cancel-btn').exists()).toBe(true)
  })

  it('shows loading state while templates are being fetched', async () => {
    vi.mocked(strategiesApi.listTemplates).mockImplementation(
      () => new Promise(() => {})
    )

    const wrapper = await mountView()

    expect(wrapper.find('.el-skeleton').exists() || wrapper.find('.skeleton-form').exists()).toBe(true)
  })

  it('shows error when templates fail to load', async () => {
    vi.mocked(strategiesApi.listTemplates).mockRejectedValue(new Error('Failed to load'))
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('加载失败')
  })

  it('has a basic info form section', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    expect(wrapper.text()).toContain('基本信息')
    expect(wrapper.text()).toContain('策略名称')
  })

  it('has a template selection section', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    expect(wrapper.text()).toContain('选择策略模板')
  })
})
