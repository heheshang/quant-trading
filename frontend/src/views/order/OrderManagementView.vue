<template>
  <div class="order-management-view">
    <!-- Page Header -->
    <div class="page-header">
      <div class="header-left">
        <el-tag
          :type="tradeMode === 'paper' ? 'primary' : 'danger'"
          size="small"
          effect="dark"
          class="mode-badge"
        >
          {{ tradeMode === 'paper' ? '🧪 模拟交易' : '🔴 实盘交易' }}
        </el-tag>
        <h1 class="page-title">订单管理</h1>
      </div>
      <div class="header-right">
        <div class="ws-status">
          <span class="ws-dot" :class="wsStatusClass" />
          <span class="ws-text">{{ wsStatusText }}</span>
        </div>
        <el-button type="primary" :icon="Plus" @click="createDialogVisible = true">
          新建委托
        </el-button>
      </div>
    </div>

    <!-- Summary Cards -->
    <OrderSummaryCards
      :account="account"
      :active-orders-count="activeOrdersCount"
      :today-fills-count="todayFillsCount"
      :today-fill-amount="todayFillAmount"
      :margin-frozen="marginFrozen"
      :loading="summaryLoading"
    />

    <!-- Tabs -->
    <div class="tab-bar">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="tab-btn"
        :class="{ 'tab-btn--active': activeTab === tab.key }"
        @click="onTabChange(tab.key)"
      >
        {{ tab.label }}
        <span v-if="tab.count > 0" class="tab-count">{{ tab.count }}</span>
      </button>
    </div>

    <!-- Tab Content: Orders -->
    <template v-if="activeTab === 'orders'">
      <OrderFilterBar
        :has-active-orders="activeOrdersCount > 0"
        @filter-change="onOrderFilterChange"
        @cancel-all-success="onCancelAllSuccess"
      />
      <OrderTable
        :orders="orders"
        :total="orderTotal"
        :page="orderPage"
        :size="orderSize"
        :loading="ordersLoading"
        @row-click="onOrderRowClick"
        @cancel-success="onCancelSuccess"
        @page-change="onOrderPageChange"
      />
    </template>

    <!-- Tab Content: Positions -->
    <template v-if="activeTab === 'positions'">
      <PositionPanel
        :positions="positions"
        :account="account"
        :current-prices="currentPrices"
        :loading="positionsLoading"
        @close-success="onClosePositionSuccess"
        @filter-change="onPositionFilterChange"
      />
    </template>

    <!-- Tab Content: Trades -->
    <template v-if="activeTab === 'trades'">
      <TradeRecordTab
        :trades="trades"
        :total="tradeTotal"
        :page="tradePage"
        :size="tradeSize"
        :loading="tradesLoading"
        @row-click="onTradeRowClick"
        @filter-change="onTradeFilterChange"
        @page-change="onTradePageChange"
      />
    </template>

    <!-- Order Create Dialog -->
    <OrderCreateDialog
      v-model="createDialogVisible"
      :symbol-configs="symbolConfigs"
      :account="account"
      :available-position-qty="availablePositionQty"
      :best-bid="bestBid"
      :best-ask="bestAsk"
      :mode="tradeMode"
      @order-created="onOrderCreated"
    />

    <!-- Order Detail Drawer -->
    <OrderDetailDrawer
      v-model="detailDrawerVisible"
      :order="selectedOrder"
      :fills="selectedOrderFills"
      :loading="detailLoading"
      @cancel-success="onCancelSuccess"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import OrderSummaryCards from '@/components/order/OrderSummaryCards.vue'
import OrderFilterBar from '@/components/order/OrderFilterBar.vue'
import OrderTable from '@/components/order/OrderTable.vue'
import OrderCreateDialog from '@/components/order/OrderCreateDialog.vue'
import OrderDetailDrawer from '@/components/order/OrderDetailDrawer.vue'
import PositionPanel from '@/components/order/PositionPanel.vue'
import TradeRecordTab from '@/components/order/TradeRecordTab.vue'
import {
  getOrders,
  getOrder,
  getAccount,
  getSymbols,
  getPositions,
  getTrades,
} from '@/api/order'
import type {
  Order,
  Trade,
  Position,
  PaperAccount,
  SymbolConfig,
  TradeMode,
} from '@/types/order'

/**
 * OrderManagementView - Container view for the order management module.
 * Contains three tabs: Orders, Positions, and Trade Records.
 * Follows Design_OrderManagement.md specs.
 */

// ===== State =====
const tradeMode = ref<TradeMode>('paper')
const activeTab = ref<'orders' | 'positions' | 'trades'>('orders')
const wsConnected = ref(false)
const wsConnecting = ref(false)

// Account
const account = ref<PaperAccount | null>(null)
const summaryLoading = ref(true)

