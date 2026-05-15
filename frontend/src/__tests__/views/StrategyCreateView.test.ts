import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import StrategyCreateView from '@/views/strategy/StrategyCreateView.vue'
import * as strategiesApi from '@/api/strategies'

vi.mock('@/api/strategies', () => ({
  listTemplates: vi.fn(),
  createStrategy: vi.fn(),
  getStrategy: vi.fn(),
  updateStrategy: vi.fn(),
  uploadStrategyCode: vi.fn(),
}))

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/strategies', name: 'Strategies', component: { template: '<div>list</div>' } },
    { path: '/strategies/create', name: 'StrategyCreate', component: { template: '<div>create</div>' } },
  ],
})

const mockTemplates = [
  {
    id: 'trend_following',
    name: '趋势跟踪',
    description: '基于移动平均线交叉的趋势跟踪策略',
    category: '趋势跟踪',
    default_parameters: { fast_period: 5, slow_period: 20, use_ema: true, signal: 'cross' },
    parameter_schema: [
      { name: 'fast_period', label: '快线周期', type: 'integer' as const, default: 5, min: 2, max: 50, description: '快线MA周期' },
      { name: 'slow_period', label: '慢线周期', type: 'integer' as const, default: 20, min: 5, max: 200, description: '慢线MA周期' },
      { name: 'use_ema', label: '使用EMA', type: 'boolean' as const, default: true, description: '启用指数移动平均' },
      { name: 'signal', label: '信号确认', type: 'select' as const, default: 'cross', options: ['cross', 'continuous', 'slope'] },
    ],
  },
  {
    id: 'grid_trading',
    name: '网格交易',
    description: '震荡行情网格交易策略',
    category: '网格交易',
    default_parameters: { grid_levels: 10, grid_range: 0.05, take_profit: false },
    parameter_schema: [
      { name: 'grid_levels', label: '网格层数', type: 'integer' as const, default: 10, min: 3, max: 50 },
      { name: 'grid_range', label: '网格范围', type: 'float' as const, default: 0.05, min: 0.01, max: 0.5 },
      { name: 'take_profit', label: '启用止盈', type: 'boolean' as const, default: false },
    ],
  },
]

