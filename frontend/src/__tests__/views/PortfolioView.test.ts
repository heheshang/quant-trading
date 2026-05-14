import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import { ref, reactive } from 'vue'

// Mock echarts before importing the component
vi.mock('echarts', () => ({
  init: vi.fn(() => ({
    setOption: vi.fn(),
    dispose: vi.fn(),
    resize: vi.fn(),
  })),
  graphic: {
    LinearGradient: vi.fn(() => ({})),
  },
}))

// Mock API composables - return flat destructured refs matching how the component uses them
vi.mock('@/composables/usePortfolioSummary', () => ({
  usePortfolioSummary: vi.fn(() => ({
    summary: ref(null),
    loading: ref(false),
    error: ref(null),
    fetch: vi.fn().mockResolvedValue(undefined),
    applyWsUpdate: vi.fn(),
  })),
}))

vi.mock('@/composables/usePortfolioPositions', () => ({
  usePortfolioPositions: vi.fn(() => ({
    positions: ref(null),
    loading: ref(false),
    error: ref(null),
    filters: reactive({ symbol: '', side: '' }),
    pagination: reactive({ page: 1, size: 10 }),
    fetch: vi.fn().mockResolvedValue(undefined),
    applyWsUpdate: vi.fn(),
  })),
}))

vi.mock('@/composables/useEquityCurve', () => ({
  useEquityCurve: vi.fn(() => ({
    curve: ref(null),
    loading: ref(false),
    error: ref(null),
    granularity: ref('day'),
    dateRange: ref(['2026-04-14', '2026-05-14']),
    fetch: vi.fn().mockResolvedValue(undefined),
  })),
}))

vi.mock('@/composables/usePortfolioPerformance', () => ({
  usePortfolioPerformance: vi.fn(() => ({
    performance: ref(null),
    loading: ref(false),
    error: ref(null),
    fetch: vi.fn().mockResolvedValue(undefined),
  })),
}))

import PortfolioView from '@/views/portfolio/PortfolioView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [{ path: '/', component: { template: '<div />' } }],
})

function createWrapper() {
  return mount(PortfolioView, {
    global: {
      plugins: [ElementPlus, router],
    },
  })
}

describe('PortfolioView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the page title', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    expect(wrapper.find('.page-title').text()).toBe('组合权益')
  })

  it('renders 4 overview metric cards', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    const cards = wrapper.findAll('.metric-card')
    expect(cards.length).toBe(4)
  })

  it('renders card labels', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    const labels = wrapper.findAll('.metric-card__label').map(el => el.text())
    expect(labels).toContain('总资产')
    expect(labels).toContain('累计盈亏')
    expect(labels).toContain('当日盈亏')
    expect(labels).toContain('收益率')
  })

  it('renders equity curve section', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    expect(wrapper.find('.equity-curve-section').exists()).toBe(true)
  })

  it('renders positions section', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    expect(wrapper.find('.positions-section').exists()).toBe(true)
  })

  it('renders performance section', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    expect(wrapper.find('.performance-section').exists()).toBe(true)
  })

  it('renders 3 performance metric cards', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    const perfCards = wrapper.findAll('.perf-metric-card')
    expect(perfCards.length).toBe(3)
  })

  it('renders performance metric labels', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    const labels = wrapper.findAll('.perf-metric-card__label').map(el => el.text())
    expect(labels).toContain('最大回撤')
    expect(labels).toContain('夏普率')
    expect(labels).toContain('胜率')
  })

  it('renders header controls', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    expect(wrapper.find('.header-controls').exists()).toBe(true)
  })

  it('renders bottom layout with positions and performance', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    expect(wrapper.find('.bottom-layout').exists()).toBe(true)
    expect(wrapper.find('.positions-section').exists()).toBe(true)
    expect(wrapper.find('.performance-section').exists()).toBe(true)
  })

  it('shows chart empty state when no curve data', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    expect(wrapper.find('.chart-empty').exists()).toBe(true)
  })

  it('has portfolio-dashboard root class', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    expect(wrapper.find('.portfolio-dashboard').exists()).toBe(true)
  })

  it('renders position table headers', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    const headers = wrapper.findAll('th')
    const headerTexts = headers.map(h => h.text())
    expect(headerTexts.some(t => t.includes('交易对'))).toBe(true)
    expect(headerTexts.some(t => t.includes('方向'))).toBe(true)
    expect(headerTexts.some(t => t.includes('浮动盈亏'))).toBe(true)
  })

  it('renders strategy table with header', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    const sectionTitles = wrapper.findAll('.section-title')
    const titleTexts = sectionTitles.map(el => el.text())
    expect(titleTexts.some(t => t.includes('策略绩效'))).toBe(true)
  })

  it('renders default values when no data', async () => {
    const wrapper = createWrapper()
    await flushPromises()
    // Should render 0 values when no data
    expect(wrapper.find('.metric-card__value').text()).toContain('0.00')
  })
})
