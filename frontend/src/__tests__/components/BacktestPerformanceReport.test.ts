import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import BacktestPerformanceReport from '@/components/backtest/BacktestPerformanceReport.vue'
import type { BacktestResultResponse, TradeRecord, EquityPoint } from '@/types/backtest'

const mockTrades: TradeRecord[] = [
  { direction: 'long', entry_time: '2024-01-01T00:00:00Z', exit_time: '2024-01-02T00:00:00Z', entry_price: 42000, exit_price: 43000, quantity: 0.1, pnl_usdt: 100, pnl_pct: 2.38, fee: 5, slippage: 1, exit_reason: 'take_profit', holding_period_ms: 86400000 },
  { direction: 'long', entry_time: '2024-01-02T00:00:00Z', exit_time: '2024-01-03T00:00:00Z', entry_price: 43000, exit_price: 44500, quantity: 0.1, pnl_usdt: 150, pnl_pct: 3.49, fee: 5, slippage: 1, exit_reason: 'signal', holding_period_ms: 86400000 },
  { direction: 'short', entry_time: '2024-01-03T00:00:00Z', exit_time: '2024-01-04T00:00:00Z', entry_price: 44500, exit_price: 44000, quantity: 0.1, pnl_usdt: -50, pnl_pct: -1.12, fee: 5, slippage: 1, exit_reason: 'stop_loss', holding_period_ms: 86400000 },
  { direction: 'long', entry_time: '2024-01-04T00:00:00Z', exit_time: '2024-01-05T00:00:00Z', entry_price: 44000, exit_price: 46000, quantity: 0.1, pnl_usdt: 200, pnl_pct: 4.55, fee: 5, slippage: 1, exit_reason: 'take_profit', holding_period_ms: 86400000 },
  { direction: 'short', entry_time: '2024-01-05T00:00:00Z', exit_time: '2024-01-06T00:00:00Z', entry_price: 46000, exit_price: 47000, quantity: 0.1, pnl_usdt: -100, pnl_pct: -2.17, fee: 5, slippage: 1, exit_reason: 'stop_loss', holding_period_ms: 86400000 },
]

const mockEquityCurve: EquityPoint[] = [
  { time: new Date('2024-01-01').getTime(), equity: 100000, drawdown_pct: 0 },
  { time: new Date('2024-01-02').getTime(), equity: 100100, drawdown_pct: 0 },
  { time: new Date('2024-01-03').getTime(), equity: 100250, drawdown_pct: 0 },
  { time: new Date('2024-01-04').getTime(), equity: 100200, drawdown_pct: 0.05 },
  { time: new Date('2024-01-05').getTime(), equity: 100400, drawdown_pct: 0 },
  { time: new Date('2024-01-06').getTime(), equity: 100300, drawdown_pct: 0.1 },
]

const mockResult: BacktestResultResponse = {
  id: 'uuid-test',
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
    total_trades: 5,
    total_fees: 25,
    total_slippage: 5,
    duration_ms: 432000000,
  },
  equity_curve: mockEquityCurve,
  trades: mockTrades,
  created_at: '2024-01-01T00:00:00Z',
}

function createWrapper(props: any = {}) {
  return mount(BacktestPerformanceReport, {
    props: {
      result: props.result === undefined ? mockResult : props.result,
      loading: props.loading ?? false,
    },
    global: {
      plugins: [ElementPlus],
    },
  })
}

