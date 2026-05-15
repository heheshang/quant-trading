<template>
  <div class="backtest-performance-report">
    <!-- Loading state -->
    <div v-if="loading" class="report-skeleton">
      <div v-for="n in 8" :key="n" class="skeleton-row">
        <div class="skeleton-label"></div>
        <div class="skeleton-value"></div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-else-if="!result" class="report-empty">
      <el-empty description="暂无绩效数据" :image-size="60" />
    </div>

    <!-- Data state: dual-column grid -->
    <div v-else class="report-grid">
      <!-- 总收益率 -->
      <div class="report-row">
        <div class="report-label">总收益率</div>
        <div class="report-value" :class="colorClass(result.metrics.total_return_pct, 'pnl')">
          {{ formatPercent(result.metrics.total_return_pct) }}
        </div>
      </div>

      <!-- 年化收益率 -->
      <div class="report-row">
        <div class="report-label">年化收益率</div>
        <div class="report-value" :class="colorClass(result.metrics.annualized_return_pct, 'pnl')">
          {{ formatPercent(result.metrics.annualized_return_pct) }}
        </div>
      </div>

      <!-- 夏普比率 -->
      <div class="report-row">
        <div class="report-label">夏普比率</div>
        <div class="report-value" :class="colorClass(result.metrics.sharpe_ratio, 'sharpe')">
          {{ result.metrics.sharpe_ratio.toFixed(2) }}
        </div>
      </div>

      <!-- 卡玛比率 -->
      <div class="report-row">
        <div class="report-label">卡玛比率</div>
        <div class="report-value" :class="colorClass(result.metrics.calmar_ratio, 'sharpe')">
          {{ result.metrics.calmar_ratio.toFixed(2) }}
        </div>
      </div>

      <!-- 最大回撤 -->
      <div class="report-row">
        <div class="report-label">最大回撤</div>
        <div class="report-value down">
          {{ formatPercent(result.metrics.max_drawdown_pct) }}
        </div>
      </div>

      <!-- 最大回撤区间 -->
      <div class="report-row">
        <div class="report-label">最大回撤区间</div>
        <div class="report-value tertiary">
          {{ maxDrawdownPeriod }}
        </div>
      </div>

      <!-- 胜率 -->
      <div class="report-row">
        <div class="report-label">胜率</div>
        <div class="report-value" :class="result.metrics.win_rate >= 50 ? 'up' : 'neutral'">
          {{ result.metrics.win_rate.toFixed(1) }}%
        </div>
      </div>

      <!-- 总交易次数 -->
      <div class="report-row">
        <div class="report-label">总交易次数</div>
        <div class="report-value neutral">
          {{ result.metrics.total_trades }}
        </div>
      </div>

      <!-- 盈利交易数 -->
      <div class="report-row">
        <div class="report-label">盈利交易数</div>
        <div class="report-value up">
          {{ winningTrades }}
        </div>
      </div>

      <!-- 亏损交易数 -->
      <div class="report-row">
        <div class="report-label">亏损交易数</div>
        <div class="report-value down">
          {{ losingTrades }}
        </div>
      </div>

      <!-- 平均盈亏 -->
      <div class="report-row">
        <div class="report-label">平均盈亏</div>
        <div class="report-value" :class="colorClass(avgPnL, 'pnl')">
          {{ formatAvgPnL }}
        </div>
      </div>

      <!-- 盈亏比 (Profit Factor) -->
      <div class="report-row">
        <div class="report-label">盈亏比</div>
        <div class="report-value" :class="colorClass(profitFactorValue, 'pf')">
          {{ profitFactorDisplay }}
        </div>
      </div>

      <!-- 平均持仓时长 -->
      <div class="report-row">
        <div class="report-label">平均持仓时长</div>
        <div class="report-value secondary">
          {{ avgHoldingPeriod }}
        </div>
      </div>

      <!-- 最大连续盈利 -->
      <div class="report-row">
        <div class="report-label">最大连续盈利</div>
        <div class="report-value up">
          {{ maxConsecutiveWins }}
        </div>
      </div>

      <!-- 最大连续亏损 -->
      <div class="report-row">
        <div class="report-label">最大连续亏损</div>
        <div class="report-value down">
          {{ maxConsecutiveLosses }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { BacktestResultResponse } from '@/types/backtest'
import { useFormat } from '@/composables/useFormat'

const { formatPercent } = useFormat()

const props = defineProps<{
  result: BacktestResultResponse | null
  loading?: boolean
}>()

// Derived metrics from trades
const trades = computed(() => props.result?.trades ?? [])

const winningTrades = computed(() => trades.value.filter(t => t.pnl_usdt > 0).length)
const losingTrades = computed(() => trades.value.filter(t => t.pnl_usdt < 0).length)

const avgPnL = computed(() => {
  if (trades.value.length === 0) return 0
  const total = trades.value.reduce((sum, t) => sum + t.pnl_usdt, 0)
  return total / trades.value.length
})

const formatAvgPnL = computed(() => {
  const val = avgPnL.value
  const sign = val > 0 ? '+' : ''
  return `${sign}${val.toFixed(2)} USDT`
})

const profitFactorValue = computed(() => props.result?.metrics.profit_factor ?? null)

const profitFactorDisplay = computed(() => {
  const pf = profitFactorValue.value
  if (pf === null) return '∞'
  return pf.toFixed(2)
})

// Max drawdown period — computed from equity curve
const maxDrawdownPeriod = computed(() => {
  const curve = props.result?.equity_curve
  if (!curve || curve.length === 0) return '--'

  let maxDDStart = 0
  let maxDDEnd = 0
  let peakIdx = 0
  let maxDD = 0

  for (let i = 1; i < curve.length; i++) {
    if (curve[i].equity > curve[peakIdx].equity) {
      peakIdx = i
    }
    const dd = (curve[peakIdx].equity - curve[i].equity) / curve[peakIdx].equity
    if (dd > maxDD) {
      maxDD = dd
      maxDDStart = peakIdx
      maxDDEnd = i
    }
  }

  const fmtDate = (ts: number) => {
    const d = new Date(ts)
    const y = d.getFullYear()
    const m = String(d.getMonth() + 1).padStart(2, '0')
    const day = String(d.getDate()).padStart(2, '0')
    return `${y}-${m}-${day}`
  }

  return `${fmtDate(curve[maxDDStart].time)} ~ ${fmtDate(curve[maxDDEnd].time)}`
})

// Average holding period
const avgHoldingPeriod = computed(() => {
  if (trades.value.length === 0) return '--'
  const totalMs = trades.value.reduce((sum, t) => sum + t.holding_period_ms, 0)
  const avgMs = totalMs / trades.value.length
  return formatDuration(avgMs)
})

// Max consecutive wins/losses
const maxConsecutiveWins = computed(() => {
  let max = 0, cur = 0
  for (const t of trades.value) {
    if (t.pnl_usdt > 0) { cur++; if (cur > max) max = cur }
    else cur = 0
  }
  return max
})

const maxConsecutiveLosses = computed(() => {
  let max = 0, cur = 0
  for (const t of trades.value) {
    if (t.pnl_usdt < 0) { cur++; if (cur > max) max = cur }
    else cur = 0
  }
  return max
})

// Color class helpers
function colorClass(value: number | null, type: 'pnl' | 'sharpe' | 'pf'): string {
  if (type === 'pnl') {
    if (value === null) return 'neutral'
    return value >= 0 ? 'up' : 'down'
  }
  if (type === 'pf') {
    if (value === null) return 'up' // ∞ is excellent
    if (value >= 2) return 'up'
    if (value >= 1.5) return 'neutral'
    return 'down'
  }
  if (type === 'sharpe') {
    if (value === null) return 'neutral'
    if (value >= 1.5) return 'up'
    if (value >= 1) return 'neutral'
    return 'warning'
  }
  return 'neutral'
}

function formatDuration(ms: number): string {
  const totalSec = Math.floor(ms / 1000)
  const days = Math.floor(totalSec / 86400)
  const hours = Math.floor((totalSec % 86400) / 3600)
  const minutes = Math.floor((totalSec % 3600) / 60)

  const parts: string[] = []
  if (days > 0) parts.push(`${days}d`)
  if (hours > 0) parts.push(`${hours}h`)
  if (minutes > 0) parts.push(`${minutes}m`)
  if (parts.length === 0) parts.push('<1m')
  return parts.join(' ')
}

defineExpose({
  winningTrades,
  losingTrades,
  avgPnL,
  formatAvgPnL,
  profitFactorValue,
  profitFactorDisplay,
  maxDrawdownPeriod,
  avgHoldingPeriod,
  maxConsecutiveWins,
  maxConsecutiveLosses,
  colorClass,
  formatDuration,
})
</script>

<style scoped lang="scss">
.backtest-performance-report {
  padding: 4px 0;
}

.report-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0;
}

