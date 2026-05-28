<template>
  <div class="order-form">
    <!-- Side Toggle: 买入/卖出 -->
    <div class="side-toggle">
      <button
        class="side-btn buy-btn"
        :class="{ active: form.side === 'buy' }"
        @click="form.side = 'buy'"
      >
        买入
      </button>
      <button
        class="side-btn sell-btn"
        :class="{ active: form.side === 'sell' }"
        @click="form.side = 'sell'"
      >
        卖出
      </button>
    </div>

    <!-- Order Type: 限价/市价 -->
    <el-form ref="formRef" :model="form" :rules="formRules" label-position="top" size="default">
      <el-form-item label="委托类型" class="form-item-compact">
        <el-radio-group v-model="form.order_type" @change="onOrderTypeChange">
          <el-radio-button value="limit">限价</el-radio-button>
          <el-radio-button value="market">市价</el-radio-button>
        </el-radio-group>
      </el-form-item>

      <!-- Price (limit orders only) -->
      <el-form-item v-if="form.order_type === 'limit'" label="价格" prop="price" class="form-item-compact">
        <el-input
          v-model="form.price"
          placeholder="输入价格"
          :precision="symbolConfig?.price_precision ?? 2"
          clearable
        >
          <template #append>{{ symbolConfig?.quote_currency ?? 'USDT' }}</template>
        </el-input>
        <div v-if="bestBid && bestAsk" class="price-hint">
          <span>买一: {{ bestBid }}</span>
          <span>卖一: {{ bestAsk }}</span>
        </div>
      </el-form-item>

      <!-- Quantity -->
      <el-form-item label="数量" prop="quantity" class="form-item-compact">
        <el-input
          v-model="form.quantity"
          placeholder="输入数量"
          clearable
        >
          <template #append>{{ symbolConfig?.base_currency ?? '' }}</template>
        </el-input>
      </el-form-item>

      <!-- Percentage Buttons -->
      <div class="percent-buttons">
        <el-button
          v-for="pct in [25, 50, 75, 100]"
          :key="pct"
          size="small"
          :type="form.side === 'buy' ? 'success' : 'danger'"
          plain
          @click="setQuantityPercent(pct)"
        >
          {{ pct }}%
        </el-button>
      </div>

      <!-- Available Balance -->
      <div class="balance-info">
        <span class="balance-label">可用余额</span>
        <span class="balance-value">
          {{ form.side === 'buy' ? formatBalance(account?.balance) : availablePositionQty }}
          {{ form.side === 'buy' ? (symbolConfig?.quote_currency ?? 'USDT') : (symbolConfig?.base_currency ?? '') }}
        </span>
      </div>

      <!-- Estimated Total (limit orders) -->
      <div v-if="form.order_type === 'limit' && estimatedTotal" class="total-info">
        <span class="total-label">预估金额</span>
        <span class="total-value">{{ formatBalance(estimatedTotal) }} {{ symbolConfig?.quote_currency ?? 'USDT' }}</span>
      </div>

      <!-- Submit Button -->
      <el-form-item class="form-item-submit">
        <el-button
          :type="form.side === 'buy' ? 'success' : 'danger'"
          :loading="submitting"
          class="submit-btn"
          @click="handleSubmit"
        >
          {{ form.side === 'buy' ? '买入' : '卖出' }}
          {{ symbolConfig?.base_currency ?? symbol }}
        </el-button>
      </el-form-item>
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { SymbolConfig, PaperAccount, CreateOrderRequest, OrderSide, OrderType, TimeInForce } from '@/types/order'
import { createOrder } from '@/api/order'

interface Props {
  symbol: string
  symbolConfig: SymbolConfig | null
  account: PaperAccount | null
  bestBid?: string | null
  bestAsk?: string | null
  availablePositionQty?: string
}

const props = withDefaults(defineProps<Props>(), {
  bestBid: null,
  bestAsk: null,
  availablePositionQty: '0',
})

const emit = defineEmits<{
  (e: 'submit', order: CreateOrderRequest): void
}>()

const formRef = ref<FormInstance>()
const submitting = ref(false)

const form = reactive({
  side: 'buy' as OrderSide,
  order_type: 'limit' as OrderType,
  price: '',
  quantity: '',
  time_in_force: 'GTC' as TimeInForce,
})

