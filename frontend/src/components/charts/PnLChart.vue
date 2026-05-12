<template>
  <div ref="chartRef" class="pnl-chart" :style="{ height: height }"></div>
  <div v-if="loading" class="chart-skeleton">
    <div class="skeleton-line"></div>
  </div>
  <div v-if="empty" class="chart-empty">
    <el-empty description="No trading data yet">
      <el-button type="primary" size="small">Start Trading</el-button>
    </el-empty>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import * as echarts from 'echarts'
import type { PnLPoint } from '@/types'

const props = withDefaults(defineProps<{
  data?: PnLPoint[]
  loading?: boolean
  empty?: boolean
  height?: string
}>(), {
  data: () => [],
  loading: false,
  empty: false,
  height: '320px',
})

const chartRef = ref<HTMLDivElement>()
let chartInstance: echarts.ECharts | null = null

function initChart() {
  if (!chartRef.value) return
  if (chartInstance) chartInstance.dispose()

  chartInstance = echarts.init(chartRef.value, undefined, {
    renderer: 'canvas',
  })

  updateChart()
  window.addEventListener('resize', handleResize)
}

function updateChart() {
  if (!chartInstance) return
  const dates = props.data.map((d) => d.date)
  const values = props.data.map((d) => d.value)

  const isUp = values.length > 0 && values[values.length - 1] >= (values[0] || 0)
  const lineColor = isUp ? 'var(--color-buy)' : 'var(--color-sell)'

  const option: echarts.EChartsOption = {
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'var(--color-surface-elevated)',
      borderColor: 'var(--color-border)',
      textStyle: { color: 'var(--color-text-primary)', fontSize: 12 },
      formatter: (params: any) => {
        const p = params[0]
        if (!p) return ''
        return `<div style="font-size:13px;font-weight:600;margin-bottom:4px">${p.axisValue}</div>
                <div>PnL: <span style="font-weight:600">${p.value.toFixed(2)}</span></div>`
      },
    },
    grid: {
      left: 60,
      right: 20,
      top: 20,
      bottom: 30,
    },
    xAxis: {
      type: 'category',
      data: dates,
      axisLine: { lineStyle: { color: 'var(--color-chart-grid)' } },
      axisTick: { show: false },
      axisLabel: { color: 'var(--color-text-tertiary)', fontSize: 11 },
      splitLine: { show: false },
    },
    yAxis: {
      type: 'value',
      splitLine: {
        lineStyle: { color: 'var(--color-chart-grid)', type: 'dashed' },
      },
      axisLabel: {
        color: 'var(--color-text-tertiary)',
        fontSize: 11,
        formatter: (v: number) => v.toFixed(0),
      },
    },
    series: [
      {
        type: 'line',
        data: values,
        smooth: true,
        symbol: 'none',
        lineStyle: { color: lineColor, width: 2 },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: lineColor + '33' },
            { offset: 1, color: lineColor + '05' },
          ]),
        },
      },
    ],
  }

  chartInstance.setOption(option, true)
}

function handleResize() {
  chartInstance?.resize()
}

watch(() => props.data, () => updateChart(), { deep: true })

onMounted(() => {
  initChart()
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  chartInstance?.dispose()
})
</script>

<style scoped lang="scss">
.pnl-chart {
  width: 100%;
}

.chart-skeleton {
  height: 320px;
  display: flex;
  align-items: flex-end;
  padding: 20px;
  .skeleton-line {
    width: 100%;
    height: 60%;
    border-radius: 4px;
    background: linear-gradient(90deg, var(--color-surface) 25%, rgba(255,255,255,0.04) 50%, var(--color-surface) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.5s ease-in-out infinite;
  }
}

.chart-empty {
  height: 320px;
  display: flex;
  align-items: center;
  justify-content: center;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
</style>
