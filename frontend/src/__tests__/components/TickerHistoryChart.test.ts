/**
 * TickerHistoryChart 组件测试
 *
 * 策略：由于 lightweight-charts 在 jsdom 中无法真正渲染图表，
 * mock 掉整个图表库，只测试组件的逻辑行为（props 响应、API 调用、事件发射）。
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'

// ============================================================================
// Global ResizeObserver mock (needed because jsdom does not implement it)
// ============================================================================

const mockRO = {
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}
vi.stubGlobal('ResizeObserver', vi.fn(() => mockRO))

// ============================================================================
// Mock data (hoisted)
// ============================================================================

vi.mock('lightweight-charts', () => ({
  createChart: vi.fn(() => ({
    remove: vi.fn(),
    applyOptions: vi.fn(),
    timeScale: vi.fn().mockReturnValue({ fitContent: vi.fn(), applyOptions: vi.fn() }),
    priceScale: vi.fn().mockReturnValue({ applyOptions: vi.fn() }),
    addCandlestickSeries: vi.fn().mockReturnValue({
      setData: vi.fn(),
      update: vi.fn(),
      applyOptions: vi.fn(),
    }),
    addHistogramSeries: vi.fn().mockReturnValue({
      setData: vi.fn(),
      update: vi.fn(),
      applyOptions: vi.fn(),
    }),
  })),
  CrosshairMode: { Normal: 1 },
}))

// Hoisted mock implementations - these are evaluated BEFORE vi.mock calls
const mockGetKline = vi.hoisted(() => vi.fn())

vi.mock('@/api/market', () => ({
  getKline: mockGetKline,
}))

// ============================================================================
// Test data (hoisted so vi.mock factory can access them)
// ============================================================================

const mockKlineData = [
  { timestamp: 1715600000000, open: 50000, high: 51000, low: 49500, close: 50500, volume: 1000 },
  { timestamp: 1715603600000, open: 50500, high: 51500, low: 50000, close: 51000, volume: 1200 },
  { timestamp: 1715607200000, open: 51000, high: 52000, low: 50500, close: 51500, volume: 1500 },
  { timestamp: 1715610800000, open: 51500, high: 52500, low: 51000, close: 52000, volume: 1100 },
  { timestamp: 1715614400000, open: 52000, high: 53000, low: 51500, close: 52500, volume: 1300 },
]

const mockFallingKlineData = [
  { timestamp: 1715600000000, open: 52500, high: 53000, low: 51500, close: 52000, volume: 1000 },
  { timestamp: 1715603600000, open: 52000, high: 52500, low: 51000, close: 51500, volume: 1200 },
  { timestamp: 1715607200000, open: 51500, high: 52000, low: 50500, close: 51000, volume: 1500 },
  { timestamp: 1715610800000, open: 51000, high: 51500, low: 50000, close: 50500, volume: 1100 },
  { timestamp: 1715614400000, open: 50500, high: 51000, low: 49500, close: 50000, volume: 1300 },
]

// ============================================================================
// Imports (after mocks)
// ============================================================================

import TickerHistoryChart from '@/components/market/TickerHistoryChart.vue'

// ============================================================================
// Test setup
// ============================================================================

const globalStubs = {
  ElRadioGroup: {
    template: '<div class="el-radio-group"><slot /></div>',
    props: ['modelValue'],
    emits: ['change', 'update:modelValue'],
  },
  ElRadioButton: {
    template: '<button class="el-radio-button">{{ label }}</button>',
    props: ['label'],
  },
  ElIcon: { template: '<i class="el-icon"><slot /></i>' },
  Loading: { template: '<span class="loading-icon" />' },
}

describe('TickerHistoryChart', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockGetKline.mockResolvedValue(mockKlineData)
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  // -------------------------------------------------------------------------
  // Rendering
  // -------------------------------------------------------------------------

  it('given symbol prop when render then displays formatted symbol name', () => {
    const wrapper = mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: false },
      global: { stubs: globalStubs },
    })
    expect(wrapper.find('.symbol-name').text()).toBe('BTC/USDT')
  })

  it('given render then chart container exists', () => {
    const wrapper = mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: false },
      global: { stubs: globalStubs },
    })
    expect(wrapper.find('.chart-container').exists()).toBe(true)
    expect(wrapper.find('.ticker-history-chart').exists()).toBe(true)
  })

  // -------------------------------------------------------------------------
  // API calls
  // -------------------------------------------------------------------------

  it('given autoLoad false when render then does not call getKline', async () => {
    mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: false },
      global: { stubs: globalStubs },
    })
    await new Promise((r) => setTimeout(r, 50))
    expect(mockGetKline).not.toHaveBeenCalled()
  })

  it('given autoLoad true when render then calls getKline with symbol and default interval', async () => {
    mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: true },
      global: { stubs: globalStubs },
    })
    await new Promise((r) => setTimeout(r, 50))
    expect(mockGetKline).toHaveBeenCalledWith('BTCUSDT', '1h')
  })

  // -------------------------------------------------------------------------
  // Data loading
  // -------------------------------------------------------------------------

  it('given autoLoad true when data loads then klineData is populated', async () => {
    const wrapper = mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: true },
      global: { stubs: globalStubs },
    })
    await new Promise((r) => setTimeout(r, 100))
    const vm = wrapper.vm as any
    expect(vm.klineData).toHaveLength(5)
    expect(vm.klineData[0].close).toBe(50500)
  })

  it('given price increases when data loads then priceDirection is price-up', async () => {
    const wrapper = mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: true },
      global: { stubs: globalStubs },
    })
    await new Promise((r) => setTimeout(r, 100))
    const vm = wrapper.vm as any
    expect(vm.priceDirection).toBe('price-up')
  })

  it('given price decreases when data loads then priceDirection is price-down', async () => {
    mockGetKline.mockResolvedValue(mockFallingKlineData)
    const wrapper = mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: true },
      global: { stubs: globalStubs },
    })
    await new Promise((r) => setTimeout(r, 100))
    const vm = wrapper.vm as any
    expect(vm.priceDirection).toBe('price-down')
  })

  it('given symbol changes when changed then reloads data with new symbol', async () => {
    const wrapper = mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: true },
      global: { stubs: globalStubs },
    })
    await new Promise((r) => setTimeout(r, 50))
    expect(mockGetKline).toHaveBeenCalledWith('BTCUSDT', '1h')

    await wrapper.setProps({ symbol: 'ETHUSDT' })
    await new Promise((r) => setTimeout(r, 50))
    expect(mockGetKline).toHaveBeenCalledWith('ETHUSDT', '1h')
  })

  // -------------------------------------------------------------------------
  // Events
  // -------------------------------------------------------------------------

  it('given interval changes then emits intervalChange event', async () => {
    const wrapper = mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: false },
      global: { stubs: globalStubs },
    })
    const vm = wrapper.vm as any
    vm.onIntervalChange('4h')
    await new Promise((r) => setTimeout(r, 10))
    expect(wrapper.emitted('intervalChange')).toBeTruthy()
  })

  it('given autoLoad true when data loads then emits priceUpdate event', async () => {
    const wrapper = mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: true },
      global: { stubs: globalStubs },
    })
    await new Promise((r) => setTimeout(r, 100))
    expect(wrapper.emitted('priceUpdate')).toBeTruthy()
  })

  // -------------------------------------------------------------------------
  // Exposed methods
  // -------------------------------------------------------------------------

  it('given addBar exposed method when called then does not throw', () => {
    const wrapper = mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: false },
      global: { stubs: globalStubs },
    })
    const vm = wrapper.vm as any
    const bar = { time: 1715618000, open: 52500, high: 53000, low: 52000, close: 52800, volume: 800 }
    expect(() => vm.addBar(bar)).not.toThrow()
  })

  it('given setData exposed method when called then does not throw', () => {
    const wrapper = mount(TickerHistoryChart, {
      props: { symbol: 'BTCUSDT', autoLoad: false },
      global: { stubs: globalStubs },
    })
    const vm = wrapper.vm as any
    const bars = [{ time: 1715618000, open: 52500, high: 53000, low: 52000, close: 52800, volume: 800 }]
    expect(() => vm.setData(bars)).not.toThrow()
  })
})
