<template>
  <el-dialog
    v-model="visible"
    title="新建委托"
    width="440px"
    :close-on-click-modal="false"
    :close-on-press-escape="!submitting"
    class="order-create-dialog"
    @closed="onClosed"
  >
    <!-- Side Toggle -->
    <div class="side-toggle">
      <button
        class="side-btn"
        :class="{ 'side-btn--active side-btn--buy': form.side === 'buy' }"
        @click="form.side = 'buy'"
      >
        买入
      </button>
      <button
        class="side-btn"
        :class="{ 'side-btn--active side-btn--sell': form.side === 'sell' }"
        @click="form.side = 'sell'"
      >
        卖出
      </button>
    </div>

    <!-- Order Type Select -->
    <div class="order-type-select">
      <button
        v-for="opt in orderTypeOptions"
        :key="opt.value"
        class="type-btn"
        :class="{
          'type-btn--active': form.order_type === opt.value,
          'type-btn--disabled': opt.disabled,
        }"
        :disabled="opt.disabled"
        @click="!opt.disabled && (form.order_type = opt.value as OrderType)"
      >
        {{ opt.label }}
        <el-icon v-if="opt.disabled" :size="12"><Lock /></el-icon>
      </button>
    </div>

    <el-form
      ref="formRef"
      :model="form"
      :rules="formRules"
      label-position="top"
      class="order-form"
      @submit.prevent
    >
      <!-- Symbol Select -->
      <el-form-item label="交易对" prop="symbol">
        <el-select
          v-model="form.symbol"
          filterable
          placeholder="选择交易对"
          class="full-width"
          @change="onSymbolChange"
        >
          <el-option
            v-for="s in enabledSymbols"
            :key="s.symbol"
            :label="s.symbol"
            :value="s.symbol"
          />
        </el-select>
      </el-form-item>

      <!-- Price Input (only for limit orders) -->
      <el-form-item v-if="form.order_type === 'limit'" label="价格" prop="price">
        <el-input
          v-model="form.price"
          placeholder="请输入委托价格"
          class="mono-input"
        >
          <template #append>{{ quoteCurrency }}</template>
        </el-input>
      </el-form-item>

      <!-- Market price hint -->
      <div v-if="form.order_type === 'market'" class="market-hint">
        <el-icon color="var(--color-warning, #f5a623)"><InfoFilled /></el-icon>
        <span>市价单将按市场最优价成交，实际成交价可能有偏差</span>
      </div>

      <!-- Quantity Input -->
      <el-form-item label="数量" prop="quantity">
        <el-input
          v-model="form.quantity"
          placeholder="请输入委托数量"
          class="mono-input"
        >
          <template #append>{{ baseCurrency }}</template>
        </el-input>
        <div v-if="estimatedAmount" class="estimated-amount">
          ≈ {{ estimatedAmount }} {{ quoteCurrency }}
        </div>
      </el-form-item>

      <!-- Quantity Slider -->
      <div v-if="currentSymbolConfig" class="quantity-slider">
        <button
          v-for="pct in [25, 50, 75, 100]"
          :key="pct"
          class="pct-btn"
          :class="{ 'pct-btn--active': activePct === pct }"
          @click="setQuantityByPercent(pct)"
        >
          {{ pct }}%
        </button>
      </div>

      <!-- Account Info -->
      <div class="account-info">
        <div class="account-info__row">
          <span class="account-info__label">
            {{ form.side === 'buy' ? '可用余额' : '可用持仓' }}
          </span>
          <span
            class="account-info__value"
            :class="{ 'account-info__value--insufficient': isInsufficient }"
          >
            {{ form.side === 'buy' ? formatMoney(account?.balance) : availablePositionQty }}
            {{ form.side === 'buy' ? quoteCurrency : baseCurrency }}
          </span>
        </div>
        <div v-if="form.side === 'buy' && account" class="account-info__row">
          <span class="account-info__label">冻结保证金</span>
          <span class="account-info__value account-info__value--frozen">
            {{ formatMoney(account.frozen_balance) }} {{ quoteCurrency }}
          </span>
        </div>
      </div>

      <!-- Order Summary -->
      <div v-if="estimatedAmount" class="order-summary">
        <div class="order-summary__row">
          <span class="order-summary__label">预估金额</span>
          <span class="order-summary__value">≈ {{ estimatedAmount }} {{ quoteCurrency }}</span>
        </div>
        <div v-if="estimatedFee" class="order-summary__row">
          <span class="order-summary__label">手续费(≈)</span>
          <span class="order-summary__value">≈ {{ estimatedFee }} {{ quoteCurrency }}</span>
        </div>
        <div v-if="estimatedFreeze" class="order-summary__row">
          <span class="order-summary__label">合计冻结</span>
          <span class="order-summary__value order-summary__value--frozen">
            ≈ {{ estimatedFreeze }} {{ quoteCurrency }}
          </span>
        </div>
      </div>

      <!-- Price Deviation Warning -->
      <div v-if="priceDeviation > 10" class="deviation-warning">
        <el-icon color="var(--color-warning, #f5a623)"><WarningFilled /></el-icon>
        <span>委托价格偏离当前市场价较大 ({{ priceDeviation.toFixed(1) }}%)</span>
      </div>
    </el-form>

    <template #footer>
      <el-button @click="visible = false" :disabled="submitting">取消</el-button>
      <el-button
        :type="form.side === 'buy' ? 'success' : 'danger'"
        :loading="submitting"
        :disabled="!canSubmit"
        @click="onSubmit"
      >
        {{ form.side === 'buy' ? '买入' : '卖出' }}
      </el-button>
    </template>
  </el-dialog>

  <!-- Confirm Dialog -->
  <el-dialog
    v-model="confirmVisible"
    title="确认提交委托"
    width="400px"
    :close-on-click-modal="false"
    class="order-confirm-dialog"
  >
    <div class="confirm-details">
      <div class="confirm-row">
        <span class="confirm-label">交易对</span>
        <span class="confirm-value">{{ form.symbol }}</span>
      </div>
      <div class="confirm-row">
        <span class="confirm-label">方向</span>
        <span class="confirm-value" :class="form.side === 'buy' ? 'text-buy' : 'text-sell'">
          {{ form.side === 'buy' ? '买入' : '卖出' }}
        </span>
      </div>
      <div class="confirm-row">
        <span class="confirm-label">类型</span>
        <span class="confirm-value">{{ form.order_type === 'limit' ? '限价' : '市价' }}</span>
      </div>
      <div v-if="form.order_type === 'limit'" class="confirm-row">
        <span class="confirm-label">价格</span>
        <span class="confirm-value mono">{{ formatPrice(form.price) }} {{ quoteCurrency }}</span>
      </div>
      <div class="confirm-row">
        <span class="confirm-label">数量</span>
        <span class="confirm-value mono">{{ form.quantity }} {{ baseCurrency }}</span>
      </div>
      <div v-if="estimatedAmount" class="confirm-row">
        <span class="confirm-label">预估金额</span>
        <span class="confirm-value mono">≈ {{ estimatedAmount }} {{ quoteCurrency }}</span>
      </div>
      <div v-if="estimatedFee" class="confirm-row">
        <span class="confirm-label">手续费(≈)</span>
        <span class="confirm-value mono">≈ {{ estimatedFee }} {{ quoteCurrency }}</span>
      </div>
    </div>

    <!-- Market order warning -->
    <div v-if="form.order_type === 'market'" class="confirm-warning">
      <el-icon color="var(--color-warning, #f5a623)"><WarningFilled /></el-icon>
      <span>市价单将按市场最优价成交，实际成交价可能有偏差</span>
    </div>

    <!-- Price deviation warning -->
    <div v-if="priceDeviation > 10" class="confirm-warning">
      <el-icon color="var(--color-warning, #f5a623)"><WarningFilled /></el-icon>
      <span>委托价格偏离当前市场价较大 ({{ priceDeviation.toFixed(1) }}%)</span>
    </div>

    <template #footer>
      <el-button @click="confirmVisible = false">取消</el-button>
      <el-button
        :type="form.side === 'buy' ? 'success' : 'danger'"
        :loading="submitting"
        @click="onConfirmSubmit"
      >
        确认提交
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { ElMessage } from 'element-plus'
import { Lock, InfoFilled, WarningFilled } from '@element-plus/icons-vue'
import { createOrder } from '@/api/order'
import type { OrderSide, OrderType, SymbolConfig, PaperAccount, CreateOrderRequest } from '@/types/order'

