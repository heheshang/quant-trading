<template>
  <div ref="chartRef" class="depth-chart"></div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import * as echarts from 'echarts'
import type { Depth } from '@/types'

const props = defineProps<{
  depth: Depth | null
  lastPrice?: number
}>()

const emit = defineEmits<{
  priceHover: [price: number | null]
}>()

const chartRef = ref<HTMLDivElement>()
let chart: echarts.ECharts | null = null
let resizeHandler: (() => void) | null = null

function initChart() {
  if (!chartRef.value) return
  chart = echarts.init(chartRef.value, undefined, { renderer: 'canvas' })

  // Tooltip 事件 → 盘口联动
  chart.on('showTip', (params: any) => {
    // 当 tooltip 显示时，传递当前价格
  })
}

function updateChart() {
  if (!chart || !props.depth) {
    if (chart) {
      chart.setOption({
        title: { text: '暂无深度数据', textStyle: { color: '#8a8f98', fontSize: 14 }, left: 'center', top: 'center' },
      })
    }
    return
  }

  const { bids, asks } = props.depth

  // 买盘按价格升序 (从低价到高价)
  const bidsSorted = [...bids].sort((a, b) => a.price - b.price)
  // 卖盘已按价格升序
  const asksSorted = [...asks]

  const bidData = bidsSorted.map(l => [l.price, l.total])
  const askData = asksSorted.map(l => [l.price, l.total])

  const option: echarts.EChartsOption = {
    animation: true,
    animationDuration: 300,
    animationEasing: 'cubicOut',
    grid: {
      left: 16,
      right: 50,
      top: 10,
      bottom: 30,
    },
    xAxis: {
      type: 'value',
      position: 'bottom',
      axisLine: { lineStyle: { color: 'rgba(255,255,255,0.06)' } },
      axisLabel: {
        color: '#8a8f98',
        fontSize: 11,
        fontFamily: 'JetBrains Mono, monospace',
        formatter: (val: number) => val.toLocaleString('en-US', { maximumFractionDigits: 2 }),
      },
      splitLine: { show: false },
    },
    yAxis: {
      type: 'value',
      position: 'right',
      axisLine: { lineStyle: { color: 'rgba(255,255,255,0.06)' } },
      axisLabel: {
        color: '#8a8f98',
        fontSize: 11,
        fontFamily: 'JetBrains Mono, monospace',
        formatter: (val: number) => {
          if (val >= 1_000_000_000) return `${(val / 1_000_000_000).toFixed(1)}B`
          if (val >= 1_000_000) return `${(val / 1_000_000).toFixed(1)}M`
          if (val >= 1_000) return `${(val / 1_000).toFixed(1)}K`
          return val.toFixed(1)
        },
      },
      splitLine: { lineStyle: { color: 'rgba(255,255,255,0.04)' } },
    },
    tooltip: {
      trigger: 'axis',
      backgroundColor: '#212223',
      borderColor: 'rgba(255,255,255,0.08)',
      textStyle: { color: '#f7f8f8', fontSize: 12 },
      formatter: (params: any) => {
        if (!Array.isArray(params) || !params.length) return ''
        const p = params[0]
        const price = p.data[0]
        const total = p.data[1]
        const side = p.seriesName === '买盘' ? '买盘' : '卖盘'
        return `价格: ${price.toLocaleString('en-US', { maximumFractionDigits: 2 })}<br/>累计量: ${total.toLocaleString('en-US', { maximumFractionDigits: 3 })}<br/>方向: ${side}`
      },
    },
    series: [
      {
        name: '买盘',
        type: 'line',
        data: bidData,
        step: false,
        smooth: false,
        lineStyle: { color: '#10b981', width: 1.5 },
        areaStyle: { color: 'rgba(16,185,129,0.15)' },
        symbol: 'none',
      },
      {
        name: '卖盘',
        type: 'line',
        data: askData,
        step: false,
        smooth: false,
        lineStyle: { color: '#e5484d', width: 1.5 },
        areaStyle: { color: 'rgba(229,72,77,0.15)' },
        symbol: 'none',
      },
    ],
  }

  // 最新价虚线分隔
  if (props.lastPrice) {
    (option as any).series.push({
      type: 'line',
      markLine: {
        silent: true,
        symbol: 'none',
        lineStyle: { type: 'dashed', color: 'rgba(113,112,255,0.5)', width: 1 },
        data: [{ xAxis: props.lastPrice }],
        label: { show: false },
      },
      data: [],
    })
  }

  chart.setOption(option, true)
}

onMounted(() => {
  initChart()
  updateChart()
  resizeHandler = () => chart?.resize()
  window.addEventListener('resize', resizeHandler)
})

onBeforeUnmount(() => {
  if (resizeHandler) {
    window.removeEventListener('resize', resizeHandler)
  }
  chart?.dispose()
  chart = null
})

watch(() => props.depth, () => {
  updateChart()
}, { deep: true })

watch(() => props.lastPrice, () => {
  updateChart()
})
</script>

<style scoped lang="scss">
.depth-chart {
  width: 100%;
  height: 280px;
  background: var(--color-deep-bg, #050607);
  border: 1px solid var(--color-border);
  border-radius: 8px;
}

@media (max-width: 1023px) {
  .depth-chart {
    height: 220px;
  }
}

@media (max-width: 767px) {
  .depth-chart {
    height: 180px;
  }
}
</style>
