<template>
  <div class="trading-view">
    <!-- Trading Header -->
    <div class="trading-header">
      <div class="header-left">
        <span class="mode-badge mode-badge--paper">模拟交易</span>
        <span class="page-title">交易执行</span>
      </div>
      <div class="header-center">
        <span v-if="account" class="balance-info">
          <span class="balance-label">余额</span>
          <span class="balance-value">¥{{ formatMoney(account.balance) }}</span>
          <span class="balance-divider">|</span>
          <span class="balance-label">冻结</span>
          <span class="balance-value balance-value--frozen">¥{{ formatMoney(account.frozen_balance) }}</span>
        </span>
      </div>
      <div class="header-right">
        <div class="ws-status" :class="wsStatusClass">
          <span class="ws-dot" />
          <span class="ws-text">{{ wsStatusText }}</span>
        </div>
        <el-select
          v-model="selectedSymbol"
          size="default"
          class="symbol-select"
          @change="onSymbolChange"
        >
          <el-option
            v-for="s in symbolList"
            :key="s"
            :label="s"
            :value="s"
          />
        </el-select>
      </div>
    </div>

    <!-- Three-column layout -->
    <div class="trading-body">
      <!-- Left: Chart Area -->
      <div class="chart-area">
        <div class="chart-placeholder">
          <el-icon :size="48" color="var(--color-text-tertiary, #64748B)"><TrendCharts /></el-icon>
          <p>K线图区域 (复用行情模块组件)</p>
          <div v-if="bestBid && bestAsk" class="ticker-price">
            <span class="ticker-bid">{{ bestBid }}</span>
            <span class="ticker-sep">/</span>
            <span class="ticker-ask">{{ bestAsk }}</span>
          </div>
        </div>
      </div>

      <!-- Right: Order Panel (Form + Tabbed List) -->
      <div class="order-panel">
        <!-- Order Form -->
        <div class="order-form-wrapper">
          <OrderForm
            :symbol="selectedSymbol"
            :symbol-config="currentSymbolConfig"
            :account="account"
            :best-bid="bestBid"
            :best-ask="bestAsk"
            :available-position-qty="availablePositionQty"
            @submit="onOrderSubmit"
          />
        </div>

        <!-- Tabbed Panel: Orders / Positions / Trades -->
        <div class="tab-panel">
          <el-tabs v-model="activeTab" class="main-tabs" @tab-change="onTabChange">
            <el-tab-pane name="current">
              <template #label>
                <span>当前委托 <el-badge v-if="activeOrderCount > 0" :value="activeOrderCount" class="tab-badge" /></span>
              </template>
            </el-tab-pane>
            <el-tab-pane label="历史委托" name="history" />
            <el-tab-pane label="持仓" name="positions" />
            <el-tab-pane label="成交记录" name="trades" />
          </el-tabs>

          <!-- Current Orders Tab -->
          <div v-show="activeTab === 'current'" class="tab-content">
            <OrderList
              ref="orderListRef"
              :symbol="selectedSymbol"
              @orders-change="onOrdersChange"
            />
          </div>

          <!-- History Orders Tab -->
          <div v-show="activeTab === 'history'" class="tab-content">
            <OrderList
              ref="historyOrderListRef"
              symbol=""
              is-history
            />
          </div>

          <!-- Positions Tab -->
          <div v-show="activeTab === 'positions'" class="tab-content">
            <PositionPanel
              :positions="positions"
              :account="account"
              :current-prices="currentPrices"
              :loading="positionsLoading"
              @close-success="onPositionClose"
            />
          </div>

          <!-- Trades Tab -->
          <div v-show="activeTab === 'trades'" class="tab-content">
            <TradeRecordTab
              :trades="trades"
              :total="tradesTotal"
              :page="tradesPage"
              :size="tradesSize"
              :loading="tradesLoading"
              @filter-change="onTradeFilterChange"
              @page-change="onTradePageChange"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessage } from 'element-plus'
import { TrendCharts } from '@element-plus/icons-vue'
import OrderForm from '@/components/trade/OrderForm.vue'
import OrderList from '@/components/trade/OrderList.vue'
import PositionPanel from '@/components/order/PositionPanel.vue'
import TradeRecordTab from '@/components/order/TradeRecordTab.vue'
import type { SymbolConfig, PaperAccount, CreateOrderRequest, Trade, Position } from '@/types/order'
import { getSymbols, getAccount, getPositions, getTrades } from '@/api/order'

const selectedSymbol = ref('BTC/USDT')
const symbolConfigs = ref<SymbolConfig[]>([])
const account = ref<PaperAccount | null>(null)
const availablePositionQty = ref('0')
const bestBid = ref<string | null>(null)
const bestAsk = ref<string | null>(null)
const activeOrderCount = ref(0)

