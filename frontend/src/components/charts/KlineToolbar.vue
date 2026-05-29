<template>
  <div class="kline-toolbar">
    <!-- Period buttons -->
    <div class="toolbar-section periods">
      <div class="period-pills">
        <button
          v-for="iv in intervals"
          :key="iv.value"
          class="period-btn"
          :class="{ active: selectedPeriod === iv.value }"
          @click="onPeriodChange(iv.value)"
        >
          {{ iv.label }}
        </button>
      </div>
    </div>

    <div class="toolbar-separator" />

    <!-- Indicator toggles -->
    <div class="toolbar-section indicators">
      <button
        class="indicator-btn"
        :class="{ active: maToggles.ma7 || maToggles.ma25 || maToggles.ma99 || maToggles.ma200 }"
        @click="maMenuVisible = !maMenuVisible"
      >
        MA
      </button>
      <div v-if="maMenuVisible" class="dropdown-menu">
        <label><input type="checkbox" v-model="maToggles.ma7" @change="emitToggle" /> MA7</label>
        <label><input type="checkbox" v-model="maToggles.ma25" @change="emitToggle" /> MA25</label>
        <label><input type="checkbox" v-model="maToggles.ma99" @change="emitToggle" /> MA99</label>
        <label><input type="checkbox" v-model="maToggles.ma200" @change="emitToggle" /> MA200</label>
      </div>

      <button
        class="indicator-btn"
        :class="{ active: emaToggles.ema9 || emaToggles.ema21 }"
        @click="emaMenuVisible = !emaMenuVisible"
      >
        EMA
      </button>
      <div v-if="emaMenuVisible" class="dropdown-menu">
        <label><input type="checkbox" v-model="emaToggles.ema9" @change="emitToggle" /> EMA9</label>
        <label><input type="checkbox" v-model="emaToggles.ema21" @change="emitToggle" /> EMA21</label>
      </div>

      <button
        class="indicator-btn"
        :class="{ active: activeIndicators.includes('boll') }"
        @click="toggleIndicator('boll')"
      >
        BOLL
      </button>
    </div>

    <div class="toolbar-separator" />

    <!-- Sub-chart toggles -->
    <div class="toolbar-section subcharts">
      <button
        class="indicator-btn"
        :class="{ active: activeIndicators.includes('macd') }"
        @click="toggleIndicator('macd')"
      >
        MACD
      </button>
      <button
        class="indicator-btn"
        :class="{ active: activeIndicators.includes('kdj') }"
        @click="toggleIndicator('kdj')"
      >
        KDJ
      </button>
      <button
        class="indicator-btn"
        :class="{ active: activeIndicators.includes('rsi') }"
        @click="toggleIndicator('rsi')"
      >
        RSI
      </button>
      <button
        class="indicator-btn"
        :class="{ active: activeIndicators.includes('atr') }"
        @click="toggleIndicator('atr')"
      >
        ATR
      </button>
      <button
        class="indicator-btn"
        :class="{ active: activeIndicators.includes('stoch') }"
        @click="toggleIndicator('stoch')"
      >
        STOCH
      </button>
    </div>

    <div class="toolbar-separator" />

    <!-- Drawing tools -->
    <div class="toolbar-section drawing">
      <button
        class="tool-btn"
        :class="{ active: activeDrawing === 'trendline' }"
        @click="selectDrawing('trendline')"
        title="Trend Line"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <line x1="2" y1="14" x2="14" y2="2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
      </button>
      <button
        class="tool-btn"
        :class="{ active: activeDrawing === 'hline' }"
        @click="selectDrawing('hline')"
        title="Horizontal Line"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <line x1="2" y1="8" x2="14" y2="8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
      </button>
      <button
        class="tool-btn"
        :class="{ active: activeDrawing === 'fib' }"
        @click="selectDrawing('fib')"
        title="Fibonacci"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <line x1="2" y1="14" x2="14" y2="2" stroke="currentColor" stroke-width="1" stroke-linecap="round" />
          <line x1="2" y1="11" x2="14" y2="5" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" opacity="0.6" />
          <line x1="2" y1="8" x2="14" y2="8" stroke="currentColor" stroke-width="0.8" stroke-linecap="round" opacity="0.4" />
        </svg>
      </button>
      <button
        class="tool-btn"
        :class="{ active: activeDrawing === 'rect' }"
        @click="selectDrawing('rect')"
        title="Rectangle"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <rect x="2" y="4" width="12" height="8" stroke="currentColor" stroke-width="1.5" fill="none" />
        </svg>
      </button>
      <button class="tool-btn" @click="clearDrawings" title="Clear">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M3 3L13 13M13 3L3 13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        </svg>
      </button>
    </div>

    <div class="toolbar-spacer" />

    <!-- Right side: symbol -->
    <div class="toolbar-section right-info">
      <span class="symbol-label">{{ symbol }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'

const props = withDefaults(defineProps<{
  symbol?: string
  interval?: string
  lastPrice?: string
  priceDirection?: 'up' | 'down' | ''
}>(), {
  symbol: '',
  interval: '1h',
  lastPrice: '',
  priceDirection: '',
})

const emit = defineEmits<{
  (e: 'period-change', interval: string): void
  (e: 'indicator-toggle', name: string, visible: boolean): void
  (e: 'drawing-select', tool: string | null): void
}>()

const intervals = [
  { label: '1m', value: '1m' },
  { label: '5m', value: '5m' },
  { label: '15m', value: '15m' },
  { label: '30m', value: '30m' },
  { label: '1h', value: '1h' },
  { label: '4h', value: '4h' },
  { label: '1d', value: '1d' },
  { label: '1w', value: '1w' },
]

const selectedPeriod = ref(props.interval || '1h')
const activeDrawing = ref<string | null>(null)
const maMenuVisible = ref(false)
const emaMenuVisible = ref(false)

const activeIndicators = ref<string[]>(['ma7', 'ma25', 'ma99', 'macd', 'kdj', 'rsi'])

const maToggles = ref({ ma7: true, ma25: true, ma99: true, ma200: false })
const emaToggles = ref({ ema9: false, ema21: false })

watch(() => props.interval, (v) => {
  selectedPeriod.value = v || '1h'
})

function onPeriodChange(val: string) {
  selectedPeriod.value = val
  emit('period-change', val)
}

function toggleIndicator(name: string) {
  const idx = activeIndicators.value.indexOf(name)
  if (idx >= 0) {
    activeIndicators.value.splice(idx, 1)
  } else {
    activeIndicators.value.push(name)
  }
  emit('indicator-toggle', name, activeIndicators.value.includes(name))
}

function emitToggle() {
  activeIndicators.value = activeIndicators.value.filter(i => !['ma7', 'ma25', 'ma99', 'ma200', 'ema9', 'ema21', 'boll'].includes(i))
  if (maToggles.value.ma7) activeIndicators.value.push('ma7')
  if (maToggles.value.ma25) activeIndicators.value.push('ma25')
  if (maToggles.value.ma99) activeIndicators.value.push('ma99')
  if (maToggles.value.ma200) activeIndicators.value.push('ma200')
  if (emaToggles.value.ema9) activeIndicators.value.push('ema9')
  if (emaToggles.value.ema21) activeIndicators.value.push('ema21')
  emit('indicator-toggle', 'ma', maToggles.value.ma7 || maToggles.value.ma25 || maToggles.value.ma99 || maToggles.value.ma200)
  emit('indicator-toggle', 'ema', emaToggles.value.ema9 || emaToggles.value.ema21)
}

function selectDrawing(tool: string) {
  if (activeDrawing.value === tool) {
    activeDrawing.value = null
    emit('drawing-select', null)
  } else {
    activeDrawing.value = tool
    emit('drawing-select', tool)
  }
}

function clearDrawings() {
  activeDrawing.value = null
  emit('drawing-select', null)
}

defineExpose({ activeIndicators, maToggles, emaToggles, toggleIndicator, clearDrawings })
</script>

<style scoped>
.kline-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  background: #1a1a1a;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
  flex-wrap: nowrap;
  overflow-x: auto;
  position: relative;
}

