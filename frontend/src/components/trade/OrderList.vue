<template>
  <div class="order-list">
    <!-- Tab bar (hidden when hideTabs=true, used inside parent tabs) -->
    <div v-if="!hideTabs" class="order-tabs-wrapper">
      <el-tabs v-model="activeTab" class="order-tabs" @tab-change="onTabChange">
        <el-tab-pane name="current">
          <template #label>
            <span>当前委托 <el-badge v-if="activeCount > 0" :value="activeCount" class="tab-badge" /></span>
          </template>
        </el-tab-pane>
        <el-tab-pane label="历史委托" name="history" />
      </el-tabs>
    </div>

    <!-- Filters -->
    <div class="filters">
      <el-select
        v-model="filterSymbol"
        placeholder="交易对"
        clearable
        size="small"
        style="width: 120px"
        @change="fetchOrders"
      >
        <el-option
          v-for="s in symbolOptions"
          :key="s"
          :label="s"
          :value="s"
        />
      </el-select>
      <el-select
        v-model="filterSide"
        placeholder="方向"
        clearable
        size="small"
        style="width: 90px"
        @change="fetchOrders"
      >
        <el-option label="买入" value="buy" />
        <el-option label="卖出" value="sell" />
      </el-select>
      <el-button
        v-if="activeTab === 'current' && activeCount > 0"
        type="danger"
        size="small"
        plain
        @click="handleCancelAll"
      >
        全部撤单
      </el-button>
    </div>

    <!-- Table -->
    <el-table
      :data="orders"
      stripe
      size="small"
      v-loading="loading"
      class="order-table"
      :empty-text="activeTab === 'current' ? '暂无未成交委托' : '暂无历史委托'"
    >
      <el-table-column prop="created_at" label="时间" width="140">
        <template #default="{ row }">
          {{ formatTime(row.created_at) }}
        </template>
      </el-table-column>
      <el-table-column prop="symbol" label="交易对" width="100" />
      <el-table-column prop="side" label="方向" width="60">
        <template #default="{ row }">
          <span :class="row.side === 'buy' ? 'side-buy' : 'side-sell'">
            {{ getOrderSideText(row.side) }}
          </span>
        </template>
      </el-table-column>
      <el-table-column prop="order_type" label="类型" width="60">
        <template #default="{ row }">
          {{ row.order_type === 'limit' ? '限价' : '市价' }}
        </template>
      </el-table-column>
      <el-table-column prop="price" label="价格" width="100">
        <template #default="{ row }">
          {{ row.price ?? '--' }}
        </template>
      </el-table-column>
      <el-table-column prop="quantity" label="数量" width="80" />
      <el-table-column prop="filled_quantity" label="已成交" width="80" />
      <el-table-column prop="status" label="状态" width="80">
        <template #default="{ row }">
          <el-tag :type="getOrderStatusType(row.status)" size="small">
            {{ getOrderStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column v-if="activeTab === 'current'" label="操作" width="70" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'pending' || row.status === 'partial_filled'"
            type="danger"
            size="small"
            link
            @click="handleCancel(row)"
          >
            撤单
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- Pagination (history only) -->
    <div v-if="activeTab === 'history' && total > 0" class="pagination">
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[20, 50, 100]"
        layout="total, sizes, prev, pager, next"
        size="small"
        @current-change="fetchOrders"
        @size-change="fetchOrders"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { Order, OrderStatus } from '@/types/order'
import { getOrderStatusType, getOrderStatusText, getOrderSideText } from '@/types/order'
import { getOrders, cancelOrder, cancelAllOrders } from '@/api/order'

interface Props {
  symbol?: string
  isHistory?: boolean
  /** Hide the internal tab bar when used as embedded component */
  hideTabs?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  symbol: '',
  isHistory: false,
  hideTabs: false,
})

const emit = defineEmits<{
  (e: 'orders-change', count: number): void
}>()

const activeTab = ref<'current' | 'history'>('current')
const loading = ref(false)
const orders = ref<Order[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)
const filterSymbol = ref(props.symbol ?? '')
const filterSide = ref<'buy' | 'sell' | ''>('')
let pollTimer: ReturnType<typeof setInterval> | null = null

const symbolOptions = ['BTC/USDT', 'ETH/USDT', 'SOL/USDT']

// If isHistory prop is true, start on history tab
if (props.isHistory) {
  activeTab.value = 'history'
}

const activeCount = computed(() => {
  return orders.value.filter(
    (o) => o.status === 'pending' || o.status === 'partial_filled',
  ).length
})

// Emit active count to parent
watch(activeCount, (count) => {
  emit('orders-change', count)
})

async function fetchOrders() {
  loading.value = true
  try {
    const isCurrent = activeTab.value === 'current'
    const status: OrderStatus | 'active' | undefined = isCurrent ? 'active' : undefined

    const res = await getOrders({
      status,
      symbol: filterSymbol.value || undefined,
      side: filterSide.value || undefined,
      page: isCurrent ? 1 : page.value,
      size: isCurrent ? 100 : pageSize.value,
    })
    orders.value = res.items
    total.value = res.total
  } catch (err: any) {
    ElMessage.error(err.message || '获取委托列表失败')
  } finally {
    loading.value = false
  }
}

function onTabChange() {
  page.value = 1
  fetchOrders()
}

async function handleCancel(order: Order) {
  try {
    await ElMessageBox.confirm('确认撤销该委托？', '撤单确认', {
      confirmButtonText: '确认',
      cancelButtonText: '取消',
      type: 'warning',
    })
    await cancelOrder(order.order_id)
    ElMessage.success('委托已撤销')
    await fetchOrders()
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err.message || '撤单失败')
    }
  }
}

