<template>
  <div class="kline-detail-view">
    <!-- Page header -->
    <div class="page-header">
      <div class="header-left">
        <el-button text @click="router.back()">
          <el-icon><ArrowLeft /></el-icon>
        </el-button>
        <h1 class="page-title">{{ symbol }} {{ intervalLabel }}</h1>
        <span class="data-points-badge">{{ klineData.length.toLocaleString() }} 根K线</span>
      </div>
      <div class="header-actions">
        <el-button @click="handleExport">导出数据</el-button>
        <el-button type="primary" @click="handleRefresh">
          <el-icon><Refresh /></el-icon>
          刷新
        </el-button>
      </div>
    </div>

    <!-- Interval selector -->
    <div class="interval-bar">
      <el-radio-group v-model="currentInterval" size="large" class="interval-pills" @change="handleIntervalChange">
        <el-radio-button v-for="iv in KLINE_INTERVALS" :key="iv.value" :value="iv.value">
          {{ iv.label }}
        </el-radio-button>
      </el-radio-group>
    </div>

    <!-- Main content: chart + indicators -->
    <div class="main-content">
      <!-- Chart area -->
      <div class="chart-wrapper">
        <div v-if="loading" class="chart-loading">
          <el-icon class="is-loading" :size="32"><Loading /></el-icon>
          <p>加载K线数据...</p>
        </div>
        <KlineChart
          v-else-if="klineData.length > 0"
          ref="chartRef"
          :data="klineData"
          :symbol="symbol"
          :interval="currentInterval"
          :dark-mode="true"
          style="height: 520px;"
        />
        <div v-else class="chart-empty">
          <el-icon :size="48"><DataLine /></el-icon>
          <p>暂无数据</p>
        </div>
      </div>

      <!-- Indicators panel -->
      <div class="indicators-panel">
        <div class="panel-header">
          <span class="panel-title">技术指标</span>
        </div>

        <!-- Price info -->
        <div class="indicator-section">
          <div class="section-title">价格信息</div>
          <div class="indicator-grid">
            <div class="indicator-item">
              <span class="indicator-label">最新价</span>
              <span class="indicator-value price-value" :class="priceChangeClass">
                {{ latestClose ?? '-' }}
              </span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">涨跌额</span>
              <span class="indicator-value" :class="priceChangeClass">
                {{ priceChange ?? '-' }}
              </span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">涨跌幅</span>
              <span class="indicator-value" :class="priceChangeClass">
                {{ priceChangePercent ?? '-' }}%
              </span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">数据范围</span>
              <span class="indicator-value">
                {{ dataRange }}
              </span>
            </div>
          </div>
        </div>

        <!-- Period stats -->
        <div class="indicator-section">
          <div class="section-title">周期统计</div>
          <div class="indicator-grid">
            <div class="indicator-item">
              <span class="indicator-label">最高价</span>
              <span class="indicator-value high-value">{{ periodHigh ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">最低价</span>
              <span class="indicator-value low-value">{{ periodLow ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">总成交量</span>
              <span class="indicator-value">{{ totalVolume ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">平均成交量</span>
              <span class="indicator-value">{{ avgVolume ?? '-' }}</span>
            </div>
          </div>
        </div>

        <!-- Quality info -->
        <div class="indicator-section">
          <div class="section-title">数据质量</div>
          <div class="indicator-grid">
            <div class="indicator-item">
              <span class="indicator-label">数据点数</span>
              <span class="indicator-value">{{ klineData.length.toLocaleString() }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">开始时间</span>
              <span class="indicator-value">{{ startTimeStr }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">结束时间</span>
              <span class="indicator-value">{{ endTimeStr }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">缺失检测</span>
              <span class="indicator-value quality-badge" :class="`quality-${qualityLevel}`">
                {{ qualityLabel }}
              </span>
            </div>
          </div>
        </div>

        <!-- MA indicator -->
        <div class="indicator-section">
          <div class="section-title">MA 均线</div>
          <div class="indicator-grid">
            <div class="indicator-item">
              <span class="indicator-label">MA5</span>
              <span class="indicator-value">{{ lastMA.ma5 ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">MA10</span>
              <span class="indicator-value">{{ lastMA.ma10 ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">MA20</span>
              <span class="indicator-value">{{ lastMA.ma20 ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">MA60</span>
              <span class="indicator-value">{{ lastMA.ma60 ?? '-' }}</span>
            </div>
          </div>
        </div>

        <!-- BOLL indicator -->
        <div class="indicator-section">
          <div class="section-title">BOLL 布林带</div>
          <div class="indicator-grid">
            <div class="indicator-item">
              <span class="indicator-label">上轨</span>
              <span class="indicator-value boll-up">{{ lastBOLL.upper ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">中轨</span>
              <span class="indicator-value boll-mid">{{ lastBOLL.mid ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">下轨</span>
              <span class="indicator-value boll-low">{{ lastBOLL.lower ?? '-' }}</span>
            </div>
          </div>
        </div>

        <!-- MACD indicator -->
        <div class="indicator-section">
          <div class="section-title">MACD</div>
          <div class="indicator-grid">
            <div class="indicator-item">
              <span class="indicator-label">DIF</span>
              <span class="indicator-value" :class="macdClass('dif')">{{ lastMACD.dif?.toFixed(4) ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">DEA</span>
              <span class="indicator-value" :class="macdClass('dea')">{{ lastMACD.dea?.toFixed(4) ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">BAR</span>
              <span class="indicator-value" :class="macdClass('bar')">{{ lastMACD.bar?.toFixed(4) ?? '-' }}</span>
            </div>
          </div>
        </div>

        <!-- RSI indicator -->
        <div class="indicator-section">
          <div class="section-title">RSI</div>
          <div class="indicator-grid">
            <div class="indicator-item">
              <span class="indicator-label">RSI(6)</span>
              <span class="indicator-value" :class="rsiClass(lastRSI.rsi6)">{{ lastRSI.rsi6?.toFixed(2) ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">RSI(12)</span>
              <span class="indicator-value" :class="rsiClass(lastRSI.rsi12)">{{ lastRSI.rsi12?.toFixed(2) ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">RSI(24)</span>
              <span class="indicator-value" :class="rsiClass(lastRSI.rsi24)">{{ lastRSI.rsi24?.toFixed(2) ?? '-' }}</span>
            </div>
          </div>
        </div>

        <!-- KDJ indicator -->
        <div class="indicator-section">
          <div class="section-title">KDJ</div>
          <div class="indicator-grid">
            <div class="indicator-item">
              <span class="indicator-label">K</span>
              <span class="indicator-value">{{ lastKDJ.k?.toFixed(2) ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">D</span>
              <span class="indicator-value">{{ lastKDJ.d?.toFixed(2) ?? '-' }}</span>
            </div>
            <div class="indicator-item">
              <span class="indicator-label">J</span>
              <span class="indicator-value">{{ lastKDJ.j?.toFixed(2) ?? '-' }}</span>
            </div>
          </div>
        </div>

        <!-- Depth panel -->
        <div class="indicator-section">
          <div class="section-title">深度面板</div>
          <div class="depth-panel">
            <div class="depth-bids">
              <div class="depth-header">买单</div>
              <div v-for="bid in depthBids" :key="bid.price" class="depth-row bid-row">
                <span class="depth-price">{{ bid.price }}</span>
                <span class="depth-amount">{{ bid.amount }}</span>
              </div>
              <div v-if="depthBids.length === 0" class="depth-empty">暂无数据</div>
            </div>
            <div class="depth-asks">
              <div class="depth-header">卖单</div>
              <div v-for="ask in depthAsks" :key="ask.price" class="depth-row ask-row">
                <span class="depth-price">{{ ask.price }}</span>
                <span class="depth-amount">{{ ask.amount }}</span>
              </div>
              <div v-if="depthAsks.length === 0" class="depth-empty">暂无数据</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, Refresh, Loading, DataLine } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { queryKlines, getQualityReport, exportKlines } from '@/api/kline'
import { KLINE_INTERVALS } from '@/types/kline'
import { getMarketWs, toInternalSymbol } from '@/api/ws'
import { useMarketWs } from '@/composables/useMarketWs'
import KlineChart from '@/components/charts/KlineChart.vue'
import type { KlineBar } from '@/components/charts/KlineChart.vue'
import type { KlineQualityReport } from '@/types/kline'

const route = useRoute()
const router = useRouter()
const chartRef = ref<InstanceType<typeof KlineChart> | null>(null)

const symbol = computed(() => route.params.symbol as string)
const interval = computed(() => route.params.interval as string)
const currentInterval = ref(interval.value)

const loading = ref(false)
const klineData = ref<KlineBar[]>([])
const qualityInfo = ref<KlineQualityReport | null>(null)

// WebSocket real-time kline updates — via window CustomEvent from trading.ts
function onKlineUpdate(event: Event): void {
  const data = (event as CustomEvent).detail as {
    symbol: string
    interval: string
    time: number
    open: number
    high: number
    low: number
    close: number
    volume: number
  }
  // Only update if interval matches current view
  if (data.interval !== currentInterval.value) return
  const bar: KlineBar = {
    time: data.time,
    open: data.open,
    high: data.high,
    low: data.low,
    close: data.close,
    volume: data.volume,
  }
  chartRef.value?.addBar(bar)
  // Also append to klineData for indicator updates
  klineData.value.push(bar)
}

// Interval label
const intervalLabel = computed(() => {
  const found = KLINE_INTERVALS.find(i => i.value === currentInterval.value)
  return found ? found.label : currentInterval.value
})

// Price change analysis
const latestClose = computed(() => {
  if (klineData.value.length === 0) return null
  return klineData.value[klineData.value.length - 1].close.toFixed(2)
})

const firstOpen = computed(() => {
  if (klineData.value.length === 0) return null
  return klineData.value[0].open
})

const priceChange = computed(() => {
  if (!latestClose.value || !firstOpen.value) return null
  const diff = parseFloat(latestClose.value) - firstOpen.value
  return diff >= 0 ? `+${diff.toFixed(2)}` : diff.toFixed(2)
})

const priceChangePercent = computed(() => {
  if (!latestClose.value || !firstOpen.value || firstOpen.value === 0) return null
  const diff = ((parseFloat(latestClose.value) - firstOpen.value) / firstOpen.value) * 100
  return diff >= 0 ? `+${diff.toFixed(2)}` : diff.toFixed(2)
})

const priceChangeClass = computed(() => {
  if (!priceChange.value) return ''
  return priceChange.value.startsWith('+') ? 'up' : 'down'
})

// Period stats
const periodHigh = computed(() => {
  if (klineData.value.length === 0) return null
  const max = Math.max(...klineData.value.map(b => b.high))
  return max.toFixed(2)
})

const periodLow = computed(() => {
  if (klineData.value.length === 0) return null
  const min = Math.min(...klineData.value.map(b => b.low))
  return min.toFixed(2)
})

const totalVolume = computed(() => {
  if (klineData.value.length === 0) return null
  const sum = klineData.value.reduce((acc, b) => acc + (b.volume ?? 0), 0)
  return sum.toLocaleString(undefined, { maximumFractionDigits: 2 })
})

const avgVolume = computed(() => {
  if (klineData.value.length === 0) return null
  const sum = klineData.value.reduce((acc, b) => acc + (b.volume ?? 0), 0)
  return (sum / klineData.value.length).toFixed(2)
})

const dataRange = computed(() => {
  if (klineData.value.length === 0) return '-'
  const count = klineData.value.length
  const first = klineData.value[0]
  const last = klineData.value[klineData.value.length - 1]
  const firstDate = new Date(first.time * 1000).toLocaleDateString('zh-CN')
  const lastDate = new Date(last.time * 1000).toLocaleDateString('zh-CN')
  return `${firstDate} ~ ${lastDate}`
})

const startTimeStr = computed(() => {
  if (klineData.value.length === 0) return '-'
  return new Date(klineData.value[0].time * 1000).toLocaleString('zh-CN')
})

const endTimeStr = computed(() => {
  if (klineData.value.length === 0) return '-'
  const last = klineData.value[klineData.value.length - 1]
  return new Date(last.time * 1000).toLocaleString('zh-CN')
})

const qualityLevel = computed(() => {
  if (!qualityInfo.value) return 'normal'
  const { gap_count, anomaly_count, duplicate_count, suspicious_count } = qualityInfo.value
  if (anomaly_count > 0 || suspicious_count > 0) return 'anomaly'
  if (gap_count > 0) return 'missing'
  if (duplicate_count > 0) return 'duplicate'
  return 'normal'
})

const qualityLabel = computed(() => {
  const q = qualityLevel.value
  const map: Record<string, string> = {
    normal: '正常', missing: '缺失', anomaly: '异常',
    duplicate: '重复', suspicious: '可疑',
  }
  return map[q] ?? q
})

// ============================================================
// Depth panel — WebSocket via useMarketWs
// ============================================================
interface DepthItem { price: string; amount: string }
const depthBids = ref<DepthItem[]>([])
const depthAsks = ref<DepthItem[]>([])

const marketWs = useMarketWs()

function onDepthMessage(msg: any) {
  if (msg.channel === 'depth') {
    const bids = msg.data?.bids ?? []
    const asks = msg.data?.asks ?? []
    depthBids.value = bids.slice(0, 10).map((b: any) => ({
      price: typeof b[0] === 'number' ? b[0].toFixed(2) : String(b[0]),
      amount: typeof b[1] === 'number' ? b[1].toFixed(4) : String(b[1]),
    }))
    depthAsks.value = asks.slice(0, 10).map((a: any) => ({
      price: typeof a[0] === 'number' ? a[0].toFixed(2) : String(a[0]),
      amount: typeof a[1] === 'number' ? a[1].toFixed(4) : String(a[1]),
    }))
  }
}

async function subscribeDepth() {
  if (!symbol.value) return
  const token = localStorage.getItem('token') ?? ''
  marketWs.onMessage(onDepthMessage)
  marketWs.connect(token)
  const channel = `depth.${symbol.value.toLowerCase()}`
  marketWs.subscribe([channel])
}

function unsubscribeDepth() {
  if (!symbol.value) return
  const channel = `depth.${symbol.value.toLowerCase()}`
  marketWs.unsubscribe([channel])
  marketWs.disconnect()
}

// ============================================================
// Technical indicator computations (pure JS)
// ============================================================

// --- MA: Simple Moving Average ---
function calcMA(closes: number[], period: number): number | null {
  if (closes.length < period) return null
  const slice = closes.slice(-period)
  return slice.reduce((a, b) => a + b, 0) / period
}

const maData = computed(() => {
  const closes = klineData.value.map(b => b.close)
  return {
    ma5: calcMA(closes, 5),
    ma10: calcMA(closes, 10),
    ma20: calcMA(closes, 20),
    ma60: calcMA(closes, 60),
  }
})

const lastMA = computed(() => {
  const closes = klineData.value.map(b => b.close)
  return {
    ma5: calcMA(closes, 5)?.toFixed(2) ?? null,
    ma10: calcMA(closes, 10)?.toFixed(2) ?? null,
    ma20: calcMA(closes, 20)?.toFixed(2) ?? null,
    ma60: calcMA(closes, 60)?.toFixed(2) ?? null,
  }
})

// --- BOLL: Bollinger Bands (20, 2) ---
function calcBOLL(closes: number[], period = 20, multiplier = 2): { upper: number | null; mid: number | null; lower: number | null } {
  if (closes.length < period) return { upper: null, mid: null, lower: null }
  const slice = closes.slice(-period)
  const mid = slice.reduce((a, b) => a + b, 0) / period
  const variance = slice.reduce((sum, v) => sum + Math.pow(v - mid, 2), 0) / period
  const stdDev = Math.sqrt(variance)
  return {
    upper: mid + multiplier * stdDev,
    mid,
    lower: mid - multiplier * stdDev,
  }
}

const lastBOLL = computed(() => {
  const closes = klineData.value.map(b => b.close)
  const b = calcBOLL(closes)
  return {
    upper: b.upper?.toFixed(2) ?? null,
    mid: b.mid?.toFixed(2) ?? null,
    lower: b.lower?.toFixed(2) ?? null,
  }
})

// --- MACD: (12, 26, 9) — DIF / DEA / BAR ---
function calcEMA(closes: number[], period: number): number[] {
  if (closes.length === 0) return []
  const k = 2 / (period + 1)
  const ema: number[] = [closes[0]]
  for (let i = 1; i < closes.length; i++) {
    ema.push(closes[i] * k + ema[i - 1] * (1 - k))
  }
  return ema
}

function calcMACD(closes: number[], fast = 12, slow = 26, signal = 9): { dif: number | null; dea: number | null; bar: number | null } {
  if (closes.length < slow) return { dif: null, dea: null, bar: null }
  const emaFast = calcEMA(closes, fast)
  const emaSlow = calcEMA(closes, slow)
  const dif = emaFast[emaFast.length - 1] - emaSlow[emaSlow.length - 1]

  // Build histogram array for signal line calc
  const hist: number[] = []
  for (let i = 0; i < emaFast.length; i++) {
    hist.push(emaFast[i] - emaSlow[i])
  }
  const signalK = 2 / (signal + 1)
  let dea = hist.slice(-signal).reduce((a, b) => a + b, 0) / signal
  const deaArr: number[] = [dea]
  for (let i = hist.length - signal; i < hist.length; i++) {
    deaArr.push(hist[i] * signalK + deaArr[deaArr.length - 1] * (1 - signalK))
  }
  dea = deaArr[deaArr.length - 1]

  const bar = 2 * (dif - dea)
  return { dif, dea, bar }
}

const lastMACD = computed(() => {
  const closes = klineData.value.map(b => b.close)
  const m = calcMACD(closes)
  return {
    dif: m.dif,
    dea: m.dea,
    bar: m.bar,
  }
})

const macdClass = (key: 'dif' | 'dea' | 'bar') => computed(() => {
  const v = lastMACD.value[key]
  if (v === null) return ''
  if (key === 'bar') return v >= 0 ? 'up' : 'down'
  return v >= 0 ? 'up' : 'down'
})

// --- RSI: (14) — relative strength index ---
function calcRSI(closes: number[], period = 14): number | null {
  if (closes.length < period + 1) return null
  let gains = 0, losses = 0
  for (let i = closes.length - period; i < closes.length; i++) {
    const diff = closes[i] - closes[i - 1]
    if (diff >= 0) gains += diff
    else losses += Math.abs(diff)
  }
  const avgGain = gains / period
  const avgLoss = losses / period
  if (avgLoss === 0) return 100
  const rs = avgGain / avgLoss
  return 100 - 100 / (1 + rs)
}

const lastRSI = computed(() => {
  const closes = klineData.value.map(b => b.close)
  return {
    rsi6: calcRSI(closes, 6),
    rsi12: calcRSI(closes, 12),
    rsi24: calcRSI(closes, 24),
  }
})

const rsiClass = (value: number | null) => {
  if (value === null) return ''
  if (value >= 70) return 'rsi-overbought'
  if (value <= 30) return 'rsi-oversold'
  return ''
}

// --- KDJ: (9, 3, 3) ---
function calcKDJ(highs: number[], lows: number[], closes: number[], n = 9, m1 = 3, m2 = 3): { k: number | null; d: number | null; j: number | null } {
  if (closes.length < n) return { k: null, d: null, j: null }
  const rsv: number[] = []
  for (let i = n - 1; i < closes.length; i++) {
    const periodHigh = Math.max(...highs.slice(i - n + 1, i + 1))
    const periodLow = Math.min(...lows.slice(i - n + 1, i + 1))
    rsv.push(periodHigh === periodLow ? 50 : (closes[i] - periodLow) / (periodHigh - periodLow) * 100)
  }
  const kArr: number[] = [50]
  const dArr: number[] = [50]
  for (let i = 1; i < rsv.length; i++) {
    kArr.push((rsv[i] + (n - 1) * kArr[i - 1]) / n)
    dArr.push((kArr[i] + (m1 - 1) * dArr[i - 1]) / m1)
  }
  const k = kArr[kArr.length - 1]
  const d = dArr[dArr.length - 1]
  const j = 3 * k - 2 * d
  return { k, d, j }
}

const lastKDJ = computed(() => {
  const highs = klineData.value.map(b => b.high)
  const lows = klineData.value.map(b => b.low)
  const closes = klineData.value.map(b => b.close)
  return calcKDJ(highs, lows, closes)
})

async function loadKlineData() {
  if (!symbol.value || !currentInterval.value) return
  loading.value = true
  try {
    const res = await queryKlines({
      symbol: symbol.value.toLowerCase(),
      interval: currentInterval.value,
      page_size: 1000,
    }) as any

    const items = res?.items ?? res?.data ?? res ?? []
    klineData.value = items.map((b: any) => ({
      time: b.timestamp ?? b.time ?? Math.floor(new Date(b.open_time).getTime() / 1000),
      open: parseFloat(b.open),
      high: parseFloat(b.high),
      low: parseFloat(b.low),
      close: parseFloat(b.close),
      volume: parseFloat(b.volume ?? 0),
    }))
  } catch {
    klineData.value = []
    ElMessage.error('加载K线数据失败')
  } finally {
    loading.value = false
  }
}

async function loadQualityReport() {
  if (!symbol.value || !currentInterval.value) return
  try {
    qualityInfo.value = await getQualityReport(symbol.value.toLowerCase(), currentInterval.value)
  } catch {
    // ignore quality report errors
  }
}

function handleIntervalChange() {
  router.push(`/kline/${symbol.value}/${currentInterval.value}`)
  loadKlineData()
}

async function handleRefresh() {
  await loadKlineData()
  await loadQualityReport()
  ElMessage.success('数据已刷新')
}

async function handleExport() {
  try {
    const blob = await exportKlines({
      symbol: symbol.value.toLowerCase(),
      interval: currentInterval.value,
      format: 'csv',
    })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${symbol.value}_${currentInterval.value}.csv`
    a.click()
    URL.revokeObjectURL(url)
    ElMessage.success('导出成功')
  } catch {
    ElMessage.error('导出失败')
  }
}

onMounted(async () => {
  await loadKlineData()
  await loadQualityReport()

  // Set up WebSocket real-time kline updates via window CustomEvent
  window.addEventListener('kline-update', onKlineUpdate)

  // Subscribe depth panel via useMarketWs
  subscribeDepth()
})

onUnmounted(() => {
  window.removeEventListener('kline-update', onKlineUpdate)
  unsubscribeDepth()
})
</script>

<style scoped>
.kline-detail-view {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  height: 100%;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.data-points-badge {
  background: var(--el-fill-color-light);
  color: var(--el-text-color-secondary);
  padding: 2px 10px;
  border-radius: 12px;
  font-size: 12px;
}

.header-actions {
  display: flex;
  gap: 8px;
}

/* Interval bar */
.interval-bar {
  margin-bottom: 4px;
}

.interval-pills :deep(.el-radio-button__inner) {
  border-radius: 16px;
  border-left: 1px solid var(--el-border-color);
  margin-right: 8px;
  background: transparent;
  color: var(--el-text-color-regular);
  transition: all 0.2s;
}

.interval-pills :deep(.el-radio-button__original-radio:checked + .el-radio-button__inner) {
  background-color: #7170ff;
  border-color: #7170ff;
  color: #fff;
  box-shadow: none;
}

.interval-pills :deep(.el-radio-button:first-child .el-radio-button__inner) {
  border-radius: 16px;
}

.interval-pills :deep(.el-radio-button:last-child .el-radio-button__inner) {
  border-radius: 16px;
}

/* Main content */
.main-content {
  display: flex;
  gap: 16px;
  flex: 1;
  min-height: 0;
}

.chart-wrapper {
  flex: 1;
  background: var(--el-bg-color);
  border-radius: 8px;
  padding: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 0;
}

.chart-loading,
.chart-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--el-text-color-secondary);
  height: 100%;
}

/* Indicators panel */
.indicators-panel {
  width: 300px;
  flex-shrink: 0;
  background: var(--el-bg-color);
  border-radius: 8px;
  padding: 16px;
  overflow-y: auto;
}

.panel-header {
  margin-bottom: 16px;
}

.panel-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--el-text-color-primary);
}

.indicator-section {
  margin-bottom: 20px;
}

.section-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--el-text-color-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 10px;
}

.indicator-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.indicator-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  background: var(--el-fill-color-light);
  padding: 8px 10px;
  border-radius: 6px;
}

.indicator-label {
  font-size: 11px;
  color: var(--el-text-color-secondary);
}

.indicator-value {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  font-family: monospace;
}

.price-value {
  font-size: 15px;
}

.high-value {
  color: #67c23a;
}

.low-value {
  color: #f56c6c;
}

.up {
  color: #67c23a;
}

.down {
  color: #f56c6c;
}

.quality-badge {
  display: inline-block;
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 500;
  font-family: inherit;
}

.quality-normal { background: rgba(103, 194, 58, 0.15); color: #67c23a; }
.quality-anomaly { background: rgba(245, 108, 108, 0.15); color: #f56c6c; }
.quality-missing { background: rgba(230, 162, 60, 0.15); color: #e6a23c; }
.quality-duplicate { background: rgba(144, 147, 153, 0.15); color: #909399; }
.quality-suspicious { background: rgba(230, 162, 60, 0.15); color: #e6a23c; }

/* BOLL colors */
.boll-up { color: #f56c6c; }
.boll-mid { color: #7170ff; }
.boll-low { color: #67c23a; }

/* RSI overbought/oversold */
.rsi-overbought { color: #f56c6c; }
.rsi-oversold { color: #67c23a; }

/* Depth panel */
.depth-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.depth-bids,
.depth-asks {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.depth-header {
  font-size: 11px;
  font-weight: 600;
  color: var(--el-text-color-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 4px;
}

.depth-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
  font-family: monospace;
}

.bid-row { background: rgba(103, 194, 58, 0.08); }
.ask-row { background: rgba(245, 108, 108, 0.08); }

.depth-price { color: var(--el-text-color-primary); font-weight: 500; }
.bid-row .depth-price { color: #67c23a; }
.ask-row .depth-price { color: #f56c6c; }

.depth-amount { color: var(--el-text-color-secondary); }

.depth-empty {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  text-align: center;
  padding: 8px 0;
}
</style>