/**
 * OrderCreateDialog - Dialog for creating new orders with buy/sell toggle,
 * order type selection, price/quantity inputs, and confirmation flow.
 * Follows Design_OrderManagement.md section 3 specs.
 */

const props = withDefaults(defineProps<{
  /** Available symbol configs */
  symbolConfigs?: SymbolConfig[]
  /** Paper account data */
  account?: PaperAccount | null
  /** Available position quantity for sell orders */
  availablePositionQty?: string
  /** Current best bid price */
  bestBid?: string | null
  /** Current best ask price */
  bestAsk?: string | null
  /** Trade mode badge */
  mode?: 'paper' | 'live'
}>(), {
  symbolConfigs: () => [],
  account: null,
  availablePositionQty: '0',
  bestBid: null,
  bestAsk: null,
  mode: 'paper',
})

const emit = defineEmits<{
  /** Emitted after order is successfully created */
  (e: 'order-created', order: CreateOrderRequest): void
}>()

const visible = defineModel<boolean>('modelValue', { default: false })
const confirmVisible = ref(false)
const submitting = ref(false)
const formRef = ref<FormInstance>()

/** Order type options (stop types disabled in MVP) */
const orderTypeOptions = [
  { value: 'limit', label: '限价', disabled: false },
  { value: 'market', label: '市价', disabled: false },
  { value: 'stop', label: '止损', disabled: true },
  { value: 'stop_limit', label: '止损限价', disabled: true },
]

