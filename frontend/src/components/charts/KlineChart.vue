<template>
  <div class="kline-chart" @contextmenu.prevent>
    <!-- OHLCV crosshair overlay -->
    <div v-if="ohlcvDisplay" class="ohlcv-overlay">
      <span class="ohlcv-item"><span class="ohlcv-label">O</span>{{ ohlcvDisplay.open }}</span>
      <span class="ohlcv-item"><span class="ohlcv-label">H</span>{{ ohlcvDisplay.high }}</span>
      <span class="ohlcv-item"><span class="ohlcv-label">L</span>{{ ohlcvDisplay.low }}</span>
      <span class="ohlcv-item"><span class="ohlcv-label">C</span>{{ ohlcvDisplay.close }}</span>
      <span class="ohlcv-item"><span class="ohlcv-label">V</span>{{ ohlcvDisplay.volume }}</span>
    </div>
    <div ref="mainChartRef" class="main-chart" />
    <div ref="macdChartRef" class="macd-chart" :class="{ visible: isSubChartVisible('macd') }" />
    <div ref="kdjChartRef" class="kdj-chart" :class="{ visible: isSubChartVisible('kdj') }" />
    <div ref="rsiChartRef" class="rsi-chart" :class="{ visible: isSubChartVisible('rsi') }" />
    <div ref="atrChartRef" class="atr-chart" :class="{ visible: isSubChartVisible('atr') }" />
    <div ref="stochChartRef" class="stoch-chart" :class="{ visible: isSubChartVisible('stoch') }" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { createChart, type IChartApi, type ISeriesApi, type CandlestickData, type Time } from 'lightweight-charts'

export interface KlineBar {
  time: number // Unix timestamp (seconds)
  open: number
  high: number
  low: number
  close: number
  volume?: number
}

/** Marker for a trade entry/exit on the chart */
export interface TradeMarker {
  time: number          // Unix timestamp (seconds)
  position: 'aboveBar' | 'belowBar' | 'insideBar'
  color: string
  shape: 'arrowUp' | 'arrowDown' | 'circle' | 'flag'
  text: string
}

/** Single MA line data */
export interface MaLine {
  period: number
  data: { open_time: number; ma: number }[]
}

/** MACD bar data */
export interface MacdBar {
  open_time: number
  macd: number
  signal: number
  histogram: number
}

/** KDJ bar data */
export interface KdjBar {
  open_time: number
  k: number
  d: number
  j: number
}

/** RSI bar data */
export interface RsiBar {
  open_time: number
  rsi: number
}

/** Bollinger Bands bar data */
export interface BollingerBar {
  open_time: number
  upper: number
  middle: number
  lower: number
}

/** EMA bar data */
export interface EmaLine {
  period: number
  data: { open_time: number; ema: number }[]
}

/** ATR bar data */
export interface AtrBar {
  open_time: number
  atr: number
}

/** Stochastic bar data */
export interface StochasticBar {
  open_time: number
  k: number
  d: number
}

export interface SubChartHeightVars {
  '--subchart-h': string
}

const props = withDefaults(defineProps<{
  data?: KlineBar[]
  symbol?: string
  interval?: string
  darkMode?: boolean
  markers?: TradeMarker[]
  maData?: MaLine[]
  emaData?: EmaLine[]
  macdData?: MacdBar[]
  kdjData?: KdjBar[]
  rsiData?: RsiBar[]
  bollingerData?: BollingerBar[]
  atrData?: AtrBar[]
  stochData?: StochasticBar[]
  visibleSubCharts?: string[]
}>(), {
  data: () => [],
  symbol: '',
  interval: '',
  darkMode: true,
  markers: () => [],
  maData: () => [],
  emaData: () => [],
  macdData: () => [],
  kdjData: () => [],
  rsiData: () => [],
  bollingerData: () => [],
  atrData: () => [],
  stochData: () => [],
  visibleSubCharts: () => ['macd', 'kdj'],
})

