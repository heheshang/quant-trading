<template>
  <div class="backtest-kline-chart">
    <div class="chart-header">
      <span class="symbol-tag">{{ symbol }}</span>
      <span class="interval-tag">{{ interval }}</span>
      <span class="trade-count">{{ markers.length / 2 }} 笔交易</span>
    </div>
    <KlineChart
      ref="chartRef"
      :data="klineData"
      :symbol="symbol"
      :interval="interval"
      :dark-mode="true"
      :markers="markers"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import KlineChart, { type KlineBar } from '@/components/charts/KlineChart.vue'
import type { TradeMarker } from '@/components/charts/KlineChart.vue'
import type { TradeRecord, BacktestResultResponse } from '@/types/backtest'
import { queryKlines } from '@/api/kline'

const props = defineProps<{
  backtestResult: BacktestResultResponse
}>()

const chartRef = ref<InstanceType<typeof KlineChart> | null>(null)
const klineData = ref<KlineBar[]>([])

const symbol = computed(() => props.backtestResult.config.symbol)
const interval = computed(() => props.backtestResult.config.interval)

/**
 * Convert TradeRecord[] → TradeMarker[] for KlineChart markers prop.
 * Entry (buy) → green arrowUp below bar
 * Exit (sell) → red arrowDown above bar
 */
const markers = computed<TradeMarker[]>(() => {
  return props.backtestResult.trades.flatMap((trade: TradeRecord): TradeMarker[] => {
    const entryTime = Math.floor(new Date(trade.entry_time).getTime() / 1000)
    const exitTime = Math.floor(new Date(trade.exit_time).getTime() / 1000)
    const isLong = trade.direction === 'long'

    const entryMarker: TradeMarker = {
      time: entryTime,
      position: 'belowBar',
      color: '#67c23a',
      shape: 'arrowUp',
      text: isLong ? '多入场' : '空入场',
    }

    const exitMarker: TradeMarker = {
      time: exitTime,
      position: 'aboveBar',
      color: '#f56c6c',
      shape: 'arrowDown',
      text: `${trade.exit_reason === 'take_profit' ? '止盈' : trade.exit_reason === 'stop_loss' ? '止损' : '信号'} ${trade.pnl_usdt >= 0 ? '+' : ''}${trade.pnl_usdt.toFixed(2)}`,
    }

    return [entryMarker, exitMarker]
  })
})

/**
 * Load kline data for the backtest period.
 * Uses the backtest config (symbol, interval, start/end date).
 */
async function loadKlineData() {
  if (!symbol.value || !interval.value) return
  try {
    const startMs = new Date(props.backtestResult.config.start_date).getTime()
    const endMs = new Date(props.backtestResult.config.end_date).getTime()
    const result = await queryKlines({
      symbol: symbol.value,
      interval: interval.value,
      start_time: startMs,
      end_time: endMs,
      page_size: 2000,
    })
    if (result.data) {
      klineData.value = result.data.map(bar => ({
        time: Math.floor(bar.open_time / 1000),
        open: bar.open,
        high: bar.high,
        low: bar.low,
        close: bar.close,
        volume: bar.quote_volume,
      }))
    }
  } catch (e) {
    console.warn('[BacktestKlineChart] failed to load kline data:', e)
  }
}

onMounted(() => {
  loadKlineData()
})
</script>

<style scoped>
.backtest-kline-chart {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
}

.chart-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #1a1a1a;
  border-bottom: 1px solid #2a2a2a;
  font-size: 12px;
}

.symbol-tag {
  font-weight: 600;
  color: #e0e0e0;
}

.interval-tag {
  background: #2d2d2d;
  color: #a0a0a0;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 11px;
}

.trade-count {
  color: #808080;
  margin-left: auto;
  font-size: 11px;
}
</style>
