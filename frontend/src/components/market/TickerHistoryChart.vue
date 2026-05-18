<template>
  <div class="ticker-history-chart">
    <div class="chart-header">
      <div class="symbol-info">
        <span class="symbol-name">{{ symbolDisplay }}</span>
        <span v-if="lastPrice" class="last-price" :class="priceDirection">
          {{ formatNumber(lastPrice) }}
          <span class="price-change" v-if="priceChange">
            {{ priceChange > 0 ? '+' : '' }}{{ formatNumber(priceChange) }}
            ({{ priceChangePercent !== null && priceChangePercent > 0 ? '+' : '' }}{{ formatPercent(priceChangePercent ?? 0) }})
          </span>
        </span>
      </div>
      <div class="chart-controls">
        <el-radio-group v-model="interval" size="small" @change="onIntervalChange">
          <el-radio-button label="1m">1m</el-radio-button>
          <el-radio-button label="5m">5m</el-radio-button>
          <el-radio-button label="15m">15m</el-radio-button>
          <el-radio-button label="1h">1H</el-radio-button>
          <el-radio-button label="4h">4H</el-radio-button>
          <el-radio-button label="1d">1D</el-radio-button>
        </el-radio-group>
      </div>
    </div>
    <div ref="chartContainerRef" class="chart-container" />
    <div v-if="loading" class="chart-loading">
      <el-icon class="is-loading"><Loading /></el-icon>
      <span>加载历史数据...</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { createChart, type IChartApi, type ISeriesApi, type CandlestickData, type Time, CrosshairMode } from 'lightweight-charts'
import { Loading } from '@element-plus/icons-vue'
import { getKline } from '@/api/market'
import type { Kline } from '@/types'
import { formatSymbol } from '@/types'

export interface HistoryBar {
  time: number   // Unix timestamp (seconds)
  open: number
  high: number
  low: number
  close: number
  volume?: number
}

const props = withDefaults(defineProps<{
  symbol?: string
  interval?: string
  darkMode?: boolean
  autoLoad?: boolean
}>(), {
  symbol: 'BTCUSDT',
  interval: '1h',
  darkMode: true,
  autoLoad: true,
})

const emit = defineEmits<{
  intervalChange: [interval: string]
  priceUpdate: [price: number, change: number, changePercent: number]
}>()

const chartContainerRef = ref<HTMLDivElement | null>(null)
let chart: IChartApi | null = null
let candleSeries: ISeriesApi<'Candlestick'> | null = null
let volumeSeries: ISeriesApi<'Histogram'> | null = null

const klineData = ref<Kline[]>([])
const loading = ref(false)
const interval = ref(props.interval)

const lastPrice = computed(() => {
  if (klineData.value.length === 0) return null
  return klineData.value[klineData.value.length - 1].close
})

const priceChange = computed(() => {
  if (klineData.value.length < 2) return null
  const first = klineData.value[0].open
  const last = klineData.value[klineData.value.length - 1].close
  return last - first
})

const priceChangePercent = computed(() => {
  if (klineData.value.length < 2) return null
  const first = klineData.value[0].open
  if (first === 0) return null
  return ((priceChange.value ?? 0) / first) * 100
})

const priceDirection = computed(() => {
  if (!priceChange.value) return ''
  return priceChange.value > 0 ? 'price-up' : 'price-down'
})

const symbolDisplay = computed(() => formatSymbol(props.symbol))

function formatNumber(value: number): string {
  return value.toFixed(2)
}

function formatPercent(value: number): string {
  return value.toFixed(2) + '%'
}

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
      mode: CrosshairMode.Normal,
      vertLine: {
        color: 'rgba(113, 112, 255, 0.5)',
        width: 1 as const,
        style: 2 as const,
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
  return {
    upColor: '#67C23A',
    downColor: '#F56C6C',
    borderUpColor: '#67C23A',
    borderDownColor: '#F56C6C',
    wickUpColor: '#67C23A',
    wickDownColor: '#F56C6C',
  }
}

function buildVolumeOptions() {
  return {
    priceFormat: { type: 'volume' as const },
    priceScaleId: 'volume',
    scaleMargins: { top: 0.8, bottom: 0 },
  }
}

function initChart() {
  if (!chartContainerRef.value) return

  if (chart) {
    chart.remove()
    chart = null
    candleSeries = null
    volumeSeries = null
  }

  chart = createChart(chartContainerRef.value, {
    ...buildChartOptions(),
    width: chartContainerRef.value.clientWidth,
    height: chartContainerRef.value.clientHeight || 400,
    autoSize: true,
  })

  candleSeries = chart.addCandlestickSeries(buildCandlestickOptions())

  volumeSeries = chart.addHistogramSeries(buildVolumeOptions())

  // Volume scale
  if (volumeSeries) {
    chart.priceScale('volume').applyOptions({
      scaleMargins: { top: 0.85, bottom: 0 },
    })
  }
}

