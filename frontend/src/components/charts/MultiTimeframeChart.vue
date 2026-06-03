<template>
  <div class="mtf-chart">
    <!-- Header: sub-chart interval selector -->
    <div class="mtf-header">
      <div class="mtf-header-left">
        <span class="mtf-label">副图</span>
        <el-radio-group
          v-model="subInterval"
          size="small"
          class="mtf-interval-group"
          @change="onSubIntervalChange"
        >
          <el-radio-button value="1h">1h</el-radio-button>
          <el-radio-button value="4h">4h</el-radio-button>
          <el-radio-button value="1d">1d</el-radio-button>
        </el-radio-group>
      </div>
      <div v-if="subSymbol" class="mtf-header-right">
        <span class="mtf-symbol">{{ subSymbol }}</span>
        <span v-if="subLastPrice" class="mtf-price">${{ subLastPrice }}</span>
      </div>
    </div>

    <!-- Sub chart -->
    <div ref="subChartRef" class="mtf-sub-chart" />

    <!-- Main chart (driven by parent) -->
    <KlineChart
      ref="mainChartRef"
      :data="mainData"
      :symbol="mainSymbol"
      :interval="mainInterval"
      :dark-mode="true"
      :ma-data="maData"
      :ema-data="emaData"
      :macd-data="macdData"
      :kdj-data="kdjData"
      :rsi-data="rsiData"
      :bollinger-data="bollingerData"
      :atr-data="atrData"
      :stoch-data="stochData"
      :visible-sub-charts="visibleSubCharts"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { createChart, type IChartApi, type ISeriesApi, type Time } from 'lightweight-charts'
import KlineChart from './KlineChart.vue'
import type { KlineBar } from './KlineChart.vue'
import { queryKlines } from '@/api/kline'
import { ElMessage } from 'element-plus'

/**
 * MultiTimeframeChart — multi-timeframe linked chart (P2-3)
 *
 * Layout: sub chart (1h/4h/1d, user-selectable) on top, main chart (driven by
 * parent's interval) on bottom.  Both charts share the same X-axis: the main
 * chart owns the visible time-range, the sub chart follows it via
 * `timeScale().subscribeVisibleTimeRangeChange()`.  This is the same pattern
 * TradingView's "Multi-chart layout" uses.
 *
 * Data flow:
 *   - Main chart: parent passes `mainData` / `mainInterval` props.
 *   - Sub chart:  this component fetches its own klines via /kline/query
 *                 using `subInterval` (1h/4h/1d).
 *   - Live updates: parent calls `addSubBar(bar)` (exposed below) when a kline
 *     WS message arrives for the (symbol, subInterval) pair.
 */

const SUB_INTERVALS = ['1h', '4h', '1d'] as const
type SubInterval = (typeof SUB_INTERVALS)[number]

