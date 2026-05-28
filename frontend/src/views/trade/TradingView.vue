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
        <KlineChart
          v-if="klineData.length > 0"
          :data="klineData"
          :symbol="selectedSymbol"
          :interval="chartInterval"
          :dark-mode="true"
        />
        <div v-else class="chart-placeholder">
          <el-icon :size="48" color="var(--color-text-tertiary, #64748B)"><TrendCharts /></el-icon>
          <p>加载K线数据中...</p>
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
import KlineChart from '@/components/charts/KlineChart.vue'
import type { SymbolConfig, PaperAccount, CreateOrderRequest, Trade, Position } from '@/types/order'
import type { KlineBar } from '@/components/charts/KlineChart.vue'
import { getSymbols, getAccount, getPositions, getTrades } from '@/api/order'
import { queryKlines } from '@/api/kline'
import { useTradingStore } from '@/stores/trading'

const selectedSymbol = ref('BTC/USDT')
const symbolConfigs = ref<SymbolConfig[]>([])
const account = ref<PaperAccount | null>(null)
const availablePositionQty = ref('0')
const bestBid = ref<string | null>(null)
const bestAsk = ref<string | null>(null)
const activeOrderCount = ref(0)

// Kline chart state
const klineData = ref<KlineBar[]>([])
const chartInterval = ref('1h')

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
const tradingStore = useTradingStore()

// Computed
const symbolList = computed(() =>
  symbolConfigs.value.filter((s) => s.enabled).map((s) => s.symbol),
)

const currentSymbolConfig = computed(() =>
  symbolConfigs.value.find((s) => s.symbol === selectedSymbol.value) ?? null,
)

const wsStatusClass = computed(() => ({
  'ws-status--connected': tradingStore.isConnected,
  'ws-status--disconnected': !tradingStore.isConnected,
}))

const wsStatusText = computed(() =>
  tradingStore.isConnected ? '已连接' : '已断开',
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
    symbolConfigs.value = (res as any)?.items ?? res ?? []
  } catch {
    // Use defaults
  }
}