// Validation rules
const formRules = computed<FormRules>(() => ({
  price: form.order_type === 'limit'
    ? [
        { required: true, message: '请输入价格', trigger: 'blur' },
        {
          validator: (_rule: unknown, value: string, callback: (err?: Error) => void) => {
            const num = parseFloat(value)
            if (isNaN(num) || num <= 0) {
              callback(new Error('价格必须大于0'))
              return
            }
            const precision = props.symbolConfig?.price_precision ?? 2
            const decimals = value.includes('.') ? value.split('.')[1]?.length ?? 0 : 0
            if (decimals > precision) {
              callback(new Error(`价格精度不超过 ${precision} 位小数`))
              return
            }
            callback()
          },
          trigger: 'blur',
        },
      ]
    : [],
  quantity: [
    { required: true, message: '请输入数量', trigger: 'blur' },
    {
      validator: (_rule: unknown, value: string, callback: (err?: Error) => void) => {
        const num = parseFloat(value)
        if (isNaN(num) || num <= 0) {
          callback(new Error('数量必须大于0'))
          return
        }
        const minQty = parseFloat(props.symbolConfig?.min_quantity ?? '0')
        if (num < minQty) {
          callback(new Error(`最小下单数量为 ${minQty}`))
          return
        }
        const precision = props.symbolConfig?.quantity_precision ?? 4
        const decimals = value.includes('.') ? value.split('.')[1]?.length ?? 0 : 0
        if (decimals > precision) {
          callback(new Error(`数量精度不超过 ${precision} 位小数`))
          return
        }
        callback()
      },
      trigger: 'blur',
    },
  ],
}))

// Estimated total (limit orders)
const estimatedTotal = computed(() => {
  if (form.order_type !== 'limit') return null
  const price = parseFloat(form.price)
  const qty = parseFloat(form.quantity)
  if (isNaN(price) || isNaN(qty) || price <= 0 || qty <= 0) return null
  return price * qty
})

// Pre-fill price when side changes
watch(() => form.side, () => {
  if (form.order_type === 'limit') {
    form.price = form.side === 'buy'
      ? (props.bestAsk ?? '')
      : (props.bestBid ?? '')
  }
})

// Pre-fill price when bestBid/bestAsk changes
watch(() => props.bestAsk, (val) => {
  if (form.side === 'buy' && form.order_type === 'limit' && !form.price) {
    form.price = val ?? ''
  }
})

watch(() => props.bestBid, (val) => {
  if (form.side === 'sell' && form.order_type === 'limit' && !form.price) {
    form.price = val ?? ''
  }
})

function onOrderTypeChange() {
  if (form.order_type === 'market') {
    form.price = ''
    form.time_in_force = 'IOC'
  } else {
    form.time_in_force = 'GTC'
    form.price = form.side === 'buy'
      ? (props.bestAsk ?? '')
      : (props.bestBid ?? '')
  }
}

function setQuantityPercent(pct: number) {
  if (!props.symbolConfig) return
  if (form.side === 'buy') {
    const balance = parseFloat(props.account?.balance ?? '0')
    const price = form.order_type === 'market'
      ? parseFloat(props.bestAsk ?? '0')
      : parseFloat(form.price || '0')
    if (price <= 0) return
    const qty = (balance * pct / 100) / price
    const precision = props.symbolConfig.quantity_precision
    form.quantity = qty.toFixed(precision)
  } else {
    const available = parseFloat(props.availablePositionQty ?? '0')
    const qty = available * pct / 100
    const precision = props.symbolConfig.quantity_precision
    form.quantity = qty.toFixed(precision)
  }
}

function formatBalance(value: string | number | null | undefined): string {
  if (value === null || value === undefined) return '--'
  const num = typeof value === 'string' ? parseFloat(value) : value
  if (isNaN(num)) return '--'
  return new Intl.NumberFormat('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(num)
}

async function handleSubmit() {
  if (!formRef.value) return
  const valid = await formRef.value.validate().catch(() => false)
  if (!valid) return

  // Check minimum notional for limit orders
  if (form.order_type === 'limit' && estimatedTotal.value) {
    const minNotional = parseFloat(props.symbolConfig?.min_notional ?? '0')
    if (estimatedTotal.value < minNotional) {
      ElMessage.warning(`最小下单金额为 ${minNotional} ${props.symbolConfig?.quote_currency ?? 'USDT'}`)
      return
    }

    // Warn if price deviates significantly from market
    const marketPrice = form.side === 'buy'
      ? parseFloat(props.bestAsk ?? '0')
      : parseFloat(props.bestBid ?? '0')
    if (marketPrice > 0) {
      const deviation = Math.abs(parseFloat(form.price) - marketPrice) / marketPrice
      if (deviation > 0.05) {
        try {
          await ElMessageBox.confirm(
            `委托价格偏离当前市场价较大 (${(deviation * 100).toFixed(1)}%)，是否确认提交？`,
            '风险提示',
            { confirmButtonText: '确认提交', cancelButtonText: '取消', type: 'warning' },
          )
        } catch {
          return
        }
      }
    }
  }

  // Market order confirmation
  if (form.order_type === 'market') {
    try {
      await ElMessageBox.confirm(
        '市价单将按市场最优价成交，实际成交价可能有偏差',
        '确认提交',
        { confirmButtonText: '确认', cancelButtonText: '取消', type: 'info' },
      )
    } catch {
      return
    }
  }

  submitting.value = true
  try {
    const request: CreateOrderRequest = {
      symbol: props.symbol,
      side: form.side,
      order_type: form.order_type,
      quantity: form.quantity,
      time_in_force: form.time_in_force,
    }
    if (form.order_type === 'limit' && form.price) {
      request.price = form.price
    }

    const order = await createOrder(request)
    emit('submit', request)
    ElMessage.success('委托已提交')

    // Reset form
    form.quantity = ''
    if (form.order_type === 'limit') {
      form.price = form.side === 'buy'
        ? (props.bestAsk ?? '')
        : (props.bestBid ?? '')
    }
  } catch (err: unknown) {
    ElMessage.error((err as { message?: string })?.message || '委托提交失败')
  } finally {
    submitting.value = false
  }
}

// Expose for testing
defineExpose({
  form,
  formRef,
  submitting,
  estimatedTotal,
})
</script>

<style scoped lang="scss">
.order-form {
  padding: 20px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 12px;
  box-shadow: var(--shadow-card);
  transition: all var(--transition-base);

  &:hover {
    box-shadow: var(--shadow-card-hover);
  }
}

.side-toggle {
  display: flex;
  gap: 0;
  margin-bottom: 20px;
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--color-border);
  box-shadow: var(--shadow-sm);
}