function toLightweightTime(t: number): Time {
  return t as Time
}

function setData(bars: HistoryBar[]) {
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
      ? 'rgba(103, 194, 58, 0.3)'
      : 'rgba(245, 108, 108, 0.3)',
  }))

  candleSeries.setData(candleData)
  volumeSeries.setData(volumeData)
  chart?.timeScale().fitContent()
}

function addBar(bar: HistoryBar) {
  if (!candleSeries || !volumeSeries) return
  const t = toLightweightTime(bar.time)
  candleSeries.update({ time: t, open: bar.open, high: bar.high, low: bar.low, close: bar.close })
  volumeSeries.update({
    time: t,
    value: bar.volume ?? 0,
    color: bar.close >= bar.open ? 'rgba(103, 194, 58, 0.3)' : 'rgba(245, 108, 108, 0.3)',
  })
}

async function loadKlineData() {
  if (!props.symbol) return
  loading.value = true
  try {
    const data = await getKline(props.symbol, interval.value)
    klineData.value = data
    if (data.length > 0) {
      const bars: HistoryBar[] = data.map(k => ({
        time: Math.floor(k.timestamp / 1000),
        open: k.open,
        high: k.high,
        low: k.low,
        close: k.close,
        volume: k.volume,
      }))
      setData(bars)

      // Emit price update for parent components
      if (bars.length > 0) {
        const last = bars[bars.length - 1]
        const first = bars[0]
        const change = last.close - first.open
        const changePercent = first.open > 0 ? (change / first.open) * 100 : 0
        emit('priceUpdate', last.close, change, changePercent)
      }
    }
  } catch (e) {
    console.error('Failed to load kline data:', e)
  } finally {
    loading.value = false
  }
}

function onIntervalChange(val: string) {
  emit('intervalChange', val)
  loadKlineData()
}

let resizeObserver: ResizeObserver | null = null

onMounted(() => {
  initChart()

  resizeObserver = new ResizeObserver(() => {
    if (chart && chartContainerRef.value) {
      chart.applyOptions({
        width: chartContainerRef.value.clientWidth,
        height: chartContainerRef.value.clientHeight,
      })
    }
  })
  if (chartContainerRef.value) {
    resizeObserver.observe(chartContainerRef.value)
  }

  if (props.autoLoad) {
    loadKlineData()
  }
})

onUnmounted(() => {
  resizeObserver?.disconnect()
  if (chart) {
    chart.remove()
    chart = null
  }
})

watch(() => props.symbol, () => {
  if (props.autoLoad) {
    loadKlineData()
  }
})

watch(() => props.darkMode, () => {
  initChart()
  if (klineData.value.length > 0) {
    const bars: HistoryBar[] = klineData.value.map(k => ({
      time: Math.floor(k.timestamp / 1000),
      open: k.open,
      high: k.high,
      low: k.low,
      close: k.close,
      volume: k.volume,
    }))
    setData(bars)
  }
})

defineExpose({
  addBar,
  setData,
  loadKlineData,
  klineData,
})
</script>

<style scoped lang="scss">
.ticker-history-chart {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 400px;
  background: var(--color-surface);
  border-radius: 8px;
  border: 1px solid var(--color-border);
}

.chart-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;

  .symbol-info {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .symbol-name {
    font-size: 16px;
    font-weight: 600;
    color: var(--color-text-primary);
  }

  .last-price {
    font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
    font-size: 16px;
    font-weight: 600;

    &.price-up {
      color: var(--color-buy);
    }
    &.price-down {
      color: var(--color-sell);
    }
  }

  .price-change {
    font-size: 13px;
    margin-left: 8px;
    font-weight: 400;
  }
}

.chart-controls {
  :deep(.el-radio-group) {
    display: flex;
  }

  :deep(.el-radio-button__inner) {
    background: transparent;
    border-color: var(--color-border);
    color: var(--color-text-tertiary);
    font-size: 12px;
    padding: 4px 10px;
    height: 28px;
    line-height: 20px;

    &:hover {
      color: var(--color-text-secondary);
    }
  }

  :deep(.el-radio-button__original-radio:checked + .el-radio-button__inner) {
    background: var(--color-accent);
    border-color: var(--color-accent);
    color: #ffffff;
    box-shadow: none;
  }
}

.chart-container {
  flex: 1;
  min-height: 300px;
}

.chart-loading {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: var(--color-text-tertiary);
  font-size: 13px;
  background: rgba(8, 9, 10, 0.8);
  padding: 16px 24px;
  border-radius: 8px;
}
</style>
