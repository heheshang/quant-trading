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
      <div class="chart-section">
        <div ref="equityChartRef" class="equity-chart-container"></div>
      </div>
      <div class="chart-section">
        <div class="drawdown-header">
          <span class="drawdown-title">回撤曲线</span>
          <span class="drawdown-info">
            最大回撤: <span class="max-dd-value">{{ formatPercent(maxDrawdown) }}</span>
          </span>
        </div>
        <div ref="drawdownChartRef" class="drawdown-chart-container"></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import * as echarts from 'echarts'
import type { EquityPoint } from '@/types/backtest'
import { useFormat } from '@/composables/useFormat'

const { formatCurrency } = useFormat()

function formatPercent(v: number): string {
  return v.toFixed(2) + '%'
}

const props = withDefaults(defineProps<{
  data?: EquityPoint[]
  loading?: boolean
  initialCapital?: number
}>(), {
  data: () => [],
  loading: false,
  initialCapital: 100000,
})

const equityChartRef = ref<HTMLDivElement>()
const drawdownChartRef = ref<HTMLDivElement>()
let equityChart: echarts.ECharts | null = null
let drawdownChart: echarts.ECharts | null = null

const maxDrawdown = computed(() => {
  if (!props.data || props.data.length === 0) return 0
  return Math.min(...props.data.map(d => d.drawdown_pct))
})

const isUp = computed(() => {
  const vals = props.data.map((d) => d.equity)
  return vals.length > 1 && vals[vals.length - 1] >= vals[0]
})

function initChart() {
  if (!equityChartRef.value) return
  if (equityChart) equityChart.dispose()

  equityChart = echarts.init(equityChartRef.value, undefined, {
    renderer: 'canvas',
  })

  if (drawdownChartRef.value) {
    if (drawdownChart) drawdownChart.dispose()
    drawdownChart = echarts.init(drawdownChartRef.value, undefined, {
      renderer: 'canvas',
    })
  }

  updateChart()
  window.addEventListener('resize', handleResize)
}

function updateChart() {
  if (!equityChart || !props.data.length) return

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

  equityChart.setOption(option, true)

  // Drawdown chart
  if (drawdownChart) {
    const drawdownValues = props.data.map((d) => d.drawdown_pct)
    const ddColor = '#e5484d'

    const ddOption: echarts.EChartsOption = {
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
                  <div>回撤: <span style="font-weight:600;color:${ddColor}">${p.value.toFixed(2)}%</span></div>`
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
          formatter: (v: number) => v.toFixed(1) + '%',
        },
      },
      series: [
        {
          type: 'line',
          data: drawdownValues,
          smooth: true,
          symbol: 'none',
          lineStyle: { color: ddColor, width: 2 },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: ddColor + '33' },
              { offset: 1, color: ddColor + '05' },
            ]),
          },
        },
      ],
    }
    drawdownChart.setOption(ddOption, true)
  }
}

function handleResize() {
  equityChart?.resize()
  drawdownChart?.resize()
}

watch(() => props.data, () => updateChart(), { deep: true })

onMounted(() => {
  initChart()
})

onUnmounted(() => {
  window.removeEventListener('resize', handleResize)
  equityChart?.dispose()
  drawdownChart?.dispose()
  equityChart = null
  drawdownChart = null
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
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.chart-section {
  width: 100%;
}

.equity-chart-container {
  width: 100%;
  height: 280px;
}

.drawdown-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.drawdown-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-secondary);
}

.drawdown-info {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.max-dd-value {
  color: #e5484d;
  font-weight: 600;
}

.drawdown-chart-container {
  width: 100%;
  height: 160px;
}

/* Loading skeleton */
.chart-loading {
  height: 280px;
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
  height: 280px;
  display: flex;
  align-items: center;
  justify-content: center;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
</style>