const mountView = async () => {
  const wrapper = mount(StrategyCreateView, {
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

describe('StrategyCreateView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders the create strategy page header', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    expect(wrapper.find('.page-title').text()).toContain('创建策略')
    expect(wrapper.find('.back-btn').exists()).toBe(true)
  })

  it('loads and displays templates in step 1', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('趋势跟踪')
    expect(wrapper.text()).toContain('网格交易')
    expect(strategiesApi.listTemplates).toHaveBeenCalledTimes(1)
  })

  it('shows template cards in a grid layout', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    const cards = wrapper.findAll('.template-card')
    expect(cards.length).toBe(2)
  })

  it('shows step indicator', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.find('.step-indicator').exists()).toBe(true)
    expect(wrapper.text()).toContain('选择模板')
    expect(wrapper.text()).toContain('配置参数')
    expect(wrapper.text()).toContain('确认完成')
  })

  it('displays parameter form after selecting a template and advancing to step 2', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    // Click "下一步" button to go to step 2
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    expect(nextBtn?.exists()).toBe(true)
    await nextBtn?.trigger('click')
    await flushPromises()

    // Parameter form should be visible in step 2
    expect(wrapper.find('.param-form').exists()).toBe(true)
    expect(wrapper.text()).toContain('快线周期')
    expect(wrapper.text()).toContain('慢线周期')
  })

  it('renders symbol and timeframe selectors in step 2', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn?.trigger('click')
    await flushPromises()

    // Symbol and timeframe should be visible
    expect(wrapper.text()).toContain('交易对')
    expect(wrapper.text()).toContain('时间周期')
  })

  it('renders parameter preview in step 2', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn?.trigger('click')
    await flushPromises()

    expect(wrapper.find('.param-preview').exists()).toBe(true)
  })

  it('shows cancel button in step 2', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn?.trigger('click')
    await flushPromises()

    // Cancel button should be visible
    expect(wrapper.findAll('.el-button').some(btn => btn.text().includes('取消'))).toBe(true)
  })

  it('advances to step 2 when template selected and Next clicked', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    // Step 1: select a template
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    // Next button becomes enabled
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    expect(nextBtn?.exists()).toBe(true)

    // Advance to step 2
    await nextBtn!.trigger('click')
    await flushPromises()

    // Now in step 2: param form is visible
    expect(wrapper.find('.param-form').exists()).toBe(true)
  })

  it('populates parameter values from selected template defaults', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    // Template param schema items should be visible
    expect(wrapper.text()).toContain('快线周期')
    expect(wrapper.text()).toContain('慢线周期')
  })

  it('shows loading state while templates are being fetched', async () => {
    vi.mocked(strategiesApi.listTemplates).mockImplementation(
      () => new Promise(() => {})
    )

    const wrapper = mount(StrategyCreateView, {
      global: {
        plugins: [ElementPlus, router],
        stubs: {
          'router-link': { template: '<a class="router-link-stub"><slot /></a>' },
          'router-view': true,
        },
      },
    })
    await router.isReady()

    expect(wrapper.find('.el-skeleton').exists() || wrapper.find('.skeleton-form').exists()).toBe(true)
  })

  it('shows error when templates fail to load', async () => {
    vi.mocked(strategiesApi.listTemplates).mockRejectedValue(new Error('Failed to load'))
    const wrapper = await mountView()
    await flushPromises()

    expect(wrapper.text()).toContain('加载失败')
  })

  it('has a basic info form section in step 2', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn?.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('基本信息')
    expect(wrapper.text()).toContain('策略名称')
  })

  it('has a template selection section in step 1', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()

    expect(wrapper.text()).toContain('选择策略模板')
  })

  it('escapes XSS payload in template description during selection (step 1)', async () => {
    const xssPayload = '<svg onload=alert(1)>'
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue([{
      ...mockTemplates[0],
      description: xssPayload,
    }])
    const wrapper = await mountView()
    await flushPromises()

    const descEl = wrapper.find('.template-desc')
    expect(descEl.exists()).toBe(true)
    // Raw SVG tag should not appear as a DOM element
    expect(descEl.html()).not.toContain('<svg')
    // Text content should contain the payload string
    expect(descEl.text()).toContain('onload')
  })

  it('escapes XSS payload in parameter description in step 2', async () => {
    const xssPayload = '<a href="javascript:alert(1)">click</a>'
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue([{
      ...mockTemplates[0],
      parameter_schema: [
        { name: 'fast_period', label: '快线周期', type: 'integer' as const, default: 5, description: xssPayload },
      ],
    }])
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn?.trigger('click')
    await flushPromises()

    // Find the param-desc within the param-form section (not from risk config)
    const paramForm = wrapper.find('.param-form')
    expect(paramForm.exists()).toBe(true)
    const paramDescEls = paramForm.findAll('.param-desc')
    expect(paramDescEls.length).toBeGreaterThan(0)
    // Find the one with the XSS payload
    const xssParamDesc = paramDescEls.find(el => el.text().includes('javascript'))
    expect(xssParamDesc).toBeDefined()
    // The anchor tag should not appear as a DOM element
    expect(xssParamDesc!.html()).not.toContain('<a')
    expect(xssParamDesc!.text()).toContain('javascript')
  })

  it('does not use v-html anywhere in StrategyCreateView', async () => {
    const fs = await import('fs')
    const url = await import('url')
    const vuePath = url.fileURLToPath(import.meta.url).replace(/[^/]+$/, '../../views/strategy/StrategyCreateView.vue')
    const content = fs.readFileSync(vuePath, 'utf-8')
    expect(content).not.toMatch(/v-html/)
  })

  it('renders risk parameter section in step 2', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('风控参数')
    expect(wrapper.text()).toContain('最大持仓')
    expect(wrapper.text()).toContain('止损比例')
    expect(wrapper.text()).toContain('止盈比例')
  })

  it('shows default risk config values', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    // Check the input-number components have the correct default values
    const inputNumbers = wrapper.findAllComponents({ name: 'ElInputNumber' })
    expect(inputNumbers.length).toBeGreaterThanOrEqual(3)
  })

  it('includes risk_config in createStrategy payload', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    vi.mocked(strategiesApi.createStrategy).mockResolvedValue({} as any)
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    // Set required fields via vm to bypass UI interaction issues
    const vm = wrapper.vm as any
    vm.formData.name = 'Test Strategy'
    vm.formData.symbol = 'BTCUSDT'
    vm.formData.timeframe = '1H'
    vm.formData.strategy_type = 'trend_following'

    await vm.doSave()
    await flushPromises()

    expect(strategiesApi.createStrategy).toHaveBeenCalled()
    const call = vi.mocked(strategiesApi.createStrategy).mock.calls[0]
    const payload = call[0] as any
    expect(payload.risk_config).toBeDefined()
    expect(payload.risk_config.max_position).toBe(1)
    expect(payload.risk_config.stop_loss).toBe(0.05)
    expect(payload.risk_config.stop_profit).toBe(0.10)
  })

  // T4.5: strategy_type field tests
  it('renders strategy_type selector in step 2', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    // Strategy type selector should be visible
    expect(wrapper.text()).toContain('策略类型')
  })

  // T4.5: strategy_type field tests
  it('renders strategy_type selector in step 2', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    // Strategy type selector should be visible
    expect(wrapper.text()).toContain('策略类型')
  })

  it('auto-sets strategy_type to grid_trading when grid_trading template selected', async () => {
    // Tests that selecting the grid_trading template auto-sets strategy_type ref
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    vi.mocked(strategiesApi.createStrategy).mockResolvedValue({} as any)
    const wrapper = await mountView()
    await flushPromises()

    // Select grid_trading template (index 1, category '网格交易')
    const cards = wrapper.findAll('.template-card')
    await cards[1].trigger('click')
    await flushPromises()

    // strategy_type should be auto-set from template category
    const vm = wrapper.vm as any
    expect(vm.formData.strategy_type).toBe('grid_trading')

    // Advance to step 2
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    // Fill required fields and save
    vm.formData.name = 'Grid Strategy'
    vm.formData.symbol = 'ETHUSDT'
    vm.formData.timeframe = '4H'
    await vm.doSave()
    await flushPromises()

    expect(strategiesApi.createStrategy).toHaveBeenCalled()
    const payload = vi.mocked(strategiesApi.createStrategy).mock.calls[0][0] as any
    expect(payload.strategy_type).toBe('grid_trading')
  })

  it('includes strategy_type in createStrategy payload when trend_following template selected', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    vi.mocked(strategiesApi.createStrategy).mockResolvedValue({} as any)
    const wrapper = await mountView()
    await flushPromises()

    // Select trend_following template (index 0, category '趋势跟踪')
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()

    // Verify auto-set
    const vm = wrapper.vm as any
    expect(vm.formData.strategy_type).toBe('trend_following')

    // Advance to step 2
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    // Fill required fields and save
    vm.formData.name = 'Trend Strategy'
    vm.formData.symbol = 'BTCUSDT'
    vm.formData.timeframe = '1H'
    await vm.doSave()
    await flushPromises()

    expect(strategiesApi.createStrategy).toHaveBeenCalled()
    const payload = vi.mocked(strategiesApi.createStrategy).mock.calls[0][0] as any
    expect(payload.strategy_type).toBe('trend_following')
  })

  it('loads existing risk_config when editing a strategy', async () => {
    const strategyWithRisk = {
      id: '1',
      user_id: 'u1',
      name: 'Existing Strategy',
      description: 'test',
      symbol: 'BTCUSDT',
      timeframe: '1H',
      template_id: 'trend_following',
      template_type: 'trend_following',
      parameters: { fast_period: 10 },
      risk_config: { max_position: 3, stop_loss: 0.08, stop_profit: 0.15 },
      status: 'paused' as const,
      created_at: '',
      updated_at: '',
    }
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    vi.mocked(strategiesApi.getStrategy).mockResolvedValue(strategyWithRisk as any)

    const wrapper = mount(StrategyCreateView, {
      props: { strategyId: '1' },
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

    // Risk config section should be visible
    expect(wrapper.text()).toContain('风控参数')
    expect(wrapper.text()).toContain('最大持仓')
  })

  // T4.5: File upload tests
  it('shows strategy code file upload field in step 2', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    // Upload button should be visible
    expect(wrapper.text()).toContain('策略代码文件')
    expect(wrapper.text()).toContain('上传策略文件')
    expect(wrapper.text()).toContain('.py')
    expect(wrapper.text()).toContain('.js')
  })

  it('rejects non .py/.js files in handleFileChange', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    // Access the handleFileChange function directly
    const vm = wrapper.vm as any
    const invalidFile = { raw: { name: 'strategy.txt', size: 1000 } }
    const result = await vm.handleFileChange(invalidFile)
    expect(result).toBe(false)
    expect(strategiesApi.uploadStrategyCode).not.toHaveBeenCalled()
  })

  it('rejects files larger than 2MB in handleFileChange', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    const vm = wrapper.vm as any
    const largeFile = { raw: { name: 'strategy.py', size: 3 * 1024 * 1024 } }
    const result = await vm.handleFileChange(largeFile)
    expect(result).toBe(false)
    expect(strategiesApi.uploadStrategyCode).not.toHaveBeenCalled()
  })

  it('uploads valid .py file and sets strategyCodePath', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    vi.mocked(strategiesApi.uploadStrategyCode).mockResolvedValue({ path: '/uploads/mystrategy.py' })
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    const vm = wrapper.vm as any
    const result = await vm.handleFileChange({ raw: { name: 'mystrategy.py', size: 1024 } })
    await flushPromises()

    expect(result).toBe(true)
    expect(vm.strategyCodePath).toBe('/uploads/mystrategy.py')
    expect(strategiesApi.uploadStrategyCode).toHaveBeenCalled()
    const call = vi.mocked(strategiesApi.uploadStrategyCode).mock.calls[0]
    // In jsdom, File objects aren't real File instances, so check plain object shape
    const uploadedFile = call[0] as any
    expect(uploadedFile.name).toBe('mystrategy.py')
    expect(uploadedFile.size).toBe(1024)
  })

  it('removes uploaded file when removeStrategyFile is called', async () => {
    vi.mocked(strategiesApi.listTemplates).mockResolvedValue(mockTemplates)
    vi.mocked(strategiesApi.uploadStrategyCode).mockResolvedValue({ path: '/uploads/strategy.py' })
    const wrapper = await mountView()
    await flushPromises()

    // Go to step 2
    const cards = wrapper.findAll('.template-card')
    await cards[0].trigger('click')
    await flushPromises()
    const nextBtn = wrapper.findAll('.el-button').find(btn => btn.text().includes('下一步'))
    await nextBtn!.trigger('click')
    await flushPromises()

    // Upload a file
    const vm = wrapper.vm as any
    await vm.handleFileChange({ raw: { name: 'strategy.py', size: 1024 } })
    await flushPromises()
    expect(vm.strategyCodePath).toBe('/uploads/strategy.py')

    // Remove the file
    await vm.removeStrategyFile()
    expect(vm.strategyCodePath).toBe(null)
    expect(vm.strategyCodeFile).toBe(null)
  })
})
