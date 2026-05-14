<template>
  <div class="trade-record-tab">
    <!-- Filter Bar -->
    <div class="trade-filter">
      <el-input
        v-model="filterSymbol"
        placeholder="搜索交易对"
        :prefix-icon="Search"
        :style="{ width: '160px' }"
        size="default"
        clearable
        @input="onSymbolInput"
        @clear="onFilterChange"
      />
      <el-select
        v-model="filterSide"
        placeholder="方向"
        :style="{ width: '120px' }"
        size="default"
        clearable
        @change="onFilterChange"
      >
        <el-option label="全部" value="" />
        <el-option label="买入" value="buy" />
        <el-option label="卖出" value="sell" />
      </el-select>
      <el-date-picker
        v-model="filterDateRange"
        type="daterange"
        range-separator="至"
        start-placeholder="开始日期"
        end-placeholder="结束日期"
        format="YYYY-MM-DD"
        value-format="YYYY-MM-DD"
        :style="{ width: '280px' }"
        size="default"
        clearable
        @change="onFilterChange"
      />
    </div>

    <!-- Trade Record Table -->
    <div class="trade-table-wrapper">
      <el-table
        v-if="!loading"
        :data="trades"
        class="trade-table"
        :empty-text="'暂无成交记录'"
        @row-click="onRowClick"
      >
        <el-table-column label="成交时间" width="100" prop="created_at">
          <template #default="{ row }">
            <span class="mono secondary">{{ formatTime(row.created_at) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="交易对" width="100" prop="symbol">
          <template #default="{ row }">
            <span class="symbol-text">{{ row.symbol }}</span>
          </template>
        </el-table-column>

        <el-table-column label="方向" width="60" align="center" prop="side">
          <template #default="{ row }">
            <el-tag
              :type="row.side === 'buy' ? 'success' : 'danger'"
              size="small"
              effect="dark"
            >
              {{ row.side === 'buy' ? '买' : '卖' }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column label="成交价" width="120" align="right" prop="price">
          <template #default="{ row }">
            <span class="mono">{{ formatPrice(row.price) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="数量" width="100" align="right" prop="quantity">
          <template #default="{ row }">
            <span class="mono">{{ row.quantity }}</span>
          </template>
        </el-table-column>

        <el-table-column label="成交额" width="120" align="right">
          <template #default="{ row }">
            <span class="mono">{{ calcNotional(row) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="手续费" width="80" align="right" prop="fee">
          <template #default="{ row }">
            <span class="mono fee-text">{{ row.fee }}</span>
          </template>
        </el-table-column>

        <el-table-column label="类型" width="60" align="center">
          <template #default="{ row }">
            <span class="maker-tag">{{ row.is_maker ? 'M' : 'T' }}</span>
          </template>
        </el-table-column>
      </el-table>

      <!-- Loading Skeleton -->
      <div v-else class="skeleton-container">
        <div v-for="i in 5" :key="i" class="skeleton-row">
          <div class="skeleton-line" style="width: 80px; height: 14px;" />
          <div class="skeleton-line" style="width: 90px; height: 14px;" />
          <div class="skeleton-line" style="width: 40px; height: 20px;" />
          <div class="skeleton-line" style="width: 100px; height: 14px;" />
          <div class="skeleton-line" style="width: 80px; height: 14px;" />
          <div class="skeleton-line" style="width: 100px; height: 14px;" />
        </div>
      </div>
    </div>

    <!-- Pagination -->
    <div v-if="total > 0" class="pagination-bar">
      <el-pagination
        v-model:current-page="currentPage"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[20, 50, 100]"
        layout="total, sizes, prev, pager, next"
        background
        small
        @current-change="onPageChange"
        @size-change="onSizeChange"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { Search } from '@element-plus/icons-vue'
import type { Trade } from '@/types/order'

/**
 * TradeRecordTab - Tab panel displaying trade records with filters,
 * pagination, and row-click to view associated order.
 * Follows Design_OrderManagement.md section 6 specs.
 */

const props = withDefaults(defineProps<{
  /** Array of trade records */
  trades?: Trade[]
  /** Total number of trades (for pagination) */
  total?: number
  /** Current page number */
  page?: number
  /** Page size */
  size?: number
  /** Loading state */
  loading?: boolean
}>(), {
  trades: () => [],
  total: 0,
  page: 1,
  size: 20,
  loading: false,
})

const emit = defineEmits<{
  /** Emitted when a trade row is clicked (view associated order) */
  (e: 'row-click', trade: Trade): void
  /** Emitted when filters change */
  (e: 'filter-change', filters: { symbol: string; side: string; startDate: string; endDate: string }): void
  /** Emitted when page changes */
  (e: 'page-change', page: number, size: number): void
}>()

const filterSymbol = ref('')
const filterSide = ref('')
const filterDateRange = ref<[string, string] | null>(null)
const currentPage = ref(props.page)
const pageSize = ref(props.size)

// Sync with parent
watch(() => props.page, (val) => { currentPage.value = val })
watch(() => props.size, (val) => { pageSize.value = val })

/** Format time as HH:mm:ss or MM-DD HH:mm */
function formatTime(isoStr: string): string {
  if (!isoStr) return ''
  const d = new Date(isoStr)
  const now = new Date()
  const isToday = d.toDateString() === now.toDateString()

  if (isToday) {
    const hh = String(d.getHours()).padStart(2, '0')
    const mm = String(d.getMinutes()).padStart(2, '0')
    const ss = String(d.getSeconds()).padStart(2, '0')
    return `${hh}:${mm}:${ss}`
  }

  const mm = String(d.getMonth() + 1).padStart(2, '0')
  const dd = String(d.getDate()).padStart(2, '0')
  const hh = String(d.getHours()).padStart(2, '0')
  const mi = String(d.getMinutes()).padStart(2, '0')
  return `${mm}-${dd} ${hh}:${mi}`
}

/** Format price with thousand separators */
function formatPrice(value: string | null | undefined): string {
  if (!value) return '0.00'
  const num = parseFloat(value)
  if (isNaN(num)) return '0.00'
  return num.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 8 })
}

/** Calculate notional value (price × quantity) */
function calcNotional(trade: Trade): string {
  const price = parseFloat(trade.price)
  const qty = parseFloat(trade.quantity)
  if (isNaN(price) || isNaN(qty)) return '0.00'
  return (price * qty).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

/** Handle row click → emit to open associated order detail */
function onRowClick(trade: Trade) {
  emit('row-click', trade)
}

/** Debounced symbol input */
let symbolTimer: ReturnType<typeof setTimeout> | null = null
function onSymbolInput() {
  if (symbolTimer) clearTimeout(symbolTimer)
  symbolTimer = setTimeout(onFilterChange, 300)
}

/** Build and emit filter change */
function onFilterChange() {
  emit('filter-change', {
    symbol: filterSymbol.value,
    side: filterSide.value,
    startDate: filterDateRange.value?.[0] ?? '',
    endDate: filterDateRange.value?.[1] ?? '',
  })
}

/** Handle page change */
function onPageChange(page: number) {
  emit('page-change', page, pageSize.value)
}

/** Handle page size change */
function onSizeChange(size: number) {
  emit('page-change', 1, size)
}
</script>

<style scoped lang="scss">
.trade-record-tab {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.trade-filter {
  display: flex;
  gap: 12px;
  align-items: center;
}

.trade-table-wrapper {
  background: var(--color-surface, #191a1b);
  border-radius: 12px;
  overflow: hidden;
}

.trade-table {
  --el-table-bg-color: transparent;
  --el-table-tr-bg-color: transparent;
  --el-table-header-bg-color: transparent;
  --el-table-row-hover-bg-color: rgba(255, 255, 255, 0.04);
  --el-table-border-color: var(--color-border, rgba(255, 255, 255, 0.08));
  --el-table-header-text-color: var(--color-text-tertiary, #8a8f98);
  --el-table-text-color: var(--color-text-primary, #f7f8f8);

  font-size: 13px;
}

.mono {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-size: 12px;
}

.secondary {
  color: var(--color-text-secondary, #d0d6e0);
}

.symbol-text {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-primary, #f7f8f8);
}

.fee-text {
  color: var(--color-frozen, #f5a623);
}

.maker-tag {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 18px;
  border-radius: 3px;
  background: var(--color-surface-elevated, #212223);
  color: var(--color-text-secondary, #d0d6e0);
  font-size: 11px;
  font-weight: 600;
}

.pagination-bar {
  display: flex;
  justify-content: flex-end;
  padding: 12px 0;
}

// Skeleton
.skeleton-container {
  padding: 8px 0;
}

.skeleton-row {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.04));
}

.skeleton-line {
  background: linear-gradient(90deg, var(--color-surface, #191a1b) 25%, rgba(255, 255, 255, 0.05) 50%, var(--color-surface, #191a1b) 75%);
  background-size: 200% 100%;
  animation: skeleton-loading 1.5s ease-in-out infinite;
  border-radius: 4px;
  height: 14px;
}

@keyframes skeleton-loading {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
</style>