describe('BacktestPerformanceReport', () => {
  it('renders the component wrapper', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.backtest-performance-report').exists()).toBe(true)
  })

  it('renders report grid with data', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.report-grid').exists()).toBe(true)
  })

  it('shows loading skeleton when loading is true', () => {
    const wrapper = createWrapper({ loading: true, result: null })
    expect(wrapper.find('.report-skeleton').exists()).toBe(true)
  })

  it('shows empty state when no result', () => {
    const wrapper = createWrapper({ result: null })
    expect(wrapper.find('.report-empty').exists()).toBe(true)
  })

  it('renders all 15 metric rows', () => {
    const wrapper = createWrapper()
    const rows = wrapper.findAll('.report-row')
    expect(rows.length).toBe(15)
  })

  it('displays total return percentage', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('+25.30%')
  })

  it('displays annualized return percentage', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('+22.10%')
  })

  it('displays sharpe ratio', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('1.85')
  })

  it('displays calmar ratio', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('2.15')
  })

  it('displays max drawdown in red', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('-12.50%')
  })

  it('displays win rate', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('62.0%')
  })

  it('displays total trades', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('5')
  })

  it('displays profit factor', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('2.30')
  })

  it('computes winning trades from trades data', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.winningTrades).toBe(3) // 100, 150, 200
  })

  it('computes losing trades from trades data', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.losingTrades).toBe(2) // -50, -100
  })

  it('computes average PnL from trades', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    // (100 + 150 - 50 + 200 - 100) / 5 = 300 / 5 = 60
    expect(vm.avgPnL).toBe(60)
  })

  it('formats average PnL with sign and unit', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatAvgPnL).toBe('+60.00 USDT')
  })

  it('computes max consecutive wins', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    // trades: win, win, loss, win, loss => max consecutive = 2
    expect(vm.maxConsecutiveWins).toBe(2)
  })

  it('computes max consecutive losses', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    // trades: win, win, loss, win, loss => max consecutive = 1
    expect(vm.maxConsecutiveLosses).toBe(1)
  })

  it('computes max drawdown period from equity curve', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.maxDrawdownPeriod).toContain('2024')
    expect(vm.maxDrawdownPeriod).toContain('~')
  })

  it('computes average holding period', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    // All trades have 86400000ms = 1 day
    expect(vm.avgHoldingPeriod).toBe('1d')
  })

  it('colorClass returns up for positive pnl', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.colorClass(10, 'pnl')).toBe('up')
    expect(vm.colorClass(-5, 'pnl')).toBe('down')
  })

  it('colorClass returns correct sharpe colors', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.colorClass(2.0, 'sharpe')).toBe('up')
    expect(vm.colorClass(1.2, 'sharpe')).toBe('neutral')
    expect(vm.colorClass(0.5, 'sharpe')).toBe('warning')
  })

  it('colorClass returns correct profit factor colors', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.colorClass(2.5, 'pf')).toBe('up')
    expect(vm.colorClass(1.7, 'pf')).toBe('neutral')
    expect(vm.colorClass(0.8, 'pf')).toBe('down')
    expect(vm.colorClass(null, 'pf')).toBe('up') // ∞ treated as good
  })

  it('formatDuration formats milliseconds to human-readable', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    // 3 days, 4 hours, 12 minutes = 274320000ms
    expect(vm.formatDuration(274320000)).toBe('3d 4h 12m')
    // Less than 1 minute
    expect(vm.formatDuration(30000)).toBe('<1m')
    // Just hours
    expect(vm.formatDuration(7200000)).toBe('2h')
  })

  it('shows "--" for max drawdown period when no equity curve', () => {
    const noCurveResult = { ...mockResult, equity_curve: [] }
    const wrapper = createWrapper({ result: noCurveResult })
    const vm = wrapper.vm as any
    expect(vm.maxDrawdownPeriod).toBe('--')
  })

  it('shows "--" for avg holding period when no trades', () => {
    const noTradesResult = { ...mockResult, trades: [] }
    const wrapper = createWrapper({ result: noTradesResult })
    const vm = wrapper.vm as any
    expect(vm.avgHoldingPeriod).toBe('--')
  })

  it('profit factor displays ∞ when null', () => {
    const pfNullResult = { ...mockResult, metrics: { ...mockResult.metrics, profit_factor: null } }
    const wrapper = createWrapper({ result: pfNullResult })
    const vm = wrapper.vm as any
    expect(vm.profitFactorDisplay).toBe('∞')
  })

  it('win rate shows up class when >= 50%', () => {
    const wrapper = createWrapper()
    // 62% win rate, should find the "胜率" row with up class value
    const html = wrapper.html()
    expect(html).toContain('胜率')
  })

  it('renders dual-column grid layout', () => {
    const wrapper = createWrapper()
    const grid = wrapper.find('.report-grid')
    expect(grid.exists()).toBe(true)
  })

  it('renders all metric labels from design spec 5.7', () => {
    const wrapper = createWrapper()
    const text = wrapper.text()
    expect(text).toContain('总收益率')
    expect(text).toContain('年化收益率')
    expect(text).toContain('夏普比率')
    expect(text).toContain('卡玛比率')
    expect(text).toContain('最大回撤')
    expect(text).toContain('最大回撤区间')
    expect(text).toContain('胜率')
    expect(text).toContain('总交易次数')
    expect(text).toContain('盈利交易数')
    expect(text).toContain('亏损交易数')
    expect(text).toContain('平均盈亏')
    expect(text).toContain('盈亏比')
    expect(text).toContain('平均持仓时长')
    expect(text).toContain('最大连续盈利')
    expect(text).toContain('最大连续亏损')
  })
})
