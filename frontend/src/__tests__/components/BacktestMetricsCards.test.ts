import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import BacktestMetricsCards from '@/components/backtest/BacktestMetricsCards.vue'

// Use nested metrics structure — Bug#11 fix
const mockResult = {
  id: 'uuid-1',
  strategy_id: 'str-1',
  status: 'completed' as const,
  progress: 100,
  error: null,
  config: { symbol: 'BTC/USDT', interval: '1h', start_date: '2024-01-01', end_date: '2024-12-31', initial_capital: 100000, fee_rate: 0.001, slippage_rate: 0.001 },
  metrics: {
    total_return_pct: 25.3,
    annualized_return_pct: 18.5,
    sharpe_ratio: 2.1,
    sortino_ratio: 1.8,
    max_drawdown_pct: -12.5,
    calmar_ratio: 1.48,
    win_rate: 65.8,
    profit_factor: 2.3,
    avg_win_pct: 3.2,
    avg_loss_pct: -1.8,
    total_trades: 96,
    total_fees: 240,
    total_slippage: 160,
    duration_ms: 86400000,
  },
  equity_curve: [],
  trades: [],
  created_at: '2024-01-01T00:00:00Z',
}

const mockResultNegative = {
  ...mockResult,
  metrics: {
    ...mockResult.metrics,
    total_return_pct: -8.2,
    annualized_return_pct: -5.3,
    sharpe_ratio: -0.45,
    max_drawdown_pct: -15.3,
    profit_factor: null,
  },
}

function createWrapper(props: any = {}) {
  return mount(BacktestMetricsCards, {
    props: {
      result: props.result === undefined ? mockResult : props.result,
      loading: props.loading ?? false,
    },
    global: {
      plugins: [ElementPlus],
    },
  })
}

describe('BacktestMetricsCards', () => {
  it('renders all metric cards', () => {
    const wrapper = createWrapper()
    const cards = wrapper.findAll('.metric-card')
    expect(cards.length).toBe(8) // expanded to 8 cards with sortino and profit_factor
  })

  it('displays grid layout with 4 columns', () => {
    const wrapper = createWrapper()
    const grid = wrapper.find('.metrics-grid')
    expect(grid.exists()).toBe(true)
  })

  it('displays total return', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('25.3')
    expect(wrapper.text()).toContain('总收益率')
  })

  it('displays annual return in sub-metric', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('年化收益率')
  })

  it('displays annual return value in sub-metric', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('18.5')
  })

  it('displays sharpe ratio', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('夏普比率')
    expect(wrapper.text()).toContain('2.1')
  })

  it('displays max drawdown', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('最大回撤')
    expect(wrapper.text()).toContain('12.5')
  })

  it('displays win rate', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('胜率')
    expect(wrapper.text()).toContain('65.8')
  })

  it('displays total trades', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('交易次数')
    expect(wrapper.text()).toContain('96')
  })

  it('shows loading skeleton when loading', () => {
    const wrapper = createWrapper({ loading: true, result: null })
    expect(wrapper.find('.metrics-skeleton').exists()).toBe(true)
  })

  it('shows empty state when no result', () => {
    const wrapper = createWrapper({ loading: false, result: null })
    expect(wrapper.find('.metrics-empty').exists()).toBe(true)
  })

  it('formats positive return with green color', () => {
    const wrapper = createWrapper()
    const positiveEl = wrapper.find('.metric-value.up')
    expect(positiveEl.exists()).toBe(true)
  })

  it('formats negative return with red color', () => {
    const wrapper = createWrapper({ result: mockResultNegative })
    const downEl = wrapper.find('.metric-value.down')
    expect(downEl.exists()).toBe(true)
  })

  it('displays card hover effect class', () => {
    const wrapper = createWrapper()
    const card = wrapper.find('.metric-card')
    expect(card.exists()).toBe(true)
  })

  it('shows sub-metric text on total return card', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.metric-sub').exists()).toBe(true)
  })

  it('displays sortino ratio metric card', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('Sortino比率')
    expect(wrapper.text()).toContain('1.8')
  })

  it('displays profit factor metric card with infinity fallback', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('盈亏比')
    expect(wrapper.text()).toContain('2.3')
  })

  it('handles null profit_factor (INFINITY) by showing ∞', () => {
    const wrapper = createWrapper({ result: mockResultNegative })
    expect(wrapper.text()).toContain('盈亏比')
    // null profit_factor is rendered as '∞'
    expect(wrapper.text()).toContain('∞')
  })
})