const mainChartRef = ref<HTMLDivElement | null>(null)
const macdChartRef = ref<HTMLDivElement | null>(null)
const kdjChartRef = ref<HTMLDivElement | null>(null)
const rsiChartRef = ref<HTMLDivElement | null>(null)
const atrChartRef = ref<HTMLDivElement | null>(null)
const stochChartRef = ref<HTMLDivElement | null>(null)
const ohlcvDisplay = ref<{ open: string; high: string; low: string; close: string; volume: string } | null>(null)
let chart: IChartApi | null = null
let macdChart: IChartApi | null = null
let kdjChart: IChartApi | null = null
let rsiChart: IChartApi | null = null
let atrChart: IChartApi | null = null
let stochChart: IChartApi | null = null
let candleSeries: ISeriesApi<'Candlestick'> | null = null
let volumeSeries: ISeriesApi<'Histogram'> | null = null
let macdHistogramSeries: ISeriesApi<'Histogram'> | null = null
let macdSignalSeries: ISeriesApi<'Line'> | null = null
let kdjKSeries: ISeriesApi<'Line'> | null = null
let kdjDSeries: ISeriesApi<'Line'> | null = null
let kdjJSeries: ISeriesApi<'Line'> | null = null
let rsiSeries: ISeriesApi<'Line'> | null = null
let bollingerUpperSeries: ISeriesApi<'Line'> | null = null
let bollingerMiddleSeries: ISeriesApi<'Line'> | null = null
let bollingerLowerSeries: ISeriesApi<'Line'> | null = null
let atrSeries: ISeriesApi<'Line'> | null = null
let stochKSeries: ISeriesApi<'Line'> | null = null
let stochDSeries: ISeriesApi<'Line'> | null = null
const maSeriesMap = new Map<number, ISeriesApi<'Line'>>()
const emaSeriesMap = new Map<number, ISeriesApi<'Line'>>()

function buildChartOptions() {
  const isDark = props.darkMode
  return {
    layout: {
      background: { color: isDark ? '#08090a' : '#ffffff' },
      textColor: isDark ? '#8a8f98' : '#374151',
      fontSize: 11,
    },
    grid: {
      vertLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
      horzLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
    },
    crosshair: {
      mode: 1, // CrosshairMode.Normal
      vertLine: {
        color: 'rgba(113, 112, 255, 0.5)',
        width: 1 as const,
        style: 2 as const, // LineStyle.Dashed
        labelBackgroundColor: '#7170ff',
      },
      horzLine: {
        color: 'rgba(113, 112, 255, 0.5)',
        width: 1 as const,
        style: 2 as const,
        labelBackgroundColor: '#7170ff',
      },
    },
    rightPriceScale: {
      borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)',
      scaleMargins: { top: 0.1, bottom: 0.25 },
    },
    timeScale: {
      borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)',
      timeVisible: true,
      secondsVisible: false,
    },
    handleScale: { axisPressedMouseMove: true },
    handleScroll: { mouseWheel: true, pressedMouseMove: true },
  }
}

function buildCandlestickOptions() {
  // Binance green #26A69A, red #EF5350
  return {
    upColor: '#26A69A',
    downColor: '#EF5350',
    borderUpColor: '#26A69A',
    borderDownColor: '#EF5350',
    wickUpColor: '#26A69A',
    wickDownColor: '#EF5350',
  }
}

function buildVolumeOptions() {
  const isDark = props.darkMode
  return {
    priceFormat: { type: 'volume' as const },
    priceScaleId: 'volume',
    scaleMargins: { top: 0.8, bottom: 0 },
  }
}

const MA_COLORS: Record<number, string> = {
  7: '#FFD700',
  25: '#A855F7',
  99: '#3B82F6',
  200: '#F56C6C',
}

const EMA_COLORS: Record<number, string> = {
  9: '#FFD700',
  21: '#A855F7',
}

function buildMaOptions(period: number) {
  return {
    color: MA_COLORS[period] ?? '#8a8f98',
    lineWidth: 1 as const,
    priceLineVisible: false,
    lastValueVisible: true,
  }
}

