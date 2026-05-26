<template>
  <div class="ticker-list-view">
    <!-- Toolbar -->
    <div class="toolbar">
      <TickerSearchBar @search="onSearch" />
      <ConnectionStatus :status="wsStatus" />
    </div>

    <!-- Table -->
    <div class="table-wrapper" v-loading="loading">
      <TickerTable
        :data="filteredTickers"
        :flash-map="flashMap"
        @sort-change="onSortChange"
      />
    </div>

    <!-- Empty state for search -->
    <div v-if="!loading && searchQuery && filteredTickers.length === 0" class="empty-search">
      <el-icon :size="48" color="var(--color-text-tertiary)"><Search /></el-icon>
      <p class="empty-title">未找到匹配的交易对</p>
      <p class="empty-subtitle">请尝试其他关键词</p>
    </div>

    <!-- Empty state for no data -->
    <div v-if="!loading && !searchQuery && tickers.length === 0" class="empty-data">
      <el-icon :size="48" color="var(--color-text-tertiary)"><TrendCharts /></el-icon>
      <p class="empty-title">暂无行情数据</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { Search, TrendCharts } from '@element-plus/icons-vue'
import type { Ticker, WsMessage, WsStatus } from '@/types'
import { getTickers } from '@/api/market'
import { usePriceFlash } from '@/composables/usePriceFlash'
import TickerTable from '@/components/market/TickerTable.vue'
import TickerSearchBar from '@/components/market/TickerSearchBar.vue'
import ConnectionStatus from '@/components/market/ConnectionStatus.vue'

const props = defineProps<{
  wsStatus: WsStatus
  onWsMessage?: (handler: (msg: WsMessage) => void) => void
}>()

const tickers = ref<Ticker[]>([])
const loading = ref(false)
const searchQuery = ref('')
const sortProp = ref('change_percent')
const sortOrder = ref<string>('descending')
const flashMap = ref<Record<string, 'flash-buy' | 'flash-sell' | ''>>({})
const previousPrices = ref<Record<string, number>>({})

// Per-symbol price flash (创建多个 usePriceFlash 不可行，使用 map)
function triggerPriceFlash(symbol: string, newPrice: number) {
  const oldPrice = previousPrices.value[symbol]
  if (oldPrice !== undefined && newPrice !== oldPrice) {
    flashMap.value[symbol] = newPrice > oldPrice ? 'flash-buy' : 'flash-sell'
    // 500ms 后清除
    setTimeout(() => {
      flashMap.value[symbol] = ''
    }, 500)
  }
  previousPrices.value[symbol] = newPrice
}

const filteredTickers = computed(() => {
  let result = [...tickers.value]

  // 搜索过滤
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(t =>
      t.symbol.toLowerCase().includes(q)
    )
  }

  // 排序
  if (sortProp.value) {
    const prop = sortProp.value as keyof Ticker
    const order = sortOrder.value === 'ascending' ? 1 : -1
    result.sort((a, b) => {
      const aVal = a[prop] as number
      const bVal = b[prop] as number
      return (aVal - bVal) * order
    })
  }

  return result
})

async function fetchTickers() {
  loading.value = true
  try {
    const data = await getTickers()
    tickers.value = data
    // 初始化 previousPrices
    data.forEach((t: Ticker) => {
      previousPrices.value[t.symbol] = t.price
    })
  } catch (e: any) {
    console.error('获取行情数据失败:', e.message)
  } finally {
    loading.value = false
  }
}

function onSearch(value: string) {
  searchQuery.value = value
}

function onSortChange(prop: string, order: string) {
  sortProp.value = prop
  sortOrder.value = order
}

// 处理 WS ticker 消息
function handleWsMessage(msg: WsMessage) {
  if (msg.type === 'ticker' && msg.data) {
    const ticker = msg.data as Ticker
    const idx = tickers.value.findIndex(t => t.symbol === ticker.symbol)
    if (idx >= 0) {
      // 触发闪烁
      triggerPriceFlash(ticker.symbol, ticker.price)
      // 更新数据
      tickers.value[idx] = { ...tickers.value[idx], ...ticker }
    }
  }
}

onMounted(() => {
  fetchTickers()
  props.onWsMessage?.(handleWsMessage)
})

// Expose handleWsMessage for parent to route messages
defineExpose({ handleWsMessage })
</script>

<style scoped lang="scss">
.ticker-list-view {
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    gap: 12px;
  }

  .table-wrapper {
    min-height: 200px;
  }

  .empty-search,
  .empty-data {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 48px 0;
    color: var(--color-text-tertiary);

    .empty-title {
      font-size: 14px;
      margin-top: 12px;
    }
    .empty-subtitle {
      font-size: 12px;
      margin-top: 4px;
    }
  }
}
</style>
