<template>
  <div class="backtest-equity-chart">
    <div class="chart-header">
      <h3 class="chart-title">权益曲线</h3>
      <div v-if="data && data.length > 0" class="chart-info">
        <span class="info-badge">
          初始: {{ formatCurrency(initialCapital) }}
        </span>
        <span class="info-divider">|</span>
        <span class="info-badge">
          最终: {{ formatCurrency(data[data.length - 1]?.equity ?? initialCapital) }}
        </span>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="loading" class="chart-loading">
      <div class="skeleton-chart">
        <div class="skeleton-line"></div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-else-if="!data || data.length === 0" class="chart-empty">
      <el-empty description="暂无权益数据" :image-size="60" />
    </div>

    <!-- Chart -->
    <div v-else class="equity-chart-wrapper">
      <div ref="chartRef" class="equity-chart-container"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import * as echarts from 'echarts'
import type { EquityPoint } from '@/types/backtest'
import { useFormat } from '@/composables/useFormat'

const { formatCurrency } = useFormat()

const props = withDefaults(defineProps<{
  data?: EquityPoint[]
  loading?: boolean
  initialCapital?: number
}>(), {
  data: () => [],
  loading: false,
  initialCapital: 100000,
})

const chartRef = ref<HTMLDivElement>()
let chartInstance: echarts.ECharts | null = null

const isUp = computed(() => {
  const vals = props.data.map((d) => d.equity)
  return vals.length > 1 && vals[vals.length - 1] >= vals[0]
})

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
  if (!chartInstance || !props.data.length) return

  const dates = props.data.map((d) => {
    const dt = new Date(d.time)
    return dt.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })
  })
  const values = props.data.map((d) => d.equity)
  // Use accent color per design spec, fallback for ECharts which can't read CSS vars at runtime
  const accentColor = '#7170ff'
  const lineColor = accentColor

  const option: echarts.EChartsOption = {
    backgroundColor: 'transparent',
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(30, 41, 59, 0.95)',
      borderColor: 'rgba(51, 65, 85, 0.8)',
      textStyle: { color: '#f1f5f9', fontSize: 12 },
      formatter: (params: any) => {
        const p = params[0]
        if (!p) return ''
        return `<div style="font-size:13px;font-weight:600;margin-bottom:4px">${p.axisValue}</div>
                <div>权益: <span style="font-weight:600">${formatCurrency(p.value)}</span></div>`
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
      axisLine: { lineStyle: { color: '#334155' } },
      axisTick: { show: false },
      axisLabel: { color: '#94a3b8', fontSize: 11 },
      splitLine: { show: false },
      boundaryGap: false,
    },
    yAxis: {
      type: 'value',
      splitLine: {
        lineStyle: { color: '#1e293b', type: 'dashed' },
      },
      axisLabel: {
        color: '#94a3b8',
        fontSize: 11,
        formatter: (v: number) => {
          if (v >= 1000000) return (v / 10000).toFixed(0) + 'w'
          return v.toLocaleString()
        },
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
        markLine: {
          silent: true,
          symbol: 'none',
          lineStyle: { color: lineColor, type: 'dashed', width: 1, opacity: 0.5 },
          data: [{ yAxis: props.initialCapital }],
          label: { show: false },
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
.backtest-equity-chart {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 24px;
}

.chart-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.chart-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.chart-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.info-badge {
  font-size: 12px;
  color: var(--color-text-tertiary);
  font-variant-numeric: tabular-nums;
}

.info-divider {
  color: var(--color-border);
}

.equity-chart-wrapper {
  width: 100%;
}

.equity-chart-container {
  width: 100%;
  height: 320px;
}

/* Loading skeleton */
.chart-loading {
  height: 320px;
  display: flex;
  align-items: flex-end;
  padding: 20px;

  .skeleton-chart {
    width: 100%;
    height: 70%;
    border-radius: 4px;
    overflow: hidden;
    position: relative;

    .skeleton-line {
      width: 100%;
      height: 100%;
      border-radius: 4px;
      background: linear-gradient(135deg,
        var(--color-surface) 0%,
        rgba(255, 255, 255, 0.03) 40%,
        var(--color-surface) 80%
      );
      background-size: 200% 200%;
      animation: shimmer 1.5s ease-in-out infinite;
      clip-path: polygon(0% 80%, 10% 75%, 20% 60%, 35% 55%, 50% 40%, 65% 45%, 80% 30%, 100% 25%, 100% 100%, 0% 100%);
    }
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
