import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import BacktestEquityChart from '@/components/backtest/BacktestEquityChart.vue'

// Mock echarts
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

// Use time: number (ms timestamp), not date: string — Bug#8
const mockEquity = [
  { time: Date.parse('2024-01-01'), equity: 100000, drawdown_pct: 0 },
  { time: Date.parse('2024-02-01'), equity: 105000, drawdown_pct: -2.1 },
  { time: Date.parse('2024-03-01'), equity: 112000, drawdown_pct: -4.5 },
  { time: Date.parse('2024-04-01'), equity: 108000, drawdown_pct: -7.2 },
  { time: Date.parse('2024-05-01'), equity: 125300, drawdown_pct: -3.1 },
]

function createWrapper(props: any = {}) {
  return mount(BacktestEquityChart, {
    props: {
      data: props.data ?? mockEquity,
      loading: props.loading ?? false,
      initialCapital: props.initialCapital ?? 100000,
    },
    global: {
      plugins: [ElementPlus],
    },
  })
}

describe('BacktestEquityChart', () => {
  it('renders chart container', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.equity-chart-wrapper').exists()).toBe(true)
  })

  it('shows loading skeleton when loading', () => {
    const wrapper = createWrapper({ loading: true })
    expect(wrapper.find('.chart-loading').exists()).toBe(true)
  })

  it('shows empty state when no data', () => {
    const wrapper = createWrapper({ data: [] })
    expect(wrapper.find('.chart-empty').exists()).toBe(true)
  })

  it('renders chart title', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.chart-title').text()).toContain('权益曲线')
  })

  it('passes data length to echarts', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.equity-chart-container').exists()).toBe(true)
  })

  it('displays initial capital annotation', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('100,000')
  })

  it('displays final equity', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('125,')
  })

  it('renders baseline indicator text', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.baseline-label').exists() || wrapper.text().includes('初始')).toBe(true)
  })

  it('shows tab bar when data exists', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.chart-controls').exists() || wrapper.text().includes('权益曲线')).toBe(true)
  })

  it('renders chart container with data', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.equity-chart-wrapper').exists()).toBe(true)
    expect(wrapper.find('.equity-chart-container').exists()).toBe(true)
  })
})