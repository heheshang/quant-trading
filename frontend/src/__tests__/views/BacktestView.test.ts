import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import BacktestView from '@/views/backtest/BacktestView.vue'
import * as backtestApi from '@/api/backtest'
import * as strategiesApi from '@/api/strategies'

vi.mock('@/api/backtest', () => ({
  runBacktest: vi.fn(),
  getBacktestResult: vi.fn(),
  cancelBacktest: vi.fn(),
  listBacktestHistory: vi.fn(),
  deleteBacktestResult: vi.fn(),
}))

vi.mock('@/api/strategies', () => ({
  listStrategies: vi.fn(),
}))

// Mock echarts to avoid Canvas dependency issues in jsdom
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

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/backtest', name: 'Backtest', component: { template: '<div>backtest</div>' } },
  ],
})

const mockStrategies = [
  { id: 'str-1', name: '趋势跟踪', template_type: 'trend_following', status: 'active' as const },
  { id: 'str-2', name: '网格交易', template_type: 'grid_trading', status: 'active' as const },
]

// Use nested BacktestResultResponse structure — Bug#11 fix
const mockResult = {
  id: 'uuid-1',
  strategy_id: 'str-1',
  status: 'completed' as const,
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
  equity_curve: [
    { time: Date.parse('2024-01-01'), equity: 100000, drawdown_pct: 0 },
    { time: Date.parse('2024-02-01'), equity: 105000, drawdown_pct: -2.1 },
    { time: Date.parse('2024-03-01'), equity: 112000, drawdown_pct: -4.5 },
    { time: Date.parse('2024-04-01'), equity: 108000, drawdown_pct: -7.2 },
    { time: Date.parse('2024-05-01'), equity: 125300, drawdown_pct: -3.1 },
  ],
  trades: [
    {
      direction: 'long' as const,
      entry_time: '2024-01-15T09:30:00Z',
      exit_time: '2024-01-20T14:00:00Z',
      entry_price: 42000,
      exit_price: 43500,
      quantity: 0.5,
      pnl_usdt: 750,
      pnl_pct: 3.57,
      fee: 21,
      slippage: 5,
      exit_reason: 'take_profit' as const,
      holding_period_ms: 160200000,
    },
    {
      direction: 'short' as const,
      entry_time: '2024-02-01T10:00:00Z',
      exit_time: '2024-02-05T16:00:00Z',
      entry_price: 44000,
      exit_price: 42800,
      quantity: 0.3,
      pnl_usdt: 360,
      pnl_pct: 2.73,
      fee: 13.2,
      slippage: 3.6,
      exit_reason: 'signal' as const,
      holding_period_ms: 100800000,
    },
  ],
  created_at: '2024-01-01T00:00:00Z',
}

const mockHistory = {
  items: [
    { id: 'uuid-1', strategy_id: 'str-1', symbol: 'BTC/USDT', status: 'completed', total_return_pct: 25.3, sharpe_ratio: 2.1, created_at: '2024-01-01T00:00:00Z' },
    { id: 'uuid-2', strategy_id: 'str-2', symbol: 'ETH/USDT', status: 'completed', total_return_pct: 12.1, sharpe_ratio: 1.5, created_at: '2024-01-02T00:00:00Z' },
  ],
  total: 2,
  page: 1,
  size: 10,
}