async function loadKlineData() {
  console.log('[TradingView] loadKlineData called, selectedSymbol:', selectedSymbol.value)
  try {
    // Map selectedSymbol like "BTC/USDT" to "btcusdt" for the backend
    const symbol = selectedSymbol.value.replace('/', '').toLowerCase()
    console.log('[TradingView] queryKlines request:', { symbol, interval: chartInterval.value, page_size: 200 })
    const res = await queryKlines({
      symbol,
      interval: chartInterval.value,
      size: 200,
    })
    console.log('[TradingView] queryKlines response:', res)
    const r = res as any
    // API returns: { code: 0, data: { data: [...], meta: {...} }, message: "success" }
    // Need to extract r.data.data for the actual kline array
    const rawData = r?.data?.data ?? r?.data ?? r ?? []
    console.log('[TradingView] rawData length:', rawData.length)
    klineData.value = rawData.map((b: any) => ({
      time: Math.floor((b.timestamp ?? b.time ?? b.open_time) / 1000),
      open: parseFloat(b.open),
      high: parseFloat(b.high),
      low: parseFloat(b.low),
      close: parseFloat(b.close),
      volume: parseFloat(b.volume ?? 0),
    }))
    console.log('[TradingView] klineData updated, count:', klineData.value.length)
  } catch (e) {
    console.error('[TradingView] loadKlineData error:', e)
    klineData.value = []
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
  tradingStore.switchSymbol(selectedSymbol.value)
  loadPositions()
  loadKlineData()
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

function onPositionClose(_symbol?: string) {
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

// Real WebSocket lifecycle
function connectWebSocket() {
  tradingStore.connect()
}

function disconnectWebSocket() {
  tradingStore.disconnect()
}

// Watch for ticker updates from store
function syncTickerFromStore() {
  const ticker = tradingStore.currentTicker
  if (ticker) {
    bestBid.value = ticker.bid
    bestAsk.value = ticker.ask
  }
}

onMounted(async () => {
  await Promise.all([loadSymbolConfigs(), loadAccount(), loadPositions(), loadTrades(), loadKlineData()])
  connectWebSocket()
  // Sync ticker when store updates
  tradingStore.$subscribe(() => {
    syncTickerFromStore()
  })
  // Listen for trade execution events to refresh order list
  window.addEventListener('trade-executed', onTradeExecuted)
})

onUnmounted(() => {
  disconnectWebSocket()
  window.removeEventListener('trade-executed', onTradeExecuted)
})

function onTradeExecuted() {
  orderListRef.value?.fetchOrders()
  loadAccount()
  loadPositions()
}
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
$color-warning: #f5a623;

// Additional variables for new styles
$color-deep-bg: #08090a;
$color-border-hover: rgba(255, 255, 255, 0.15);

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes pulseSubtle {
  0% { box-shadow: 0 0 0 0 rgba(16, 185, 129, 0.4); }
  70% { box-shadow: 0 0 0 6px rgba(16, 185, 129, 0); }
  100% { box-shadow: 0 0 0 0 rgba(16, 185, 129, 0); }
}

:root {
  --header-height: 60px;
  --transition-base: 0.3s ease;
  --transition-fast: 0.2s ease;
  --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
  --gradient-accent: linear-gradient(135deg, #7170ff 0%, #a5a4ff 100%);
  --color-bg: #{$color-bg};
  --color-surface: #{$color-surface};
  --color-surface-elevated: #{$color-surface-elevated};
  --color-deep-bg: #{$color-deep-bg};
  --color-border: #{$color-border};
  --color-border-hover: #{$color-border-hover};
  --color-text-primary: #{$color-text-primary};
  --color-text-secondary: #{$color-text-secondary};
  --color-text-tertiary: #{$color-text-tertiary};
  --color-accent: #{$color-accent};
  --color-buy: #{$color-buy};
  --color-sell: #{$color-sell};
  --color-warning: #{$color-warning};
}

.trading-view {
  display: flex;
  flex-direction: column;
  height: calc(100vh - var(--header-height));
  background: var(--color-bg);
  animation: fadeIn var(--transition-base);
}

.trading-header {
  height: 56px;
  background: rgba(25, 26, 27, 0.95);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-bottom: 1px solid $color-border;
  padding: 0 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
  box-shadow: var(--shadow-sm);

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-center {
    display: flex;
    align-items: center;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }
}

.mode-badge {
  padding: 4px 12px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.5px;
  text-transform: uppercase;

  &--paper {
    background: var(--gradient-accent);
    color: #fff;
    box-shadow: 0 0 12px rgba(113, 112, 255, 0.3);
  }

  &--live {
     background: rgba(229, 72, 77, 0.12);
     color: $color-live-mode;
  }
}

.page-title {
  font-size: 16px;
  font-weight: 600;
  color: $color-text-primary;
  letter-spacing: -0.3px;
}

.balance-info {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  padding: 6px 12px;
  background: rgba(255, 255, 255, 0.03);
  border-radius: 8px;
  border: 1px solid $color-border;
}

.balance-label {
  color: $color-text-tertiary;
  font-weight: 500;
}

.balance-value {
  color: $color-text-primary;
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-weight: 700;
  letter-spacing: -0.3px;

  &--frozen {
    color: $color-warning;
  }
}

.balance-divider {
  color: $color-border;
  margin: 0 4px;
}

.ws-status {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid $color-border;
  transition: all var(--transition-fast);

  &--connected {
    .ws-dot {
      background: $color-buy;
      box-shadow: 0 0 8px rgba(16, 185, 129, 0.6);
      animation: pulseSubtle 2s ease-in-out infinite;
    }
    .ws-text {
      color: $color-buy;
      font-weight: 600;
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

.ws-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  transition: all var(--transition-fast);
}

.ws-text {
  font-size: 12px;
  font-weight: 500;
  transition: color var(--transition-fast);
}

.symbol-select {
  width: 160px;

  :deep(.el-input__wrapper) {
    background: var(--color-surface-elevated) !important;
    border-radius: 8px;
    transition: all var(--transition-fast);

    &:hover {
      box-shadow: 0 0 0 1px var(--color-border-hover) inset !important;
    }
  }
}

.trading-body {
  flex: 1;
  display: flex;
  gap: 0;
  min-height: 0;
  border: 1px solid $color-border;
  border-top: none;
  border-radius: 0 0 12px 12px;
  overflow: hidden;
  box-shadow: var(--shadow-md);
}

.chart-area {
  flex: 2;
  min-width: 400px;
  background: var(--color-deep-bg);
  border-right: 1px solid $color-border;
  position: relative;

  &::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background:
      linear-gradient(rgba(255, 255, 255, 0.02) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255, 255, 255, 0.02) 1px, transparent 1px);
    background-size: 50px 50px;
    pointer-events: none;
  }
}

.chart-placeholder {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: $color-text-tertiary;
  font-size: 14px;
  gap: 12px;
  animation: fadeIn var(--transition-base);
}

.ticker-price {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 20px;
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-size: 18px;
  padding: 8px 16px;
  background: rgba(255, 255, 255, 0.03);
  border-radius: 8px;
  border: 1px solid $color-border;
}

.ticker-bid {
  color: $color-buy;
  font-weight: 700;
  text-shadow: 0 0 8px rgba(16, 185, 129, 0.4);
}

.ticker-ask {
  color: $color-sell;
  font-weight: 700;
  text-shadow: 0 0 8px rgba(229, 72, 77, 0.4);
}

.ticker-sep {
  color: $color-text-tertiary;
  font-weight: 600;
}

.order-panel {
  width: 380px;
  min-width: 360px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-left: 1px solid $color-border;
  overflow: hidden;
  background: var(--color-surface);
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
  background: var(--color-surface);
}

.main-tabs {
  height: 42px;
  flex-shrink: 0;

  :deep(.el-tabs__header) {
    margin: 0;
    padding: 0 12px;
    height: 42px;
    border-bottom: 1px solid $color-border;
    background: var(--color-surface-elevated);
  }

  :deep(.el-tabs__nav-wrap) {
    &::after {
      display: none;
    }
  }

  :deep(.el-tabs__item) {
    padding: 0 16px;
    height: 42px;
    line-height: 42px;
    font-size: 13px;
    font-weight: 500;
    color: $color-text-secondary;
    transition: all var(--transition-fast);

    &.is-active {
      color: $color-accent;
      font-weight: 600;
    }

    &:hover {
      color: $color-text-primary;
      background: rgba(255, 255, 255, 0.03);
    }
  }

  :deep(.el-tabs__nav) {
    height: 42px;
  }

  :deep(.el-tabs__active-bar) {
    background: var(--gradient-accent);
    height: 3px;
    border-radius: 3px 3px 0 0;
  }
}

.tab-badge {
  margin-left: 4px;

  :deep(.el-badge__content) {
    font-size: 10px;
    font-weight: 700;
  }
}

.tab-content {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  animation: fadeIn var(--transition-base);
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
