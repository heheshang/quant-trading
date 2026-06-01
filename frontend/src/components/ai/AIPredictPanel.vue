<template>
  <div class="ai-predict-panel">
    <!-- Header -->
    <div class="panel-header">
      <div class="panel-title">
        <span class="title-text">AI 预测</span>
        <span v-if="symbol" class="symbol-badge">{{ symbol }}</span>
      </div>
      <div class="ws-status" :class="statusClass">
        <span class="ws-dot" :class="statusClass"></span>
        <span class="ws-text">{{ statusText }}</span>
      </div>
    </div>

    <!-- Loading / No Data -->
    <div v-if="!prediction" class="empty-state">
      <span class="empty-icon">📡</span>
      <span class="empty-text">{{ emptyText }}</span>
    </div>

    <!-- Prediction Content -->
    <template v-else>
      <!-- Signal + Confidence Row -->
      <div class="signal-row">
        <span class="signal-tag" :class="prediction.direction">
          {{ signalLabel }}
        </span>
        <div class="confidence-wrap">
          <span class="confidence-label">置信度</span>
          <span class="confidence-value">{{ confidenceText }}</span>
        </div>
      </div>

      <!-- Analysis -->
      <div class="analysis-section">
        <p class="analysis-text">{{ prediction.analysis }}</p>
      </div>

      <!-- Price Target -->
      <div v-if="prediction.price_target != null" class="price-target">
        <span class="pt-label">目标价</span>
        <span class="pt-value">{{ formatPrice(prediction.price_target) }}</span>
      </div>

      <!-- Indicators Collapsible -->
      <div v-if="hasIndicators" class="indicators-section">
        <button class="indicators-toggle" @click="indicatorsOpen = !indicatorsOpen">
          <span class="toggle-text">技术指标</span>
          <span class="toggle-arrow">{{ indicatorsOpen ? '▲' : '▼' }}</span>
        </button>
        <div v-if="indicatorsOpen" class="indicators-grid">
          <!-- RSI -->
          <div v-if="prediction.indicators.rsi_14 != null" class="indicator-item">
            <span class="ind-label">RSI(14)</span>
            <span class="ind-value" :class="rsiClass">{{ prediction.indicators.rsi_14.toFixed(2) }}</span>
          </div>

          <!-- MACD -->
          <div v-if="prediction.indicators.macd != null" class="indicator-item macd-item">
            <span class="ind-label">MACD</span>
            <span class="ind-value">
              {{ prediction.indicators.macd.toFixed(2) }}
              <span v-if="prediction.indicators.macd_signal != null" class="ind-sub">信号 {{ prediction.indicators.macd_signal.toFixed(2) }}</span>
            </span>
          </div>

          <!-- Bollinger Bands -->
          <div v-if="prediction.indicators.bb_upper != null" class="indicator-item bb-item">
            <span class="ind-label">布林带</span>
            <span class="ind-value">
              {{ prediction.indicators.bb_upper.toFixed(2) }}
              <span class="ind-sub">中 {{ prediction.indicators.bb_middle?.toFixed(2) }}</span>
              {{ prediction.indicators.bb_lower?.toFixed(2) }}
            </span>
          </div>

          <!-- Moving Averages -->
          <div v-if="prediction.indicators.sma_7 != null || prediction.indicators.sma_25 != null" class="indicator-item">
            <span class="ind-label">均线</span>
            <span class="ind-value">
              <span v-if="prediction.indicators.sma_7 != null">MA7 {{ prediction.indicators.sma_7.toFixed(2) }}</span>
              <span v-if="prediction.indicators.sma_25 != null"> MA25 {{ prediction.indicators.sma_25.toFixed(2) }}</span>
            </span>
          </div>

          <!-- Volume MA -->
          <div v-if="prediction.indicators.volume_ma_20 != null" class="indicator-item">
            <span class="ind-label">量比</span>
            <span class="ind-value">{{ prediction.indicators.volume_ma_20.toFixed(2) }}</span>
          </div>

          <!-- ATR -->
          <div v-if="prediction.indicators.atr_14 != null" class="indicator-item">
            <span class="ind-label">ATR(14)</span>
            <span class="ind-value">{{ prediction.indicators.atr_14.toFixed(2) }}</span>
          </div>

          <!-- Current Price -->
          <div v-if="prediction.indicators.current_price != null" class="indicator-item">
            <span class="ind-label">当前价</span>
            <span class="ind-value">{{ formatPrice(prediction.indicators.current_price) }}</span>
          </div>
        </div>
      </div>

      <!-- Generated At -->
      <div class="generated-time">
        {{ formatTime(prediction.generated_at) }}
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { AIPredictResponse, AIWsStatus, AIPredictSignal, AIDirection } from '@/types/ai'