const mountView = async () => {
  const wrapper = mount(BacktestView, {
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

describe('BacktestView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the page header', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    vi.mocked(backtestApi.listBacktestHistory).mockResolvedValue(mockHistory)
    const wrapper = await mountView()

    expect(wrapper.find('.page-title').text()).toBe('回测')
  })

  it('renders configuration form section', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    vi.mocked(backtestApi.listBacktestHistory).mockResolvedValue(mockHistory)
    const wrapper = await mountView()

    expect(wrapper.find('.config-panel').exists()).toBe(true)
  })

  it('shows empty state when no result and not running', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    vi.mocked(backtestApi.listBacktestHistory).mockResolvedValue(mockHistory)
    const wrapper = await mountView()

    expect(wrapper.find('.empty-state').exists()).toBe(true)
  })

  it('shows progress panel when running', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    vi.mocked(backtestApi.listBacktestHistory).mockResolvedValue(mockHistory)
    vi.mocked(backtestApi.runBacktest).mockImplementation(
      () => new Promise(() => {})
    )
    const wrapper = await mountView()

    ;(wrapper.vm as any).backtestState = 'running'
    await flushPromises()

    expect(wrapper.find('.progress-panel').exists()).toBe(true)
  })

  it('displays error alert when backtest fails', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    vi.mocked(backtestApi.listBacktestHistory).mockResolvedValue(mockHistory)
    const wrapper = await mountView()

    ;(wrapper.vm as any).backtestState = 'failed'
    ;(wrapper.vm as any).error = '回测执行失败：数据不足'
    await flushPromises()

    expect(wrapper.find('.error-section').exists()).toBe(true)
    expect(wrapper.text()).toContain('回测执行失败')
  })

  it('shows results section after backtest completes', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    vi.mocked(backtestApi.listBacktestHistory).mockResolvedValue(mockHistory)
    const wrapper = await mountView()

    ;(wrapper.vm as any).backtestState = 'completed'
    ;(wrapper.vm as any).result = mockResult
    await flushPromises()

    expect(wrapper.find('.results-section').exists()).toBe(true)
  })

  it('renders top action bar in results', async () => {
    const wrapper = await mountView()
    ;(wrapper.vm as any).backtestState = 'completed'
    ;(wrapper.vm as any).result = mockResult
    await flushPromises()

    expect(wrapper.find('.result-action-bar').exists()).toBe(true)
    expect(wrapper.text()).toContain('回测结果')
  })

  it('renders tab navigation in results', async () => {
    const wrapper = await mountView()
    ;(wrapper.vm as any).backtestState = 'completed'
    ;(wrapper.vm as any).result = mockResult
    await flushPromises()

    expect(wrapper.find('.result-tabs').exists()).toBe(true)
    expect(wrapper.text()).toContain('权益曲线')
    expect(wrapper.text()).toContain('交易明细')
  })

  it('shows progress bar during running state', async () => {
    const wrapper = await mountView()
    ;(wrapper.vm as any).backtestState = 'running'
    await flushPromises()

    expect(wrapper.find('.progress-bar').exists()).toBe(true)
  })

  it('shows cancel button during running state', async () => {
    const wrapper = await mountView()
    ;(wrapper.vm as any).backtestState = 'running'
    await flushPromises()

    expect(wrapper.find('.cancel-btn').exists()).toBe(true)
  })

  it('shows re-run button in results action bar', async () => {
    const wrapper = await mountView()
    ;(wrapper.vm as any).backtestState = 'completed'
    ;(wrapper.vm as any).result = mockResult
    await flushPromises()

    expect(wrapper.find('.rerun-btn').exists()).toBe(true)
    expect(wrapper.find('.rerun-btn').text()).toContain('重新回测')
  })

  it('switches between tabs when clicked', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    vi.mocked(backtestApi.listBacktestHistory).mockResolvedValue(mockHistory)
    const wrapper = await mountView()
    ;(wrapper.vm as any).backtestState = 'completed'
    ;(wrapper.vm as any).result = mockResult
    await flushPromises()

    expect((wrapper.vm as any).activeTab).toBe(0)

    ;(wrapper.vm as any).activeTab = 1
    await flushPromises()
    expect((wrapper.vm as any).activeTab).toBe(1)
  })

  it('renders equity chart when result available', async () => {
    const wrapper = await mountView()
    ;(wrapper.vm as any).backtestState = 'completed'
    ;(wrapper.vm as any).result = mockResult
    await flushPromises()

    expect(wrapper.text()).toContain('权益曲线')
  })

  it('renders trades table when result available', async () => {
    const wrapper = await mountView()
    ;(wrapper.vm as any).backtestState = 'completed'
    ;(wrapper.vm as any).result = mockResult
    await flushPromises()

    expect(wrapper.text()).toContain('交易记录')
  })

  it('renders metrics when result available', async () => {
    const wrapper = await mountView()
    ;(wrapper.vm as any).backtestState = 'completed'
    ;(wrapper.vm as any).result = mockResult
    await flushPromises()

    expect(wrapper.text()).toContain('总收益率')
    expect(wrapper.text()).toContain('夏普比率')
    expect(wrapper.text()).toContain('交易次数')
  })

  it('has config section title', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    vi.mocked(backtestApi.listBacktestHistory).mockResolvedValue(mockHistory)
    const wrapper = await mountView()

    expect(wrapper.text()).toContain('回测配置')
  })
})