// Orders
const orders = ref<Order[]>([])
const orderTotal = ref(0)
const orderPage = ref(1)
const orderSize = ref(20)
const ordersLoading = ref(true)
const orderFilters = ref({
  status: [] as string[],
  side: '',
  orderType: '',
  symbol: '',
  startDate: '',
  endDate: '',
})

// Positions
const positions = ref<Position[]>([])
const positionsLoading = ref(false)
const currentPrices = ref<Record<string, string>>({})

// Trades
const trades = ref<Trade[]>([])
const tradeTotal = ref(0)
const tradePage = ref(1)
const tradeSize = ref(20)
const tradesLoading = ref(false)

// Symbol configs
const symbolConfigs = ref<SymbolConfig[]>([])

// Dialogs
const createDialogVisible = ref(false)
const detailDrawerVisible = ref(false)
const selectedOrder = ref<Order | null>(null)
const selectedOrderFills = ref<Trade[]>([])
const detailLoading = ref(false)

// Market data
const bestBid = ref<string | null>(null)
const bestAsk = ref<string | null>(null)
const availablePositionQty = ref('0')

// ===== Computed =====
const activeOrdersCount = computed(() =>
  orders.value.filter((o) => o.status === 'pending' || o.status === 'partial_filled').length,
)

const todayFillsCount = computed(() => {
  const today = new Date().toDateString()
  return trades.value.filter((t) => new Date(t.created_at).toDateString() === today).length
})

const todayFillAmount = computed(() => {
  const today = new Date().toDateString()
  const todayTrades = trades.value.filter((t) => new Date(t.created_at).toDateString() === today)
  const total = todayTrades.reduce((sum, t) => sum + parseFloat(t.price) * parseFloat(t.quantity), 0)
  return total.toFixed(2)
})

const marginFrozen = computed(() => account.value?.frozen_balance ?? '')

const wsStatusClass = computed(() => {
  if (wsConnecting.value) return 'ws-dot--connecting'
  if (wsConnected.value) return 'ws-dot--connected'
  return 'ws-dot--disconnected'
})

const wsStatusText = computed(() => {
  if (wsConnecting.value) return '重连中'
  if (wsConnected.value) return '已连接'
  return '已断开'
})

const tabs = computed(() => [
  { key: 'orders' as const, label: '委托管理', count: orderTotal.value },
  { key: 'positions' as const, label: '持仓管理', count: positions.value.length },
  { key: 'trades' as const, label: '成交记录', count: tradeTotal.value },
])

// ===== Data Loading =====

async function loadAccount() {
  try {
    account.value = await getAccount()
  } catch {
    // Will show empty state
  } finally {
    summaryLoading.value = false
  }
}

async function loadSymbolConfigs() {
  try {
    const res = await getSymbols()
    symbolConfigs.value = res.items
  } catch {
    // Use defaults
  }
}

async function loadOrders() {
  ordersLoading.value = true
  try {
    const params: Record<string, unknown> = {
      page: orderPage.value,
      size: orderSize.value,
    }
    if (orderFilters.value.status.length > 0) {
      params.status = orderFilters.value.status[0]
    }
    if (orderFilters.value.side) {
      params.side = orderFilters.value.side
    }
    if (orderFilters.value.symbol) {
      params.symbol = orderFilters.value.symbol
    }
    if (orderFilters.value.startDate) {
      params.start_date = orderFilters.value.startDate
    }
    if (orderFilters.value.endDate) {
      params.end_date = orderFilters.value.endDate
    }
    const res = await getOrders(params)
    orders.value = res.items
    orderTotal.value = res.total
  } catch {
    orders.value = []
    orderTotal.value = 0
  } finally {
    ordersLoading.value = false
  }
}

async function loadPositions() {
  positionsLoading.value = true
  try {
    positions.value = await getPositions()
  } catch {
    positions.value = []
  } finally {
    positionsLoading.value = false
  }
}

async function loadTrades() {
  tradesLoading.value = true
  try {
    const params: Record<string, unknown> = {
      page: tradePage.value,
      size: tradeSize.value,
    }
    const res = await getTrades(params)
    trades.value = res.items
    tradeTotal.value = res.total
  } catch {
    trades.value = []
    tradeTotal.value = 0
  } finally {
    tradesLoading.value = false
  }
}

async function loadPositionQty() {
  try {
    const posList = await getPositions()
    // Sum up available quantity for all positions
    const totalAvailable = posList.reduce(
      (sum, p) => sum + parseFloat(p.available_quantity), 0,
    )
    availablePositionQty.value = totalAvailable.toFixed(8)
  } catch {
    availablePositionQty.value = '0'
  }
}

// ===== Event Handlers =====

