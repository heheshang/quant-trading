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
    // Default mocks so mounted hook never throws
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({ data: [] } as any)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue({ data: [] } as any)
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
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({ data: mockStrategies } as any)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue({ data: mockTemplates } as any)
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
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({ data: mockStrategies } as any)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue({ data: mockTemplates } as any)
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
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({ data: mockStrategies } as any)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue({ data: mockTemplates } as any)
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

  // --- NEW: Enhanced coverage tests ---

  it('reset clears all form values', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({ data: mockStrategies } as any)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue({ data: mockTemplates } as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    vm.form.strategy_id = 'str-1'
    vm.form.symbol = 'BTC/USDT'
    vm.form.timeframe = '4h'
    vm.form.initial_capital = 50000
    vm.form.fee_rate = 0.002
    vm.form.slippage_rate = 0.003
    vm.form.dateRange = ['2024-01-01', '2024-12-31']
    vm.form.strategy_params = { fast_period: 10 }

    vm.handleReset()
    await flushPromises()

    expect(vm.form.strategy_id).toBe('')
    expect(vm.form.symbol).toBe('')
    expect(vm.form.timeframe).toBe('1h')
    expect(vm.form.initial_capital).toBe(100000)
    expect(vm.form.fee_rate).toBe(0.001)
    expect(vm.form.slippage_rate).toBe(0.001)
    expect(vm.form.dateRange).toEqual([])
    expect(vm.form.strategy_params).toEqual({})
  })

  it('timeframe select has all 8 options', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    expect(vm.timeframes).toHaveLength(8)
    expect(vm.timeframes.map((tf: any) => tf.value)).toEqual(
      ['1m', '5m', '15m', '30m', '1h', '4h', '1d', '1w']
    )
  })

  it('fee rate hint shows percentage conversion', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    expect(wrapper.text()).toContain('0.001 = 0.1%')
  })

  it('strategy params section is hidden when no strategy selected', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({ data: mockStrategies } as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    vm.form.strategy_id = ''
    await flushPromises()

    expect(wrapper.find('.strategy-params-section').exists()).toBe(false)
  })

  it('loadTemplateParams handles missing template gracefully', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({ data: mockStrategies } as any)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue({ data: [] } as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    vm.form.strategy_id = 'str-1'
    await vm.loadTemplateParams()
    await flushPromises()

    expect(vm.selectedTemplate).toBeNull()
  })

  it('isFormValid returns false with missing fields', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    // Empty form is invalid — test via button disabled state
    const runBtn = wrapper.find('.run-btn')
    expect(runBtn.exists()).toBe(true)
    const btnComp = runBtn.findComponent({ name: 'ElButton' })
    expect(btnComp.props('disabled')).toBe(true)

    // Fill in all fields to make form valid
    vm.form.strategy_id = 'str-1'
    vm.form.symbol = 'BTC/USDT'
    vm.form.timeframe = '1h'
    vm.form.initial_capital = 100000
    vm.form.fee_rate = 0.001
    vm.form.slippage_rate = 0.001
    vm.form.dateRange = ['2024-01-01', '2024-12-31']
    await flushPromises()

    // Now the button should be enabled
    expect(btnComp.props('disabled')).toBe(false)
  })

  it('loads strategies and templates on mount', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({ data: mockStrategies } as any)
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue({ data: mockTemplates } as any)
    createWrapper()
    await flushPromises()

    expect(strategiesApi.listStrategies).toHaveBeenCalledTimes(1)
    expect(strategiesApi.listTemplates).toHaveBeenCalledTimes(1)
  })

  it('handles loadStrategies error gracefully', async () => {
    vi.mocked(strategiesApi.listStrategies).mockRejectedValue(new Error('Network error'))
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue({ data: [] } as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    expect(vm.strategies).toEqual([])
  })

  it('handles loadTemplates error gracefully', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue({ data: mockStrategies } as any)
    vi.mocked(strategiesApi.listTemplates).mockRejectedValue(new Error('Network error'))
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    expect(vm.templates).toEqual([])
  })

  it('getParamRule returns required rule for all param types', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    const selectParam = { name: 'type', label: '类型', type: 'select' }
    const rules = vm.getParamRule(selectParam)
    expect(rules).toHaveLength(1)
    expect(rules[0].required).toBe(true)
  })

  it('getParamRule adds min/max rules for numeric params', async () => {
    vi.mocked(strategiesApi.listStrategies).mockResolvedValue(mockStrategies as any)
    const wrapper = createWrapper()
    await flushPromises()

    const vm = wrapper.vm as any
    const numericParam = { name: 'period', label: '周期', type: 'integer', min: 5, max: 50 }
    const rules = vm.getParamRule(numericParam)
    expect(rules).toHaveLength(3) // required + min + max
    expect(rules[1].min).toBe(5)
    expect(rules[2].max).toBe(50)
  })
})