async function handleCancelAll() {
  try {
    await ElMessageBox.confirm(
      `确认撤销全部 ${activeCount.value} 个未成交委托？`,
      '批量撤单',
      { confirmButtonText: '确认', cancelButtonText: '取消', type: 'warning' },
    )
    const res = await cancelAllOrders({
      symbol: filterSymbol.value || undefined,
      side: filterSide.value || undefined,
    })
    ElMessage.success(`成功撤销 ${res.cancelled_count} 个委托${res.failed_count > 0 ? `，${res.failed_count} 个无法撤销` : ''}`)
    await fetchOrders()
  } catch (err: any) {
    if (err !== 'cancel') {
      ElMessage.error(err.message || '批量撤单失败')
    }
  }
}

function formatTime(dateStr: string): string {
  if (!dateStr) return '--'
  const d = new Date(dateStr)
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

// Poll for updates (every 5s for current, no polling for history)
function startPolling() {
  stopPolling()
  if (activeTab.value === 'current') {
    pollTimer = setInterval(fetchOrders, 5000)
  }
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

onMounted(() => {
  fetchOrders()
  startPolling()
})

onUnmounted(() => {
  stopPolling()
})

// Re-poll on tab change
watch(activeTab, () => {
  startPolling()
})

defineExpose({
  activeTab,
  orders,
  activeCount,
  fetchOrders,
})
</script>

<style scoped lang="scss">
.order-list {
  background: var(--color-surface, #0E1223);
  border: 1px solid var(--color-border, #334155);
  border-radius: 8px;
  padding: 12px;
}

.order-tabs {
  :deep(.el-tabs__header) {
    margin-bottom: 8px;
  }
}

.tab-badge {
  margin-left: 4px;
  :deep(.el-badge__content) {
    font-size: 10px;
  }
}

.filters {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 12px;
}

.order-table {
  width: 100%;
  font-size: 12px;

  :deep(.el-table__header th) {
    font-size: 11px;
    color: var(--color-text-secondary, #94A3B8);
  }
}

.side-buy {
  color: #67C23A;
  font-weight: 600;
}

.side-sell {
  color: #F56C6C;
  font-weight: 600;
}

.pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: 12px;
}
</style>
