<template>
  <div class="kline-list-view">
    <div class="page-header">
      <h1 class="page-title">K线数据</h1>
      <div class="header-actions">
        <el-button type="primary" @click="router.push('/kline/import')">
          <el-icon><Upload /></el-icon> 导入数据
        </el-button>
        <el-button @click="router.push('/kline/quality')">
          <el-icon><DataAnalysis /></el-icon> 质量报告
        </el-button>
        <el-button @click="router.push('/kline/export')">
          <el-icon><Download /></el-icon> 导出
        </el-button>
      </div>
    </div>

    <!-- Filter Bar -->
    <el-card shadow="never" class="filter-card">
      <div class="filter-bar">
        <el-select v-model="filterForm.symbol" placeholder="选择交易对" filterable clearable class="filter-select" @change="handleFilterChange">
          <el-option v-for="s in availableSymbols" :key="s" :label="s" :value="s" />
        </el-select>
        <el-select v-model="filterForm.interval" placeholder="选择周期" clearable class="filter-select" @change="handleFilterChange">
          <el-option v-for="iv in KLINE_INTERVALS" :key="iv.value" :label="iv.label" :value="iv.value" />
        </el-select>
        <el-date-picker
          v-model="filterForm.dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          value-format="x"
          class="filter-date"
          @change="handleFilterChange"
        />
        <el-button type="primary" :loading="loading" @click="fetchData">
          <el-icon><Search /></el-icon> 查询
        </el-button>
      </div>
    </el-card>

    <!-- Chart Preview -->
    <el-card v-if="chartData.length > 0" shadow="never" class="chart-card">
      <template #header>
        <div class="card-header">
          <span>数据预览</span>
          <span class="preview-count">{{ chartData.length }} 条</span>
        </div>
      </template>
      <div ref="chartContainer" class="chart-container"></div>
    </el-card>

    <!-- Data Table -->
    <el-card shadow="never" class="table-card">
      <template #header>
        <div class="card-header">
          <span>数据列表</span>
          <span class="total-count">共 {{ pagination.total }} 条</span>
        </div>
      </template>

      <el-table
        v-loading="loading"
        :data="tableData"
        stripe
        border
        :height="400"
        empty-text="暂无数据，请先导入或查询"
      >
        <el-table-column label="时间" prop="open_time" width="180" sortable>
          <template #default="{ row }">
            {{ formatTimestamp(row.open_time) }}
          </template>
        </el-table-column>
        <el-table-column label="开盘" prop="open" width="120" align="right">
          <template #default="{ row }">
            {{ formatNumber(row.open) }}
          </template>
        </el-table-column>
        <el-table-column label="最高" prop="high" width="120" align="right">
          <template #default="{ row }">
            {{ formatNumber(row.high) }}
          </template>
        </el-table-column>
        <el-table-column label="最低" prop="low" width="120" align="right">
          <template #default="{ row }">
            {{ formatNumber(row.low) }}
          </template>
        </el-table-column>
        <el-table-column label="收盘" prop="close" width="120" align="right">
          <template #default="{ row }">
            <span :class="row.close >= row.open ? 'price-up' : 'price-down'">
              {{ formatNumber(row.close) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="成交量" prop="volume" width="140" align="right">
          <template #default="{ row }">
            {{ formatVolume(row.volume) }}
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag v-if="row.is_gap" type="warning" size="small">缺口</el-tag>
            <el-tag v-else type="success" size="small" effect="plain">正常</el-tag>
          </template>
        </el-table-column>
      </el-table>

      <!-- Pagination -->
      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="pagination.page"
          v-model:page-size="pagination.pageSize"
          :page-sizes="[100, 500, 1000, 5000]"
          :total="pagination.total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { Upload, DataAnalysis, Download, Search } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { queryKlines, getKlineSymbols } from '@/api/kline'
import { KLINE_INTERVALS } from '@/types/kline'
import type { KlineData } from '@/types/kline'

const router = useRouter()

// Filter form
const filterForm = reactive({
  symbol: '',
  interval: '',
  dateRange: [] as [number, number] | null,
})

// Table state
const loading = ref(false)
const tableData = ref<KlineData[]>([])
const chartData = ref<KlineData[]>([])
const chartContainer = ref<HTMLDivElement>()

// Available symbols
const availableSymbols = ref<string[]>([])

// Pagination
const pagination = reactive({
  page: 1,
  pageSize: 100,
  total: 0,
})

// Chart instance
let chartInstance: echarts.ECharts | null = null

// Fetch available symbols
async function fetchSymbols() {
  try {
    const symbols = await getKlineSymbols()
    availableSymbols.value = symbols.map((s) => s.symbol)
  } catch {
    // symbols list is optional
  }
}

// Fetch kline data
async function fetchData() {
  if (!filterForm.symbol) {
    ElMessage.warning('请选择交易对')
    return
  }
  if (!filterForm.interval) {
    ElMessage.warning('请选择周期')
    return
  }

  loading.value = true
  try {
    const [startTime, endTime] = filterForm.dateRange ?? [undefined, undefined]
    const res = await queryKlines({
      symbol: filterForm.symbol,
      interval: filterForm.interval,
      start_time: startTime,
      end_time: endTime,
      page: pagination.page,
      page_size: pagination.pageSize,
    })
    tableData.value = res.data
    pagination.total = res.meta.total

    // Load up to 500 rows into chart preview
    if (res.data.length > 0) {
      const previewRes = await queryKlines({
        symbol: filterForm.symbol,
        interval: filterForm.interval,
        start_time: startTime,
        end_time: endTime,
        page: 1,
        page_size: 500,
      })
      chartData.value = previewRes.data
      await nextTick()
      renderChart()
    } else {
      chartData.value = []
    }
  } catch (err: unknown) {
    ElMessage.error(err instanceof Error ? err.message : '查询失败')
  } finally {
    loading.value = false
  }
}

function handleFilterChange() {
  pagination.page = 1
}

function handlePageChange() {
  fetchData()
}

function handleSizeChange() {
  pagination.page = 1
  fetchData()
}

// Format helpers
function formatTimestamp(ts: number): string {
  return new Date(ts).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' })
}

function formatNumber(n: number): string {
  return n.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 4 })
}