/** Form state */
const form = ref({
  side: 'buy' as OrderSide,
  order_type: 'limit' as OrderType,
  symbol: '',
  price: '',
  quantity: '',
})

/** Enabled symbol configs */
const enabledSymbols = computed(() => props.symbolConfigs.filter((s) => s.enabled))

/** Current symbol config */
const currentSymbolConfig = computed(() =>
  enabledSymbols.value.find((s) => s.symbol === form.value.symbol),
)

/** Base currency (e.g. BTC) */
const baseCurrency = computed(() => currentSymbolConfig.value?.base_currency ?? '')

/** Quote currency (e.g. USDT) */
const quoteCurrency = computed(() => currentSymbolConfig.value?.quote_currency ?? 'USDT')

/** Estimated order amount */
const estimatedAmount = computed(() => {
  if (!form.value.quantity) return ''
  const qty = parseFloat(form.value.quantity)
  if (isNaN(qty) || qty <= 0) return ''
  const price = form.value.order_type === 'limit'
    ? parseFloat(form.value.price)
    : (form.value.side === 'buy' ? parseFloat(props.bestAsk ?? '0') : parseFloat(props.bestBid ?? '0'))
  if (isNaN(price) || price <= 0) return ''
  return (price * qty).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
})

/** Estimated fee */
const estimatedFee = computed(() => {
  if (!estimatedAmount.value || !currentSymbolConfig.value) return ''
  const amount = parseFloat(estimatedAmount.value.replace(/,/g, ''))
  const feeRate = parseFloat(currentSymbolConfig.value.fee_rate)
  if (isNaN(feeRate)) return ''
  return (amount * feeRate).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
})

