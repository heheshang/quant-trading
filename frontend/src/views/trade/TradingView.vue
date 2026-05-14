<template>
  <div class="trading-view">
    <!-- Trading Header -->
    <div class="trading-header">
      <div class="header-left">
        <el-tag type="primary" size="small" effect="dark">模拟交易</el-tag>
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
      <div v-if="bestBid && bestAsk" class="header-right">
        <span class="price-info">买一: <span class="price-bid">{{ bestBid }}</span></span>
        <span class="price-divider">/</span>
        <span class="price-info">卖一: <span class="price-ask">{{ bestAsk }}</span></span>
      </div>
    </div>

    <!-- Three-column layout -->
    <div class="trading-body">
      <!-- Left: Chart Area (placeholder) -->
      <div class="chart-area">
        <div class="chart-placeholder">
          <el-icon :size="48" color="var(--color-text-tertiary, #64748B)"><TrendCharts /></el-icon>
          <p>K线图区域 (复用行情模块组件)</p>
        </div>
      </div>

      <!-- Center: Order Form -->
      <div class="order-area">
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

      <!-- Right: Order List -->
      <div class="list-area">
        <OrderList
          ref="orderListRef"
          :symbol="selectedSymbol"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { TrendCharts } from '@element-plus/icons-vue'
import OrderForm from '@/components/trade/OrderForm.vue'
import OrderList from '@/components/trade/OrderList.vue'
import type { SymbolConfig, PaperAccount, CreateOrderRequest } from '@/types/order'
import { getSymbols, getAccount, getPositions } from '@/api/order'

const selectedSymbol = ref('BTC/USDT')
const symbolConfigs = ref<SymbolConfig[]>([])
const account = ref<PaperAccount | null>(null)
const availablePositionQty = ref('0')
const bestBid = ref<string | null>(null)
const bestAsk = ref<string | null>(null)
const orderListRef = ref<InstanceType<typeof OrderList> | null>(null)

const symbolList = computed(() =>
  symbolConfigs.value.filter((s) => s.enabled).map((s) => s.symbol),
)

const currentSymbolConfig = computed(() =>
  symbolConfigs.value.find((s) => s.symbol === selectedSymbol.value) ?? null,
)

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

async function loadPosition() {
  try {
    const positions = await getPositions()
    const pos = positions.find(
      (p) => p.symbol === selectedSymbol.value && p.mode === 'paper',
    )
    availablePositionQty.value = pos?.available_quantity ?? '0'
  } catch {
    availablePositionQty.value = '0'
  }
}

function onSymbolChange() {
  bestBid.value = null
  bestAsk.value = null
  loadPosition()
}

function onOrderSubmit(_order: CreateOrderRequest) {
  // Refresh order list and account after submission
  orderListRef.value?.fetchOrders()
  loadAccount()
  loadPosition()
}

onMounted(async () => {
  await Promise.all([loadSymbolConfigs(), loadAccount(), loadPosition()])
})
</script>

<style scoped lang="scss">
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
  padding: 12px 16px;
  background: var(--color-surface, #0E1223);
  border: 1px solid var(--color-border, #334155);
  border-radius: 8px 8px 0 0;
  border-bottom: none;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.symbol-select {
  width: 160px;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.price-info {
  font-size: 13px;
  color: var(--color-text-secondary, #94A3B8);
}

.price-bid {
  color: #67C23A;
  font-weight: 600;
}

.price-ask {
  color: #F56C6C;
  font-weight: 600;
}

.price-divider {
  color: var(--color-text-tertiary, #64748B);
}

.trading-body {
  flex: 1;
  display: flex;
  gap: 0;
  min-height: 0;
  border: 1px solid var(--color-border, #334155);
  border-top: none;
  border-radius: 0 0 8px 8px;
  overflow: hidden;
}

.chart-area {
  flex: 1;
  min-width: 300px;
  background: var(--color-surface, #0E1223);
  border-right: 1px solid var(--color-border, #334155);
}

.chart-placeholder {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--color-text-tertiary, #64748B);
  font-size: 14px;
  gap: 8px;
}

.order-area {
  width: 360px;
  min-width: 300px;
  flex-shrink: 0;
  border-right: 1px solid var(--color-border, #334155);
  overflow-y: auto;
}

.list-area {
  flex: 1;
  min-width: 300px;
  overflow-y: auto;
}
</style>
