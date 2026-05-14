<template>
  <div class="last-price-center" v-if="lastPrice !== undefined && lastPrice !== null">
    <div class="price-main" :class="direction">
      {{ formatNumber(lastPrice) }}
    </div>
    <div class="price-change" :class="direction">
      {{ formatChangeValue(change) }} ({{ formatPercentValue(changePercent) }})
      <span class="change-arrow">{{ change > 0 ? '▲' : change < 0 ? '▼' : '' }}</span>
    </div>
    <div class="spread-row">
      <span class="spread-label">买一</span>
      <span class="spread-value mono">{{ formatNumber(bidPrice) }}</span>
      <span class="spread-divider">│</span>
      <span class="spread-label">价差</span>
      <span class="spread-value mono">{{ formatNumber(spread) }}</span>
      <span class="spread-divider">│</span>
      <span class="spread-label">卖一</span>
      <span class="spread-value mono">{{ formatNumber(askPrice) }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useFormat } from '@/composables/useFormat'

const props = defineProps<{
  lastPrice: number
  change: number
  changePercent: number
  bidPrice: number
  askPrice: number
}>()

const { formatNumber: fmtNumber, formatPercent } = useFormat()

const direction = computed(() => {
  if (props.change > 0) return 'up'
  if (props.change < 0) return 'down'
  return ''
})

const spread = computed(() => Math.abs(props.askPrice - props.bidPrice))

function formatNumber(value: number): string {
  return fmtNumber(value, 2)
}

function formatChangeValue(value: number): string {
  const sign = value > 0 ? '+' : ''
  return `${sign}${fmtNumber(value, 2)}`
}

function formatPercentValue(value: number): string {
  return formatPercent(value)
}
</script>

<style scoped lang="scss">
.last-price-center {
  text-align: center;
  padding: 16px 12px;
  background: var(--color-deep-bg, #050607);
  border: 1px solid var(--color-border);
  border-radius: 8px;

  .price-main {
    font-size: 24px;
    font-weight: 700;
    font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
    color: var(--color-text-primary);

    &.up { color: var(--color-buy); }
    &.down { color: var(--color-sell); }
  }

  .price-change {
    font-size: 14px;
    font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
    margin-top: 4px;
    color: var(--color-text-secondary);

    &.up { color: var(--color-buy); }
    &.down { color: var(--color-sell); }

    .change-arrow { font-size: 10px; }
  }

  .spread-row {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    font-size: 12px;
    color: var(--color-text-tertiary);

    .spread-value {
      font-size: 13px;
    }
    .spread-divider {
      color: var(--color-border);
    }
  }
}

.mono {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
}
</style>