function onTabChange(tab: 'orders' | 'positions' | 'trades') {
  activeTab.value = tab
  if (tab === 'positions' && positions.value.length === 0) {
    loadPositions()
  }
  if (tab === 'trades' && trades.value.length === 0) {
    loadTrades()
  }
}

function onOrderFilterChange(filters: {
  status: string[]
  side: string
  orderType: string
  symbol: string
  startDate: string
  endDate: string
}) {
  orderFilters.value = filters
  orderPage.value = 1
  loadOrders()
}

function onOrderPageChange(page: number, size: number) {
  orderPage.value = page
  orderSize.value = size
  loadOrders()
}

async function onOrderRowClick(order: Order) {
  selectedOrder.value = order
  selectedOrderFills.value = []
  detailDrawerVisible.value = true
  detailLoading.value = true
  try {
    const fullOrder = await getOrder(order.order_id)
    selectedOrder.value = fullOrder
    // Get fills for this order
    const tradeRes = await getTrades({ page: 1, size: 50 })
    selectedOrderFills.value = tradeRes.items.filter((t) => t.order_id === order.order_id)
  } catch {
    // Use the order data we already have
  } finally {
    detailLoading.value = false
  }
}

function onCancelSuccess(_orderId: string) {
  loadOrders()
  loadAccount()
}

function onCancelAllSuccess(_count: number) {
  loadOrders()
  loadAccount()
}

function onOrderCreated() {
  loadOrders()
  loadAccount()
  loadPositionQty()
}

function onClosePositionSuccess(_positionId: number) {
  loadPositions()
  loadAccount()
}

function onPositionFilterChange(_filters: { side: string; symbol: string }) {
  // Client-side filtering handled in PositionPanel
}

function onTradeRowClick(trade: Trade) {
  // Find the associated order and open detail drawer
  const order = orders.value.find((o) => o.order_id === trade.order_id)
  if (order) {
    onOrderRowClick(order)
  }
}

function onTradeFilterChange(_filters: { symbol: string; side: string; startDate: string; endDate: string }) {
  tradePage.value = 1
  loadTrades()
}

function onTradePageChange(page: number, size: number) {
  tradePage.value = page
  tradeSize.value = size
  loadTrades()
}

// ===== Lifecycle =====
let refreshTimer: ReturnType<typeof setInterval> | null = null

onMounted(async () => {
  await Promise.all([
    loadAccount(),
    loadSymbolConfigs(),
    loadOrders(),
    loadPositionQty(),
  ])

  // Set up periodic refresh (30s)
  refreshTimer = setInterval(() => {
    if (activeTab.value === 'orders') loadOrders()
    else if (activeTab.value === 'positions') loadPositions()
    else if (activeTab.value === 'trades') loadTrades()
    loadAccount()
  }, 30000)
})

onUnmounted(() => {
  if (refreshTimer) {
    clearInterval(refreshTimer)
    refreshTimer = null
  }
})
</script>

<style scoped lang="scss">
.order-management-view {
  display: flex;
  flex-direction: column;
  gap: 0;
  height: 100%;
  padding: 0 16px 16px;
  overflow-y: auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 56px;
  padding: 0;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
  margin-bottom: 16px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.mode-badge {
  font-size: 12px;
  height: 24px;
  padding: 0 8px;
  border-radius: 12px;
}

.page-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text-primary, #f7f8f8);
  margin: 0;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}

.ws-status {
  display: flex;
  align-items: center;
  gap: 6px;
}

.ws-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;

  &--connected {
    background: var(--color-success, #10b981);
  }

  &--connecting {
    background: var(--color-warning, #f5a623);
    animation: blink 0.5s ease infinite;
  }

  &--disconnected {
    background: var(--color-text-tertiary, #8a8f98);
  }
}

@keyframes blink {
  50% { opacity: 0.3; }
}

.ws-text {
  font-size: 12px;
  color: var(--color-text-tertiary, #8a8f98);
}

// Tab bar
.tab-bar {
  display: flex;
  gap: 24px;
  padding: 0 0 0 0;
  margin-bottom: 12px;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
}

.tab-btn {
  position: relative;
  padding: 10px 0;
  border: none;
  background: transparent;
  color: var(--color-text-secondary, #d0d6e0);
  font-size: 14px;
  cursor: pointer;
  transition: color 0.2s;

  &--active {
    color: var(--color-text-primary, #f7f8f8);

    &::after {
      content: '';
      position: absolute;
      bottom: -1px;
      left: 0;
      right: 0;
      height: 2px;
      background: var(--color-accent, #7170ff);
      border-radius: 1px;
    }
  }

  &:hover:not(.tab-btn--active) {
    color: var(--color-text-primary, #f7f8f8);
  }
}

.tab-count {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 9px;
  background: rgba(255, 255, 255, 0.08);
  color: var(--color-text-tertiary, #8a8f98);
  font-size: 11px;
  margin-left: 6px;
}
</style>