.side-btn {
  flex: 1;
  padding: 10px 0;
  border: none;
  cursor: pointer;
  font-size: 14px;
  font-weight: 600;
  background: var(--color-surface-elevated);
  color: var(--color-text-secondary);
  transition: all var(--transition-fast);
  position: relative;
  overflow: hidden;

  &::before {
    content: '';
    position: absolute;
    top: 0;
    left: -100%;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent);
    transition: left 0.5s;
  }

  &:hover::before {
    left: 100%;
  }

  &.buy-btn.active {
    background: var(--gradient-success);
    color: #fff;
    box-shadow: 0 0 15px rgba(16, 185, 129, 0.4);
  }

  &.sell-btn.active {
    background: var(--gradient-danger);
    color: #fff;
    box-shadow: 0 0 15px rgba(229, 72, 77, 0.4);
  }
}

.form-item-compact {
  :deep(.el-form-item__label) {
    font-size: 13px;
    color: var(--color-text-secondary);
    padding-bottom: 6px;
    font-weight: 500;
  }
  margin-bottom: 16px;

  :deep(.el-input__wrapper) {
    background: var(--color-surface-elevated) !important;
    border-radius: 8px;
    transition: all var(--transition-fast);
    box-shadow: 0 0 0 1px var(--color-border) inset !important;

    &:hover {
      box-shadow: 0 0 0 1px var(--color-border-hover) inset !important;
    }

    &.is-focus {
      box-shadow:
        0 0 0 1px var(--color-accent) inset,
        0 0 12px rgba(113, 112, 255, 0.2) !important;
    }
  }

  :deep(.el-input-group__append) {
    background: var(--color-surface-elevated);
    border-color: var(--color-border);
    color: var(--color-text-tertiary);
    font-weight: 500;
  }
}

.price-hint {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--color-text-tertiary);
  margin-top: 6px;
  padding: 4px 8px;
  background: rgba(255, 255, 255, 0.02);
  border-radius: 4px;
  border: 1px solid var(--color-border);
}

.percent-buttons {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;

  .el-button {
    flex: 1;
    padding: 6px 0;
    font-size: 12px;
    font-weight: 600;
    border-radius: 6px;
    transition: all var(--transition-fast);

    &:hover {
      transform: translateY(-2px);
      box-shadow: var(--shadow-sm);
    }
  }
}

.balance-info,
.total-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  margin-bottom: 12px;
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.02);
  border-radius: 8px;
  border: 1px solid var(--color-border);
}

.balance-label,
.total-label {
  color: var(--color-text-secondary);
  font-weight: 500;
}

.balance-value,
.total-value {
  color: var(--color-text-primary);
  font-weight: 700;
  font-family: var(--font-mono);
  letter-spacing: -0.3px;
}

.form-item-submit {
  margin-bottom: 0;
  margin-top: 20px;
}

.submit-btn {
  width: 100%;
  font-weight: 700;
  font-size: 15px;
  padding: 12px 0;
  border-radius: 10px;
  letter-spacing: 0.5px;
  transition: all var(--transition-fast);
  position: relative;
  overflow: hidden;

  &::before {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    width: 0;
    height: 0;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.3);
    transform: translate(-50%, -50%);
    transition: width 0.6s, height 0.6s;
  }

  &:active::before {
    width: 300px;
    height: 300px;
  }

  &:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: var(--shadow-lg);
  }

  &:active:not(:disabled) {
    transform: translateY(0);
  }
}
</style>