// Tab state
const activeTab = ref<'current' | 'history' | 'positions' | 'trades'>('current')

// Refs
const orderListRef = ref<InstanceType<typeof OrderList> | null>(null)
const historyOrderListRef = ref<InstanceType<typeof OrderList> | null>(null)

// Positions
const positions = ref<Position[]>([])
const positionsLoading = ref(false)
const currentPrices = ref<Record<string, string>>({})

// Trades
const trades = ref<Trade[]>([])
const tradesTotal = ref(0)
const tradesPage = ref(1)
const tradesSize = ref(20)
const tradesLoading = ref(false)

// WebSocket status
const wsConnected = ref(false)

// Computed
const symbolList = computed(() =>
  symbolConfigs.value.filter((s) => s.enabled).map((s) => s.symbol),
)

const currentSymbolConfig = computed(() =>
  symbolConfigs.value.find((s) => s.symbol === selectedSymbol.value) ?? null,
)

const wsStatusClass = computed(() => ({
  'ws-status--connected': wsConnected.value,
  'ws-status--disconnected': !wsConnected.value,
}))

const wsStatusText = computed(() =>
  wsConnected.value ? '已连接' : '已断开',
)

// Formatters
function formatMoney(value: string | number | undefined | null): string {
  if (!value && value !== 0) return '0.00'
  const num = typeof value === 'string' ? parseFloat(value) : value
  if (isNaN(num)) return '0.00'
  return num.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

// Data loaders
async function loadSymbolConfigs() {
  try {
    const res = await getSymbols()
    symbolConfigs.value = res.items
  } catch {
    // Use defaults
  }
}

async function loadAccount() {
  try {
    account.value = await getAccount()
  } catch {
    // Will show empty balance
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

async function loadTrades(filters?: { symbol: string; side: string; startDate: string; endDate: string }) {
  tradesLoading.value = true
  try {
    const res = await getTrades({
      symbol: filters?.symbol || selectedSymbol.value,
      side: (filters?.side as 'buy' | 'sell') || undefined,
      start_date: filters?.startDate || undefined,
      end_date: filters?.endDate || undefined,
      page: tradesPage.value,
      size: tradesSize.value,
    })
    trades.value = res.items
    tradesTotal.value = res.total
  } catch {
    trades.value = []
    tradesTotal.value = 0
  } finally {
    tradesLoading.value = false
  }
}

// Event handlers
function onSymbolChange() {
  bestBid.value = null
  bestAsk.value = null
  loadPositions()
  if (activeTab.value === 'trades') {
    loadTrades()
  }
}

function onOrderSubmit(_order: CreateOrderRequest) {
  // Refresh order list and account after submission
  orderListRef.value?.fetchOrders()
  loadAccount()
  loadPositions()
}

function onTabChange(tab: string) {
  if (tab === 'positions') {
    loadPositions()
  } else if (tab === 'trades') {
    loadTrades()
  }
}

function onOrdersChange(count: number) {
  activeOrderCount.value = count
}

function onPositionClose() {
  loadPositions()
  loadAccount()
}

function onTradeFilterChange(filters: { symbol: string; side: string; startDate: string; endDate: string }) {
  tradesPage.value = 1
  loadTrades(filters)
}

function onTradePageChange(page: number, size: number) {
  tradesPage.value = page
  tradesSize.value = size
  loadTrades()
}

// WebSocket simulation (in real app, connect to ws://localhost:8080/ws/trade)
let wsTimer: ReturnType<typeof setInterval> | null = null

function connectWebSocket() {
  // Simulate WS connection status
  wsConnected.value = true
  // Simulate price updates every 5s
  wsTimer = setInterval(() => {
    if (selectedSymbol.value === 'BTC/USDT') {
      bestBid.value = (49500 + Math.random() * 100).toFixed(2)
      bestAsk.value = (49510 + Math.random() * 100).toFixed(2)
    }
  }, 5000)
}

function disconnectWebSocket() {
  if (wsTimer) {
    clearInterval(wsTimer)
    wsTimer = null
  }
  wsConnected.value = false
}

onMounted(async () => {
  await Promise.all([loadSymbolConfigs(), loadAccount(), loadPositions(), loadTrades()])
  connectWebSocket()
})

onUnmounted(() => {
  disconnectWebSocket()
})
</script>

<style scoped lang="scss">
// Design tokens (from Design_TradingExecution.md)
$color-bg: #08090a;
$color-surface: #191a1b;
$color-surface-elevated: #212223;
$color-text-primary: #f7f8f8;
$color-text-secondary: #d0d6e0;
$color-text-tertiary: #8a8f98;
$color-accent: #7170ff;
$color-border: rgba(255, 255, 255, 0.08);
$color-buy: #67C23A;
$color-sell: #F56C6C;
$color-frozen: #f5a623;
$color-paper-mode: #60a5fa;
$color-live-mode: #e5484d;

.trading-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 0;
}

.trading-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 16px;
  height: 56px;
  background: $color-bg;
  border: 1px solid $color-border;
  border-radius: 8px 8px 0 0;
  border-bottom: none;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.mode-badge {
  display: inline-flex;
  align-items: center;
  padding: 0 8px;
  height: 24px;
  border-radius: 16px;
  font-size: 12px;
  font-weight: 500;

  &--paper {
    background: rgba(96, 165, 250, 0.12);
    color: $color-paper-mode;
  }

  &--live {
    background: rgba(229, 72, 77, 0.12);
    color: $color-live-mode;
  }
}

.page-title {
  font-size: 18px;
  font-weight: 600;
  color: $color-text-primary;
}

.header-center {
  display: flex;
  align-items: center;
  gap: 8px;
}

.balance-info {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-size: 14px;
}

.balance-label {
  color: $color-text-tertiary;
}

.balance-value {
  color: $color-text-primary;

  &--frozen {
    color: $color-frozen;
  }
}

.balance-divider {
  color: $color-text-tertiary;
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
  font-size: 12px;

  .ws-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .ws-text {
    font-size: 12px;
  }

  &--connected {
    .ws-dot {
      background: $color-buy;
    }
    .ws-text {
      color: $color-buy;
    }
  }

  &--disconnected {
    .ws-dot {
      background: $color-text-tertiary;
    }
    .ws-text {
      color: $color-text-tertiary;
    }
  }
}

.symbol-select {
  width: 160px;
}

.trading-body {
  flex: 1;
  display: flex;
  gap: 0;
  min-height: 0;
  border: 1px solid $color-border;
  border-top: none;
  border-radius: 0 0 8px 8px;
  overflow: hidden;
}

.chart-area {
  flex: 2;
  min-width: 400px;
  background: $color-bg;
  border-right: 1px solid $color-border;
}

.chart-placeholder {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: $color-text-tertiary;
  font-size: 14px;
  gap: 8px;
}

.ticker-price {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 16px;
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-size: 16px;
}

.ticker-bid {
  color: $color-buy;
  font-weight: 600;
}

.ticker-ask {
  color: $color-sell;
  font-weight: 600;
}

.ticker-sep {
  color: $color-text-tertiary;
}

.order-panel {
  width: 380px;
  min-width: 360px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-left: 1px solid $color-border;
  overflow: hidden;
}

.order-form-wrapper {
  border-bottom: 1px solid $color-border;
  overflow-y: auto;
}

.tab-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: $color-surface;
}

.main-tabs {
  height: 40px;
  flex-shrink: 0;

  :deep(.el-tabs__header) {
    margin: 0;
    padding: 0 12px;
    height: 40px;
    border-bottom: 1px solid $color-border;
  }

  :deep(.el-tabs__nav-wrap) {
    &::after {
      display: none;
    }
  }

  :deep(.el-tabs__item) {
    padding: 0 16px;
    height: 40px;
    line-height: 40px;
    font-size: 13px;
    color: $color-text-secondary;

    &.is-active {
      color: $color-text-primary;
    }

    &:hover {
      color: $color-text-primary;
    }
  }

  :deep(.el-tabs__nav) {
    height: 40px;
  }

  :deep(.el-tabs__active-bar) {
    background-color: $color-accent;
    height: 2px;
  }
}

.tab-badge {
  margin-left: 4px;

  :deep(.el-badge__content) {
    font-size: 10px;
  }
}

.tab-content {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
}

// Responsive
@media (max-width: 1200px) {
  .order-panel {
    width: 320px;
    min-width: 300px;
  }
}

@media (max-width: 900px) {
  .trading-body {
    flex-direction: column;
  }

  .chart-area {
    flex: none;
    height: 300px;
    min-width: unset;
    border-right: none;
    border-bottom: 1px solid $color-border;
  }

  .order-panel {
    width: 100%;
    min-width: unset;
    height: 400px;
    border-left: none;
  }
}

@media (max-width: 600px) {
  .trading-header {
    flex-wrap: wrap;
    height: auto;
    padding: 12px;
    gap: 8px;
  }

  .header-center {
    order: 3;
    width: 100%;
    justify-content: center;
  }

  .order-panel {
    height: 500px;
  }
}
</style>
