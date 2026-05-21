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
  padding: 16px;
  background: var(--color-surface, #0E1223);
  border: 1px solid var(--color-border, #334155);
  border-radius: 8px;
}

.side-toggle {
  display: flex;
  gap: 0;
  margin-bottom: 16px;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid var(--color-border, #334155);
}

.side-btn {
  flex: 1;
  padding: 8px 0;
  border: none;
  cursor: pointer;
  font-size: 14px;
  font-weight: 600;
  background: var(--color-muted, #1A1E2F);
  color: var(--color-text-secondary, #94A3B8);
  transition: all 0.2s;

  &.buy-btn.active {
    background: #67C23A;
    color: #fff;
  }

  &.sell-btn.active {
    background: #F56C6C;
    color: #fff;
  }
}

.form-item-compact {
  :deep(.el-form-item__label) {
    font-size: 12px;
    color: var(--color-text-secondary, #94A3B8);
    padding-bottom: 4px;
  }
  margin-bottom: 12px;
}

.price-hint {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--color-text-tertiary, #64748B);
  margin-top: 4px;
}

.percent-buttons {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;

  .el-button {
    flex: 1;
    padding: 4px 0;
  }
}

.balance-info,
.total-info {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  margin-bottom: 8px;
}

.balance-label,
.total-label {
  color: var(--color-text-secondary, #94A3B8);
}

.balance-value,
.total-value {
  color: var(--color-text-primary, #F8FAFC);
  font-weight: 500;
}

.form-item-submit {
  margin-bottom: 0;
  margin-top: 16px;
}

.submit-btn {
  width: 100%;
  font-weight: 600;
  font-size: 15px;
  padding: 10px 0;
}
</style>
