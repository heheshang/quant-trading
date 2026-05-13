import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import BacktestConfigForm from '@/components/backtest/BacktestConfigForm.vue'
import * as strategiesApi from '@/api/strategies'

vi.mock('@/api/strategies', () => ({
  listStrategies: vi.fn(),
  listTemplates: vi.fn(),
}))

const mockStrategies = [
  { id: 'str-1', name: '趋势跟踪', template_type: 'trend_following', status: 'active' as const },
  { id: 'str-2', name: '网格交易', template_type: 'grid_trading', status: 'active' as const },
]

const mockTemplates = [
  {
    id: 'trend_following',
    name: '趋势跟踪',
    description: '基于移动平均线的趋势策略',
    category: '趋势',
    default_parameters: { fast_period: 10, slow_period: 30 },
    parameter_schema: [
      { name: 'fast_period', label: '快线周期', type: 'integer' as const, default: 10, min: 5, max: 50 },
      { name: 'slow_period', label: '慢线周期', type: 'integer' as const, default: 30, min: 10, max: 200 },
    ],
  },
  {
    id: 'grid_trading',
    name: '网格交易',
    description: '在价格区间内自动低买高卖',
    category: '套利',
    default_parameters: { grid_levels: 10, price_range: 5000 },
    parameter_schema: [
      { name: 'grid_levels', label: '网格层数', type: 'integer' as const, default: 10, min: 2, max: 50 },
      { name: 'price_range', label: '价格范围', type: 'float' as const, default: 5000, min: 100, max: 100000 },
    ],
  },
]

function createWrapper() {
  return mount(BacktestConfigForm, {
    global: {
      plugins: [ElementPlus],
    },
  })
}

describe('BacktestConfigForm', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders all form fields', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    expect(wrapper.find('.strategy-select').exists()).toBe(true)
    expect(wrapper.find('.symbol-input').exists()).toBe(true)
    expect(wrapper.find('.timeframe-select').exists()).toBe(true)
    expect(wrapper.find('.date-range').exists()).toBe(true)
    expect(wrapper.find('.capital-input').exists()).toBe(true)
    expect(wrapper.find('.run-btn').exists()).toBe(true)
  })

  it('loads strategies for the selector', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    expect(strategiesApi.listStrategies).toHaveBeenCalledTimes(1)
    const select = wrapper.findComponent({ name: 'ElSelect' })
    expect(select.exists()).toBe(true)
  })

  it('shows strategy select with placeholder', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    const select = wrapper.findComponent({ name: 'ElSelect' })
    expect(select.props('placeholder')).toBeTruthy()
  })

  it('has date range picker', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    expect(wrapper.findComponent({ name: 'ElDatePicker' }).exists()).toBe(true)
  })

  it('has numeric input for initial capital', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    const capitalInput = wrapper.find('.capital-input')
    expect(capitalInput.exists()).toBe(true)
  })

  it('run button is disabled when form is invalid', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    const btnComp = wrapper.findComponent({ name: 'ElButton' })
    expect(btnComp.props('disabled')).toBe(true)
  })

  it('run button shows loading state when running', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    await wrapper.setProps({ loading: true })
    const btnComp = wrapper.findComponent({ name: 'ElButton' })
    expect(btnComp.props('loading')).toBe(true)
  })

  it('has reset button', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    expect(wrapper.find('.reset-btn').exists()).toBe(true)
  })

  it('emits run event with form data when triggered', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    vm.form.strategy_id = 'str-1'
    vm.form.symbol = 'BTC/USDT'
    vm.form.timeframe = '1h'
    vm.form.initial_capital = 100000
    vm.form.fee_rate = 0.001
    vm.form.slippage_rate = 0.001
    vm.form.dateRange = ['2024-01-01', '2024-12-31']
    vm.form.strategy_params = { fast_period: 10, slow_period: 30 }

    vm.handleRun()
    await flushPromises()

    const emitted = wrapper.emitted('run')
    expect(emitted).toBeTruthy()
    expect(emitted![0][0]).toMatchObject({
      strategy_id: 'str-1',
      symbol: 'BTC/USDT',
      interval: '1h',
      initial_capital: 100000,
      fee_rate: 0.001,
      slippage_rate: 0.001,
      start_date: '2024-01-01',
      end_date: '2024-12-31',
      strategy_params: { fast_period: 10, slow_period: 30 },
    })
  })

  it('renders strategy params section when a strategy is selected', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    vm.form.strategy_id = 'str-1'
    await vm.loadTemplateParams()
    await flushPromises()

    // Should show strategy params section
    expect(wrapper.find('.strategy-params-section').exists()).toBe(true)
  })

  it('renders with correct section title', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('回测配置')
  })

  it('shows strategy params with correct param fields', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    vm.form.strategy_id = 'str-1'
    await vm.loadTemplateParams()
    await flushPromises()

    expect(wrapper.text()).toContain('快线周期')
    expect(wrapper.text()).toContain('慢线周期')
  })

  it('clears params when strategy changes', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    vm.form.strategy_id = 'str-1'
    await vm.loadTemplateParams()
    await flushPromises()

    expect(vm.form.strategy_params).toBeTruthy()

    // Change strategy
    vm.form.strategy_id = 'str-2'
    await vm.loadTemplateParams()
    await flushPromises()

    expect(vm.form.strategy_params).toEqual({ grid_levels: 10, price_range: 5000 })
  })

  it('disables all form fields when loading=true', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    // All el-input-number, el-select, el-date-picker, el-slider should have :disabled="true" when loading
    await wrapper.setProps({ loading: true })
    await flushPromises()

    const strategySelect = wrapper.find('.strategy-select')
    expect(strategySelect.exists()).toBe(true)
    // The el-select inside should be disabled
    const selectComponent = strategySelect.findComponent({ name: 'ElSelect' })
    expect(selectComponent.props('disabled')).toBe(true)
  })
})
