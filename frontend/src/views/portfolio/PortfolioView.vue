<template>
  <div class="portfolio-dashboard">
    <!-- Header -->
    <div class="portfolio-header">
      <h1 class="page-title">组合权益</h1>
      <div class="header-controls">
        <el-date-picker
          v-model="dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          format="YYYY-MM-DD"
          value-format="YYYY-MM-DD"
          :style="{ width: '280px' }"
          @change="handleDateRangeChange"
        />
        <el-segmented v-model="granularity" :options="granularityOptions" @change="handleGranularityChange" />
        <el-button
          circle
          plain
          :icon="Refresh"
          :class="{ 'refresh-spinning': isRefreshing }"
          @click="handleRefresh"
        />
      </div>
    </div>

    <!-- Equity Overview Cards (4 columns) -->
    <el-row :gutter="16" class="overview-cards">
      <el-col :xs="24" :sm="12" :lg="6">
        <div class="metric-card" :class="{ 'metric-card--loading': summaryLoading }">
          <div class="metric-card__header">
            <span class="metric-card__label">总资产</span>
            <el-icon class="metric-card__icon" :size="24"><Wallet /></el-icon>
          </div>
          <div v-if="summaryLoading" class="metric-card__skeleton">
            <div class="skeleton-line" style="width: 120px; height: 28px;" />
            <div class="skeleton-line" style="width: 60px; height: 14px; margin-top: 8px;" />
          </div>
          <template v-else>
            <div class="metric-card__value">¥{{ formatMoney(summary?.total_equity) }}</div>
            <div class="metric-card__sub">{{ summary?.total_positions ?? 0 }} 个持仓</div>
          </template>
        </div>
      </el-col>

      <el-col :xs="24" :sm="12" :lg="6">
        <div class="metric-card" :class="{ 'metric-card--loading': summaryLoading }">
          <div class="metric-card__header">
            <span class="metric-card__label">累计盈亏</span>
            <el-icon class="metric-card__icon" :size="24" :style="{ color: pnlColor(summary?.cumulative_pnl) }">
              <TrendCharts />
            </el-icon>
          </div>
          <div v-if="summaryLoading" class="metric-card__skeleton">
            <div class="skeleton-line" style="width: 100px; height: 28px;" />
            <div class="skeleton-line" style="width: 50px; height: 14px; margin-top: 8px;" />
          </div>
          <template v-else>
            <div class="metric-card__value" :style="{ color: pnlColor(summary?.cumulative_pnl) }">
              {{ pnlPrefix(summary?.cumulative_pnl) }}¥{{ formatMoney(summary?.cumulative_pnl) }}
            </div>
            <div class="metric-card__sub" :style="{ color: pnlColor(summary?.cumulative_pnl_rate) }">
              {{ formatRate(summary?.cumulative_pnl_rate) }}
            </div>
          </template>
        </div>
      </el-col>

      <el-col :xs="24" :sm="12" :lg="6">
        <div class="metric-card" :class="{ 'metric-card--loading': summaryLoading }">
          <div class="metric-card__header">
            <span class="metric-card__label">当日盈亏</span>
            <el-icon class="metric-card__icon" :size="24" :style="{ color: pnlColor(summary?.daily_pnl) }">
              <Calendar />
            </el-icon>
          </div>
          <div v-if="summaryLoading" class="metric-card__skeleton">
            <div class="skeleton-line" style="width: 100px; height: 28px;" />
            <div class="skeleton-line" style="width: 50px; height: 14px; margin-top: 8px;" />
          </div>
          <template v-else>
            <div class="metric-card__value" :style="{ color: pnlColor(summary?.daily_pnl) }">
              {{ pnlPrefix(summary?.daily_pnl) }}¥{{ formatMoney(summary?.daily_pnl) }}
            </div>
            <div class="metric-card__sub" :style="{ color: pnlColor(summary?.daily_pnl_rate) }">
              {{ formatRate(summary?.daily_pnl_rate) }}
            </div>
          </template>
        </div>
      </el-col>

      <el-col :xs="24" :sm="12" :lg="6">
        <div class="metric-card" :class="{ 'metric-card--loading': summaryLoading }">
          <div class="metric-card__header">
            <span class="metric-card__label">收益率</span>
            <el-icon class="metric-card__icon" :size="24" :style="{ color: pnlColor(summary?.cumulative_pnl_rate) }">
              <DataLine />
            </el-icon>
          </div>
          <div v-if="summaryLoading" class="metric-card__skeleton">
            <div class="skeleton-line" style="width: 80px; height: 28px;" />
          </div>
          <template v-else>
            <div class="metric-card__value" :style="{ color: pnlColor(summary?.cumulative_pnl_rate) }">
              {{ formatRate(summary?.cumulative_pnl_rate) }}
            </div>
          </template>
        </div>
      </el-col>
    </el-row>

    <!-- Equity Curve Chart -->
    <div class="equity-curve-section">
      <div class="section-card">
        <div class="section-title">权益曲线</div>
        <div v-if="equityCurveLoading" class="chart-loading">
          <el-skeleton :rows="5" animated />
        </div>
        <div v-else-if="equityCurve && equityCurve.points.length > 0" ref="chartContainer" class="equity-chart" />
        <div v-else class="chart-empty">
          <el-icon :size="32" color="var(--color-text-tertiary)"><TrendCharts /></el-icon>
          <span>暂无权益数据</span>
        </div>
      </div>
    </div>

    <!-- Bottom: Positions (60%) + Performance (40%) -->
    <div class="bottom-layout">
      <!-- Left: Position Summary Table -->
      <div class="positions-section">
        <div class="section-card">
          <div class="section-header">
            <span class="section-title">持仓汇总</span>
            <el-badge v-if="summary?.total_positions" :value="summary.total_positions" type="info" />
          </div>

          <el-table
            v-loading="positionsLoading"
            :data="positions?.items ?? []"
            :default-sort="{ prop: 'unrealized_pnl', order: 'descending' }"
            style="width: 100%"
            :header-cell-style="{ background: 'var(--color-surface)', color: 'var(--color-text-secondary)' }"
            :row-class-name="positionRowClass"
            empty-text="暂无持仓"
          >
            <el-table-column prop="symbol" label="交易对" min-width="100">
              <template #default="{ row }">
                <span class="symbol-text">{{ row.symbol }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="side" label="方向" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.side === 'long' ? 'success' : 'danger'" size="small">
                  {{ row.side === 'long' ? '多' : '空' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="quantity" label="数量" width="120" align="right">
              <template #default="{ row }">
                <span class="mono-text">{{ row.quantity }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="avg_price" label="均价" width="120" align="right">
              <template #default="{ row }">
                <span class="mono-text">{{ row.avg_price }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="current_price" label="现价" width="120" align="right">
              <template #default="{ row }">
                <span class="mono-text">{{ row.current_price }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="unrealized_pnl" label="浮动盈亏" width="160" align="right" sortable>
              <template #default="{ row }">
                <div class="pnl-cell">
                  <span :style="{ color: pnlColor(row.unrealized_pnl) }" class="mono-text">
                    {{ pnlPrefix(row.unrealized_pnl) }}¥{{ formatMoney(row.unrealized_pnl) }}
                  </span>
                  <span :style="{ color: pnlColor(row.unrealized_pnl_rate) }" class="pnl-rate">
                    {{ formatRate(row.unrealized_pnl_rate) }}
                  </span>
                </div>
              </template>
            </el-table-column>
          </el-table>

          <div v-if="positions && positions.total > (positions.size ?? 10)" class="pagination-bar">
            <el-pagination
              small
              layout="total, prev, pager, next"
              :total="positions.total"
              :page-size="positions.size ?? 10"
              :current-page="positions.page ?? 1"
              @current-change="handlePageChange"
            />
          </div>
        </div>
      </div>

      <!-- Right: Performance Metrics + Strategy Table -->
      <div class="performance-section">
        <!-- Performance Metrics Cards -->
        <div class="perf-metrics-row">
          <div class="perf-metric-card">
            <div class="perf-metric-card__label">最大回撤</div>
            <div class="perf-metric-card__value color-negative">
              {{ formatRate(performance?.max_drawdown) }}
            </div>
          </div>
          <div class="perf-metric-card">
            <div class="perf-metric-card__label">夏普率</div>
            <div class="perf-metric-card__value" :style="{ color: sharpeColor }">
              {{ performance?.sharpe_ratio ?? '--' }}
            </div>
          </div>
          <div class="perf-metric-card">
            <div class="perf-metric-card__label">胜率</div>
            <div class="perf-metric-card__value" :style="{ color: winRateColor }">
              {{ formatRate(performance?.win_rate) }}
            </div>
          </div>
        </div>

        <!-- Strategy Comparison Table -->
        <div class="section-card" style="margin-top: 16px;">
          <div class="section-title">策略绩效</div>

          <el-table
            v-loading="performanceLoading"
            :data="performance?.strategies ?? []"
            :default-sort="{ prop: 'total_pnl', order: 'descending' }"
            style="width: 100%"
            :header-cell-style="{ background: 'var(--color-surface)', color: 'var(--color-text-secondary)' }"
            empty-text="暂无策略数据"
          >
            <el-table-column prop="strategy_name" label="策略名称" min-width="100">
              <template #default="{ row }">
                <span class="strategy-link">{{ row.strategy_name }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="total_pnl" label="总盈亏" width="110" align="right" sortable>
              <template #default="{ row }">
                <span :style="{ color: pnlColor(row.total_pnl) }" class="mono-text">
                  {{ pnlPrefix(row.total_pnl) }}¥{{ formatMoney(row.total_pnl) }}
                </span>
              </template>
            </el-table-column>
            <el-table-column prop="total_pnl_rate" label="盈亏率" width="90" align="right" sortable>
              <template #default="{ row }">
                <span :style="{ color: pnlColor(row.total_pnl_rate) }" class="mono-text">
                  {{ formatRate(row.total_pnl_rate) }}
                </span>
              </template>
            </el-table-column>
            <el-table-column prop="max_drawdown" label="最大回撤" width="90" align="right">
              <template #default="{ row }">
                <span class="color-negative mono-text">{{ formatRate(row.max_drawdown) }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="trade_count" label="交易次数" width="80" align="center">
              <template #default="{ row }">
                <span class="mono-text">{{ row.trade_count }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="win_rate" label="胜率" width="80" align="right">
              <template #default="{ row }">
                <span :style="{ color: winRateColorFor(row.win_rate) }" class="mono-text">
                  {{ formatRate(row.win_rate) }}
                </span>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </div>
    </div>

    <!-- Error state -->
    <el-alert
      v-if="summaryError || positionsError || performanceError"
      :title="summaryError || positionsError || performanceError || '加载失败'"
      type="error"
      show-icon
      closable
      style="margin-top: 16px;"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { Refresh, Wallet, TrendCharts, Calendar, DataLine } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { usePortfolioSummary } from '@/composables/usePortfolioSummary'
import { usePortfolioPositions } from '@/composables/usePortfolioPositions'
import { useEquityCurve } from '@/composables/useEquityCurve'
import { usePortfolioPerformance } from '@/composables/usePortfolioPerformance'

const { summary, loading: summaryLoading, error: summaryError, fetch: fetchSummary, applyWsUpdate: applySummaryWs } = usePortfolioSummary()
const { positions, loading: positionsLoading, error: positionsError, fetch: fetchPositions, pagination, applyWsUpdate: applyPositionsWs } = usePortfolioPositions()
const { curve: equityCurve, loading: equityCurveLoading, error: equityCurveError, granularity, dateRange, fetch: fetchEquityCurve } = useEquityCurve()
const { performance, loading: performanceLoading, error: performanceError, fetch: fetchPerformance } = usePortfolioPerformance()

const isRefreshing = ref(false)
const chartContainer = ref<HTMLElement | null>(null)
let chartInstance: echarts.ECharts | null = null

const granularityOptions = [
  { label: '小时', value: 'hour' },
  { label: '日', value: 'day' },
  { label: '周', value: 'week' },
]

// === Computed colors ===
const sharpeColor = computed(() => {
  const val = Number(performance.value?.sharpe_ratio ?? 0)
  if (val >= 2) return 'var(--color-positive)'
  if (val >= 1) return 'var(--color-warning)'
  return 'var(--color-negative)'
})

const winRateColor = computed(() => {
  const val = Number(performance.value?.win_rate ?? 0)
  if (val >= 60) return 'var(--color-positive)'
  if (val >= 40) return 'var(--color-warning)'
  return 'var(--color-negative)'
})

function winRateColorFor(rate: string): string {
  const val = Number(rate)
  if (val >= 60) return 'var(--color-positive)'
  if (val >= 40) return 'var(--color-warning)'
  return 'var(--color-negative)'
}

// === Formatting helpers ===
function formatMoney(value?: string | null): string {
  if (!value) return '0.00'
  const num = Number(value)
  if (isNaN(num)) return value
  return new Intl.NumberFormat('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(Math.abs(num))
}

function formatRate(value?: string | null): string {
  if (!value) return '0.00%'
  const num = Number(value)
  if (isNaN(num)) return `${value}%`
  const sign = num > 0 ? '+' : ''
  return `${sign}${num.toFixed(2)}%`
}

function pnlColor(value?: string | null): string {
  if (!value) return 'var(--color-neutral)'
  const num = Number(value)
  if (num > 0) return 'var(--color-positive)'
  if (num < 0) return 'var(--color-negative)'
  return 'var(--color-neutral)'
}

function pnlPrefix(value?: string | null): string {
  if (!value) return ''
  const num = Number(value)
  return num > 0 ? '+' : ''
}

function positionRowClass({ row }: { row: { side: string } }): string {
  return row.side === 'long' ? 'row-long' : 'row-short'
}

// === ECharts ===
function initChart() {
  if (!chartContainer.value) return
  chartInstance = echarts.init(chartContainer.value, 'dark')
  renderChart()
}

function renderChart() {
  if (!chartInstance || !equityCurve.value?.points?.length) return

  const points = equityCurve.value.points
  const data = points.map((p) => [p.timestamp, Number(p.equity)])

  chartInstance.setOption({
    backgroundColor: 'transparent',
    grid: { left: 64, right: 24, top: 16, bottom: 32 },
    xAxis: {
      type: 'time',
      axisLine: { lineStyle: { color: 'rgba(255,255,255,0.08)' } },
      axisLabel: { color: 'var(--color-text-tertiary)', fontSize: 11 },
      splitLine: { show: false },
    },
    yAxis: {
      type: 'value',
      axisLabel: {
        color: 'var(--color-text-tertiary)',
        fontSize: 11,
        formatter: (val: number) => '¥' + formatMoney(String(val)),
      },
      splitLine: { lineStyle: { color: 'rgba(255,255,255,0.04)' } },
    },
    series: [{
      type: 'line',
      smooth: 0.3,
      data,
      areaStyle: {
        color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
          { offset: 0, color: 'rgba(59, 130, 246, 0.20)' },
          { offset: 1, color: 'rgba(59, 130, 246, 0.00)' },
        ]),
      },
      lineStyle: { color: '#3B82F6', width: 2 },
      itemStyle: { color: '#3B82F6' },
      showSymbol: false,
    }],
    tooltip: {
      trigger: 'axis',
      backgroundColor: '#212223',
      borderColor: 'rgba(255,255,255,0.08)',
      textStyle: { color: '#f7f8f8', fontFamily: 'JetBrains Mono, monospace' },
      formatter: (params: any) => {
        const p = Array.isArray(params) ? params[0] : params
        return `${p.axisValueLabel}<br/>权益: ¥${formatMoney(String(p.value[1]))}`
      },
    },
  })
}

// === Event handlers ===
async function handleRefresh() {
  isRefreshing.value = true
  await Promise.all([
    fetchSummary(),
    fetchPositions(),
    fetchEquityCurve(),
    fetchPerformance(),
  ])
  isRefreshing.value = false
}

function handleDateRangeChange() {
  fetchEquityCurve()
}

function handleGranularityChange() {
  fetchEquityCurve()
}

function handlePageChange(page: number) {
  pagination.page = page
  fetchPositions()
}

// === Lifecycle ===
onMounted(async () => {
  await Promise.all([
    fetchSummary(),
    fetchPositions(),
    fetchEquityCurve(),
    fetchPerformance(),
  ])

  await nextTick()
  initChart()
})

watch(() => equityCurve.value, () => {
  nextTick(() => {
    if (!chartInstance && chartContainer.value) {
      initChart()
    } else {
      renderChart()
    }
  })
})

onUnmounted(() => {
  if (chartInstance) {
    chartInstance.dispose()
    chartInstance = null
  }
})
</script>

<style scoped lang="scss">
// === Design tokens (local scope) ===
@import url('https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Work+Sans:wght@400;500;600&display=swap');

$radius-lg: 12px;
$shadow-glow-primary: 0 4px 16px rgba(59, 130, 246, 0.35);
$color-primary: #3B82F6;

// Font mixins
@mixin font-heading {
  font-family: 'Outfit', var(--font-ui), sans-serif;
}

@mixin font-body {
  font-family: 'Work Sans', var(--font-ui), sans-serif;
}

@mixin card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
}

@mixin glow-hover {
  border-color: var(--color-primary);
  box-shadow: var(--shadow-glow-primary);
}

.portfolio-dashboard {
  max-width: 1344px;
}

// === Header ===
.portfolio-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 56px;
  padding: 0 16px;
  border-bottom: 1px solid var(--color-border);
  margin-bottom: 20px;

  .page-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--color-text-primary);
    margin: 0;
    @include font-heading;
  }

  .header-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }
}

@keyframes spin-once {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.refresh-spinning :deep(.el-icon) {
  animation: spin-once 0.6s ease;
}

// === Overview Cards ===
.overview-cards {
  margin-bottom: 20px;
}

.metric-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 20px;
  transition: transform 0.2s ease, border-color 0.2s ease, box-shadow 0.2s ease;

  &:hover {
    transform: translateY(-2px);
    border-color: var(--color-border-hover);
    @include glow-hover;
  }

  &__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  &__label {
    font-size: 13px;
    color: var(--color-text-tertiary);
  }

  &__icon {
    color: var(--color-primary);
    opacity: 0.4;
  }

  &__value {
    font-size: 28px;
    font-weight: 700;
    color: var(--color-text-primary);
    font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  }

  &__sub {
    font-size: 12px;
    color: var(--color-text-tertiary);
    margin-top: 4px;
  }

  &__skeleton {
    .skeleton-line {
      background: linear-gradient(90deg, #191a1b 25%, #212223 50%, #191a1b 75%);
      background-size: 200% 100%;
      animation: skeleton-shimmer 1.5s infinite;
      border-radius: 4px;
    }
  }
}

@keyframes skeleton-shimmer {
  from { background-position: 200% 0; }
  to { background-position: -200% 0; }
}

// === Equity Curve ===
.equity-curve-section {
  margin-bottom: 20px;
}

.section-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 16px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin-bottom: 12px;
  @include font-heading;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;

  .section-title {
    margin-bottom: 0;
  }
}

.equity-chart {
  width: 100%;
  height: 280px;
}

.chart-loading,
.chart-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 280px;
  gap: 8px;
  color: var(--color-text-tertiary);
  font-size: 13px;
}

// === Bottom Layout ===
.bottom-layout {
  display: flex;
  gap: 16px;
  align-items: flex-start;
}

.positions-section {
  flex: 6;
  min-width: 0;
}

.performance-section {
  flex: 4;
  min-width: 0;
}

@media (max-width: 1199px) {
  .bottom-layout {
    flex-direction: column;
  }
  .positions-section,
  .performance-section {
    flex: none;
    width: 100%;
  }
}

// === Table Styles ===
.symbol-text {
  font-weight: 600;
  color: var(--color-text-primary);
}

.mono-text {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-size: 13px;
}

.pnl-cell {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;

  .pnl-rate {
    font-size: 12px;
    opacity: 0.75;
  }
}

:deep(.row-long) {
  background-color: rgba(103, 194, 58, 0.04);
}

:deep(.row-short) {
  background-color: rgba(245, 108, 108, 0.04);
}

.strategy-link {
  color: var(--color-primary);
  cursor: pointer;

  &:hover {
    text-decoration: underline;
  }
}

.color-negative {
  color: var(--color-negative) !important;
}

.pagination-bar {
  display: flex;
  justify-content: flex-end;
  margin-top: 12px;
}

// === Performance Metrics ===
.perf-metrics-row {
  display: flex;
  gap: 12px;
}

.perf-metric-card {
  flex: 1;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 16px;

  &__label {
    font-size: 12px;
    color: var(--color-text-tertiary);
    margin-bottom: 8px;
  }

  &__value {
    font-size: 22px;
    font-weight: 700;
    font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  }
}

@media (max-width: 767px) {
  .portfolio-header {
    flex-direction: column;
    height: auto;
    padding: 12px 16px;
    gap: 12px;
  }

  .header-controls {
    flex-wrap: wrap;
    width: 100%;
  }

  .perf-metrics-row {
    flex-direction: column;
  }
}
</style>