function renderMaLines() {
  if (!chart || props.maData.length === 0) return

  for (const maLine of props.maData) {
    const existingSeries = maSeriesMap.get(maLine.period)
    if (existingSeries) {
      existingSeries.setData([])
      chart.removeSeries(existingSeries)
      maSeriesMap.delete(maLine.period)
    }

    const series = chart.addLineSeries(buildMaOptions(maLine.period))
    const lineData = maLine.data.map(d => ({
      time: toLightweightTime(d.open_time),
      value: d.ma,
    }))
    series.setData(lineData as any)
    maSeriesMap.set(maLine.period, series)
  }
}

function renderEmaLines() {
  if (!chart || props.emaData.length === 0) return

  for (const emaLine of props.emaData) {
    const existingSeries = emaSeriesMap.get(emaLine.period)
    if (existingSeries) {
      existingSeries.setData([])
      chart.removeSeries(existingSeries)
      emaSeriesMap.delete(emaLine.period)
    }

    const series = chart.addLineSeries({
      color: EMA_COLORS[emaLine.period] ?? '#8a8f98',
      lineWidth: 1 as const,
      priceLineVisible: false,
      lastValueVisible: true,
    })
    const lineData = emaLine.data.map(d => ({
      time: toLightweightTime(d.open_time),
      value: d.ema,
    }))
    series.setData(lineData as any)
    emaSeriesMap.set(emaLine.period, series)
  }
}

function renderBollingerLines() {
  if (!chart) return

  // Clean up existing BB series
  if (bollingerUpperSeries) {
    chart.removeSeries(bollingerUpperSeries)
    bollingerUpperSeries = null
  }
  if (bollingerMiddleSeries) {
    chart.removeSeries(bollingerMiddleSeries)
    bollingerMiddleSeries = null
  }
  if (bollingerLowerSeries) {
    chart.removeSeries(bollingerLowerSeries)
    bollingerLowerSeries = null
  }

  if (!props.bollingerData || props.bollingerData.length === 0) return

  const bbOptions = (color: string) => ({
    color,
    lineWidth: 1 as const,
    priceLineVisible: false,
    lastValueVisible: false,
  })

  bollingerUpperSeries = chart.addLineSeries(bbOptions('rgba(113, 112, 255, 0.7)'))
  bollingerMiddleSeries = chart.addLineSeries(bbOptions('rgba(113, 112, 255, 0.5)'))
  bollingerLowerSeries = chart.addLineSeries(bbOptions('rgba(113, 112, 255, 0.7)'))

  const bars = props.bollingerData
  bollingerUpperSeries.setData(bars.map(b => ({ time: toLightweightTime(b.open_time), value: b.upper })) as any)
  bollingerMiddleSeries.setData(bars.map(b => ({ time: toLightweightTime(b.open_time), value: b.middle })) as any)
  bollingerLowerSeries.setData(bars.map(b => ({ time: toLightweightTime(b.open_time), value: b.lower })) as any)
}

function initChart() {
  if (!mainChartRef.value) return

  if (chart) {
    chart.remove()
    chart = null
    candleSeries = null
    volumeSeries = null
    maSeriesMap.clear()
  }

  chart = createChart(mainChartRef.value, {
    ...buildChartOptions(),
    autoSize: true,
  })

  candleSeries = chart.addCandlestickSeries(buildCandlestickOptions())

  volumeSeries = chart.addHistogramSeries(buildVolumeOptions())

  // Markers series for trade entry/exit points
  // Note: markers are applied via candleSeries.setMarkers() — no separate series needed

  // Volume scale
  if (volumeSeries) {
    chart.priceScale('volume').applyOptions({
      scaleMargins: { top: 0.85, bottom: 0 },
    })
  }

  if (props.data.length > 0) {
    setData(props.data)
  }

  // Crosshair OHLCV display
  chart.subscribeCrosshairMove((param) => {
    if (!param.time || !param.seriesData.size) {
      ohlcvDisplay.value = null
      return
    }
    const candleData = param.seriesData.get(candleSeries! as any)
    if (candleData) {
      const c = candleData as CandlestickData<Time>
      ohlcvDisplay.value = {
        open: typeof c.open === 'number' ? c.open.toFixed(2) : String(c.open),
        high: typeof c.high === 'number' ? c.high.toFixed(2) : String(c.high),
        low: typeof c.low === 'number' ? c.low.toFixed(2) : String(c.low),
        close: typeof c.close === 'number' ? c.close.toFixed(2) : String(c.close),
        volume: (param.seriesData.get(volumeSeries!) as any)?.value !== undefined
          ? String((param.seriesData.get(volumeSeries!) as any)?.value ?? '')
          : '',
      }
    }
  })

  renderMaLines()
  renderEmaLines()
  renderBollingerLines()
  // Init sub-charts immediately (DOM exists); they'll render when data arrives
  initMacdChart()
  initKdjChart()
  initRsiChart()
  initAtrChart()
  initStochChart()
}

