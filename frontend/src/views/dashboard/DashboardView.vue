<template>
  <div class="dashboard-view">
    <div class="page-header">
      <h1>Dashboard</h1>
    </div>

    <!-- Loading State -->
    <template v-if="loading">
      <div class="stats-row">
        <div v-for="n in 4" :key="n" class="stat-card stat-card--skeleton">
          <div class="skeleton-label"></div>
          <div class="skeleton-value"></div>
        </div>
      </div>
      <div class="chart-card chart-card--skeleton">
        <div class="skeleton-chart"></div>
      </div>
    </template>

    <!-- Error State -->
    <template v-else-if="error">
      <div class="error-state">
        <el-result icon="error" title="Failed to load dashboard" sub-title="Something went wrong">
          <template #extra>
            <el-button type="primary" @click="loadData">Retry</el-button>
          </template>
        </el-result>
      </div>
    </template>

    <!-- Normal State -->
    <template v-else>
      <!-- Stat Cards Row -->
      <div class="stats-row">
        <div class="stat-card">
          <div class="stat-header">
            <el-icon class="stat-icon"><TrendCharts /></el-icon>
            <span class="stat-label">Total PnL</span>
          </div>
          <div class="stat-value" :class="pnlClass(stats.totalPnl)">
            {{ f.formatCurrency(stats.totalPnl) }}
          </div>
          <div class="stat-change" :class="pnlClass(stats.totalPnl)">
            <span v-if="stats.totalPnl > 0">&#9650;</span>
            <span v-else-if="stats.totalPnl < 0">&#9660;</span>
            <span v-else>&mdash;</span>
            {{ f.formatPercent(stats.totalPnl / 10000) }}
          </div>
        </div>
        <div class="stat-card">
          <div class="stat-header">
            <el-icon class="stat-icon"><Aim /></el-icon>
            <span class="stat-label">Win Rate</span>
          </div>
          <div class="stat-value winrate">{{ f.formatPercent(stats.winRate) }}</div>
          <div class="stat-change neutral">Last 30 days</div>
        </div>
        <div class="stat-card">
          <div class="stat-header">
            <el-icon class="stat-icon"><Lightning /></el-icon>
            <span class="stat-label">Sharpe Ratio</span>
          </div>
          <div class="stat-value sharpe">{{ f.formatNumber(stats.sharpeRatio, 2) }}</div>
          <div class="stat-change neutral">Annualized</div>
        </div>
        <div class="stat-card">
          <div class="stat-header">
            <el-icon class="stat-icon"><Briefcase /></el-icon>
            <span class="stat-label">Active Positions</span>
          </div>
          <div class="stat-value positions">{{ stats.activePositions }}</div>
          <div class="stat-change neutral">Currently open</div>
        </div>
      </div>

      <!-- PnL Chart -->
      <div class="chart-card">
        <div class="chart-header">
          <h3>Profit &amp; Loss</h3>
          <div class="chart-range">
            <button
              v-for="r in ranges"
              :key="r"
              class="range-pill"
              :class="{ active: activeRange === r }"
              @click="activeRange = r"
            >
              {{ r }}
            </button>
          </div>
        </div>
        <PnLChart :data="pnlData" />
      </div>

      <!-- Bottom Row -->
      <div class="bottom-row">
        <!-- Recent Trades -->
        <div class="section-card">
          <div class="section-header">
            <h3>Recent Trades</h3>
            <router-link to="/trading" class="section-link">View all &rarr;</router-link>
          </div>
          <el-table :data="recentTrades" style="width: 100%" size="small" stripe>
            <el-table-column prop="time" label="Time" width="80" />
            <el-table-column prop="symbol" label="Symbol" width="100" />
            <el-table-column prop="side" label="Side" width="70">
              <template #default="{ row }">
                <el-tag :type="row.side === 'buy' ? 'success' : 'danger'" size="small" effect="dark">
                  {{ row.side.toUpperCase() }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="price" label="Price" width="100" align="right">
              <template #default="{ row }">{{ f.formatPrice(row.price) }}</template>
            </el-table-column>
            <el-table-column prop="quantity" label="Qty" width="80" align="right">
              <template #default="{ row }">{{ f.formatNumber(row.quantity, 4) }}</template>
            </el-table-column>
            <el-table-column prop="pnl" label="PnL" width="100" align="right">
              <template #default="{ row }">
                <span :class="pnlClass(row.pnl)">{{ f.formatCurrency(row.pnl) }}</span>
              </template>
            </el-table-column>
          </el-table>
          <el-empty v-if="recentTrades.length === 0" description="No recent trades" :image-size="60" />
        </div>

        <!-- Top Strategies -->
        <div class="section-card">
          <div class="section-header">
            <h3>Top Strategies</h3>
            <router-link to="/strategies" class="section-link">View all &rarr;</router-link>
          </div>
          <div v-for="s in topStrategies" :key="s.id" class="strategy-item">
            <div class="strategy-info">
              <span class="strategy-name">{{ s.name }}</span>
              <span class="strategy-desc">{{ s.description }}</span>
            </div>
            <div class="strategy-metrics">
              <span class="strategy-pnl" :class="pnlClass(s.pnl)">{{ f.formatCurrency(s.pnl) }}</span>
              <span class="strategy-label">Sharpe {{ f.formatNumber(s.sharpe, 2) }}</span>
            </div>
          </div>
          <el-empty v-if="topStrategies.length === 0" description="No strategies yet" :image-size="60" />
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useFormat } from '@/composables/useFormat'
import PnLChart from '@/components/charts/PnLChart.vue'
import type { DashboardStats, PnLPoint } from '@/types'