/** Estimated freeze amount (buy limit orders only) */
const estimatedFreeze = computed(() => {
  if (form.value.side !== 'buy' || form.value.order_type !== 'limit') return ''
  if (!estimatedAmount.value || !estimatedFee.value) return ''
  const amount = parseFloat(estimatedAmount.value.replace(/,/g, ''))
  const fee = parseFloat(estimatedFee.value.replace(/,/g, ''))
  return (amount + fee).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
})

/** Active percentage button */
const activePct = computed(() => {
  if (!form.value.quantity || !currentSymbolConfig.value) return 0
  const qty = parseFloat(form.value.quantity)
  const maxQty = getMaxQuantity()
  if (maxQty === 0) return 0
  const pct = Math.round((qty / maxQty) * 100)
  if ([25, 50, 75, 100].includes(pct)) return pct
  return 0
})

/** Whether balance/position is insufficient */
const isInsufficient = computed(() => {
  if (!form.value.quantity) return false
  if (form.value.side === 'buy') {
    if (!props.account || !form.value.price) return false
    const cost = parseFloat(form.value.price) * parseFloat(form.value.quantity)
    return cost > parseFloat(props.account.balance)
  } else {
    return parseFloat(form.value.quantity) > parseFloat(props.availablePositionQty)
  }
})

/** Price deviation from market price (%) */
const priceDeviation = computed(() => {
  if (form.value.order_type !== 'limit' || !form.value.price) return 0
  const price = parseFloat(form.value.price)
  const marketPrice = form.value.side === 'buy'
    ? parseFloat(props.bestAsk ?? '0')
    : parseFloat(props.bestBid ?? '0')
  if (marketPrice === 0) return 0
  return Math.abs((price - marketPrice) / marketPrice) * 100
})

/** Whether submit button should be enabled */
const canSubmit = computed(() => {
  if (submitting.value) return false
  if (!form.value.symbol || !form.value.quantity) return false
  if (form.value.order_type === 'limit' && !form.value.price) return false
  if (isInsufficient.value) return false
  const qty = parseFloat(form.value.quantity)
  if (isNaN(qty) || qty <= 0) return false
  if (form.value.order_type === 'limit') {
    const price = parseFloat(form.value.price)
    if (isNaN(price) || price <= 0) return false
  }
  return true
})

/** Form validation rules */
const formRules = computed<FormRules>(() => ({
  symbol: [{ required: true, message: '请选择交易对', trigger: 'change' }],
  price: form.value.order_type === 'limit'
    ? [
        { required: true, message: '请输入委托价格', trigger: 'blur' },
        {
          validator: (_rule: unknown, value: string, callback: (err?: Error) => void) => {
            const num = parseFloat(value)
            if (isNaN(num) || num <= 0) callback(new Error('价格必须大于0'))
            else callback()
          },
          trigger: 'blur',
        },
      ]
    : [],
  quantity: [
    { required: true, message: '请输入委托数量', trigger: 'blur' },
    {
      validator: (_rule: unknown, value: string, callback: (err?: Error) => void) => {
        const num = parseFloat(value)
        if (isNaN(num) || num <= 0) callback(new Error('数量必须大于0'))
        else callback()
      },
      trigger: 'blur',
    },
  ],
}))

/** Get max quantity based on side */
function getMaxQuantity(): number {
  if (!currentSymbolConfig.value) return 0
  if (form.value.side === 'buy') {
    if (!props.account || form.value.order_type === 'limit' && !form.value.price) return 0
    const balance = parseFloat(props.account?.balance ?? '0')
    const price = form.value.order_type === 'limit'
      ? parseFloat(form.value.price)
      : parseFloat(props.bestAsk ?? '0')
    if (price <= 0) return 0
    return Math.floor((balance / price) * Math.pow(10, currentSymbolConfig.value.quantity_precision)) / Math.pow(10, currentSymbolConfig.value.quantity_precision)
  } else {
    return parseFloat(props.availablePositionQty)
  }
}

