<template>
  <div ref="chartContainerRef" class="kline-chart" />
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { createChart, type IChartApi, type ISeriesApi, type CandlestickData, type Time } from 'lightweight-charts'

export interface KlineBar {
  time: number // Unix timestamp (seconds)
  open: number
  high: number
  low: number
  close: number
  volume?: number
}

const props = withDefaults(defineProps<{
  data?: KlineBar[]
  symbol?: string
  interval?: string
  darkMode?: boolean
}>(), {
  data: () => [],
  symbol: '',
  interval: '',
  darkMode: true,
})

const chartContainerRef = ref<HTMLDivElement | null>(null)
let chart: IChartApi | null = null
let candleSeries: ISeriesApi<'Candlestick'> | null = null
let volumeSeries: ISeriesApi<'Histogram'> | null = null

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
        width: 1,
        style: 2, // LineStyle.Dashed
        labelBackgroundColor: '#7170ff',
      },
      horzLine: {
        color: 'rgba(113, 112, 255, 0.5)',
        width: 1,
        style: 2,
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
  const isDark = props.darkMode
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

  if (props.data.length > 0) {
    setData(props.data)
  }
}

function toLightweightTime(t: number): Time {
  // Kline stores seconds, lightweight-charts expects seconds as number
  return t as Time
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
  chart?.timeScale().fitContent()
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
})

onUnmounted(() => {
  resizeObserver?.disconnect()
  if (chart) {
    chart.remove()
    chart = null
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

defineExpose({ addBar, setData })
</script>

<style scoped>
.kline-chart {
  width: 100%;
  height: 100%;
  min-height: 300px;
}
</style>