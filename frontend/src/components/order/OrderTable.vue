<template>
  <div class="order-table-wrapper">
    <!-- Loading Skeleton -->
    <template v-if="loading">
      <div v-for="i in 5" :key="i" class="skeleton-row">
        <div class="skeleton-line" style="width: 80px; height: 14px;" />
        <div class="skeleton-line" style="width: 90px; height: 14px;" />
        <div class="skeleton-line" style="width: 40px; height: 20px;" />
        <div class="skeleton-line" style="width: 50px; height: 14px;" />
        <div class="skeleton-line" style="width: 100px; height: 14px;" />
        <div class="skeleton-line" style="width: 80px; height: 14px;" />
        <div class="skeleton-line" style="width: 80px; height: 14px;" />
        <div class="skeleton-line" style="width: 60px; height: 20px;" />
        <div class="skeleton-line" style="width: 60px; height: 28px;" />
      </div>
    </template>

    <!-- Table -->
    <el-table
      v-else
      :data="orders"
      class="order-table"
      :row-class-name="getRowClassName"
      @row-click="onRowClick"
      :empty-text="emptyText"
    >
      <!-- Time Column -->
      <el-table-column label="时间" width="100" prop="created_at">
        <template #default="{ row }">
          <span class="mono-text secondary-text">{{ formatTime(row.created_at) }}</span>
        </template>
      </el-table-column>

      <!-- Symbol Column -->
      <el-table-column label="交易对" width="100" prop="symbol">
        <template #default="{ row }">
          <span class="symbol-text">{{ row.symbol }}</span>
        </template>
      </el-table-column>

      <!-- Side Column -->
      <el-table-column label="方向" width="60" align="center" prop="side">
        <template #default="{ row }">
          <el-tag
            :type="row.side === 'buy' ? 'success' : 'danger'"
            size="small"
            effect="dark"
            class="side-tag"
          >
            {{ row.side === 'buy' ? '买' : '卖' }}
          </el-tag>
        </template>
      </el-table-column>

      <!-- Order Type Column -->
      <el-table-column label="类型" width="80" align="center" prop="order_type">
        <template #default="{ row }">
          <span
            :class="{
              'type-disabled': row.order_type === 'stop' || row.order_type === 'stop_limit',
            }"
          >
            {{ getOrderTypeText(row.order_type) }}
          </span>
        </template>
      </el-table-column>

      <!-- Price Column -->
      <el-table-column label="价格" width="120" align="right" prop="price">
        <template #default="{ row }">
          <span v-if="row.order_type === 'market'" class="mono-text secondary-text">市价</span>
          <span v-else class="mono-text">{{ formatPrice(row.price) }}</span>
        </template>
      </el-table-column>

      <!-- Quantity Column -->
      <el-table-column label="数量" width="100" align="right" prop="quantity">
        <template #default="{ row }">
          <span class="mono-text">{{ row.quantity }}</span>
        </template>
      </el-table-column>

      <!-- Filled Column with Progress Bar -->
      <el-table-column label="已成交" width="130" align="right">
        <template #default="{ row }">
          <div class="filled-cell">
            <span class="mono-text">
              {{ row.filled_quantity }} / {{ row.quantity }}
            </span>
            <div v-if="parseFloat(row.filled_quantity) > 0" class="fill-progress">
              <div
                class="fill-progress-bar"
                :class="row.side === 'buy' ? 'fill-buy' : 'fill-sell'"
                :style="{ width: getFillPercent(row) + '%' }"
              />
              <span v-if="getFillPercent(row) < 100" class="fill-percent">
                {{ getFillPercent(row) }}%
              </span>
            </div>
          </div>
        </template>
      </el-table-column>

      <!-- Avg Fill Price Column -->
      <el-table-column label="成交均价" width="120" align="right">
        <template #default="{ row }">
          <span v-if="row.avg_fill_price" class="mono-text">{{ formatPrice(row.avg_fill_price) }}</span>
          <span v-else class="mono-text secondary-text">—</span>
        </template>
      </el-table-column>

      <!-- Status Column -->
      <el-table-column label="状态" width="100" align="center" prop="status">
        <template #default="{ row }">
          <el-tag
            :type="getOrderStatusType(row.status)"
            size="small"
            effect="plain"
            class="status-pill"
          >
            {{ getOrderStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>

      <!-- Actions Column -->
      <el-table-column label="操作" width="100" align="center">
        <template #default="{ row }">
          <el-button
            v-if="isCancellable(row.status)"
            text
            size="small"
            class="cancel-btn"
            :loading="cancellingIds.has(row.order_id)"
            @click.stop="onCancelOrder(row)"
          >
            撤单
          </el-button>
          <el-button
            v-else
            text
            size="small"
            class="detail-btn"
            @click.stop="onRowClick(row)"
          >
            详情
          </el-button>
        </template>
      </el-table-column>
    </el-table>

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
import { ElMessage, ElMessageBox } from 'element-plus'
import { cancelOrder } from '@/api/order'
import type { Order, OrderStatus } from '@/types/order'
import { getOrderStatusType, getOrderStatusText } from '@/types/order'

/**
 * OrderTable - Table displaying order list with status pills, fill progress bars,
 * cancel/detail actions, and pagination.
 * Follows Design_OrderManagement.md section 2.6 specs.
 */

const props = withDefaults(defineProps<{
  /** Array of orders to display */
  orders: Order[]
  /** Total number of orders (for pagination) */
  total?: number
  /** Current page number */
  page?: number
  /** Page size */
  size?: number
  /** Loading state */
  loading?: boolean
  /** Empty state text */
  emptyText?: string
}>(), {
  total: 0,
  page: 1,
  size: 20,
  loading: false,
  emptyText: '暂无委托记录',
})

const emit = defineEmits<{
  /** Emitted when a row is clicked (view detail) */
  (e: 'row-click', order: Order): void
  /** Emitted when an order is successfully cancelled */
  (e: 'cancel-success', orderId: string): void
  /** Emitted when page or size changes */
  (e: 'page-change', page: number, size: number): void
}>()

const currentPage = ref(props.page)
const pageSize = ref(props.size)
const cancellingIds = ref(new Set<string>())

// Sync with parent
watch(() => props.page, (val) => { currentPage.value = val })
watch(() => props.size, (val) => { pageSize.value = val })

/** Format ISO datetime to MM-DD HH:mm */
function formatTime(isoStr: string): string {
  if (!isoStr) return ''
  const d = new Date(isoStr)
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

/** Get order type display text */
function getOrderTypeText(type: string): string {
  const map: Record<string, string> = {
    limit: '限价',
    market: '市价',
    stop: '止损',
    stop_limit: '止损限价',
  }
  return map[type] ?? type
}

/** Calculate fill percentage */
function getFillPercent(order: Order): number {
  const filled = parseFloat(order.filled_quantity)
  const total = parseFloat(order.quantity)
  if (total === 0) return 0
  return Math.round((filled / total) * 100)
}

/** Whether an order can be cancelled */
function isCancellable(status: OrderStatus): boolean {
  return status === 'pending' || status === 'partial_filled'
}

/** Row class name for status-based styling */
function getRowClassName({ row }: { row: Order }): string {
  return `order-row order-row--${row.status}`
}

/** Handle row click → open detail drawer */
function onRowClick(order: Order) {
  emit('row-click', order)
}

/** Cancel order with confirmation */
async function onCancelOrder(order: Order) {
  try {
    await ElMessageBox.confirm(
      `确认撤销 ${order.symbol} ${order.side === 'buy' ? '买入' : '卖出'} ${order.quantity} 委托？`,
      '撤销委托',
      {
        confirmButtonText: '确认撤单',
        cancelButtonText: '取消',
        type: 'warning',
      },
    )
    cancellingIds.value.add(order.order_id)
    await cancelOrder(order.order_id)
    ElMessage.success('委托已撤销')
    emit('cancel-success', order.order_id)
  } catch (err: unknown) {
    if (err !== 'cancel' && err instanceof Error) {
      ElMessage.error(`撤单失败: ${err.message}`)
    }
  } finally {
    cancellingIds.value.delete(order.order_id)
  }
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
.order-table-wrapper {
  background: var(--color-surface, #191a1b);
  border-radius: 12px;
  overflow: hidden;
}

.order-table {
  --el-table-bg-color: transparent;
  --el-table-tr-bg-color: transparent;
  --el-table-header-bg-color: transparent;
  --el-table-row-hover-bg-color: rgba(255, 255, 255, 0.04);
  --el-table-border-color: var(--color-border, rgba(255, 255, 255, 0.08));
  --el-table-header-text-color: var(--color-text-tertiary, #8a8f98);
  --el-table-text-color: var(--color-text-primary, #f7f8f8);

  font-size: 13px;
}

.mono-text {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-size: 12px;
}

.secondary-text {
  color: var(--color-text-tertiary, #8a8f98);
}

.symbol-text {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-primary, #f7f8f8);
}

.side-tag {
  font-size: 12px;
  height: 20px;
  line-height: 18px;
  padding: 0 6px;
}

.type-disabled {
  color: var(--color-text-tertiary, #8a8f98);
  font-style: italic;
  font-size: 12px;
}

.status-pill {
  border-radius: 10px;
  font-size: 12px;
  height: 22px;
  line-height: 20px;
}

.filled-cell {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.fill-progress {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 3px;
  width: 60px;
  background: var(--color-border, rgba(255, 255, 255, 0.08));
  border-radius: 2px;
  position: relative;
  margin-top: 2px;
}

.fill-progress-bar {
  height: 100%;
  border-radius: 2px;
  transition: width 0.3s ease;
}

.fill-buy {
  background: var(--color-buy, #67C23A);
}

.fill-sell {
  background: var(--color-sell, #F56C6C);
}

.fill-percent {
  font-size: 11px;
  color: var(--color-text-tertiary, #8a8f98);
  margin-left: 4px;
  white-space: nowrap;
}

.cancel-btn {
  color: var(--color-error, #e5484d);

  &:hover {
    color: var(--color-error, #e5484d);
    opacity: 0.8;
  }
}

.detail-btn {
  color: var(--color-accent, #7170ff);

  &:hover {
    color: var(--color-accent-hover, #8b8aff);
  }
}

.pagination-bar {
  display: flex;
  justify-content: flex-end;
  padding: 12px 16px;
  border-top: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
}

// Skeleton rows
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

// Row flash animation on status change
:deep(.order-row) {
  transition: background-color 0.3s ease;
}
</style>
