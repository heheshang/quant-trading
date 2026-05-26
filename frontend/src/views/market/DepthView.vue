<template>
  <div class="depth-view">
    <!-- Toolbar -->
    <div class="toolbar">
      <div class="toolbar-left">
        <span class="toolbar-label">交易对:</span>
        <el-select
          v-model="selectedSymbol"
          placeholder="选择交易对..."
          filterable
          class="symbol-select"
          @change="onSymbolChange"
        >
          <el-option
            v-for="s in symbols"
            :key="s"
            :label="formatSymbol(s)"
            :value="s"
          />
        </el-select>
      </div>
      <DepthLevelSelector
        v-model="selectedLevels"
        :user-role="userRole"
      />
    </div>

    <!-- Loading -->
    <div v-if="depthLoading" class="loading-skeleton" v-loading="true">
      <div v-for="i in 10" :key="i" class="skeleton-row"></div>
    </div>

    <!-- Depth Content -->
    <template v-else>
      <div v-if="depthError" class="depth-error">
        <el-alert :title="depthError" type="error" show-icon :closable="false" />
      </div>

      <template v-if="depth">
        <!-- OrderBook -->
        <OrderBookTable
          :depth="depth"
          :last-price="lastPrice"
          :change="tickerChange"
          :change-percent="tickerChangePercent"
          :highlighted-price="highlightedPrice"
          @price-hover="onPriceHover"
        />

        <!-- DepthChart -->
        <div class="chart-section">
          <DepthChart
            :depth="depth"
            :last-price="lastPrice"
          />
        </div>
      </template>

      <div v-else class="empty-depth">
        <el-icon :size="48" color="var(--color-text-tertiary)"><DataLine /></el-icon>
        <p>暂无深度数据</p>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { DataLine } from '@element-plus/icons-vue'
import type { Ticker, WsMessage, WsStatus } from '@/types'
import { formatSymbol } from '@/types/market'
import { useDepth } from '@/composables/useDepth'
import { getTicker } from '@/api/market'
import DepthLevelSelector from '@/components/market/DepthLevelSelector.vue'
import OrderBookTable from '@/components/market/OrderBookTable.vue'
import DepthChart from '@/components/market/DepthChart.vue'
import { ElMessage } from 'element-plus'

const props = defineProps<{
  wsStatus: WsStatus
  userRole?: string
  onWsMessage?: (handler: (msg: WsMessage) => void) => void
}>()

// Available symbols (will be populated from tickers)
const symbols = ref<string[]>(['BTCUSDT', 'ETHUSDT', 'BNBUSDT', 'SOLUSDT', 'XRPUSDT', 'DOGEUSDT', 'ADAUSDT', 'AVAXUSDT', 'DOTUSDT', 'LINKUSDT'])
const selectedSymbol = ref('BTCUSDT')
const selectedLevels = ref(10)
const highlightedPrice = ref<number | null>(null)

// Current ticker info for last price center
const lastPrice = ref<number>(0)
const tickerChange = ref(0)
const tickerChangePercent = ref(0)

const symbolRef = computed(() => selectedSymbol.value as string)
const levelsRef = computed(() => selectedLevels.value as number)

const { depth, loading: depthLoading, error: depthError, fetchDepth, handleSnapshot, handleUpdate } = useDepth(symbolRef, levelsRef)

function onSymbolChange() {
  // Fetch ticker info for the new symbol
  loadTickerInfo()
}

async function loadTickerInfo() {
  try {
    const ticker = await getTicker(selectedSymbol.value)
    lastPrice.value = ticker.price
    tickerChange.value = ticker.change
    tickerChangePercent.value = ticker.change_percent
  } catch {
    // Silently fail, depth data is primary
  }
}

function onPriceHover(price: number | null) {
  highlightedPrice.value = price
}

// Handle WS messages for depth
function handleWsMessage(msg: WsMessage) {
  if (msg.type === 'depth' && msg.symbol === selectedSymbol.value) {
    handleSnapshot(msg)
  } else if (msg.type === 'depth_update' && msg.symbol === selectedSymbol.value) {
    handleUpdate(msg)
  } else if (msg.type === 'ticker' && msg.data) {
    const ticker = msg.data as Ticker
    if (ticker.symbol === selectedSymbol.value) {
      lastPrice.value = ticker.price
      tickerChange.value = ticker.change
      tickerChangePercent.value = ticker.change_percent
    }
  }
}

onMounted(() => {
  fetchDepth()
  loadTickerInfo()
  props.onWsMessage?.(handleWsMessage)
})

defineExpose({ handleWsMessage })
</script>

<style scoped lang="scss">
.depth-view {
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    gap: 12px;
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .toolbar-label {
    font-size: 13px;
    color: var(--color-text-secondary);
  }

  .symbol-select {
    width: 180px;

    :deep(.el-input__wrapper) {
      background: var(--color-surface);
      border: 1px solid var(--color-border);
      box-shadow: none;
      height: 32px;

      &.is-focus {
        border-color: var(--color-accent);
      }
    }

    :deep(.el-input__inner) {
      color: var(--color-text-primary);
      font-size: 13px;
    }
  }

  .loading-skeleton {
    min-height: 300px;
    padding: 16px;

    .skeleton-row {
      height: 32px;
      margin-bottom: 4px;
      background: var(--color-surface);
      border-radius: 4px;
    }
  }

  .depth-error {
    padding: 16px 0;
  }

  .chart-section {
    margin-top: 16px;
  }

  .empty-depth {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 48px 0;
    color: var(--color-text-tertiary);

    p {
      font-size: 14px;
      margin-top: 12px;
    }
  }
}
</style>