.toolbar-section {
  display: flex;
  align-items: center;
  gap: 4px;
}

.toolbar-separator {
  width: 1px;
  height: 20px;
  background: rgba(255, 255, 255, 0.1);
  margin: 0 6px;
  flex-shrink: 0;
}

.toolbar-spacer {
  flex: 1;
}

.period-pills {
  display: flex;
  gap: 2px;
}

.period-btn {
  padding: 4px 10px;
  font-size: 12px;
  font-weight: 500;
  background: transparent;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  color: #8a8f98;
  cursor: pointer;
  transition: all 0.2s;
}

.period-btn:hover {
  color: #f7f8f8;
  border-color: rgba(255, 255, 255, 0.2);
}

.period-btn.active {
  background: #7170ff;
  border-color: #7170ff;
  color: #fff;
  box-shadow: 0 0 8px rgba(113, 112, 255, 0.4);
}

.indicator-btn {
  padding: 4px 10px;
  font-size: 11px;
  font-weight: 600;
  background: transparent;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  color: #8a8f98;
  cursor: pointer;
  transition: all 0.2s;
}

.indicator-btn:hover {
  color: #f7f8f8;
  border-color: rgba(255, 255, 255, 0.2);
}

.indicator-btn.active {
  background: rgba(113, 112, 255, 0.2);
  border-color: #7170ff;
  color: #7170ff;
}

.tool-btn {
  padding: 5px 7px;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: transparent;
  color: #8a8f98;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
}

.tool-btn:hover {
  color: #f7f8f8;
  border-color: rgba(255, 255, 255, 0.15);
}

.tool-btn.active {
  background: rgba(113, 112, 255, 0.2);
  border-color: #7170ff;
  color: #7170ff;
}

.dropdown-menu {
  position: absolute;
  top: 100%;
  left: 0;
  background: #212223;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  padding: 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  z-index: 1000;
  min-width: 120px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.dropdown-menu label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #d0d6e0;
  cursor: pointer;
  padding: 2px 0;
}

.dropdown-menu input[type="checkbox"] {
  width: 14px;
  height: 14px;
  accent-color: #7170ff;
}

.symbol-label {
  font-size: 13px;
  font-weight: 700;
  color: #f7f8f8;
  font-family: 'JetBrains Mono', monospace;
}

.right-info {
  display: flex;
  align-items: center;
  gap: 4px;
}
</style>