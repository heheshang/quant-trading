import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import BacktestEquityChart from '@/components/backtest/BacktestEquityChart.vue'
import type { EquityPoint } from '@/types/backtest'

// Mock ECharts entirely — Canvas is not available in jsdom
vi.mock('echarts', () => ({
  default: {
    init: vi.fn(() => ({
      setOption: vi.fn(),
      dispose: vi.fn(),
      resize: vi.fn(),
    })),
    graphic: {
      LinearGradient: vi.fn(),
    },
  },
  init: vi.fn(() => ({
    setOption: vi.fn(),
    dispose: vi.fn(),
    resize: vi.fn(),
  })),
  graphic: {
    LinearGradient: vi.fn(),
  },
}))

const mockData: EquityPoint[] = [
  { time: new Date('2024-01-01').getTime(), equity: 100000, drawdown_pct: 0 },
  { time: new Date('2024-01-02').getTime(), equity: 101500, drawdown_pct: 0 },
  { time: new Date('2024-01-03').getTime(), equity: 103200, drawdown_pct: 0 },
  { time: new Date('2024-01-04').getTime(), equity: 102800, drawdown_pct: 0.04 },
  { time: new Date('2024-01-05').getTime(), equity: 105000, drawdown_pct: 0 },
]

function createWrapper(props: any = {}) {
  return mount(BacktestEquityChart, {
    props: {
      data: props.data === undefined ? mockData : props.data,
      loading: props.loading ?? false,
      initialCapital: props.initialCapital ?? 100000,
    },
    global: {
      plugins: [ElementPlus],
    },
  })
}

describe('BacktestEquityChart', () => {
  it('renders the component wrapper', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.backtest-equity-chart').exists()).toBe(true)
  })

  it('renders chart title', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('权益曲线')
  })

  it('shows loading skeleton when loading', () => {
    const wrapper = createWrapper({ loading: true, data: [] })
    expect(wrapper.find('.chart-loading').exists()).toBe(true)
  })

  it('shows empty state when no data', () => {
    const wrapper = createWrapper({ data: [] })
    expect(wrapper.find('.chart-empty').exists()).toBe(true)
  })

  it('shows chart wrapper when data is present', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.equity-chart-wrapper').exists()).toBe(true)
  })

  it('displays initial capital in info badge', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('初始')
    expect(wrapper.text()).toContain('100,000.00')
  })

  it('displays final equity in info badge', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('最终')
    // Final equity = 105000
    expect(wrapper.text()).toContain('105,000.00')
  })

  it('does not show info badges when no data', () => {
    const wrapper = createWrapper({ data: [] })
    expect(wrapper.find('.chart-info').exists()).toBe(false)
  })

  it('renders chart container div', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.equity-chart-container').exists()).toBe(true)
  })

  it('accepts custom initialCapital prop', () => {
    const wrapper = createWrapper({ initialCapital: 50000 })
    expect(wrapper.text()).toContain('50,000.00')
  })

  it('initializes echarts on mount', async () => {
    const echarts = await import('echarts')
    createWrapper()
    expect(echarts.init).toHaveBeenCalled()
  })
})
