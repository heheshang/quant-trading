<template>
  <div class="order-filter-bar">
    <div class="filter-group">
      <!-- Status Filter -->
      <el-select
        v-model="filterStatus"
        multiple
        collapse-tags
        collapse-tags-tooltip
        placeholder="状态"
        :style="{ width: '180px' }"
        size="default"
        clearable
        @change="emitChange"
      >
        <el-option
          v-for="opt in statusOptions"
          :key="opt.value"
          :label="opt.label"
          :value="opt.value"
        />
      </el-select>

      <!-- Side Filter -->
      <el-select
        v-model="filterSide"
        placeholder="方向"
        :style="{ width: '120px' }"
        size="default"
        clearable
        @change="emitChange"
      >
        <el-option label="全部" value="" />
        <el-option label="买入" value="buy" />
        <el-option label="卖出" value="sell" />
      </el-select>

      <!-- Order Type Filter -->
      <el-select
        v-model="filterOrderType"
        placeholder="类型"
        :style="{ width: '120px' }"
        size="default"
        clearable
        @change="emitChange"
      >
        <el-option label="全部" value="" />
        <el-option label="限价" value="limit" />
        <el-option label="市价" value="market" />
      </el-select>

      <!-- Symbol Search -->
      <el-input
        v-model="filterSymbol"
        placeholder="搜索交易对"
        :prefix-icon="Search"
        :style="{ width: '160px' }"
        size="default"
        clearable
        @input="onSymbolInput"
        @clear="emitChange"
      />

      <!-- Date Range -->
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
        :default-time="defaultTime"
        @change="emitChange"
      />
    </div>

    <div class="filter-actions">
      <!-- Clear Filters -->
      <el-button
        v-if="hasActiveFilters"
        :icon="CircleClose"
        text
        size="default"
        class="clear-btn"
        @click="clearFilters"
      >
        清空筛选
      </el-button>

      <!-- Cancel All Orders -->
      <el-button
        text
        size="default"
        class="cancel-all-btn"
        :disabled="!hasActiveOrders"
        :loading="cancellingAll"
        @click="onCancelAll"
      >
        全部撤单
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Search, CircleClose } from '@element-plus/icons-vue'
import { ElMessageBox, ElMessage } from 'element-plus'
import { cancelAllOrders } from '@/api/order'
import type { OrderStatus, OrderSide, OrderType } from '@/types/order'
import { getOrderStatusText } from '@/types/order'

/**
 * OrderFilterBar - Filter bar for the order list with status/side/type/symbol/date filters.
 * Includes cancel-all-orders functionality.
 * Follows Design_OrderManagement.md section 2.5 specs.
 */

const props = withDefaults(defineProps<{
  /** Whether there are active orders that can be cancelled */
  hasActiveOrders?: boolean
}>(), {
  hasActiveOrders: false,
})

const emit = defineEmits<{
  /** Emitted when any filter value changes */
  (e: 'filter-change', filters: OrderFilterState): void
  /** Emitted after cancel-all succeeds */
  (e: 'cancel-all-success', count: number): void
}>()

/** Filter state interface */
export interface OrderFilterState {
  status: OrderStatus[]
  side: OrderSide | ''
  orderType: OrderType | ''
  symbol: string
  startDate: string
  endDate: string
}

/** Status dropdown options */
const statusOptions = [
  { value: 'pending', label: getOrderStatusText('pending') },
  { value: 'partial_filled', label: getOrderStatusText('partial_filled') },
  { value: 'filled', label: getOrderStatusText('filled') },
  { value: 'cancelled', label: getOrderStatusText('cancelled') },
  { value: 'expired', label: getOrderStatusText('expired') },
  { value: 'rejected', label: getOrderStatusText('rejected') },
] as const

const filterStatus = ref<OrderStatus[]>([])
const filterSide = ref<OrderSide | ''>('')
const filterOrderType = ref<OrderType | ''>('')
const filterSymbol = ref('')
const filterDateRange = ref<[string, string] | null>(null)
const cancellingAll = ref(false)

/** Default time for date range picker */
const defaultTime = [
  new Date(2000, 0, 1, 0, 0, 0),
  new Date(2000, 0, 1, 23, 59, 59),
]

/** Whether any filter is active */
const hasActiveFilters = computed(() => {
  return (
    filterStatus.value.length > 0 ||
    filterSide.value !== '' ||
    filterOrderType.value !== '' ||
    filterSymbol.value !== '' ||
    filterDateRange.value !== null
  )
})

/** Debounced symbol input handler */
let symbolTimer: ReturnType<typeof setTimeout> | null = null
function onSymbolInput() {
  if (symbolTimer) clearTimeout(symbolTimer)
  symbolTimer = setTimeout(() => {
    emitChange()
  }, 300)
}

/** Build and emit current filter state */
function emitChange() {
  emit('filter-change', {
    status: filterStatus.value,
    side: filterSide.value,
    orderType: filterOrderType.value,
    symbol: filterSymbol.value,
    startDate: filterDateRange.value?.[0] ?? '',
    endDate: filterDateRange.value?.[1] ?? '',
  })
}

/** Reset all filters */
function clearFilters() {
  filterStatus.value = []
  filterSide.value = ''
  filterOrderType.value = ''
  filterSymbol.value = ''
  filterDateRange.value = null
  emitChange()
}

/** Cancel all active orders with confirmation */
async function onCancelAll() {
  try {
    await ElMessageBox.confirm(
      '确认撤销所有活跃委托？此操作不可撤回。',
      '全部撤单',
      {
        confirmButtonText: '确认撤单',
        cancelButtonText: '取消',
        type: 'warning',
        confirmButtonClass: 'el-button--danger',
      },
    )

    cancellingAll.value = true
    const result = await cancelAllOrders()
    ElMessage.success(`已撤销 ${result.cancelled_count} 个委托`)
    emit('cancel-all-success', result.cancelled_count)
  } catch (err: unknown) {
    if (err !== 'cancel' && err instanceof Error) {
      ElMessage.error(`撤单失败: ${err.message}`)
    }
  } finally {
    cancellingAll.value = false
  }
}
</script>

<style scoped lang="scss">
.order-filter-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  gap: 12px;
  flex-wrap: wrap;
}

.filter-group {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.filter-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.clear-btn {
  color: var(--color-text-tertiary, #8a8f98);
}

.cancel-all-btn {
  color: var(--color-error, #e5484d);

  &:hover {
    color: #fff;
    background-color: var(--color-error, #e5484d);
  }

  &:disabled {
    color: var(--color-text-tertiary, #8a8f98);
    opacity: 0.5;
  }
}
</style>