function initMacdChart() {
  if (!macdChartRef.value) return

  if (macdChart) return // already initialized

  const isDark = props.darkMode
  macdChart = createChart(macdChartRef.value, {
    layout: {
      background: { color: isDark ? '#08090a' : '#ffffff' },
      textColor: isDark ? '#8a8f98' : '#374151',
      fontSize: 10,
    },
    grid: {
      vertLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
      horzLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
    },
    crosshair: { mode: 0 },
    rightPriceScale: { borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)' },
    timeScale: {
      borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)',
      timeVisible: true,
      secondsVisible: false,
    },
    handleScale: false,
    handleScroll: false,
    width: mainChartRef.value?.clientWidth ?? 600,
    height: 120,
    autoSize: true,
  })

  macdHistogramSeries = macdChart.addHistogramSeries({
    priceFormat: { type: 'price' as const, precision: 4, minMove: 0.0001 },
    priceScaleId: 'macd',
  })
  macdChart.priceScale('macd').applyOptions({ scaleMargins: { top: 0.1, bottom: 0.1 } })

  macdSignalSeries = macdChart.addLineSeries({
    color: '#3B82F6',
    lineWidth: 1 as const,
    priceLineVisible: false,
    lastValueVisible: false,
  })

  renderMacdData()
}

function renderMacdData() {
  if (!macdChart || !macdHistogramSeries || !macdSignalSeries || !props.macdData.length) return

  const bars = props.macdData
  macdHistogramSeries.setData(bars.map(b => ({
    time: toLightweightTime(b.open_time),
    value: b.histogram,
    color: b.histogram >= 0 ? 'rgba(103, 194, 58, 0.5)' : 'rgba(245, 108, 108, 0.5)',
  })) as any)

  macdSignalSeries.setData(bars.map(b => ({
    time: toLightweightTime(b.open_time),
    value: b.signal,
  })) as any)

  macdChart.timeScale().fitContent()
}

function initKdjChart() {
  if (!kdjChartRef.value) return

  if (kdjChart) return // already initialized

  const isDark = props.darkMode
  kdjChart = createChart(kdjChartRef.value, {
    layout: {
      background: { color: isDark ? '#08090a' : '#ffffff' },
      textColor: isDark ? '#8a8f98' : '#374151',
      fontSize: 10,
    },
    grid: {
      vertLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
      horzLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
    },
    crosshair: { mode: 0 },
    rightPriceScale: { borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)' },
    timeScale: {
      borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)',
      timeVisible: true,
      secondsVisible: false,
    },
    handleScale: false,
    handleScroll: false,
    width: mainChartRef.value?.clientWidth ?? 600,
    height: 120,
    autoSize: true,
  })

  kdjKSeries = kdjChart.addLineSeries({ color: '#FFD700', lineWidth: 1 as const, priceLineVisible: false, lastValueVisible: false })
  kdjDSeries = kdjChart.addLineSeries({ color: '#A855F7', lineWidth: 1 as const, priceLineVisible: false, lastValueVisible: false })
  kdjJSeries = kdjChart.addLineSeries({ color: '#3B82F6', lineWidth: 1 as const, priceLineVisible: false, lastValueVisible: false })

  renderKdjData()
}

