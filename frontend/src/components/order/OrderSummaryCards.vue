<template>
  <el-row :gutter="16" class="order-summary-cards">
    <!-- Active Orders Card -->
    <el-col :xs="24" :sm="12" :lg="6">
      <div class="summary-card" :class="{ 'summary-card--loading': loading }">
        <div class="summary-card__header">
          <span class="summary-card__label">活跃委托</span>
          <el-icon class="summary-card__icon" :size="24" color="var(--color-accent, #7170ff)">
            <List />
          </el-icon>
        </div>
        <template v-if="loading">
          <div class="skeleton-line" style="width: 60px; height: 28px;" />
          <div class="skeleton-line" style="width: 120px; height: 14px; margin-top: 8px;" />
        </template>
        <template v-else>
          <div class="summary-card__value">{{ activeOrdersCount }}</div>
          <div v-if="marginFrozen" class="summary-card__sub summary-card__sub--frozen">
            保证金 ¥{{ formatMoney(marginFrozen) }}
          </div>
        </template>
      </div>
    </el-col>

    <!-- Today Fills Card -->
    <el-col :xs="24" :sm="12" :lg="6">
      <div class="summary-card" :class="{ 'summary-card--loading': loading }">
        <div class="summary-card__header">
          <span class="summary-card__label">今日成交</span>
          <el-icon class="summary-card__icon" :size="24" color="var(--color-success, #10b981)">
            <CircleCheck />
          </el-icon>
        </div>
        <template v-if="loading">
          <div class="skeleton-line" style="width: 60px; height: 28px;" />
          <div class="skeleton-line" style="width: 120px; height: 14px; margin-top: 8px;" />
        </template>
        <template v-else>
          <div class="summary-card__value summary-card__value--filled">{{ todayFillsCount }}</div>
          <div v-if="todayFillAmount" class="summary-card__sub">
            成交额 ¥{{ formatMoney(todayFillAmount) }}
          </div>
        </template>
      </div>
    </el-col>

    <!-- Frozen Balance Card -->
    <el-col :xs="24" :sm="12" :lg="6">
      <div class="summary-card" :class="{ 'summary-card--loading': loading }">
        <div class="summary-card__header">
          <span class="summary-card__label">冻结保证金</span>
          <el-icon class="summary-card__icon" :size="24" color="var(--color-frozen, #f5a623)">
            <Lock />
          </el-icon>
        </div>
        <template v-if="loading">
          <div class="skeleton-line" style="width: 100px; height: 28px;" />
          <div class="skeleton-line" style="width: 80px; height: 14px; margin-top: 8px;" />
        </template>
        <template v-else>
          <div class="summary-card__value summary-card__value--frozen">¥{{ formatMoney(frozenBalance) }}</div>
          <div v-if="equity" class="summary-card__sub summary-card__sub--tertiary">
            占总权益 {{ frozenPercent }}%
          </div>
        </template>
      </div>
    </el-col>

    <!-- Available Balance Card -->
    <el-col :xs="24" :sm="12" :lg="6">
      <div class="summary-card" :class="{ 'summary-card--loading': loading }">
        <div class="summary-card__header">
          <span class="summary-card__label">可用余额</span>
          <el-icon class="summary-card__icon" :size="24" color="var(--color-accent, #7170ff)">
            <Wallet />
          </el-icon>
        </div>
        <template v-if="loading">
          <div class="skeleton-line" style="width: 100px; height: 28px;" />
          <div class="skeleton-line" style="width: 80px; height: 14px; margin-top: 8px;" />
        </template>
        <template v-else>
          <div class="summary-card__value">¥{{ formatMoney(availableBalance) }}</div>
          <div v-if="equity" class="summary-card__sub">
            权益 ¥{{ formatMoney(equity) }}
          </div>
        </template>
      </div>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { List, CircleCheck, Lock, Wallet } from '@element-plus/icons-vue'
import type { PaperAccount } from '@/types/order'

/**
 * OrderSummaryCards - Four summary statistic cards for the order management page.
 * Shows active orders count, today's fills, frozen balance, and available balance.
 * Follows Design_OrderManagement.md section 2.3 specs.
 */

const props = withDefaults(defineProps<{
  /** Current paper account data */
  account: PaperAccount | null
  /** Number of active (pending + partial_filled) orders */
  activeOrdersCount?: number
  /** Number of today's fill notifications */
  todayFillsCount?: number
  /** Today's total fill amount in USDT */
  todayFillAmount?: string
  /** Frozen margin from active orders */
  marginFrozen?: string
  /** Loading state */
  loading?: boolean
}>(), {
  activeOrdersCount: 0,
  todayFillsCount: 0,
  todayFillAmount: '',
  marginFrozen: '',
  loading: false,
})

/** Available balance from account */
const availableBalance = computed(() => props.account?.balance ?? '0')

/** Frozen balance from account */
const frozenBalance = computed(() => props.account?.frozen_balance ?? '0')

/** Equity from account */
const equity = computed(() => props.account?.equity ?? '')

/** Frozen as percentage of equity */
const frozenPercent = computed(() => {
  if (!props.account?.equity || !props.account?.frozen_balance) return '0'
  const eq = parseFloat(props.account.equity)
  const fr = parseFloat(props.account.frozen_balance)
  if (eq === 0) return '0'
  return ((fr / eq) * 100).toFixed(1)
})

/**
 * Format money value with thousand separators and 2 decimal places.
 * @param value - Decimal string or number
 */
function formatMoney(value: string | number | undefined): string {
  if (!value && value !== 0) return '0.00'
  const num = typeof value === 'string' ? parseFloat(value) : value
  if (isNaN(num)) return '0.00'
  return num.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}
</script>

<style scoped lang="scss">
.order-summary-cards {
  margin-bottom: 16px;
}

.summary-card {
  background: var(--color-surface, #191a1b);
  border: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
  border-radius: 12px;
  padding: 20px;
  height: 100%;
}

.summary-card__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.summary-card__label {
  font-size: 13px;
  color: var(--color-text-tertiary, #8a8f98);
}

.summary-card__icon {
  opacity: 0.4;
}

.summary-card__value {
  font-size: 28px;
  font-weight: 700;
  color: var(--color-text-primary, #f7f8f8);
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  line-height: 1.2;

  &--filled {
    color: var(--color-filled, #67C23A);
  }

  &--frozen {
    color: var(--color-frozen, #f5a623);
  }
}

.summary-card__sub {
  font-size: 12px;
  color: var(--color-text-secondary, #d0d6e0);
  margin-top: 4px;

  &--frozen {
    color: var(--color-frozen, #f5a623);
  }

  &--tertiary {
    color: var(--color-text-tertiary, #8a8f98);
  }
}

.skeleton-line {
  background: var(--color-surface, #191a1b);
  border-radius: 4px;
}

.summary-card--loading {
  .summary-card__header {
    margin-bottom: 16px;
  }
}
</style>
