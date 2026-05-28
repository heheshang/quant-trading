<template>
  <div class="ticker-table-wrapper">
    <el-table
      :data="data"
      :default-sort="{ prop: 'change_percent', order: 'descending' }"
      stripe
      style="width: 100%"
      :header-cell-style="headerCellStyle"
      :cell-style="cellStyle"
      @sort-change="onSortChange"
      empty-text="暂无行情数据"
    >
      <el-table-column
        prop="symbol"
        label="交易对"
        width="140"
        sortable="custom"
      >
        <template #default="{ row }">
          <div class="symbol-cell">
            <span class="symbol-name">{{ formatSymbol(row.symbol) }}</span>
            <span class="symbol-fullname">{{ getSymbolName(row.symbol) }}</span>
          </div>
        </template>
      </el-table-column>

      <el-table-column
        prop="price"
        label="最新价"
        width="140"
        sortable="custom"
        align="right"
      >
        <template #default="{ row }">
          <span
            class="price-cell"
            :class="getFlashClass(row.symbol)"
          >
            {{ formatNumber(row.price) }}
          </span>
        </template>
      </el-table-column>

      <el-table-column
        prop="change_percent"
        label="24h涨跌幅"
        width="120"
        sortable="custom"
        align="right"
      >
        <template #default="{ row }">
          <span :class="getChangeClass(row.change)">
            {{ formatPercent(row.change_percent) }}
            <span class="change-arrow">{{ getArrow(row.change) }}</span>
          </span>
        </template>
      </el-table-column>

      <el-table-column
        prop="high"
        label="24h最高"
        width="120"
        sortable="custom"
        align="right"
        class-name="hide-on-mobile"
      >
        <template #default="{ row }">
          <span class="mono">{{ formatNumber(row.high) }}</span>
        </template>
      </el-table-column>

      <el-table-column
        prop="low"
        label="24h最低"
        width="120"
        sortable="custom"
        align="right"
        class-name="hide-on-mobile"
      >
        <template #default="{ row }">
          <span class="mono">{{ formatNumber(row.low) }}</span>
        </template>
      </el-table-column>

      <el-table-column
        prop="volume"
        label="24h成交量"
        width="130"
        sortable="custom"
        align="right"
      >
        <template #default="{ row }">
          <span class="mono">{{ formatVolume(row.volume) }}</span>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup lang="ts">
import type { Ticker } from '@/types'
import { formatSymbol, SYMBOL_NAMES } from '@/types/market'
import { useFormat } from '@/composables/useFormat'

const props = defineProps<{
  data: Ticker[]
  flashMap?: Record<string, 'flash-buy' | 'flash-sell' | ''>
}>()

const emit = defineEmits<{
  sortChange: [prop: string, order: string]
}>()

const { formatPercent, formatNumber: fmtNumber } = useFormat()

function formatNumber(value: number): string {
  return fmtNumber(value, 2)
}

function formatVolume(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(2)}K`
  return fmtNumber(value, 2)
}

function getSymbolName(symbol: string): string {
  return SYMBOL_NAMES[symbol] || ''
}

function getChangeClass(change: number): string {
  if (change > 0) return 'change-up'
  if (change < 0) return 'change-down'
  return 'change-flat'
}

function getArrow(change: number): string {
  if (change > 0) return '▲'
  if (change < 0) return '▼'
  return ''
}

function getFlashClass(symbol: string): string {
  return props.flashMap?.[symbol] || ''
}

function onSortChange({ prop, order }: { prop: string; order: string | null }) {
  emit('sortChange', prop, order || '')
}

const headerCellStyle = {
  background: 'var(--color-surface)',
  color: 'var(--color-text-tertiary)',
  fontSize: '12px',
  fontWeight: 500,
  borderBottom: '1px solid var(--color-border)',
}

const cellStyle = {
  borderBottom: '1px solid var(--color-border)',
}

defineExpose({
  formatSymbol,
  getFlashClass,
  formatVolume,
  getChangeClass,
  getArrow,
})
</script>

<style scoped lang="scss">
.ticker-table-wrapper {
  :deep(.el-table) {
    --el-table-bg-color: transparent;
    --el-table-tr-bg-color: transparent;
    --el-table-header-bg-color: var(--color-surface);
    --el-table-row-hover-bg-color: rgba(113, 112, 255, 0.05);
    --el-table-border-color: var(--color-border);
    --el-table-text-color: var(--color-text-primary);
    --el-table-header-text-color: var(--color-text-secondary);
    font-size: 14px;
    border-radius: 8px;
    overflow: hidden;

    .el-table__row {
      height: 52px;
      transition: all var(--transition-fast);

      &:hover {
        transform: scale(1.005);
      }
    }

    th.el-table__cell {
      background: var(--color-surface-elevated) !important;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      font-size: 11px;
    }
  }

  :deep(.el-table__body tr:hover > td) {
    background: var(--color-ticker-row-hover) !important;
  }
}

.symbol-cell {
  display: flex;
  flex-direction: column;
  line-height: 1.4;
  gap: 2px;

  .symbol-name {
    font-weight: 700;
    color: var(--color-text-primary);
    font-size: 14px;
    letter-spacing: -0.2px;
  }
  .symbol-fullname {
    font-size: 11px;
    color: var(--color-text-tertiary);
    font-weight: 500;
  }
}

.mono {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-weight: 600;
}

.price-cell {
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-weight: 700;
  font-size: 14px;
  transition: all var(--transition-fast);
  padding: 4px 8px;
  border-radius: 4px;
  letter-spacing: -0.3px;

  &.flash-buy {
    animation: flash-buy 0.6s ease-out forwards;
  }
  &.flash-sell {
    animation: flash-sell 0.6s ease-out forwards;
  }
}

.change-up {
  color: var(--color-buy);
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  gap: 4px;

  .change-arrow {
    font-size: 10px;
    filter: drop-shadow(0 0 4px rgba(16, 185, 129, 0.5));
  }
}

.change-down {
  color: var(--color-sell);
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  gap: 4px;

  .change-arrow {
    font-size: 10px;
    filter: drop-shadow(0 0 4px rgba(229, 72, 77, 0.5));
  }
}

.change-flat {
  color: var(--color-text-secondary);
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  font-weight: 600;
}

@keyframes flash-buy {
  0% {
    background-color: rgba(16, 185, 129, 0.4);
    color: var(--color-buy);
    transform: scale(1.05);
  }
  100% {
    background-color: transparent;
    color: var(--color-text-primary);
    transform: scale(1);
  }
}

@keyframes flash-sell {
  0% {
    background-color: rgba(229, 72, 77, 0.4);
    color: var(--color-sell);
    transform: scale(1.05);
  }
  100% {
    background-color: transparent;
    color: var(--color-text-primary);
    transform: scale(1);
  }
}

// Responsive: hide high/low on mobile
@media (max-width: 767px) {
  :deep(.hide-on-mobile) {
    display: none;
  }
}
</style>