function renderKdjData() {
  if (!kdjChart || !kdjKSeries || !kdjDSeries || !kdjJSeries || !props.kdjData.length) return

  const bars = props.kdjData
  kdjKSeries.setData(bars.map(b => ({ time: toLightweightTime(b.open_time), value: b.k })) as any)
  kdjDSeries.setData(bars.map(b => ({ time: toLightweightTime(b.open_time), value: b.d })) as any)
  kdjJSeries.setData(bars.map(b => ({ time: toLightweightTime(b.open_time), value: b.j })) as any)

  kdjChart.timeScale().fitContent()
}

function initRsiChart() {
  if (!rsiChartRef.value) return

  if (rsiChart) return // already initialized

  const isDark = props.darkMode
  rsiChart = createChart(rsiChartRef.value, {
    layout: {
      background: { color: isDark ? '#08090a' : '#ffffff' },
      textColor: isDark ? '#8a8f98' : '#374151',
      fontSize: 10,
    },
    grid: {
      vertLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
      horzLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
    },
    crosshair: { mode: 0 },
    rightPriceScale: { borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)' },
    timeScale: {
      borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)',
      timeVisible: true,
      secondsVisible: false,
    },
    handleScale: false,
    handleScroll: false,
    width: mainChartRef.value?.clientWidth ?? 600,
    height: 120,
    autoSize: true,
  })

  rsiSeries = rsiChart.addLineSeries({
    color: '#A855F7',
    lineWidth: 1 as const,
    priceLineVisible: false,
    lastValueVisible: false,
  })

  renderRsiData()
}

function renderRsiData() {
  if (!rsiChart || !rsiSeries || !props.rsiData.length) return

  rsiSeries.setData(props.rsiData.map(b => ({
    time: toLightweightTime(b.open_time),
    value: b.rsi,
  })) as any)

  rsiChart.timeScale().fitContent()
}

function initAtrChart() {
  if (!atrChartRef.value) return
  if (atrChart) return

  const isDark = props.darkMode
  atrChart = createChart(atrChartRef.value, {
    layout: {
      background: { color: isDark ? '#08090a' : '#ffffff' },
      textColor: isDark ? '#8a8f98' : '#374151',
      fontSize: 10,
    },
    grid: {
      vertLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
      horzLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
    },
    crosshair: { mode: 0 },
    rightPriceScale: { borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)' },
    timeScale: {
      borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)',
      timeVisible: true,
      secondsVisible: false,
    },
    handleScale: false,
    handleScroll: false,
    width: mainChartRef.value?.clientWidth ?? 600,
    height: 120,
    autoSize: true,
  })

  atrSeries = atrChart.addLineSeries({
    color: '#26A69A',
    lineWidth: 1 as const,
    priceLineVisible: false,
    lastValueVisible: false,
  })

  renderAtrData()
}

function renderAtrData() {
  if (!atrChart || !atrSeries || !props.atrData.length) return

  atrSeries.setData(props.atrData.map(b => ({
    time: toLightweightTime(b.open_time),
    value: b.atr,
  })) as any)

  atrChart.timeScale().fitContent()
}

function initStochChart() {
  if (!stochChartRef.value) return
  if (stochChart) return

  const isDark = props.darkMode
  stochChart = createChart(stochChartRef.value, {
    layout: {
      background: { color: isDark ? '#08090a' : '#ffffff' },
      textColor: isDark ? '#8a8f98' : '#374151',
      fontSize: 10,
    },
    grid: {
      vertLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
      horzLines: { color: isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.08)' },
    },
    crosshair: { mode: 0 },
    rightPriceScale: { borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)' },
    timeScale: {
      borderColor: isDark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.1)',
      timeVisible: true,
      secondsVisible: false,
    },
    handleScale: false,
    handleScroll: false,
    width: mainChartRef.value?.clientWidth ?? 600,
    height: 120,
    autoSize: true,
  })

  stochKSeries = stochChart.addLineSeries({ color: '#FFD700', lineWidth: 1 as const, priceLineVisible: false, lastValueVisible: false })
  stochDSeries = stochChart.addLineSeries({ color: '#A855F7', lineWidth: 1 as const, priceLineVisible: false, lastValueVisible: false })

  renderStochData()
}

