/**
 * MultiTimeframeChart 组件测试 (P2-3)
 *
 * 策略：mock 掉 lightweight-charts 整个图表库（jsdom 不可用），只测试组件
 * 的逻辑行为：
 *   1. 双图渲染（主图 + 副图 容器都挂载）
 *   2. 副图周期切换（1h → 4h → 1d）触发 queryKlines 并刷新数据
 *   3. 数据同步：监听 `kline-visible-range-change` 事件并调用 sub chart 的
 *      setVisibleRange，使副图 X 轴跟随主图
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'

// ============================================================================
// ResizeObserver mock (jsdom)
// ============================================================================

const mockRO = {
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}
vi.stubGlobal('ResizeObserver', vi.fn(() => mockRO))

// ============================================================================
// lightweight-charts mock — 跟踪 setVisibleRange / subscribeVisibleTimeRangeChange
// ============================================================================
//
// IMPORTANT: we share a single object for `timeScale` so that vi.clearAllMocks
// (used in beforeEach) doesn't blow away the mockReturnValue on a fresh vi.fn.
// vi.clearAllMocks() resets mockReturnValue on plain vi.fn() instances in
// vitest ≥0.31, so we use mockImplementation to make the return value resilient.

const setVisibleRangeMock = vi.fn()
const subscribeVisibleTimeRangeChangeMock = vi.fn()
const timeScaleMock = {
  fitContent: vi.fn(),
  applyOptions: vi.fn(),
  setVisibleRange: setVisibleRangeMock,
  subscribeVisibleTimeRangeChange: subscribeVisibleTimeRangeChangeMock,
}

vi.mock('lightweight-charts', () => ({
  createChart: vi.fn(() => ({
    remove: vi.fn(),
    applyOptions: vi.fn(),
    timeScale: vi.fn(() => timeScaleMock),
    priceScale: vi.fn(() => ({ applyOptions: vi.fn() })),
    addCandlestickSeries: vi.fn(() => ({
      setData: vi.fn(),
      update: vi.fn(),
      applyOptions: vi.fn(),
    })),
    addHistogramSeries: vi.fn(() => ({
      setData: vi.fn(),
      update: vi.fn(),
      applyOptions: vi.fn(),
    })),
    addLineSeries: vi.fn(() => ({
      setData: vi.fn(),
      update: vi.fn(),
      applyOptions: vi.fn(),
    })),
  })),
  CrosshairMode: { Normal: 1 },
}))

// ============================================================================
// @/api/kline mock — queryKlines
// ============================================================================

const mockQueryKlines = vi.hoisted(() => vi.fn())

vi.mock('@/api/kline', () => ({
  queryKlines: mockQueryKlines,
}))

// ============================================================================
// Test data
// ============================================================================

const sub1hData = Array.from({ length: 50 }, (_, i) => ({
  timestamp: 1715600000000 + i * 3_600_000, // 1h in ms
  open: 50000 + i * 10,
  high: 50100 + i * 10,
  low: 49900 + i * 10,
  close: 50050 + i * 10,
  volume: 100 + i,
}))

const sub4hData = Array.from({ length: 30 }, (_, i) => ({
  timestamp: 1715600000000 + i * 4 * 3_600_000,
  open: 51000 + i * 5,
  high: 51100 + i * 5,
  low: 50900 + i * 5,
  close: 51050 + i * 5,
  volume: 200 + i,
}))

const sub1dData = Array.from({ length: 20 }, (_, i) => ({
  timestamp: 1715600000000 + i * 86_400_000,
  open: 52000 + i * 2,
  high: 52100 + i * 2,
  low: 51900 + i * 2,
  close: 52050 + i * 2,
  volume: 1000 + i,
}))

// ============================================================================
// Imports (after mocks)
// ============================================================================

import MultiTimeframeChart from '@/components/charts/MultiTimeframeChart.vue'
import ElMessage from 'element-plus'

// Element Plus 静默 toast，避免污染测试输出
vi.mock('element-plus', async (importOriginal) => {
  const mod = await importOriginal<typeof import('element-plus')>()
  return {
    ...mod,
    ElMessage: { warning: vi.fn(), error: vi.fn(), success: vi.fn() },
  }
})

// ============================================================================
// Global stubs
// ============================================================================

const globalStubs = {
  ElRadioGroup: {
    template: '<div class="el-radio-group"><slot /></div>',
    props: ['modelValue'],
    emits: ['change', 'update:modelValue'],
  },
  ElRadioButton: {
    template: '<button class="el-radio-button" :data-value="value"><slot /></button>',
    props: ['value'],
  },
  // Stub the embedded main KlineChart so we don't try to mount lightweight-charts twice
  KlineChart: {
    template: '<div class="kline-chart-stub" />',
    props: [
      'data', 'symbol', 'interval', 'darkMode',
      'maData', 'emaData', 'macdData', 'kdjData',
      'rsiData', 'bollingerData', 'atrData', 'stochData',
      'visibleSubCharts',
    ],
  },
}

// ============================================================================
// Tests
// ============================================================================

describe('MultiTimeframeChart (P2-3)', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockQueryKlines.mockImplementation(({ interval }: { interval: string }) => {
      if (interval === '1h') return Promise.resolve({ data: { data: sub1hData } })
      if (interval === '4h') return Promise.resolve({ data: { data: sub4hData } })
      if (interval === '1d') return Promise.resolve({ data: { data: sub1dData } })
      return Promise.resolve({ data: { data: [] } })
    })
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  // -------------------------------------------------------------------------
  // 1. 双图渲染
  // -------------------------------------------------------------------------

  it('given symbol when mounted then sub chart container and main chart stub render', async () => {
    const wrapper = mount(MultiTimeframeChart, {
      props: {
        mainData: [],
        mainSymbol: 'BTCUSDT',
        mainInterval: '1m',
        subLimit: 200,
      },
      global: { stubs: globalStubs },
    })
    await flushPromises()

    // 副图挂载点（.mtf-sub-chart）
    expect(wrapper.find('.mtf-sub-chart').exists()).toBe(true)
    // 内部 KlineChart 渲染了 stub
    expect(wrapper.find('.kline-chart-stub').exists()).toBe(true)
    // interval 切换控件 (3 个 radio)
    const radios = wrapper.findAll('.el-radio-button')
    expect(radios).toHaveLength(3)
    wrapper.unmount()
  })

  it('given sub chart loads then queryKlines is called with correct symbol/interval', async () => {
    const wrapper = mount(MultiTimeframeChart, {
      props: {
        mainData: [],
        mainSymbol: 'BTCUSDT',
        mainInterval: '1m',
      },
      global: { stubs: globalStubs },
    })
    await flushPromises()

    expect(mockQueryKlines).toHaveBeenCalledWith(
      expect.objectContaining({ symbol: 'BTCUSDT', interval: '1h', size: expect.any(Number) }),
    )
    wrapper.unmount()
  })

  // -------------------------------------------------------------------------
  // 2. 副图周期切换
  // -------------------------------------------------------------------------

  it('given sub interval change when user picks 4h then queryKlines refetches with 4h', async () => {
    const wrapper = mount(MultiTimeframeChart, {
      props: {
        mainData: [],
        mainSymbol: 'BTCUSDT',
        mainInterval: '1m',
        initialSubInterval: '1h',
      },
      global: { stubs: globalStubs },
    })
    await flushPromises()

    // Reset mock to ignore the initial 1h fetch
    mockQueryKlines.mockClear()

    // Find the 4h radio button and click it
    const radio4h = wrapper.findAll('.el-radio-button').find(
      (b: any) => b.attributes('data-value') === '4h',
    )!
    expect(radio4h.exists()).toBe(true)

    // Trigger the @change on the ElRadioGroup stub — the component listens for
    // 'change' on the group, not the buttons.  Easiest path: call the
    // onSubIntervalChange method directly via the component instance.
    const vm = wrapper.vm as any
    vm.onSubIntervalChange('4h')
    await flushPromises()

    expect(mockQueryKlines).toHaveBeenCalledWith(
      expect.objectContaining({ symbol: 'BTCUSDT', interval: '4h' }),
    )
    // subInterval ref updates
    expect(vm.subInterval).toBe('4h')
    // emits to parent
    expect(wrapper.emitted('sub-interval-change')).toBeTruthy()
    expect(wrapper.emitted('sub-interval-change')![0]).toEqual(['4h'])
    wrapper.unmount()
  })

  it('given sub interval changes to 1d then sub chart data is replaced with 1d bars', async () => {
    const wrapper = mount(MultiTimeframeChart, {
      props: {
        mainData: [],
        mainSymbol: 'ETHUSDT',
        mainInterval: '1m',
        initialSubInterval: '1h',
      },
      global: { stubs: globalStubs },
    })
    await flushPromises()

    const vm = wrapper.vm as any
    vm.onSubIntervalChange('1d')
    await flushPromises()

    expect(vm.subData).toHaveLength(sub1dData.length)
    expect(vm.subData[0].time).toBe(Math.floor(sub1dData[0].timestamp / 1000))
    wrapper.unmount()
  })

  // -------------------------------------------------------------------------
  // 3. 数据同步 (X 轴同步)
  // -------------------------------------------------------------------------

  it('given main chart dispatches kline-visible-range-change then sub chart visible range syncs', async () => {
    const wrapper = mount(MultiTimeframeChart, {
      props: {
        mainData: [],
        mainSymbol: 'BTCUSDT',
        mainInterval: '1m',
      },
      global: { stubs: globalStubs },
    })
    await flushPromises()

    setVisibleRangeMock.mockClear()

    // Simulate the main KlineChart dispatching the range event
    const fromSec = 1_715_600_000
    const toSec = 1_715_700_000
    window.dispatchEvent(
      new CustomEvent('kline-visible-range-change', {
        detail: { from: fromSec, to: toSec },
      }),
    )
    await flushPromises()

    expect(setVisibleRangeMock).toHaveBeenCalledWith(
      expect.objectContaining({ from: fromSec, to: toSec }),
    )

    wrapper.unmount()
  })

  it('given multiple kline-visible-range-change events then sub chart follows each', async () => {
    const wrapper = mount(MultiTimeframeChart, {
      props: {
        mainData: [],
        mainSymbol: 'BTCUSDT',
        mainInterval: '1m',
      },
      global: { stubs: globalStubs },
    })
    await flushPromises()

    // Clear all calls so far (mount + subData load may have caused some
    // internal chart calls we don't care about)
    setVisibleRangeMock.mockClear()

    // Two consecutive pans
    window.dispatchEvent(
      new CustomEvent('kline-visible-range-change', {
        detail: { from: 1_715_600_000, to: 1_715_700_000 },
      }),
    )
    // wait one rAF tick so suppressNextSync guard resets
    await new Promise((r) => requestAnimationFrame(() => r(null)))
    window.dispatchEvent(
      new CustomEvent('kline-visible-range-change', {
        detail: { from: 1_715_650_000, to: 1_715_750_000 },
      }),
    )
    await flushPromises()

    // Each event should call setVisibleRange exactly once
    expect(setVisibleRangeMock).toHaveBeenCalledTimes(2)
    // Check last two calls (in case earlier mount lifecycle called it indirectly)
    const calls = setVisibleRangeMock.mock.calls
    expect(calls[calls.length - 2][0]).toMatchObject({ from: 1_715_600_000, to: 1_715_700_000 })
    expect(calls[calls.length - 1][0]).toMatchObject({ from: 1_715_650_000, to: 1_715_750_000 })

    wrapper.unmount()
  })

  // -------------------------------------------------------------------------
  // Bonus: addSubBar exposed method
  // -------------------------------------------------------------------------

  it('given addSubBar called with a new bar then sub data array is appended', async () => {
    const wrapper = mount(MultiTimeframeChart, {
      props: {
        mainData: [],
        mainSymbol: 'BTCUSDT',
        mainInterval: '1m',
        initialSubInterval: '1h',
      },
      global: { stubs: globalStubs },
    })
    await flushPromises()

    const vm = wrapper.vm as any
    const newBar = {
      time: 1_715_800_000,
      open: 60000,
      high: 60100,
      low: 59900,
      close: 60050,
      volume: 500,
    }
    vm.addSubBar(newBar)
    await flushPromises()

    expect(vm.subData[vm.subData.length - 1]).toEqual(newBar)
    expect(wrapper.emitted('sub-bar')).toBeTruthy()
    wrapper.unmount()
  })
})
