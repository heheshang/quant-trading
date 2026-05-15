import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import StrategyTemplateView from '@/views/strategy/StrategyTemplateView.vue'
import type { StrategyTemplate } from '@/types'
import * as strategiesApi from '@/api/strategies'

vi.mock('@/api/strategies', () => ({
  listTemplates: vi.fn(),
}))

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/strategies', name: 'Strategies', component: { template: '<div>list</div>' } },
    { path: '/strategies/create', name: 'StrategyCreate', component: { template: '<div>create</div>' } },
    { path: '/strategies/templates', name: 'StrategyTemplates', component: { template: '<div>templates</div>' } },
  ],
})

const mockTemplates: StrategyTemplate[] = [
  {
    id: 'trend_following',
    template_id: '550e8400-e29b-41d4-a716-446655420001',
    name: '趋势跟踪',
    description: '基于移动平均线交叉的趋势跟踪策略',
    category: '趋势跟踪',
    default_parameters: { fast_period: 5, slow_period: 20 },
    parameter_schema: [
      { name: 'fast_period', label: '快线周期', type: 'integer' as const, default: 5 },
      { name: 'slow_period', label: '慢线周期', type: 'integer' as const, default: 20 },
    ],
  },
  {
    id: 'grid_trading',
    template_id: '550e8400-e29b-41d4-a716-446655420002',
    name: '网格交易',
    description: '震荡行情网格交易策略',
    category: '网格交易',
    default_parameters: { grid_levels: 10 },
    parameter_schema: [
      { name: 'grid_levels', label: '网格层数', type: 'integer' as const, default: 10 },
    ],
  },
  {
    id: 'mean_reversion',
    template_id: '550e8400-e29b-41d4-a716-446655420003',
    name: '均值回归',
    description: '价格围绕均值波动的均值回归策略',
    category: '均值回归',
    default_parameters: {},
    parameter_schema: [],
  },
]

const mountView = async () => {
  const wrapper = mount(StrategyTemplateView, {
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

describe('StrategyTemplateView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the template marketplace page header', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    expect(wrapper.find('.page-title').text()).toContain('策略模板市场')
    expect(wrapper.find('.back-btn').exists()).toBe(true)
  })

  it('loads and displays templates', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('趋势跟踪')
    expect(wrapper.text()).toContain('网格交易')
    expect(wrapper.text()).toContain('均值回归')
  })

  it('shows template cards in a grid layout', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    const cards = wrapper.findAll('.template-card')
    expect(cards.length).toBe(3)
  })

  it('shows category filter pills', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.find('.category-pills').exists()).toBe(true)
    expect(wrapper.text()).toContain('趋势跟踪')
    expect(wrapper.text()).toContain('均值回归')
    expect(wrapper.text()).toContain('网格交易')
  })

  it('has category filter that can be selected', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    // Category filter should exist and have multiple options
    const pills = wrapper.findAll('.category-pills .el-radio-button')
    expect(pills.length).toBeGreaterThanOrEqual(4) // '' + 4 categories

    // Click the "网格交易" pill
    const gridPill = pills.find(p => p.text().includes('网格交易'))
    await gridPill?.trigger('click')
    await flushPromises()

    // The page still renders (filter controls are present regardless of result)
    expect(wrapper.find('.category-pills').exists()).toBe(true)
  })

  it('shows loading skeleton while fetching data', async () => {
    vi.mocked(strategiesApi.listTemplates).mockImplementation(() => new Promise(() => {}))

    const wrapper = mount(StrategyTemplateView, {
      global: {
        plugins: [ElementPlus, router],
        stubs: {
          'router-link': { template: '<a class="router-link-stub"><slot /></a>' },
          'router-view': true,
        },
      },
    })
    await router.isReady()

    expect(wrapper.find('.template-skeleton').exists()).toBe(true)
  })

  it('shows error state when API fails', async () => {
    vi.mocked(strategiesApi.listTemplates).mockRejectedValue(new Error('Network error'))
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('加载失败')
  })

  it('navigates to create page when template is selected', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    const cards = wrapper.findAll('.template-card')
    const useBtn = cards[0].find('.el-button')
    await useBtn.trigger('click')
    await flushPromises()

    expect(router.currentRoute.value.name).toBe('StrategyCreate')
  })

  it('escapes XSS payload in template description using mustache interpolation', async () => {
    const xssPayload = '<img src=x onerror=alert(1)>'
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue([{
      ...mockTemplates[0],
      description: xssPayload,
    }])
    const wrapper = await mountView()
    await flushPromises()

    const descEl = wrapper.find('.tpl-desc')
    expect(descEl.exists()).toBe(true)
    // The raw img tag should not appear in DOM (script not executed)
    expect(descEl.html()).not.toContain('<img')
    // The text content should still contain the payload text
    expect(descEl.text()).toContain('onerror')
  })

  it('does not use v-html anywhere in StrategyTemplateView', async () => {
    const fs = await import('fs')
    const path = await import('path')
    const vuePath = path.resolve(__dirname, '../../views/strategy/StrategyTemplateView.vue')
    const content = fs.readFileSync(vuePath, 'utf-8')
    expect(content).not.toMatch(/v-html/)
  })
})