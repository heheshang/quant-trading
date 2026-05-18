<template>
  <div class="backtest-history-list">
    <div class="history-header">
      <h3 class="section-title">回测历史</h3>
      <div class="header-actions">
        <span v-if="items && items.length > 0" class="item-count">
          共 {{ items.length }} 条
        </span>
        <el-button
          size="small"
          class="refresh-btn"
          :loading="loading"
          @click="$emit('refresh')"
        >
          <el-icon style="margin-right: 4px"><Refresh /></el-icon>
          刷新
        </el-button>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="loading && (!items || items.length === 0)" class="table-loading">
      <div class="skeleton-row" v-for="n in 5" :key="n">
        <div class="skeleton-cell" v-for="m in 6" :key="m"></div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-else-if="!items || items.length === 0" class="table-empty">
      <el-empty description="暂无回测历史" :image-size="60" />
    </div>

    <!-- Data table -->
    <div v-else class="history-table-wrapper">
      <el-table
        :data="paginatedItems"
        stripe
        size="small"
        style="width: 100%"
        :row-class-name="rowClassName"
        @row-click="handleRowClick"
        highlight-current-row
      >
        <el-table-column
          prop="strategy_id"
          label="策略ID"
          min-width="120"
          show-overflow-tooltip
        />
        <el-table-column
          prop="symbol"
          label="交易对"
          width="120"
        />
        <el-table-column
          prop="status"
          label="状态"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag
              :type="statusTagType(row.status)"
              size="small"
              effect="plain"
            >
              {{ statusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="total_return_pct"
          label="总收益率"
          width="120"
          align="right"
          sortable
        >
          <template #default="{ row }">
            <span :class="row.total_return_pct >= 0 ? 'return-up' : 'return-down'">
              {{ formatPercent(row.total_return_pct) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column
          prop="sharpe_ratio"
          label="夏普比率"
          width="110"
          align="right"
          sortable
        >
          <template #default="{ row }">
            <span class="sharpe-value">{{ row.sharpe_ratio?.toFixed(2) ?? '--' }}</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_at"
          label="创建时间"
          width="170"
          :formatter="formatDateCol"
        />
        <el-table-column
          label="操作"
          width="80"
          align="center"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button
              type="danger"
              size="small"
              text
              @click.stop="handleDelete(row)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- Pagination -->
      <div class="pagination-wrapper" v-if="items.length > pageSize">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50]"
          :total="items.length"
          layout="sizes, prev, pager, next, total"
          background
          small
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { Refresh } from '@element-plus/icons-vue'
import type { BacktestSummary } from '@/types/backtest'
import { useFormat } from '@/composables/useFormat'

const { formatPercent, formatDateTime } = useFormat()

const props = withDefaults(defineProps<{
  items?: BacktestSummary[]
  loading?: boolean
  currentId?: string
}>(), {
  items: () => [],
  loading: false,
  currentId: '',
})

const emit = defineEmits<{
  select: [item: BacktestSummary]
  delete: [id: string]
  refresh: []
}>()

// Pagination state
const currentPage = ref(1)
const pageSize = ref(10)

// Compute paginated items
const paginatedItems = computed(() => {
  if (!props.items) return []
  const start = (currentPage.value - 1) * pageSize.value
  const end = start + pageSize.value
  return props.items.slice(start, end)
})

// Reset pagination to page 1 when items change
watch(
  () => props.items?.length ?? 0,
  () => {
    currentPage.value = 1
  }
)

function statusTagType(status: string): string {
  switch (status) {
    case 'completed': return 'success'
    case 'running': return 'primary'
    case 'pending': return 'warning'
    case 'failed': return 'danger'
    default: return 'info'
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case 'completed': return '已完成'
    case 'running': return '运行中'
    case 'pending': return '等待中'
    case 'failed': return '失败'
    default: return status
  }
}

function rowClassName({ row }: { row: BacktestSummary }): string {
  return row.id === props.currentId ? 'current-row-highlight' : ''
}

function handleRowClick(row: BacktestSummary) {
  emit('select', row)
}

function handleDelete(row: BacktestSummary) {
  emit('delete', row.id)
}

function formatDateCol(_row: any, _col: any, cellValue: string) {
  return formatDateTime(cellValue)
}

// Expose helpers for testing
defineExpose({
  statusTagType,
  statusLabel,
  rowClassName,
  handleRowClick,
  handleDelete,
  formatDateCol,
  paginatedItems,
  currentPage,
  pageSize,
})
</script>

<style scoped lang="scss">
.backtest-history-list {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 24px;
}

.history-header {
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

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.item-count {
  font-size: 13px;
  color: var(--color-text-tertiary);
}

.history-table-wrapper {
  width: 100%;
  overflow-x: auto;
}

.return-up {
  color: var(--color-buy, #22c55e);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.return-down {
  color: var(--color-sell, #ef4444);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.sharpe-value {
  font-variant-numeric: tabular-nums;
  color: var(--color-text-primary);
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

:deep(.current-row-highlight) {
  background-color: rgba(113, 112, 255, 0.08) !important;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
</style>