/** Set quantity by percentage */
function setQuantityByPercent(pct: number) {
  const maxQty = getMaxQuantity()
  const qty = maxQty * (pct / 100)
  const precision = currentSymbolConfig.value?.quantity_precision ?? 4
  form.value.quantity = qty.toFixed(precision)
}

/** Handle symbol change */
function onSymbolChange() {
  form.value.price = ''
  form.value.quantity = ''
  // Auto-fill price from best bid/ask
  nextTick(() => {
    if (form.value.order_type === 'limit') {
      form.value.price = form.value.side === 'buy'
        ? (props.bestAsk ?? '')
        : (props.bestBid ?? '')
    }
  })
}

/** Handle side change - update default price */
watch(() => form.value.side, () => {
  if (form.value.order_type === 'limit') {
    form.value.price = form.value.side === 'buy'
      ? (props.bestAsk ?? '')
      : (props.bestBid ?? '')
  }
  form.value.quantity = ''
})

/** Handle order type change */
watch(() => form.value.order_type, () => {
  if (form.value.order_type === 'market') {
    form.value.price = ''
  } else {
    form.value.price = form.value.side === 'buy'
      ? (props.bestAsk ?? '')
      : (props.bestBid ?? '')
  }
})

/** Open the dialog */
function open() {
  visible.value = true
}

/** Handle dialog close - reset form */
function onClosed() {
  form.value = {
    side: 'buy',
    order_type: 'limit',
    symbol: '',
    price: '',
    quantity: '',
  }
  formRef.value?.resetFields()
  confirmVisible.value = false
}

/** Submit button → show confirm dialog */
async function onSubmit() {
  const valid = await formRef.value?.validate().catch(() => false)
  if (!valid) return
  confirmVisible.value = true
}

/** Confirm submit → call API */
async function onConfirmSubmit() {
  submitting.value = true
  try {
    const req: CreateOrderRequest = {
      symbol: form.value.symbol,
      side: form.value.side,
      order_type: form.value.order_type,
      quantity: form.value.quantity,
    }
    if (form.value.order_type === 'limit' && form.value.price) {
      req.price = form.value.price
    }
    await createOrder(req)
    ElMessage.success('委托已提交')
    emit('order-created', req)
    confirmVisible.value = false
    visible.value = false
  } catch (err: unknown) {
    if (err instanceof Error) {
      ElMessage.error(`提交失败: ${err.message}`)
    }
  } finally {
    submitting.value = false
  }
}

/** Format money value */
function formatMoney(value: string | number | undefined | null): string {
  if (!value && value !== 0) return '0.00'
  const num = typeof value === 'string' ? parseFloat(value) : value
  if (isNaN(num)) return '0.00'
  return num.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

/** Format price */
function formatPrice(value: string | undefined): string {
  if (!value) return '0.00'
  const num = parseFloat(value)
  if (isNaN(num)) return '0.00'
  return num.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 8 })
}

defineExpose({ open })
</script>