@media (max-width: 768px) {
  .report-grid {
    grid-template-columns: 1fr;
  }
}

.report-row {
  padding: 12px;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.06));
  display: flex;
  justify-content: space-between;
  align-items: center;

  &:nth-last-child(-n+2) {
    border-bottom: none;
  }
}

@media (max-width: 768px) {
  .report-row:last-child {
    border-bottom: none;
  }
}

.report-label {
  font-size: 13px;
  color: var(--color-text-tertiary, rgba(255, 255, 255, 0.45));
  flex-shrink: 0;
}

.report-value {
  font-size: 14px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  text-align: right;
  font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace;

  &.up {
    color: #10b981;
  }

  &.down {
    color: #E5484D;
  }

  &.neutral {
    color: var(--color-text-primary, rgba(255, 255, 255, 0.92));
  }

  &.warning {
    color: #f5a623;
  }

  &.secondary {
    color: var(--color-text-secondary, rgba(255, 255, 255, 0.68));
  }

  &.tertiary {
    color: var(--color-text-tertiary, rgba(255, 255, 255, 0.45));
  }
}

.report-skeleton {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 0;
}

.skeleton-row {
  padding: 12px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.skeleton-label {
  height: 14px;
  width: 60%;
  border-radius: 4px;
  background: linear-gradient(90deg, var(--color-surface, #1a1a2e) 25%, rgba(255,255,255,0.04) 50%, var(--color-surface, #1a1a2e) 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
}

.skeleton-value {
  height: 16px;
  width: 30%;
  border-radius: 4px;
  background: linear-gradient(90deg, var(--color-surface, #1a1a2e) 25%, rgba(255,255,255,0.04) 50%, var(--color-surface, #1a1a2e) 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite 0.2s;
}

.report-empty {
  padding: 40px;
  text-align: center;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
</style>
