<template>
  <el-drawer
    v-model="visible"
    :title="`委托详情 #${order?.order_id ?? ''}`"
    direction="rtl"
    size="420px"
    :close-on-press-escape="true"
    class="order-detail-drawer"
    @closed="onClosed"
  >
    <template v-if="loading">
      <div class="drawer-skeleton">
        <div class="skeleton-line" style="width: 120px; height: 24px;" />
        <div class="skeleton-line" style="width: 200px; height: 18px; margin-top: 8px;" />
        <div class="skeleton-line" style="width: 100%; height: 1px; margin: 20px 0;" />
        <div v-for="i in 6" :key="i" class="skeleton-line" style="width: 100%; height: 16px; margin-top: 12px;" />
      </div>
    </template>

    <template v-else-if="order">
      <!-- Header: Symbol + Side + Type + Status -->
      <div class="detail-header">
        <div class="detail-symbol">{{ order.symbol }}</div>
        <div class="detail-badges">
          <el-tag
            :type="order.side === 'buy' ? 'success' : 'danger'"
            size="small"
            effect="dark"
          >
            {{ order.side === 'buy' ? '买入' : '卖出' }}
          </el-tag>
          <span class="detail-type-label">
            {{ order.order_type === 'limit' ? '限价' : order.order_type === 'market' ? '市价' : order.order_type }}
          </span>
          <el-tag
            :type="getOrderStatusType(order.status)"
            size="small"
            effect="plain"
            class="status-pill"
          >
            {{ getOrderStatusText(order.status) }}
          </el-tag>
        </div>
      </div>

      <!-- Order Info Section -->
      <div class="detail-section">
        <div class="section-title">委托信息</div>
        <div class="detail-row">
          <span class="detail-label">委托价格</span>
          <span class="detail-value mono">
            {{ order.order_type === 'market' ? '市价' : formatPrice(order.price) }}
            {{ order.order_type !== 'market' ? quoteCurrency : '' }}
          </span>
        </div>
        <div class="detail-row">
          <span class="detail-label">委托数量</span>
          <span class="detail-value mono">{{ order.quantity }} {{ baseCurrency }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">已成交</span>
          <span class="detail-value mono" :class="{ 'text-partial': order.status === 'partial_filled' }">
            {{ order.filled_quantity }} {{ baseCurrency }}
          </span>
        </div>
        <div class="detail-row">
          <span class="detail-label">成交均价</span>
          <span v-if="order.avg_fill_price" class="detail-value mono">
            {{ formatPrice(order.avg_fill_price) }} {{ quoteCurrency }}
          </span>
          <span v-else class="detail-value detail-value--muted">—</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">有效期</span>
          <span class="detail-value">{{ order.time_in_force }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">手续费</span>
          <span class="detail-value mono detail-value--frozen">{{ order.fee }} {{ quoteCurrency }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">创建时间</span>
          <span class="detail-value mono detail-value--secondary">{{ formatDateTime(order.created_at) }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">更新时间</span>
          <span class="detail-value mono detail-value--secondary">{{ formatDateTime(order.updated_at) }}</span>
        </div>
      </div>

      <!-- Margin Info Section (only for buy limit orders) -->
      <div v-if="showMarginInfo" class="detail-section">
        <div class="section-title">保证金信息</div>
        <div class="detail-row">
          <span class="detail-label">冻结金额</span>
          <span class="detail-value mono">{{ formatPrice(frozenAmount) }} {{ quoteCurrency }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">已释放</span>
          <span class="detail-value mono">{{ formatPrice(releasedAmount) }} {{ quoteCurrency }}</span>
        </div>
        <div class="detail-row">
          <span class="detail-label">仍冻结</span>
          <span class="detail-value mono detail-value--frozen">{{ formatPrice(remainingFrozen) }} {{ quoteCurrency }}</span>
        </div>
      </div>

      <!-- Strategy ID (D10) -->
      <div v-if="order.strategy_id" class="detail-section">
        <div class="section-title">策略信号</div>
        <div class="detail-row">
          <span class="detail-label">策略ID</span>
          <span class="detail-value mono">{{ order.strategy_id }}</span>
        </div>
      </div>

      <!-- Fill Records Section -->
      <div class="detail-section">
        <div class="section-title">成交明细</div>
        <template v-if="fills.length > 0">
          <div class="fill-table">
            <div class="fill-header">
              <span>成交价</span>
              <span>数量</span>
              <span>手续费</span>
              <span>时间</span>
              <span>类型</span>
            </div>
            <div v-for="fill in fills" :key="fill.trade_id" class="fill-row">
              <span class="mono">{{ formatPrice(fill.price) }}</span>
              <span class="mono">{{ fill.quantity }}</span>
              <span class="mono fill-fee">{{ fill.fee }}</span>
              <span class="mono secondary">{{ formatTime(fill.created_at) }}</span>
              <span class="fill-maker-tag">{{ fill.is_maker ? 'M' : 'T' }}</span>
            </div>
          </div>
        </template>
        <div v-else class="empty-fills">暂无成交记录</div>
      </div>

      <!-- Bottom Action: Cancel Button -->
      <div v-if="isCancellable" class="drawer-footer">
        <el-button
          type="danger"
          :loading="cancelling"
          @click="onCancelOrder"
        >
          撤销委托
        </el-button>
      </div>
    </template>
  </el-drawer>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { cancelOrder } from '@/api/order'
import type { Order, Trade, OrderStatus } from '@/types/order'
import { getOrderStatusType, getOrderStatusText } from '@/types/order'

/**
 * OrderDetailDrawer - Right-side drawer showing full order details,
 * margin info, fill records, and cancel action.
 * Follows Design_OrderManagement.md section 4 specs.
 */

const props = withDefaults(defineProps<{
  /** The order to display */
  order?: Order | null
  /** Fill records for this order */
  fills?: Trade[]
  /** Loading state */
  loading?: boolean
}>(), {
  order: null,
  fills: () => [],
  loading: false,
})

const emit = defineEmits<{
  /** Emitted after order is successfully cancelled */
  (e: 'cancel-success', orderId: string): void
}>()

const visible = defineModel<boolean>('modelValue', { default: false })
const cancelling = ref(false)

/** Base currency from symbol (e.g. BTC from BTC/USDT) */
const baseCurrency = computed(() => {
  const parts = props.order?.symbol?.split('/')
  return parts?.[0] ?? ''
})

/** Quote currency from symbol */
const quoteCurrency = computed(() => {
  const parts = props.order?.symbol?.split('/')
  return parts?.[1] ?? 'USDT'
})

/** Whether to show margin info section (buy limit orders only) */
const showMarginInfo = computed(() => {
  return props.order?.side === 'buy' && props.order?.order_type === 'limit'
})

/** Frozen amount = price × quantity */
const frozenAmount = computed(() => {
  if (!props.order) return '0'
  const price = parseFloat(props.order.price ?? '0')
  const qty = parseFloat(props.order.quantity)
  return (price * qty).toFixed(2)
})

/** Released amount = avg_fill_price × filled_quantity */
const releasedAmount = computed(() => {
  if (!props.order || !props.order.avg_fill_price) return '0'
  const avgPrice = parseFloat(props.order.avg_fill_price)
  const filled = parseFloat(props.order.filled_quantity)
  return (avgPrice * filled).toFixed(2)
})

/** Remaining frozen = frozen - released */
const remainingFrozen = computed(() => {
  const frozen = parseFloat(frozenAmount.value)
  const released = parseFloat(releasedAmount.value)
  return Math.max(0, frozen - released).toFixed(2)
})

/** Whether order can be cancelled */
const isCancellable = computed(() => {
  return props.order?.status === 'pending' || props.order?.status === 'partial_filled'
})

/** Format price with thousand separators */
function formatPrice(value: string | null | undefined): string {
  if (!value) return '0.00'
  const num = parseFloat(value)
  if (isNaN(num)) return '0.00'
  return num.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 8 })
}

/** Format ISO datetime to YYYY-MM-DD HH:mm:ss */
function formatDateTime(isoStr: string): string {
  if (!isoStr) return ''
  const d = new Date(isoStr)
  return d.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  })
}

/** Format ISO datetime to HH:mm */
function formatTime(isoStr: string): string {
  if (!isoStr) return ''
  const d = new Date(isoStr)
  const hh = String(d.getHours()).padStart(2, '0')
  const mi = String(d.getMinutes()).padStart(2, '0')
  return `${hh}:${mi}`
}

/** Cancel order with confirmation */
async function onCancelOrder() {
  if (!props.order) return
  try {
    await ElMessageBox.confirm(
      `确认撤销 ${props.order.symbol} ${props.order.side === 'buy' ? '买入' : '卖出'} 委托？`,
      '撤销委托',
      {
        confirmButtonText: '确认撤单',
        cancelButtonText: '取消',
        type: 'warning',
      },
    )
    cancelling.value = true
    await cancelOrder(props.order.order_id)
    ElMessage.success('委托已撤销')
    emit('cancel-success', props.order.order_id)
    visible.value = false
  } catch (err: unknown) {
    if (err !== 'cancel' && err instanceof Error) {
      ElMessage.error(`撤单失败: ${err.message}`)
    }
  } finally {
    cancelling.value = false
  }
}

/** Handle drawer close */
function onClosed() {
  cancelling.value = false
}
</script>

<style scoped lang="scss">
.order-detail-drawer {
  :deep(.el-drawer) {
    background: var(--color-surface-elevated, #212223);
    border-left: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
  }

  :deep(.el-drawer__title) {
    color: var(--color-text-primary, #f7f8f8);
    font-weight: 600;
  }

  :deep(.el-drawer__body) {
    padding: 24px;
    color: var(--color-text-primary, #f7f8f8);
  }
}

.detail-header {
  margin-bottom: 24px;
}

.detail-symbol {
  font-size: 24px;
  font-weight: 700;
  color: var(--color-text-primary, #f7f8f8);
  margin-bottom: 8px;
}

.detail-badges {
  display: flex;
  align-items: center;
  gap: 8px;
}

.detail-type-label {
  font-size: 13px;
  color: var(--color-text-secondary, #d0d6e0);
}

.status-pill {
  border-radius: 10px;
}

.detail-section {
  margin-bottom: 20px;
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-tertiary, #8a8f98);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 12px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.06));
}

.detail-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 0;
}

.detail-label {
  font-size: 13px;
  color: var(--color-text-tertiary, #8a8f98);
}

.detail-value {
  font-size: 14px;
  color: var(--color-text-primary, #f7f8f8);

  &.mono {
    font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  }

  &--secondary {
    font-size: 12px;
    color: var(--color-text-secondary, #d0d6e0);
  }

  &--muted {
    color: var(--color-text-tertiary, #8a8f98);
  }

  &--frozen {
    color: var(--color-frozen, #f5a623);
  }
}

.text-partial {
  color: var(--color-partial, #60a5fa);
}

// Fill records table
.fill-table {
  font-size: 12px;
}

.fill-header {
  display: grid;
  grid-template-columns: 1fr 80px 80px 60px 40px;
  gap: 8px;
  padding: 6px 0;
  color: var(--color-text-tertiary, #8a8f98);
  font-size: 11px;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.06));
}

.fill-row {
  display: grid;
  grid-template-columns: 1fr 80px 80px 60px 40px;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.04));
}

.mono {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
}

.secondary {
  color: var(--color-text-secondary, #d0d6e0);
}

.fill-fee {
  color: var(--color-frozen, #f5a623);
}

.fill-maker-tag {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 18px;
  border-radius: 3px;
  background: var(--color-surface, #191a1b);
  color: var(--color-text-secondary, #d0d6e0);
  font-size: 10px;
  font-weight: 600;
}

.empty-fills {
  color: var(--color-text-tertiary, #8a8f98);
  font-size: 13px;
  padding: 8px 0;
}

.drawer-footer {
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
}

// Skeleton
.drawer-skeleton {
  .skeleton-line {
    background: linear-gradient(90deg, var(--color-surface, #191a1b) 25%, rgba(255, 255, 255, 0.05) 50%, var(--color-surface, #191a1b) 75%);
    background-size: 200% 100%;
    animation: skeleton-loading 1.5s ease-in-out infinite;
    border-radius: 4px;
  }
}

@keyframes skeleton-loading {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
</style>