const f = useFormat()
const loading = ref(true)
const error = ref(false)
const activeRange = ref('1M')
const ranges = ['1W', '1M', '3M', '6M', '1Y', 'All']

const stats = ref<DashboardStats>({
  totalPnl: 0,
  winRate: 0,
  sharpeRatio: 0,
  activePositions: 0,
})

const pnlData = ref<PnLPoint[]>([])
const recentTrades = ref<any[]>([])
const topStrategies = ref<any[]>([])

function pnlClass(value: number) {
  if (value > 0) return 'positive'
  if (value < 0) return 'negative'
  return 'neutral'
}

function loadData() {
  loading.value = true
  error.value = false

  // Mock data
  setTimeout(() => {
    stats.value = {
      totalPnl: 45231.67,
      winRate: 62.5,
      sharpeRatio: 1.89,
      activePositions: 8,
    }

    pnlData.value = [
      { date: '2025-11', value: 10000 },
      { date: '2025-12', value: 15230 },
      { date: '2026-01', value: 18900 },
      { date: '2026-02', value: 22340 },
      { date: '2026-03', value: 28190 },
      { date: '2026-04', value: 37540 },
      { date: '2026-05', value: 45231.67 },
    ]

    recentTrades.value = [
      { id: 1, symbol: 'BTC/USDT', side: 'buy', price: 45678.90, quantity: 0.1234, pnl: 234.56, time: '14:32:15' },
      { id: 2, symbol: 'ETH/USDT', side: 'sell', price: 3120.45, quantity: 1.5000, pnl: -89.12, time: '14:28:03' },
      { id: 3, symbol: 'SOL/USDT', side: 'buy', price: 142.30, quantity: 5.0000, pnl: 345.00, time: '14:15:44' },
      { id: 4, symbol: 'BTC/USDT', side: 'sell', price: 45800.00, quantity: 0.0500, pnl: 121.10, time: '13:55:22' },
      { id: 5, symbol: 'ETH/USDT', side: 'buy', price: 3090.20, quantity: 0.8000, pnl: 45.60, time: '13:42:10' },
    ]

    topStrategies.value = [
      { id: 1, name: 'Grid Bot BTC', description: 'Mean reversion grid on BTC', pnl: 12890.45, sharpe: 2.13, status: 'running' },
      { id: 2, name: 'MA Crossover ETH', description: 'EMA 12/26 crossover', pnl: 8901.23, sharpe: 1.76, status: 'running' },
      { id: 3, name: 'Arbitrage SOL', description: 'Cross-exchange arbitrage', pnl: 5432.10, sharpe: 1.45, status: 'running' },
      { id: 4, name: 'Momentum Trader', description: 'RSI + MACD momentum', pnl: 3200.89, sharpe: 1.22, status: 'paused' },
    ]

    loading.value = false
  }, 600)
}

onMounted(() => {
  loadData()
})
</script>

<style scoped lang="scss">
.dashboard-view {
  max-width: 1344px;
  overflow: hidden;
  animation: fadeIn var(--transition-base);
}

.page-header {
  margin-bottom: 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;

  h1 {
    font-size: 22px;
    font-weight: 600;
    font-family: var(--font-heading);
    color: var(--color-text-primary);
    margin: 0;
    letter-spacing: -0.3px;
  }
}

// Stat Cards
.stats-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 24px;
}

@media (max-width: 1024px) {
  .stats-row {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 640px) {
  .stats-row {
    grid-template-columns: 1fr;
  }
}

.stat-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 18px 20px;
  transition: all var(--transition-base);
  box-shadow: var(--shadow-card);
  position: relative;
  overflow: hidden;

  &::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--gradient-primary);
    opacity: 0;
    transition: opacity var(--transition-base);
  }

  &:hover {
    border-color: var(--color-border-hover);
    transform: translateY(-2px);
    box-shadow: var(--shadow-card-hover);

    &::before {
      opacity: 1;
    }
  }

  &--skeleton {
    .skeleton-label {
      width: 60px;
      height: 14px;
      border-radius: 4px;
      background: linear-gradient(90deg, var(--color-surface-elevated) 25%, rgba(255,255,255,0.05) 50%, var(--color-surface-elevated) 75%);
      background-size: 200% 100%;
      margin-bottom: 12px;
      animation: shimmer 1.5s ease-in-out infinite;
    }
    .skeleton-value {
      width: 100px;
      height: 28px;
      border-radius: 4px;
      background: linear-gradient(90deg, var(--color-surface-elevated) 25%, rgba(255,255,255,0.05) 50%, var(--color-surface-elevated) 75%);
      background-size: 200% 100%;
      animation: shimmer 1.5s ease-in-out infinite 0.2s;
    }
  }
}