function formatVolume(v: number): string {
  if (v >= 1_000_000) return (v / 1_000_000).toFixed(2) + 'M'
  if (v >= 1_000) return (v / 1_000).toFixed(2) + 'K'
  return v.toFixed(2)
}

// ECharts candlestick + volume
function renderChart() {
  if (!chartContainer.value || chartData.value.length === 0) return
  if (chartInstance) {
    chartInstance.dispose()
  }
  chartInstance = echarts.init(chartContainer.value)

  const times = chartData.value.map((d) => new Date(d.open_time).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' }))
  const ohlc = chartData.value.map((d) => [d.open, d.close, d.low, d.high])
  const volumes = chartData.value.map((d) => d.volume)
  const colors = chartData.value.map((d) => (d.close >= d.open ? '#22C55E' : '#EF4444'))

  chartInstance.setOption({
    tooltip: { trigger: 'axis', axisPointer: { type: 'cross' } },
    grid: [{ left: 60, right: 20, top: 20, height: '60%' }, { left: 60, right: 20, top: '75%', height: '15%' }],
    xAxis: [{ type: 'category', data: times, gridIndex: 0, axisLabel: { show: false } }, { type: 'category', data: times, gridIndex: 1, axisLabel: { fontSize: 10 } }],
    yAxis: [{ scale: true, gridIndex: 0, axisLabel: { fontSize: 10 } }, { scale: true, gridIndex: 1, axisLabel: { fontSize: 10 } }],
    series: [
      { type: 'candlestick', data: ohlc, xAxisIndex: 0, yAxisIndex: 0 },
      { type: 'bar', data: volumes.map((v, i) => ({ value: v, itemStyle: { color: colors[i] } })), xAxisIndex: 1, yAxisIndex: 1 },
    ],
  })
}

onMounted(() => {
  fetchSymbols()
  window.addEventListener('resize', () => chartInstance?.resize())
})

onUnmounted(() => {
  chartInstance?.dispose()
  window.removeEventListener('resize', () => chartInstance?.resize())
})
</script>

<style scoped lang="scss">
.kline-list-view {
  max-width: 1344px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 24px;
}

.page-title {
  font-size: 24px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.filter-card {
  margin-bottom: 16px;
  background: var(--color-surface);
  border-color: var(--color-border);
}

.filter-bar {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
}

.filter-select {
  width: 160px;
}

.filter-date {
  width: 280px;
}

.chart-card {
  margin-bottom: 16px;
  background: var(--color-surface);
  border-color: var(--color-border);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.preview-count,
.total-count {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.chart-container {
  height: 320px;
  width: 100%;
}

.table-card {
  background: var(--color-surface);
  border-color: var(--color-border);
}

.pagination-wrapper {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.price-up {
  color: #22C55E;
  font-weight: 600;
}

.price-down {
  color: #EF4444;
  font-weight: 600;
}
</style>