const props = withDefaults(defineProps<{
  /** Main chart kline data (parent-owned). */
  mainData?: KlineBar[]
  /** Main chart symbol (e.g. "BTCUSDT"). */
  mainSymbol?: string
  /** Main chart interval (e.g. "1m").  Display only — data comes from `mainData`. */
  mainInterval?: string
  /** Sub chart initial interval. Default: 1h. */
  initialSubInterval?: SubInterval
  /** Number of klines to fetch for the sub chart. */
  subLimit?: number
  /** Indicator overlays (forwarded to the internal KlineChart). */
  maData?: any[]
  emaData?: any[]
  macdData?: any[]
  kdjData?: any[]
  rsiData?: any[]
  bollingerData?: any[]
  atrData?: any[]
  stochData?: any[]
  visibleSubCharts?: string[]
}>(), {
  mainData: () => [],
  mainSymbol: '',
  mainInterval: '',
  initialSubInterval: '1h',
  subLimit: 200,
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

const emit = defineEmits<{
  (e: 'sub-interval-change', interval: SubInterval): void
  (e: 'sub-data-loaded', data: KlineBar[]): void
  (e: 'sub-bar', bar: KlineBar): void
}>()

const subInterval = ref<SubInterval>(props.initialSubInterval)
const subSymbol = ref<string>(props.mainSymbol)
const subData = ref<KlineBar[]>([])
const subLastPrice = ref<string>('')

const subChartRef = ref<HTMLDivElement | null>(null)
const mainChartRef = ref<InstanceType<typeof KlineChart> | null>(null)

let subChart: IChartApi | null = null
let subCandleSeries: ISeriesApi<'Candlestick'> | null = null
let subVolumeSeries: ISeriesApi<'Histogram'> | null = null
let suppressNextSync = false

function buildSubChartOptions() {
  return {
    layout: {
      background: { color: '#08090a' },
      textColor: '#8a8f98',
      fontSize: 10,
    },
    grid: {
      vertLines: { color: 'rgba(255,255,255,0.06)' },
      horzLines: { color: 'rgba(255,255,255,0.06)' },
    },
    crosshair: {
      mode: 1 as const,
      vertLine: {
        color: 'rgba(59, 130, 246, 0.4)',
        width: 1 as const,
        style: 2 as const,
        labelBackgroundColor: '#3B82F6',
      },
      horzLine: {
        color: 'rgba(59, 130, 246, 0.4)',
        width: 1 as const,
        style: 2 as const,
        labelBackgroundColor: '#3B82F6',
      },
    },
    rightPriceScale: {
      borderColor: 'rgba(255,255,255,0.08)',
      scaleMargins: { top: 0.1, bottom: 0.25 },
    },
    timeScale: {
      borderColor: 'rgba(255,255,255,0.08)',
      timeVisible: true,
      secondsVisible: false,
    },
    handleScale: { axisPressedMouseMove: true },
    handleScroll: { mouseWheel: true, pressedMouseMove: true },
  }
}

function initSubChart() {
  if (!subChartRef.value) return

  // Re-init guard
  if (subChart) {
    subChart.remove()
    subChart = null
    subCandleSeries = null
    subVolumeSeries = null
  }

  subChart = createChart(subChartRef.value, {
    ...buildSubChartOptions(),
    autoSize: true,
  })

  subCandleSeries = subChart.addCandlestickSeries({
    upColor: '#26A69A',
    downColor: '#EF5350',
    borderUpColor: '#26A69A',
    borderDownColor: '#EF5350',
    wickUpColor: '#26A69A',
    wickDownColor: '#EF5350',
  })

  subVolumeSeries = subChart.addHistogramSeries({
    priceFormat: { type: 'volume' as const },
    priceScaleId: 'volume',
  })
  if (subVolumeSeries) {
    subChart.priceScale('volume').applyOptions({
      scaleMargins: { top: 0.85, bottom: 0 },
    })
  }

  if (subData.value.length > 0) {
    setSubData(subData.value)
  }
}

function setSubData(data: KlineBar[]) {
  if (!subCandleSeries || !subVolumeSeries) return
  subCandleSeries.setData(
    data.map(b => ({
      time: b.time as Time,
      open: b.open,
      high: b.high,
      low: b.low,
      close: b.close,
    })) as any,
  )
  subVolumeSeries.setData(
    data.map(b => ({
      time: b.time as Time,
      value: b.volume ?? 0,
      color: b.close >= b.open ? 'rgba(38, 166, 154, 0.4)' : 'rgba(239, 83, 80, 0.4)',
    })) as any,
  )
  if (data.length > 0) {
    subLastPrice.value = data[data.length - 1].close.toFixed(2)
  }
}

async function loadSubData() {
  if (!props.mainSymbol) {
    subData.value = []
    return
  }
  try {
    const symbol = props.mainSymbol.replace('/', '')
    const res: any = await queryKlines({
      symbol,
      interval: subInterval.value,
      size: props.subLimit,
    })
    const raw = res?.data?.data ?? res?.data ?? res ?? []
    const bars: KlineBar[] = (Array.isArray(raw) ? raw : []).map((b: any) => ({
      time: Math.floor((b.timestamp ?? b.time ?? b.open_time) / 1000),
      open: parseFloat(b.open),
      high: parseFloat(b.high),
      low: parseFloat(b.low),
      close: parseFloat(b.close),
      volume: parseFloat(b.volume ?? 0),
    }))
    subData.value = bars
    subSymbol.value = props.mainSymbol
    if (bars.length > 0) {
      setSubData(bars)
    } else {
      // Clear the chart when backend returns no data
      subCandleSeries?.setData([] as any)
      subVolumeSeries?.setData([] as any)
      subLastPrice.value = ''
    }
    emit('sub-data-loaded', bars)
  } catch (e) {
    console.error('[MultiTimeframeChart] loadSubData error:', e)
    subData.value = []
    ElMessage.warning(`加载 ${subInterval.value} K线失败`)
  }
}

function onSubIntervalChange(next: SubInterval | string | number | boolean | undefined) {
  const v = String(next) as SubInterval
  if (!SUB_INTERVALS.includes(v)) return
  subInterval.value = v
  emit('sub-interval-change', v)
  loadSubData()
}

/**
 * Sync the sub chart's visible range from the main chart.
 * This is the core of "multi-timeframe linkage" — the main chart owns
 * navigation, the sub chart follows.  We listen for the
 * `kline-visible-range-change` window event that KlineChart dispatches
 * (see KlineChart.vue initChart()).
 */
function attachTimeRangeSync() {
  // No-op: the window-event listener is wired in onMounted.  This function
  // is kept as a hook for future bidirectional sync (e.g. syncing sub → main
  // when the user pans the sub chart).
  void mainChartRef.value
}

/**
 * Update the sub chart's visible time range to mirror the main chart.
 * Called by the parent (or by a window event the main chart emits).
 */
function setSubVisibleRange(fromSec: number, toSec: number) {
  if (!subChart) return
  const ts = subChart.timeScale()
  if (!ts || typeof ts.setVisibleRange !== 'function') return
  suppressNextSync = true
  ts.setVisibleRange({
    from: fromSec as Time,
    to: toSec as Time,
  } as any)
  // Reset guard on next animation frame so the sub chart can be panned
  // independently afterwards
  requestAnimationFrame(() => {
    suppressNextSync = false
  })
}

function onVisibleRangeEvent(e: Event) {
  if (suppressNextSync) return
  const detail = (e as CustomEvent<{ from: number; to: number }>).detail
  if (!detail) return
  setSubVisibleRange(detail.from, detail.to)
}

/**
 * Append/update a single bar in the sub chart (live update path).
 * Parent calls this when a kline WS message arrives.
 */
function addSubBar(bar: KlineBar) {
  if (!subCandleSeries || !subVolumeSeries) return
  const t = bar.time as Time
  subCandleSeries.update({
    time: t,
    open: bar.open,
    high: bar.high,
    low: bar.low,
    close: bar.close,
  } as any)
  subVolumeSeries.update({
    time: t,
    value: bar.volume ?? 0,
    color: bar.close >= bar.open ? 'rgba(38, 166, 154, 0.4)' : 'rgba(239, 83, 80, 0.4)',
  } as any)
  // Update or insert into the cached array
  const last = subData.value[subData.value.length - 1]
  if (last && last.time === bar.time) {
    subData.value = [...subData.value.slice(0, -1), bar]
  } else if (!last || bar.time > last.time) {
    subData.value = [...subData.value, bar]
  }
  subLastPrice.value = bar.close.toFixed(2)
  emit('sub-bar', bar)
}

function fitSubContent() {
  subChart?.timeScale().fitContent()
}

defineExpose({
  /** Reload sub chart klines for the current symbol + interval. */
  reload: loadSubData,
  /** Push a single live bar into the sub chart. */
  addSubBar,
  /** Force the sub chart to fit its content. */
  fitContent: fitSubContent,
  /** Sync sub chart's visible range to a [fromSec, toSec] window. */
  setSubVisibleRange,
  /** Current sub chart interval (read-only). */
  get subInterval(): SubInterval { return subInterval.value },
  /** Current sub chart data (read-only). */
  get subData(): KlineBar[] { return subData.value },
})

// ── lifecycle ─────────────────────────────────────────────────────────────

onMounted(async () => {
  await nextTick()
  initSubChart()
  await loadSubData()
  attachTimeRangeSync()
  // Listen for range sync events from the main KlineChart
  window.addEventListener('kline-visible-range-change', onVisibleRangeEvent)
})

onBeforeUnmount(() => {
  window.removeEventListener('kline-visible-range-change', onVisibleRangeEvent)
  if (subChart) {
    subChart.remove()
    subChart = null
    subCandleSeries = null
    subVolumeSeries = null
  }
})

// Re-fetch sub data when the parent changes symbol
watch(() => props.mainSymbol, (newSym, oldSym) => {
  if (newSym !== oldSym) {
    subSymbol.value = newSym
    loadSubData()
  }
})

// Re-fetch sub data when mainData is cleared (parent symbol switch)
watch(() => props.mainData, (newData) => {
  if (newData.length === 0) {
    subData.value = []
    subCandleSeries?.setData([] as any)
    subVolumeSeries?.setData([] as any)
  }
})
</script>

<style scoped>
.mtf-chart {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--color-bg, #08090a);
}

.mtf-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 36px;
  padding: 0 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  flex-shrink: 0;
  background: rgba(25, 26, 27, 0.5);
}

.mtf-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mtf-header-right {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: 'JetBrains Mono', 'SF Mono', monospace;
  font-size: 12px;
}

.mtf-label {
  color: var(--color-text-tertiary, #8a8f98);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.mtf-interval-group {
  --el-radio-button-checked-bg-color: var(--color-primary, #3B82F6);
  --el-radio-button-checked-text-color: #fff;
}

.mtf-symbol {
  color: var(--color-text-primary, #f7f8f8);
  font-weight: 600;
}

.mtf-price {
  color: var(--color-primary, #3B82F6);
  font-weight: 600;
}

.mtf-sub-chart {
  flex: 0 0 35%;
  min-height: 180px;
  width: 100%;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  position: relative;
}
</style>