.stat-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.stat-icon {
  font-size: 18px;
  color: var(--color-primary);
  filter: drop-shadow(0 0 4px var(--color-primary-glow));
}

.stat-label {
  font-size: 12px;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--color-text-tertiary);
}

.stat-value {
  font-size: 26px;
  font-weight: 600;
  font-family: var(--font-heading);
  color: var(--color-text-primary);
  margin-bottom: 4px;
  letter-spacing: -0.5px;

  &.winrate {
    color: var(--color-primary);
    text-shadow: 0 0 10px var(--color-primary-glow);
  }
  &.sharpe {
    color: var(--color-primary);
    text-shadow: 0 0 10px var(--color-primary-glow);
  }
  &.positions {
    color: var(--color-primary);
    text-shadow: 0 0 10px var(--color-primary-glow);
  }
}

.stat-change {
  font-size: 12px;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 4px;

  &.positive {
    color: var(--color-stat-positive);
    animation: pulseSubtle 2s ease-in-out infinite;
  }
  &.negative {
    color: var(--color-stat-negative);
  }
  &.neutral {
    color: var(--color-text-tertiary);
  }
}

// Chart Card
.chart-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 20px;
  margin-bottom: 24px;
  box-shadow: var(--shadow-card);
  transition: all var(--transition-base);

  &:hover {
    box-shadow: var(--shadow-card-hover);
    border-color: var(--color-border-hover);
  }

  &--skeleton {
    .skeleton-chart {
      height: 320px;
      border-radius: 8px;
      background: linear-gradient(90deg, var(--color-surface-elevated) 25%, rgba(255,255,255,0.03) 50%, var(--color-surface-elevated) 75%);
      background-size: 200% 100%;
      animation: shimmer 1.5s ease-in-out infinite;
    }
  }
}

.chart-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;

  h3 {
    font-size: 15px;
    font-weight: 600;
    font-family: var(--font-heading);
    color: var(--color-text-primary);
    margin: 0;
    letter-spacing: -0.2px;
  }
}

.chart-range {
  display: flex;
  gap: 4px;
  background: rgba(255, 255, 255, 0.04);
  padding: 3px;
  border-radius: var(--radius-md);
}

.range-pill {
  padding: 5px 12px;
  border-radius: var(--radius-sm);
  border: none;
  background: transparent;
  color: var(--color-text-tertiary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);

  &:hover {
    color: var(--color-text-secondary);
    background: rgba(255, 255, 255, 0.04);
  }

  &.active {
    background: var(--gradient-primary);
    color: #fff;
    box-shadow: var(--shadow-glow-primary);
  }
}

// Bottom Row
.bottom-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

@media (max-width: 1024px) {
  .bottom-row {
    grid-template-columns: 1fr;
  }
}

.section-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 18px 20px;
  box-shadow: var(--shadow-card);
  transition: all var(--transition-base);

  &:hover {
    box-shadow: var(--shadow-card-hover);
    border-color: var(--color-border-hover);
  }
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 14px;

  h3 {
    font-size: 14px;
    font-weight: 600;
    font-family: var(--font-heading);
    color: var(--color-text-primary);
    margin: 0;
    letter-spacing: -0.2px;
  }
}

.section-link {
  font-size: 13px;
  color: var(--color-accent);
  text-decoration: none;
  font-weight: 500;
  transition: all var(--transition-fast);
  padding: 4px 8px;
  border-radius: 4px;

  &:hover {
    color: var(--color-accent-hover);
    background: var(--color-accent-light);
  }
}

// Strategy Items
.strategy-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px;
  margin: 0 -8px;
  border-radius: 8px;
  transition: all var(--transition-fast);
  border-bottom: 1px solid transparent;

  &:hover {
    background: rgba(255, 255, 255, 0.03);
    border-bottom-color: var(--color-border);
  }

  &:last-child {
    border-bottom: none;
  }
}

.strategy-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.strategy-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-primary);
  letter-spacing: -0.2px;
}

.strategy-desc {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.strategy-metrics {
  text-align: right;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.strategy-pnl {
  display: block;
  font-size: 15px;
  font-weight: 700;
  font-family: var(--font-mono);
  letter-spacing: -0.3px;
}

.strategy-label {
  font-size: 11px;
  color: var(--color-text-tertiary);
  font-weight: 500;
}

// States
.error-state {
  display: flex;
  justify-content: center;
  padding: 80px 0;
  animation: fadeIn var(--transition-base);
}

// Animations
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

// Colors
.positive { color: var(--color-stat-positive); }
.negative { color: var(--color-stat-negative); }
.neutral { color: var(--color-text-tertiary); }
</style>
