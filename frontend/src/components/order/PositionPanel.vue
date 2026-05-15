<template>
  <div class="position-panel">
    <!-- Position Summary Bar -->
    <div v-if="account" class="position-summary">
      <div class="summary-item">
        <span class="summary-label">可用余额</span>
        <span class="summary-value mono">{{ formatMoney(account.balance) }}</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">冻结</span>
        <span class="summary-value mono summary-value--frozen">{{ formatMoney(account.frozen_balance) }}</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">权益</span>
        <span class="summary-value mono">{{ formatMoney(account.equity) }}</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">初始资金</span>
        <span class="summary-value mono summary-value--secondary">{{ formatMoney(account.initial_balance) }}</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">累计盈亏</span>
        <span class="summary-value mono" :class="pnlClass(account.total_pnl)">
          {{ formatPnl(account.total_pnl) }}
        </span>
      </div>
      <div class="summary-item">
        <span class="summary-label">持仓数</span>
        <span class="summary-value summary-value--secondary">{{ account.positions_count }}</span>
      </div>
      <div class="summary-item">
        <span class="summary-label">活跃委托</span>
        <span class="summary-value summary-value--secondary">{{ account.active_orders_count }}</span>
      </div>
    </div>

    <!-- Filter Bar -->
    <div class="position-filter">
      <el-select
        v-model="filterSide"
        placeholder="方向"
        :style="{ width: '120px' }"
        size="default"
        clearable
        @change="onFilterChange"
      >
        <el-option label="全部" value="" />
        <el-option label="多头" value="long" />
        <el-option label="空头" value="short" />
      </el-select>
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
    </div>

    <!-- Positions Table -->
    <div class="positions-table-wrapper">
      <el-table
        v-if="!loading"
        :data="filteredPositions"
        class="positions-table"
        :empty-text="'暂无持仓'"
      >
        <el-table-column label="交易对" width="100" prop="symbol">
          <template #default="{ row }">
            <span class="symbol-text">{{ row.symbol }}</span>
          </template>
        </el-table-column>

        <el-table-column label="方向" width="60" align="center" prop="side">
          <template #default="{ row }">
            <el-tag
              :type="row.side === 'long' ? 'success' : 'danger'"
              size="small"
              effect="dark"
            >
              {{ row.side === 'long' ? '多' : '空' }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column label="持仓数量" width="100" align="right" prop="quantity">
          <template #default="{ row }">
            <span class="mono">{{ row.quantity }}</span>
          </template>
        </el-table-column>

        <el-table-column label="可用数量" width="100" align="right" prop="available_quantity">
          <template #default="{ row }">
            <span class="mono secondary">{{ row.available_quantity }}</span>
          </template>
        </el-table-column>

        <el-table-column label="开仓均价" width="120" align="right" prop="avg_entry_price">
          <template #default="{ row }">
            <span class="mono">{{ formatPrice(row.avg_entry_price) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="当前价" width="120" align="right">
          <template #default="{ row }">
            <span class="mono" :class="{ 'price-flash': isFlashing(row.id) }">
              {{ getCurrentPrice(row.symbol) || '—' }}
            </span>
          </template>
        </el-table-column>

        <el-table-column label="浮动盈亏" width="140" align="right">
          <template #default="{ row }">
            <span class="mono" :class="pnlClass(row.unrealized_pnl)">
              {{ formatPnl(row.unrealized_pnl) }}
            </span>
          </template>
        </el-table-column>

        <el-table-column label="盈亏率" width="80" align="right">
          <template #default="{ row }">
            <span class="mono" :class="pnlClass(row.unrealized_pnl)">
              {{ formatPnlPercent(row) }}
            </span>
          </template>
        </el-table-column>

        <el-table-column label="操作" width="160" align="center">
          <template #default="{ row }">
            <el-button
              text
              size="small"
              @click="onPartialClose(row)"
            >
              部分平仓
            </el-button>
            <el-button
              text
              size="small"
              type="danger"
              :loading="closingIds.has(row.id)"
              @click="onClosePosition(row)"
            >
              全部平仓
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <!-- Loading Skeleton -->
      <div v-else class="skeleton-container">
        <div v-for="i in 3" :key="i" class="skeleton-row">
          <div class="skeleton-line" style="width: 90px; height: 14px;" />
          <div class="skeleton-line" style="width: 40px; height: 20px;" />
          <div class="skeleton-line" style="width: 80px; height: 14px;" />
          <div class="skeleton-line" style="width: 80px; height: 14px;" />
          <div class="skeleton-line" style="width: 100px; height: 14px;" />
          <div class="skeleton-line" style="width: 100px; height: 14px;" />
        </div>
      </div>
    </div>
  </div>

  <!-- Close Position Confirm Dialog -->
  <el-dialog
    v-model="closeDialogVisible"
    title="平仓确认"
    width="400px"
    :close-on-click-modal="false"
    class="close-position-dialog"
  >
    <div class="close-confirm-content">
      <p>
        确认平仓 <strong>{{ closeTarget?.symbol }}</strong>
        {{ closeQuantity || closeTarget?.quantity }} {{ baseCurrency(closeTarget) }}？
      </p>
      <p class="close-hint">将以市价{{ closeTarget?.side === 'long' ? '卖出' : '买入' }}</p>
      <div v-if="closeEstimatedAmount" class="close-estimate">
        <div class="close-estimate-row">
          <span>预估金额</span>
          <span class="mono">≈ {{ closeEstimatedAmount }} USDT</span>
        </div>
      </div>
    </div>

    <!-- Partial close quantity input -->
    <div v-if="isPartialClose" class="partial-input">
      <el-input
        v-model="closeQuantity"
        placeholder="输入平仓数量"
        class="mono-input"
      >
        <template #append>{{ baseCurrency(closeTarget) }}</template>
      </el-input>
    </div>

    <template #footer>
      <el-button @click="closeDialogVisible = false">取消</el-button>
      <el-button
        type="danger"
        :loading="closingPosition"
        @click="onConfirmClose"
      >
        确认平仓
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { Search } from '@element-plus/icons-vue'
import { closePosition } from '@/api/order'
import type { Position, PaperAccount } from '@/types/order'

/**
 * PositionPanel - Tab panel displaying positions with summary bar,
 * filter, table, and close position functionality.
 * Follows Design_OrderManagement.md section 5 specs.
 */

const props = withDefaults(defineProps<{
  /** Array of positions */
  positions?: Position[]
  /** Paper account data */
  account?: PaperAccount | null
  /** Current prices from WS (symbol → price) */
  currentPrices?: Record<string, string>
  /** Loading state */
  loading?: boolean
}>(), {
  positions: () => [],
  account: null,
  currentPrices: () => ({}),
  loading: false,
})

const emit = defineEmits<{
  /** Emitted after position is closed */
  (e: 'close-success', symbol: string): void
  /** Emitted when filter changes */
  (e: 'filter-change', filters: { side: string; symbol: string }): void
}>()

const filterSide = ref('')
const filterSymbol = ref('')
const closingIds = ref(new Set<number>())
const closeDialogVisible = ref(false)
const closeTarget = ref<Position | null>(null)
const closeQuantity = ref('')
const closingPosition = ref(false)
const isPartialClose = ref(false)
const flashingIds = ref(new Set<number>())

/** Filtered positions based on side and symbol */
const filteredPositions = computed(() => {
  let list = [...props.positions]
  if (filterSide.value) {
    list = list.filter((p) => p.side === filterSide.value)
  }
  if (filterSymbol.value) {
    const q = filterSymbol.value.toUpperCase()
    list = list.filter((p) => p.symbol.toUpperCase().includes(q))
  }
  return list
})

/** Get current price for a symbol */
function getCurrentPrice(symbol: string): string {
  const price = props.currentPrices[symbol]
  if (!price) return ''
  return formatPrice(price)
}

/** Check if a position row is flashing (price update) */
function isFlashing(id: number): boolean {
  return flashingIds.value.has(id)
}

/** Get base currency from position symbol */
function baseCurrency(pos: Position | null): string {
  if (!pos) return ''
  return pos.symbol.split('/')[0] ?? ''
}

/** Get close estimated amount */
const closeEstimatedAmount = computed(() => {
  if (!closeTarget.value) return ''
  const symbol = closeTarget.value.symbol
  const price = props.currentPrices[symbol]
  if (!price) return ''
  const qty = closeQuantity.value || closeTarget.value.quantity
  const amount = parseFloat(price) * parseFloat(qty)
  return amount.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
})

/** Handle filter change */
function onFilterChange() {
  emit('filter-change', { side: filterSide.value, symbol: filterSymbol.value })
}

/** Debounced symbol input */
let symbolTimer: ReturnType<typeof setTimeout> | null = null
function onSymbolInput() {
  if (symbolTimer) clearTimeout(symbolTimer)
  symbolTimer = setTimeout(onFilterChange, 300)
}

/** Open close position dialog (full close) */
function onClosePosition(pos: Position) {
  closeTarget.value = pos
  closeQuantity.value = pos.quantity
  isPartialClose.value = false
  closeDialogVisible.value = true
}

/** Open partial close dialog */
function onPartialClose(pos: Position) {
  closeTarget.value = pos
  closeQuantity.value = ''
  isPartialClose.value = true
  closeDialogVisible.value = true
}

/** Confirm close position */
async function onConfirmClose() {
  if (!closeTarget.value) return
  if (isPartialClose.value && !closeQuantity.value) {
    ElMessage.warning('请输入平仓数量')
    return
  }
  closingPosition.value = true
  try {
    await closePosition(closeTarget.value.symbol, isPartialClose.value ? closeQuantity.value : undefined)
    ElMessage.success('平仓委托已提交')
    emit('close-success', closeTarget.value.symbol)
    closeDialogVisible.value = false
  } catch (err: unknown) {
    if (err instanceof Error) {
      ElMessage.error(`平仓失败: ${err.message}`)
    }
  } finally {
    closingPosition.value = false
  }
}

/** Format money with thousand separators */
function formatMoney(value: string | number | undefined | null): string {
  if (!value && value !== 0) return '0.00'
  const num = typeof value === 'string' ? parseFloat(value) : value
  if (isNaN(num)) return '0.00'
  return num.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

/** Format price */
function formatPrice(value: string | number | null | undefined): string {
  if (!value && value !== 0) return '0.00'
  const num = typeof value === 'string' ? parseFloat(value) : value
  if (isNaN(num)) return '0.00'
  return num.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 8 })
}

/** Format PnL with +/- prefix */
function formatPnl(value: string | number | undefined | null): string {
  if (!value && value !== 0) return '¥0.00'
  const num = typeof value === 'string' ? parseFloat(value) : value
  if (isNaN(num)) return '¥0.00'
  const prefix = num > 0 ? '+¥' : num < 0 ? '-¥' : '¥'
  return prefix + Math.abs(num).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

/** Format PnL percent */
function formatPnlPercent(pos: Position): string {
  const pnl = parseFloat(pos.unrealized_pnl)
  const entry = parseFloat(pos.avg_entry_price)
  const qty = parseFloat(pos.quantity)
  if (entry === 0 || qty === 0) return '0.00%'
  const cost = entry * qty
  const pct = (pnl / cost) * 100
  const prefix = pct > 0 ? '+' : ''
  return `${prefix}${pct.toFixed(2)}%`
}

/** CSS class for PnL coloring */
function pnlClass(value: string | number | undefined | null): string {
  if (!value && value !== 0) return ''
  const num = typeof value === 'string' ? parseFloat(value) : value
  if (isNaN(num)) return ''
  if (num > 0) return 'text-profit'
  if (num < 0) return 'text-loss'
  return ''
}
</script>

<style scoped lang="scss">
.position-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.position-summary {
  display: flex;
  align-items: center;
  gap: 24px;
  padding: 12px 16px;
  background: var(--color-surface, #191a1b);
  border-radius: 8px;
  flex-wrap: wrap;
}

.summary-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.summary-label {
  font-size: 11px;
  color: var(--color-text-tertiary, #8a8f98);
}

.summary-value {
  font-size: 14px;
  color: var(--color-text-primary, #f7f8f8);
  font-weight: 500;

  &.mono {
    font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
    font-size: 13px;
  }

  &--frozen {
    color: var(--color-frozen, #f5a623);
  }

  &--secondary {
    color: var(--color-text-secondary, #d0d6e0);
    font-weight: 400;
  }
}

.position-filter {
  display: flex;
  gap: 12px;
  align-items: center;
}

.positions-table-wrapper {
  background: var(--color-surface, #191a1b);
  border-radius: 12px;
  overflow: hidden;
}

.positions-table {
  --el-table-bg-color: transparent;
  --el-table-tr-bg-color: transparent;
  --el-table-header-bg-color: transparent;
  --el-table-row-hover-bg-color: rgba(255, 255, 255, 0.04);
  --el-table-border-color: var(--color-border, rgba(255, 255, 255, 0.08));
  --el-table-header-text-color: var(--color-text-tertiary, #8a8f98);
  --el-table-text-color: var(--color-text-primary, #f7f8f8);
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
}

.text-profit {
  color: var(--color-buy, #67C23A);
}

.text-loss {
  color: var(--color-sell, #F56C6C);
}

.price-flash {
  background-color: rgba(255, 255, 255, 0.08);
}

// Close position dialog
.close-confirm-content {
  p {
    margin: 4px 0;
    color: var(--color-text-primary, #f7f8f8);
  }
}

.close-hint {
  font-size: 13px;
  color: var(--color-text-tertiary, #8a8f98);
}

.close-estimate {
  margin-top: 12px;
  padding: 8px 12px;
  background: var(--color-surface, #191a1b);
  border-radius: 8px;
}

.close-estimate-row {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  color: var(--color-text-primary, #f7f8f8);
}

.partial-input {
  margin-top: 16px;
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
  background: var(--color-surface-elevated, #212223);
  border-radius: 4px;
  height: 14px;
}
</style>