interface Props {
  prediction?: AIPredictResponse | null
  status?: AIWsStatus
  symbol?: string
}

const props = withDefaults(defineProps<Props>(), {
  prediction: null,
  status: 'idle',
  symbol: '',
})

const indicatorsOpen = ref(false)

// ─── Computed ───────────────────────────────────────────────────────────────

const hasIndicators = computed(() => {
  if (!props.prediction?.indicators) return false
  const ind = props.prediction.indicators
  return (
    ind.rsi_14 != null ||
    ind.macd != null ||
    ind.bb_upper != null ||
    ind.sma_7 != null ||
    ind.sma_25 != null ||
    ind.volume_ma_20 != null ||
    ind.atr_14 != null
  )
})

const signalLabel = computed(() => {
  const map: Record<AIPredictSignal, string> = {
    strong_buy: '强烈买入',
    buy: '买入',
    neutral: '中性',
    sell: '卖出',
    strong_sell: '强烈卖出',
  }
  return map[props.prediction?.signal ?? 'neutral']
})

const confidenceText = computed(() => {
  const val = props.prediction?.confidence ?? 0
  return `${(val * 100).toFixed(0)}%`
})

const statusClass = computed(() => props.status)

const statusText = computed(() => {
  const map: Record<AIWsStatus, string> = {
    idle: '未连接',
    connecting: '连接中...',
    connected: '已连接',
    disconnected: '连接中断',
    reconnecting: '重连中...',
    error: '连接错误',
  }
  return map[props.status ?? 'idle'] || '未连接'
})

const emptyText = computed(() => {
  const map: Record<AIWsStatus, string> = {
    idle: '点击连接获取 AI 预测',
    connecting: '正在连接 AI 服务...',
    connected: '等待数据...',
    disconnected: '连接已断开',
    reconnecting: '正在重连...',
    error: '连接出错',
  }
  return map[props.status ?? 'idle'] || '等待数据'
})

const rsiClass = computed(() => {
  const rsi = props.prediction?.indicators?.rsi_14
  if (rsi == null) return ''
  if (rsi >= 70) return 'overbought'
  if (rsi <= 30) return 'oversold'
  return ''
})

// ─── Formatters ─────────────────────────────────────────────────────────────

function formatPrice(value: number): string {
  if (value >= 1000) return value.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
  if (value >= 1) return value.toFixed(4)
  return value.toFixed(6)
}

function formatTime(isoString: string): string {
  if (!isoString) return ''
  try {
    const d = new Date(isoString)
    return d.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit' })
  } catch {
    return isoString
  }
}
</script>

<style scoped lang="scss">
// ─── Design Tokens (Bloomberg Terminal + Trust Blue #3B82F6) ─────────────────
$bg-card: #0f1419;
$border-card: #1e2a3a;
$bg-elevated: #161d27;
$color-accent: #3B82F6;
$color-text-primary: #e7e9ea;
$color-text-secondary: #71767b;
$color-text-tertiary: #536471;

$color-long: #16a34a;   // green for long/buy
$color-short: #dc2626;   // red for short/sell
$color-neutral: #71767b;

// ─── Panel ───────────────────────────────────────────────────────────────────

.ai-predict-panel {
  background: $bg-card;
  border: 1px solid $border-card;
  border-radius: 8px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 280px;
  font-family: 'SF Mono', 'Menlo', 'Monaco', monospace;
}

