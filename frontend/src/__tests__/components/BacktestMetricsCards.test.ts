import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import BacktestMetricsCards from '@/components/backtest/BacktestMetricsCards.vue'
import type { BacktestResultResponse } from '@/types/backtest'

const mockResult: BacktestResultResponse = {
  id: 'uuid-1',
  strategy_id: 'str-1',
  status: 'completed',
  progress: 100,
  error: null,
  config: {
    symbol: 'BTC/USDT',
    interval: '1h',
    start_date: '2024-01-01',
    end_date: '2024-12-31',
    initial_capital: 100000,
    fee_rate: 0.001,
    slippage_rate: 0.001,
  },
  metrics: {
    total_return_pct: 25.3,
    annualized_return_pct: 22.1,
    sharpe_ratio: 1.85,
    sortino_ratio: 2.1,
    max_drawdown_pct: -12.5,
    calmar_ratio: 2.15,
    win_rate: 62.0,
    profit_factor: 2.3,
    avg_win_pct: 3.47,
    avg_loss_pct: -1.65,
    total_trades: 85,
    total_fees: 25,
    total_slippage: 5,
    duration_ms: 432000000,
  },
  equity_curve: [],
  trades: [],
  created_at: '2024-01-01T00:00:00Z',
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
  it('renders the component wrapper', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.backtest-metrics-cards').exists()).toBe(true)
  })

  it('renders section title', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('绩效指标')
  })

  it('shows loading skeleton when loading', () => {
    const wrapper = createWrapper({ loading: true, result: null })
    expect(wrapper.find('.metrics-skeleton').exists()).toBe(true)
  })

  it('shows empty state when no result', () => {
    const wrapper = createWrapper({ result: null })
    expect(wrapper.find('.metrics-empty').exists()).toBe(true)
  })

  it('renders metrics grid with data', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.metrics-grid').exists()).toBe(true)
  })

  it('displays total return', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('+25.30%')
  })

  it('displays annualized return', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('+22.10%')
  })

  it('displays sharpe ratio', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('1.85')
  })

  it('displays max drawdown', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('-12.50%')
  })

  it('displays win rate', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('62.0%')
  })

  it('displays total trades', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('85')
  })

  it('displays sortino ratio', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('2.10')
  })

  it('displays profit factor', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('2.30')
  })

  it('renders 8 metric cards', () => {
    const wrapper = createWrapper()
    const cards = wrapper.findAll('.metric-card')
    expect(cards.length).toBe(8)
  })

  it('renders negative returns with down class', () => {
    const negResult = {
      ...mockResult,
      metrics: { ...mockResult.metrics, total_return_pct: -5.2, annualized_return_pct: -4.1 },
    }
    const wrapper = createWrapper({ result: negResult })
    expect(wrapper.text()).toContain('-5.20%')
    expect(wrapper.text()).toContain('-4.10%')
  })

  it('renders profit factor as ∞ when null', () => {
    const pfNullResult = {
      ...mockResult,
      metrics: { ...mockResult.metrics, profit_factor: null },
    }
    const wrapper = createWrapper({ result: pfNullResult })
    expect(wrapper.text()).toContain('∞')
  })

  it('displays avg win/loss in profit factor card sub text', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('均盈')
    expect(wrapper.text()).toContain('均亏')
  })
})
