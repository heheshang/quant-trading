<template>
  <div class="watchlist-panel">
    <div class="panel-header">
      <h3 class="panel-title">
        <el-icon><Star /></el-icon>
        自选列表
      </h3>
      <button class="add-btn" @click="showAddDialog = true" title="添加交易对">
        <el-icon><Plus /></el-icon>
      </button>
    </div>

    <div class="watchlist-items" v-if="items && items.length > 0">
      <div
        v-for="item in items"
        :key="item.symbol"
        class="watchlist-item"
        :class="{ active: item.symbol === activeSymbol }"
        @click="onSelect(item.symbol)"
      >
        <div class="item-left">
          <span class="item-symbol">{{ formatSymbol(item.symbol) }}</span>
          <span class="item-name">{{ getSymbolName(item.symbol) }}</span>
        </div>
        <div class="item-right">
          <span class="item-price" :class="getPriceClass(item.change)">
            {{ formatNumber(item.price) }}
          </span>
          <span class="item-change" :class="getPriceClass(item.change)">
            {{ item.change >= 0 ? '+' : '' }}{{ formatPercent(item.change_percent) }}
          </span>
        </div>
        <button class="remove-btn" @click.stop="removeItem(item.symbol)" title="移除">
          <el-icon><Close /></el-icon>
        </button>
      </div>
    </div>

    <div class="empty-state" v-else>
      <el-icon :size="32" color="var(--color-text-tertiary)"><Star /></el-icon>
      <p class="empty-text">暂无自选交易对</p>
      <el-button size="small" type="primary" @click="showAddDialog = true">
        添加自选
      </el-button>
    </div>

    <!-- Add Symbol Dialog -->
    <el-dialog
      v-model="showAddDialog"
      title="添加自选交易对"
      width="400px"
      :close-on-click-modal="true"
    >
      <div class="add-dialog-content">
        <el-select
          v-model="selectedSymbol"
          filterable
          placeholder="选择交易对"
          size="large"
          style="width: 100%"
        >
          <el-option
            v-for="s in availableSymbols"
            :key="s.symbol"
            :label="`${s.display_name} (${s.symbol})`"
            :value="s.symbol"
          />
        </el-select>
      </div>
      <template #footer>
        <el-button @click="showAddDialog = false">取消</el-button>
        <el-button type="primary" @click="addSymbol" :disabled="!selectedSymbol">
          添加
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Star, Plus, Close } from '@element-plus/icons-vue'
import { formatSymbol, SYMBOL_NAMES, type Ticker } from '@/types'

export interface WatchlistItem {
  symbol: string
  price: number
  change: number
  change_percent: number
}

const props = defineProps<{
  items?: WatchlistItem[]
  activeSymbol?: string
}>()

const emit = defineEmits<{
  select: [symbol: string]
  add: [symbol: string]
  remove: [symbol: string]
}>()

const showAddDialog = ref(false)
const selectedSymbol = ref('')

// Predefined available symbols for selection
const availableSymbols = [
  { symbol: 'BTCUSDT', display_name: 'Bitcoin' },
  { symbol: 'ETHUSDT', display_name: 'Ethereum' },
  { symbol: 'BNBUSDT', display_name: 'BNB' },
  { symbol: 'SOLUSDT', display_name: 'Solana' },
  { symbol: 'XRPUSDT', display_name: 'XRP' },
  { symbol: 'DOGEUSDT', display_name: 'Dogecoin' },
  { symbol: 'ADAUSDT', display_name: 'Cardano' },
  { symbol: 'AVAXUSDT', display_name: 'Avalanche' },
  { symbol: 'DOTUSDT', display_name: 'Polkadot' },
  { symbol: 'LINKUSDT', display_name: 'Chainlink' },
]

function getSymbolName(symbol: string): string {
  return SYMBOL_NAMES[symbol] || ''
}

function formatNumber(value: number): string {
  return value.toFixed(2)
}

function formatPercent(value: number): string {
  return value.toFixed(2) + '%'
}

function getPriceClass(change: number): string {
  if (change > 0) return 'price-up'
  if (change < 0) return 'price-down'
  return ''
}

function onSelect(symbol: string) {
  emit('select', symbol)
}

function addSymbol() {
  if (selectedSymbol.value) {
    emit('add', selectedSymbol.value)
    selectedSymbol.value = ''
    showAddDialog.value = false
  }
}

function removeItem(symbol: string) {
  emit('remove', symbol)
}

defineExpose({
  showAddDialog,
  selectedSymbol,
  formatSymbol,
  getSymbolName,
  formatNumber,
  formatPercent,
  getPriceClass,
  onSelect,
  addSymbol,
  removeItem,
})
</script>

<style scoped lang="scss">
.watchlist-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--color-surface);
  border-radius: 8px;
  border: 1px solid var(--color-border);
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);

  .panel-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;
    font-weight: 600;
    color: var(--color-text-primary);
    margin: 0;

    .el-icon {
      color: var(--color-accent);
    }
  }

  .add-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: none;
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.2s;

    &:hover {
      background: var(--color-surface-elevated);
      color: var(--color-accent);
    }
  }
}

.watchlist-items {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.watchlist-item {
  display: flex;
  align-items: center;
  padding: 10px 16px;
  cursor: pointer;
  transition: background 0.15s;
  position: relative;

  &:hover {
    background: var(--color-surface-elevated);

    .remove-btn {
      opacity: 1;
    }
  }

  &.active {
    background: rgba(113, 112, 255, 0.1);
    border-left: 2px solid var(--color-accent);
  }

  .item-left {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;

    .item-symbol {
      font-size: 13px;
      font-weight: 600;
      color: var(--color-text-primary);
    }

    .item-name {
      font-size: 11px;
      color: var(--color-text-tertiary);
    }
  }

  .item-right {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;

    .item-price {
      font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
      font-size: 13px;
      font-weight: 500;
      color: var(--color-text-primary);
    }

    .item-change {
      font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
      font-size: 11px;
    }
  }

  .remove-btn {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: none;
    background: transparent;
    color: var(--color-text-tertiary);
    cursor: pointer;
    border-radius: 4px;
    opacity: 0;
    transition: all 0.2s;

    &:hover {
      background: rgba(229, 72, 77, 0.2);
      color: var(--color-sell);
    }
  }
}

.price-up {
  color: var(--color-buy) !important;
}

.price-down {
  color: var(--color-sell) !important;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 32px 16px;
  gap: 12px;

  .empty-text {
    font-size: 13px;
    color: var(--color-text-tertiary);
    margin: 0;
  }
}

.add-dialog-content {
  padding: 8px 0;
}
</style>