// ─── Header ─────────────────────────────────────────────────────────────────

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.panel-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.title-text {
  font-size: 13px;
  font-weight: 700;
  color: $color-text-primary;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.symbol-badge {
  font-size: 11px;
  font-weight: 600;
  color: $color-accent;
  background: rgba($color-accent, 0.12);
  border: 1px solid rgba($color-accent, 0.3);
  border-radius: 4px;
  padding: 1px 6px;
  letter-spacing: 0.05em;
}

// ─── WS Status ───────────────────────────────────────────────────────────────

.ws-status {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  color: $color-text-tertiary;
}

.ws-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;

  &.idle { background: transparent; border: 1px solid $color-text-tertiary; }
  &.connecting { background: #f59e0b; }
  &.connected { background: $color-long; }
  &.disconnected { background: $color-short; }
  &.reconnecting { background: #f59e0b; }
  &.error { background: $color-short; }
}

.ws-text {
  &.connected { color: $color-long; }
  &.disconnected { color: $color-short; }
  &.reconnecting { color: #f59e0b; }
  &.error { color: $color-short; }
}

// ─── Empty State ─────────────────────────────────────────────────────────────

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px 0;
}

.empty-icon {
  font-size: 24px;
}

.empty-text {
  font-size: 12px;
  color: $color-text-tertiary;
}

// ─── Signal Row ───────────────────────────────────────────────────────────────

.signal-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.signal-tag {
  font-size: 12px;
  font-weight: 700;
  padding: 3px 10px;
  border-radius: 4px;
  letter-spacing: 0.05em;
  text-transform: uppercase;

  &.long {
    color: $color-long;
    background: rgba($color-long, 0.12);
    border: 1px solid rgba($color-long, 0.3);
  }
  &.short {
    color: $color-short;
    background: rgba($color-short, 0.12);
    border: 1px solid rgba($color-short, 0.3);
  }
  &.neutral {
    color: $color-neutral;
    background: rgba($color-neutral, 0.08);
    border: 1px solid $border-card;
  }
}

.confidence-wrap {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.confidence-label {
  font-size: 11px;
  color: $color-text-tertiary;
}

.confidence-value {
  font-size: 18px;
  font-weight: 700;
  color: $color-accent;
  letter-spacing: -0.02em;
}

// ─── Analysis ────────────────────────────────────────────────────────────────

.analysis-section {
  background: $bg-elevated;
  border: 1px solid $border-card;
  border-radius: 6px;
  padding: 10px 12px;
}

.analysis-text {
  font-size: 12px;
  line-height: 1.6;
  color: $color-text-primary;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}

// ─── Price Target ────────────────────────────────────────────────────────────

.price-target {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: $bg-elevated;
  border: 1px solid $border-card;
  border-radius: 6px;
}

.pt-label {
  font-size: 11px;
  color: $color-text-tertiary;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.pt-value {
  font-size: 14px;
  font-weight: 700;
  color: $color-accent;
  letter-spacing: -0.01em;
}

// ─── Indicators ──────────────────────────────────────────────────────────────

.indicators-section {
  border: 1px solid $border-card;
  border-radius: 6px;
  overflow: hidden;
}

.indicators-toggle {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: $bg-elevated;
  border: none;
  cursor: pointer;
  font-size: 12px;
}

.toggle-text {
  color: $color-text-secondary;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.toggle-arrow {
  color: $color-text-tertiary;
  font-size: 10px;
}

.indicators-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1px;
  background: $border-card;
  border-top: 1px solid $border-card;
}

.indicator-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 10px;
  background: $bg-elevated;
}

.ind-label {
  font-size: 10px;
  color: $color-text-tertiary;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.ind-value {
  font-size: 12px;
  color: $color-text-primary;
  font-weight: 600;

  &.overbought { color: $color-short; }
  &.oversold { color: $color-long; }
}

.ind-sub {
  font-size: 10px;
  color: $color-text-tertiary;
  font-weight: 400;
}

.trend-tag {
  display: inline-block;
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 11px;

  &.bullish {
    color: $color-long;
    background: rgba($color-long, 0.1);
  }
  &.bearish {
    color: $color-short;
    background: rgba($color-short, 0.1);
  }
  &.neutral {
    color: $color-neutral;
    background: rgba($color-neutral, 0.08);
  }
}

// ─── Generated Time ───────────────────────────────────────────────────────────

.generated-time {
  font-size: 10px;
  color: $color-text-tertiary;
  text-align: right;
  letter-spacing: 0.03em;
}
</style>