<template>
  <div class="backtest-trades-table">
    <div class="table-header">
      <h3 class="section-title">交易记录</h3>
      <div class="table-actions">
        <span v-if="trades && trades.length > 0" class="trade-count">
          共 {{ trades.length }} 笔
        </span>
        <el-button
          size="small"
          class="export-btn"
          :disabled="!trades || trades.length === 0"
          @click="handleExport"
        >
          <el-icon style="margin-right: 4px"><Download /></el-icon>
          导出结果
        </el-button>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="loading" class="table-loading">
      <div class="skeleton-row" v-for="n in 5" :key="n">
        <div class="skeleton-cell" v-for="m in 7" :key="m"></div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-else-if="!trades || trades.length === 0" class="table-empty">
      <el-empty description="暂无交易记录" :image-size="60" />
    </div>

    <!-- Data table -->
    <div v-else class="trades-table-wrapper">
      <el-table
        :data="paginatedTrades"
        stripe
        size="small"
        style="width: 100%"
        :default-sort="{ prop: 'exit_time', order: 'descending' }"
      >
        <el-table-column
          prop="entry_time"
          label="开仓时间"
          width="170"
          :formatter="formatDateCol"
        />
        <el-table-column
          prop="direction"
          label="方向"
          width="80"
        >
          <template #default="{ row }">
            <el-tag
              :type="row.direction === 'long' ? 'success' : 'danger'"
              size="small"
              effect="plain"
            >
              {{ row.direction === 'long' ? '多头' : '空头' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="entry_price"
          label="开仓价"
          width="120"
          align="right"
          :formatter="formatPriceCol"
        />
        <el-table-column
          prop="exit_price"
          label="平仓价"
          width="120"
          align="right"
          :formatter="formatPriceCol"
        />
        <el-table-column
          prop="quantity"
          label="数量"
          width="100"
          align="right"
          :formatter="formatQtyCol"
        />
        <el-table-column
          prop="pnl_usdt"
          label="盈亏"
          width="120"
          align="right"
          sortable
        >
          <template #default="{ row }">
            <span :class="row.pnl_usdt >= 0 ? 'pnl-positive' : 'pnl-negative'">
              {{ row.pnl_usdt >= 0 ? '+' : '' }}{{ formatCurrency(row.pnl_usdt) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column
          prop="pnl_pct"
          label="盈亏%"
          width="100"
          align="right"
          sortable
        >
          <template #default="{ row }">
            <span :class="row.pnl_pct >= 0 ? 'pnl-positive' : 'pnl-negative'">
              {{ row.pnl_pct >= 0 ? '+' : '' }}{{ row.pnl_pct.toFixed(2) }}%
            </span>
          </template>
        </el-table-column>
        <el-table-column
          prop="fee"
          label="手续费"
          width="100"
          align="right"
          :formatter="formatFeeCol"
        />
        <el-table-column
          prop="slippage"
          label="滑点"
          width="100"
          align="right"
          :formatter="formatFeeCol"
        />
        <el-table-column
          prop="holding_period_ms"
          label="持仓时长"
          width="120"
          align="center"
          :formatter="formatHoldingPeriod"
        />
        <el-table-column
          prop="exit_reason"
          label="平仓原因"
          width="120"
          align="center"
        >
          <template #default="{ row }">
            <el-tag
              v-if="row.exit_reason"
              :type="exitReasonTagType(row.exit_reason)"
              size="small"
              effect="dark"
            >
              {{ exitReasonLabel(row.exit_reason) }}
            </el-tag>
            <span v-else class="exit-reason-none">--</span>
          </template>
        </el-table-column>
      </el-table>

      <!-- Pagination -->
      <div class="pagination-wrapper" v-if="trades.length > pageSize">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="trades.length"
          layout="sizes, prev, pager, next, jumper, total"
          background
          small
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { Download } from '@element-plus/icons-vue'
import type { TradeRecord } from '@/types/backtest'
import { useFormat } from '@/composables/useFormat'

const { formatCurrency, formatDate, formatPrice, formatDateTime } = useFormat()

const props = defineProps<{
  trades?: TradeRecord[]
  loading?: boolean
}>()

// Pagination state
const currentPage = ref(1)
const pageSize = ref(10)

// Compute paginated trades
const paginatedTrades = computed(() => {
  if (!props.trades) return []
  const start = (currentPage.value - 1) * pageSize.value
  const end = start + pageSize.value
  return props.trades.slice(start, end)
})

// Reset pagination to page 1 when trade data changes significantly
watch(
  () => props.trades?.length ?? 0,
  () => {
    currentPage.value = 1
  }
)

function exitReasonTagType(reason: string): string {
  switch (reason) {
    case 'take_profit': return 'success'
    case 'stop_loss': return 'danger'
    case 'signal': return 'primary'
    default: return 'info'
  }
}

function exitReasonLabel(reason: string): string {
  switch (reason) {
    case 'take_profit': return '止盈'
    case 'stop_loss': return '止损'
    case 'signal': return '信号'
    default: return reason
  }
}

function formatDateCol(_row: any, _col: any, cellValue: string) {
  return formatDateTime(cellValue)
}

function formatPriceCol(_row: any, _col: any, cellValue: number) {
  return formatPrice(cellValue)
}

function formatQtyCol(_row: any, _col: any, cellValue: number) {
  return cellValue?.toFixed(4) ?? '--'
}

function formatFeeCol(_row: any, _col: any, cellValue: number) {
  return cellValue != null ? cellValue.toFixed(4) : '--'
}

function formatHoldingPeriod(_row: any, _col: any, cellValue: number | string) {
  const ms = typeof cellValue === 'number' ? cellValue : parseInt(String(cellValue))
  if (!ms || ms <= 0) return '--'
  const seconds = Math.floor(ms / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  const days = Math.floor(hours / 24)
  if (days > 0) return `${days}d ${hours % 24}h`
  if (hours > 0) return `${hours}h ${minutes % 60}m`
  if (minutes > 0) return `${minutes}m`
  return `${seconds}s`
}

function handleExport() {
  const trades = props.trades
  if (!trades || trades.length === 0) return

  // Build CSV header
  const headers = [
    '开仓时间', '方向', '开仓价', '平仓价', '数量',
    '盈亏', '盈亏%', '手续费', '滑点', '持仓时长', '平仓原因'
  ]

  // Build CSV rows
  const rows = trades.map((t) => [
    formatDateTime(t.entry_time),
    t.direction === 'long' ? '多头' : '空头',
    formatPrice(t.entry_price),
    formatPrice(t.exit_price),
    t.quantity.toFixed(4),
    t.pnl_usdt.toFixed(2),
    t.pnl_pct.toFixed(2) + '%',
    t.fee.toFixed(4),
    t.slippage.toFixed(4),
    formatHoldingPeriod(null, null, t.holding_period_ms),
    t.exit_reason ? exitReasonLabel(t.exit_reason) : ''
  ])

  // Combine into CSV content
  const csvContent = [
    headers.join(','),
    ...rows.map((row) =>
      row.map((cell) => {
        // Escape quotes and wrap in quotes if contains comma or quote
        const str = String(cell)
        if (str.includes(',') || str.includes('"') || str.includes('\n')) {
          return '"' + str.replace(/"/g, '""') + '"'
        }
        return str
      }).join(',')
    )
  ].join('\n')

  // Add BOM for Excel UTF-8 compatibility
  const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.setAttribute('download', `trades_export_${Date.now()}.csv`)
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)

  ElMessage.success(`已导出 ${trades.length} 条交易记录`)
}

// Expose internal functions for testing via defineExpose
defineExpose({
  exitReasonLabel,
  exitReasonTagType,
  formatDateCol,
  formatPriceCol,
  formatQtyCol,
  formatFeeCol,
  formatHoldingPeriod,
  handleExport,
  paginatedTrades,
  currentPage,
  pageSize,
})
</script>

<script lang="ts">
export default {
  name: 'BacktestTradesTable',
}
</script>

<style scoped lang="scss">
.backtest-trades-table {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 24px;
}

.table-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.table-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.trade-count {
  font-size: 13px;
  color: var(--color-text-tertiary);
}

.trades-table-wrapper {
  width: 100%;
  overflow-x: auto;
}

.pnl-positive {
  color: var(--color-buy, #22c55e);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.pnl-negative {
  color: var(--color-sell, #ef4444);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.exit-reason-none {
  color: var(--color-text-tertiary);
  font-size: 12px;
}

/* Pagination */
.pagination-wrapper {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
  padding-top: 8px;
}

/* Loading skeleton */
.table-loading {
  padding: 16px 0;

  .skeleton-row {
    display: flex;
    gap: 16px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--color-border);
  }

  .skeleton-cell {
    flex: 1;
    height: 14px;
    border-radius: 4px;
    background: linear-gradient(90deg, var(--color-surface) 25%, rgba(255,255,255,0.04) 50%, var(--color-surface) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.5s ease-in-out infinite;
  }
}

.table-empty {
  padding: 40px 0;
  display: flex;
  justify-content: center;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
</style>