function renderStochData() {
  if (!stochChart || !stochKSeries || !stochDSeries || !props.stochData.length) return

  const bars = props.stochData
  stochKSeries.setData(bars.map(b => ({ time: toLightweightTime(b.open_time), value: b.k })) as any)
  stochDSeries.setData(bars.map(b => ({ time: toLightweightTime(b.open_time), value: b.d })) as any)

  stochChart.timeScale().fitContent()
}

function isSubChartVisible(name: string): boolean {
  return props.visibleSubCharts.includes(name)
}

function toLightweightTime(t: number): Time {
  // Backend returns milliseconds; lightweight-charts expects Unix seconds
  return (t >= 1e12 ? Math.floor(t / 1000) : t) as Time
}

function setData(bars: KlineBar[]) {
  if (!candleSeries || !volumeSeries || bars.length === 0) return

  const candleData: CandlestickData<Time>[] = bars.map(bar => ({
    time: toLightweightTime(bar.time),
    open: bar.open,
    high: bar.high,
    low: bar.low,
    close: bar.close,
  }))

  const volumeData = bars.map(bar => ({
    time: toLightweightTime(bar.time),
    value: bar.volume ?? 0,
    color: bar.close >= bar.open
      ? 'rgba(103, 194, 58, 0.3)'   // green with opacity
      : 'rgba(245, 108, 108, 0.3)', // red with opacity
  }))

  candleSeries.setData(candleData)
  volumeSeries.setData(volumeData)
  // Apply trade markers
  setMarkers(props.markers)
  chart?.timeScale().fitContent()
}

function setMarkers(markers: TradeMarker[]) {
  if (!candleSeries || markers.length === 0) return
  const lwMarkers = markers.map(m => ({
    time: toLightweightTime(m.time) as Time,
    position: m.position,
    color: m.color,
    shape: m.shape,
    text: m.text,
  }))
  candleSeries.setMarkers(lwMarkers as any)
}

function addBar(bar: KlineBar) {
  if (!candleSeries || !volumeSeries) return
  const t = toLightweightTime(bar.time)
  candleSeries.update({ time: t, open: bar.open, high: bar.high, low: bar.low, close: bar.close })
  volumeSeries.update({
    time: t,
    value: bar.volume ?? 0,
    color: bar.close >= bar.open ? 'rgba(103, 194, 58, 0.3)' : 'rgba(245, 108, 108, 0.3)',
  })
}

let resizeObserver: ResizeObserver | null = null
let containerObserver: ResizeObserver | null = null

onMounted(() => {
  initChart()

  // Resize chart when container changes
  const container = mainChartRef.value?.closest('.chart-area') as HTMLElement | null
  if (container) {
    containerObserver = new ResizeObserver(() => {
      resizeAllCharts()
    })
    containerObserver.observe(container)
  }

  resizeObserver = new ResizeObserver(() => {
    resizeAllCharts()
  })
  if (mainChartRef.value) {
    resizeObserver.observe(mainChartRef.value)
  }
})

function resizeAllCharts() {
  const targetWidth = mainChartRef.value?.clientWidth ?? 0
  const targetHeight = mainChartRef.value?.clientHeight ?? 0

  if (chart) {
    chart.applyOptions({ width: targetWidth, height: targetHeight })
  }
  if (macdChart) {
    macdChart.applyOptions({ width: targetWidth })
  }
  if (kdjChart) {
    kdjChart.applyOptions({ width: targetWidth })
  }
  if (rsiChart) {
    rsiChart.applyOptions({ width: targetWidth })
  }
  if (atrChart) {
    atrChart.applyOptions({ width: targetWidth })
  }
  if (stochChart) {
    stochChart.applyOptions({ width: targetWidth })
  }
}

onUnmounted(() => {
  resizeObserver?.disconnect()
  containerObserver?.disconnect()
  if (chart) {
    chart.remove()
    chart = null
  }
  if (macdChart) {
    macdChart.remove()
    macdChart = null
  }
  if (kdjChart) {
    kdjChart.remove()
    kdjChart = null
  }
  if (rsiChart) {
    rsiChart.remove()
    rsiChart = null
  }
  if (atrChart) {
    atrChart.remove()
    atrChart = null
  }
  if (stochChart) {
    stochChart.remove()
    stochChart = null
  }
})