<style scoped lang="scss">
.order-create-dialog {
  :deep(.el-dialog) {
    background: var(--color-surface-elevated, #212223);
    border-radius: 12px;
    border: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
  }

  :deep(.el-dialog__title) {
    color: var(--color-text-primary, #f7f8f8);
    font-weight: 600;
  }

  :deep(.el-dialog__body) {
    color: var(--color-text-primary, #f7f8f8);
  }
}

.side-toggle {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.side-btn {
  flex: 1;
  height: 40px;
  border: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
  border-radius: 8px;
  background: transparent;
  color: var(--color-text-secondary, #d0d6e0);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;

  &:hover:not(.side-btn--active) {
    background: rgba(255, 255, 255, 0.04);
  }

  &--buy.side-btn--active {
    background: var(--color-buy, #67C23A);
    color: #fff;
    border-color: var(--color-buy, #67C23A);
  }

  &--sell.side-btn--active {
    background: var(--color-sell, #F56C6C);
    color: #fff;
    border-color: var(--color-sell, #F56C6C);
  }

  &:hover:not(.side-btn--active) {
    &.side-btn--buy {
      background: rgba(103, 194, 58, 0.08);
    }
    &.side-btn--sell {
      background: rgba(245, 108, 108, 0.08);
    }
  }
}

.order-type-select {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
  padding-bottom: 8px;
}

.type-btn {
  padding: 6px 12px;
  border: none;
  background: transparent;
  color: var(--color-text-secondary, #d0d6e0);
  font-size: 14px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
  height: 32px;

  &--active {
    color: var(--color-text-primary, #f7f8f8);
    border-bottom-color: var(--color-accent, #7170ff);
  }

  &--disabled {
    color: var(--color-text-tertiary, #8a8f98);
    font-size: 12px;
    cursor: not-allowed;
    opacity: 0.5;
  }
}

.order-form {
  margin-top: 8px;
}

.full-width {
  width: 100%;
}

.market-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  background: rgba(245, 166, 35, 0.08);
  border-radius: 8px;
  font-size: 12px;
  color: var(--color-warning, #f5a623);
  margin-bottom: 12px;
}

.quantity-slider {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.pct-btn {
  flex: 1;
  height: 28px;
  border: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
  border-radius: 4px;
  background: var(--color-surface-elevated, #212223);
  color: var(--color-text-secondary, #d0d6e0);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;

  &--active {
    background: var(--color-accent, #7170ff);
    color: #fff;
    border-color: var(--color-accent, #7170ff);
  }
}

.account-info {
  background: var(--color-surface, #191a1b);
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 12px;
}

.account-info__row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 2px 0;
}

.account-info__label {
  font-size: 12px;
  color: var(--color-text-tertiary, #8a8f98);
}

.account-info__value {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-size: 14px;
  color: var(--color-text-primary, #f7f8f8);

  &--frozen {
    color: var(--color-frozen, #f5a623);
    font-size: 12px;
  }

  &--insufficient {
    color: var(--color-error, #e5484d);
    animation: blink 0.5s ease 3;
  }
}

@keyframes blink {
  50% { opacity: 0.5; }
}

.order-summary {
  background: var(--color-surface-elevated, #212223);
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 12px;
}

.order-summary__row {
  display: flex;
  justify-content: space-between;
  padding: 2px 0;
}

.order-summary__label {
  font-size: 12px;
  color: var(--color-text-tertiary, #8a8f98);
}

.order-summary__value {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-size: 14px;
  color: var(--color-text-primary, #f7f8f8);

  &--frozen {
    color: var(--color-frozen, #f5a623);
  }
}

.deviation-warning {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  background: rgba(245, 166, 35, 0.08);
  border-radius: 8px;
  font-size: 12px;
  color: var(--color-warning, #f5a623);
  margin-bottom: 8px;
}

.estimated-amount {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-size: 12px;
  color: var(--color-text-tertiary, #8a8f98);
  margin-top: 4px;
}

// Confirm dialog
.confirm-details {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.confirm-row {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
}

.confirm-label {
  color: var(--color-text-tertiary, #8a8f98);
  font-size: 14px;
}

.confirm-value {
  color: var(--color-text-primary, #f7f8f8);
  font-size: 14px;
  font-weight: 500;

  &.mono {
    font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  }
}

.text-buy { color: var(--color-buy, #67C23A); }
.text-sell { color: var(--color-sell, #F56C6C); }

.confirm-warning {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  background: rgba(245, 166, 35, 0.08);
  border-radius: 8px;
  font-size: 12px;
  color: var(--color-warning, #f5a623);
  margin-top: 12px;
}
</style>
