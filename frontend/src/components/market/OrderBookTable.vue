<template>
  <div class="orderbook-container">
    <!-- 卖盘 (红色, 左侧) -->
    <div class="orderbook-side asks-side">
      <div class="orderbook-header">
        <span>价格</span>
        <span>数量</span>
        <span>累计量</span>
      </div>
      <div class="orderbook-rows">
        <div
          v-for="(level, idx) in displayAsks"
          :key="'ask-' + idx"
          class="orderbook-row ask-row"
          :class="{ highlighted: highlightedPrice === level.price }"
        >
          <span class="depth-bar sell-bar" :style="{ width: getBarWidth(level.quantity, maxAskQty) + '%' }"></span>
          <span class="row-cell price sell-price">{{ formatDepthNumber(level.price) }}</span>
          <span class="row-cell">{{ formatDepthNumber(level.quantity) }}</span>
          <span class="row-cell secondary">{{ formatDepthNumber(level.total) }}</span>
        </div>
      </div>
    </div>

    <!-- 最新价格中心区 -->
    <div class="last-price-center" v-if="lastPrice !== null">
      <div class="price-main" :class="priceDirection">
        {{ formatDepthNumber(lastPrice) }}
      </div>
      <div class="price-change" :class="priceDirection">
        {{ formatChange(change) }} ({{ formatPercent(changePercent) }})
        <span class="change-arrow">{{ (change ?? 0) > 0 ? '▲' : (change ?? 0) < 0 ? '▼' : '' }}</span>
      </div>
      <div class="spread-row">
        <span class="spread-label">买一</span>
        <span class="spread-value mono">{{ formatDepthNumber(bidPrice) }}</span>
        <span class="spread-divider">│</span>
        <span class="spread-label">价差</span>
        <span class="spread-value mono">{{ formatDepthNumber(spread) }}</span>
        <span class="spread-divider">│</span>
        <span class="spread-label">卖一</span>
        <span class="spread-value mono">{{ formatDepthNumber(askPrice) }}</span>
      </div>
    </div>

    <!-- 买盘 (绿色, 右侧) -->
    <div class="orderbook-side bids-side">
      <div class="orderbook-header">
        <span>累计量</span>
        <span>数量</span>
        <span>价格</span>
      </div>
      <div class="orderbook-rows">
        <div
          v-for="(level, idx) in displayBids"
          :key="'bid-' + idx"
          class="orderbook-row bid-row"
          :class="{ highlighted: highlightedPrice === level.price }"
        >
          <span class="depth-bar buy-bar" :style="{ width: getBarWidth(level.quantity, maxBidQty) + '%' }"></span>
          <span class="row-cell secondary">{{ formatDepthNumber(level.total) }}</span>
          <span class="row-cell">{{ formatDepthNumber(level.quantity) }}</span>
          <span class="row-cell price buy-price">{{ formatDepthNumber(level.price) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Depth, DepthLevel } from '@/types'
import { useFormat } from '@/composables/useFormat'

const props = defineProps<{
  depth: Depth | null
  lastPrice?: number
  change?: number
  changePercent?: number
  highlightedPrice?: number | null
}>()

const { formatPercent: fmtPercent, formatNumber: fmtNumber } = useFormat()

function formatDepthNumber(value: number | undefined): string {
  if (value === undefined || value === null) return '--'
  return fmtNumber(value, 2)
}

function formatChange(value: number | undefined): string {
  if (value === undefined || value === null) return '--'
  const sign = value > 0 ? '+' : ''
  return `${sign}${fmtNumber(value, 2)}`
}

function formatPercent(value: number | undefined): string {
  if (value === undefined || value === null) return '--'
  return fmtPercent(value)
}

// 卖盘展示：价格升序，最上面是远端高价，最下面是最优卖一价
const displayAsks = computed(() => {
  if (!props.depth?.asks) return []
  // asks 已按价格升序排列，反转显示（远端在上）
  return [...props.depth.asks].reverse()
})

// 买盘展示：价格降序，最上面是最优买一价
const displayBids = computed(() => {
  if (!props.depth?.bids) return []
  return props.depth.bids
})

const maxAskQty = computed(() => {
  if (!props.depth?.asks?.length) return 1
  return Math.max(...props.depth.asks.map(l => l.quantity), 1)
})

const maxBidQty = computed(() => {
  if (!props.depth?.bids?.length) return 1
  return Math.max(...props.depth.bids.map(l => l.quantity), 1)
})

function getBarWidth(quantity: number, maxQty: number): number {
  return (quantity / maxQty) * 100
}

// 价格方向
const priceDirection = computed(() => {
  if (!props.change) return ''
  return props.change > 0 ? 'up' : props.change < 0 ? 'down' : ''
})

// 买一卖一价格
const bidPrice = computed(() => props.depth?.bids?.[0]?.price ?? 0)
const askPrice = computed(() => props.depth?.asks?.[0]?.price ?? 0)
const spread = computed(() => askPrice.value - bidPrice.value)
</script>

<style scoped lang="scss">
.orderbook-container {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0;
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-size: 13px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  overflow: hidden;
}

.orderbook-side {
  padding: 0;
}

.orderbook-header {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  padding: 8px 12px;
  font-size: 12px;
  color: var(--color-text-tertiary);
  font-weight: 500;
  border-bottom: 1px solid var(--color-border);
}

.asks-side .orderbook-header span:nth-child(1),
.asks-side .orderbook-header span:nth-child(2),
.asks-side .orderbook-header span:nth-child(3) {
  text-align: right;
}

.bids-side .orderbook-header span:nth-child(1),
.bids-side .orderbook-header span:nth-child(2) {
  text-align: left;
}
.bids-side .orderbook-header span:nth-child(3) {
  text-align: left;
}

.orderbook-rows {
  position: relative;
}

.orderbook-row {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  padding: 4px 12px;
  height: 32px;
  align-items: center;
  position: relative;
  transition: background 0.15s;

  &:hover {
    background: var(--color-surface-elevated);
  }

  &.highlighted {
    background: var(--color-surface-elevated);
  }
}

.asks-side .row-cell {
  text-align: right;
}
.bids-side .row-cell {
  text-align: left;
}

.row-cell {
  z-index: 1;
  color: var(--color-text-primary);

  &.secondary {
    color: var(--color-text-secondary);
  }
}

.sell-price {
  color: var(--color-sell);
}
.buy-price {
  color: var(--color-buy);
}

.depth-bar {
  position: absolute;
  top: 0;
  bottom: 0;
  z-index: 0;
  transition: width 0.3s ease-out;
}

.sell-bar {
  right: 0;
  background: var(--color-sell-bar, rgba(229, 72, 77, 0.25));
}

.buy-bar {
  left: 0;
  background: var(--color-buy-bar, rgba(16, 185, 129, 0.25));
}

// 中间价格区
.last-price-center {
  grid-column: 1 / -1;
  text-align: center;
  padding: 16px 12px;
  border-top: 1px solid var(--color-border);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-deep-bg, #050607);

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

    .spread-label { }
    .spread-value { font-size: 13px; }
    .spread-divider { color: var(--color-border); }
  }
}

.mono {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
}

// Mobile: 上下堆叠
@media (max-width: 767px) {
  .orderbook-container {
    grid-template-columns: 1fr;
  }
  .last-price-center {
    order: -1;
  }
}
</style>