watch(() => props.data, (newData) => {
  if (newData && newData.length > 0) {
    setData(newData)
  }
}, { deep: true })

watch(() => props.darkMode, () => {
  initChart()
})

watch(() => props.markers, (newMarkers) => {
  setMarkers(newMarkers)
}, { deep: true })

watch(() => props.maData, (newMaData) => {
  if (newMaData && newMaData.length > 0) {
    renderMaLines()
  }
}, { deep: true })

watch(() => props.macdData, async () => {
  if (props.macdData && props.macdData.length > 0) {
    if (!macdChart) {
      await nextTick()
      initMacdChart()
    } else {
      renderMacdData()
    }
  }
}, { deep: true })

watch(() => props.kdjData, async () => {
  if (props.kdjData && props.kdjData.length > 0) {
    if (!kdjChart) {
      await nextTick()
      initKdjChart()
    } else {
      renderKdjData()
    }
  }
}, { deep: true })

watch(() => props.rsiData, async () => {
  if (props.rsiData && props.rsiData.length > 0) {
    if (!rsiChart) {
      await nextTick()
      initRsiChart()
    } else {
      renderRsiData()
    }
  }
}, { deep: true })

watch(() => props.bollingerData, () => {
  if (props.bollingerData && props.bollingerData.length > 0) {
    renderBollingerLines()
  }
}, { deep: true })

watch(() => props.emaData, () => {
  if (props.emaData && props.emaData.length > 0) {
    renderEmaLines()
  }
}, { deep: true })

watch(() => props.atrData, async () => {
  if (props.atrData && props.atrData.length > 0) {
    if (!atrChart) {
      await nextTick()
      initAtrChart()
    } else {
      renderAtrData()
    }
  }
}, { deep: true })

watch(() => props.stochData, async () => {
  if (props.stochData && props.stochData.length > 0) {
    if (!stochChart) {
      await nextTick()
      initStochChart()
    } else {
      renderStochData()
    }
  }
}, { deep: true })

defineExpose({ addBar, setData, setMarkers })
</script>

<style scoped>
.kline-chart {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.main-chart {
  flex: 1 1 auto;
  min-height: 0;
  width: 100%;
}

.macd-chart {
  flex: 0 0 auto;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  overflow: hidden;
  opacity: 0;
  transition: opacity 0.3s ease;
  height: 120px;
}
.macd-chart.visible {
  opacity: 1;
}

.kdj-chart {
  flex: 0 0 auto;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  overflow: hidden;
  opacity: 0;
  transition: opacity 0.3s ease;
  height: 120px;
}
.kdj-chart.visible {
  opacity: 1;
}

.rsi-chart {
  flex: 0 0 auto;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  overflow: hidden;
  opacity: 0;
  transition: opacity 0.3s ease;
  height: 120px;
}
.rsi-chart.visible {
  opacity: 1;
}

.atr-chart {
  flex: 0 0 auto;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  overflow: hidden;
  opacity: 0;
  transition: opacity 0.3s ease;
  height: 120px;
}

.stoch-chart {
  flex: 0 0 auto;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  overflow: hidden;
  opacity: 0;
  transition: opacity 0.3s ease;
  height: 120px;
}

.ohlcv-overlay {
  position: absolute;
  top: 8px;
  left: 12px;
  z-index: 100;
  display: flex;
  gap: 12px;
  padding: 6px 12px;
  background: rgba(24, 25, 26, 0.9);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  font-family: 'JetBrains Mono', 'SF Mono', monospace;
  font-size: 12px;
  pointer-events: none;
  backdrop-filter: blur(4px);
}

.ohlcv-item {
  display: flex;
  align-items: center;
  gap: 4px;
  color: #d0d6e0;
}

.ohlcv-label {
  color: #7170ff;
  font-weight: 700;
}
</style>
