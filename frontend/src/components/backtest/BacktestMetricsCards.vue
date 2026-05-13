<template>
  <div class="backtest-metrics-cards">
    <div class="metrics-header">
      <h3 class="section-title">绩效指标</h3>
    </div>

    <!-- Loading state -->
    <div v-if="loading" class="metrics-skeleton">
      <div class="skeleton-grid">
        <div v-for="n in 6" :key="n" class="skeleton-card">
          <div class="skeleton-label"></div>
          <div class="skeleton-value"></div>
          <div class="skeleton-sub"></div>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-else-if="!result" class="metrics-empty">
      <el-empty description="暂无回测结果" :image-size="60" />
    </div>

    <!-- Data state -->
    <div v-else class="metrics-grid">
      <div class="metric-card">
        <div class="metric-label">总收益率</div>
        <div class="metric-value" :class="result.metrics.total_return_pct >= 0 ? 'up' : 'down'">
          {{ formatPercent(result.metrics.total_return_pct) }}
        </div>
        <div class="metric-sub" :class="result.metrics.annualized_return_pct >= 0 ? 'up' : 'down'">
          年化 {{ formatPercent(result.metrics.annualized_return_pct) }}
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-label">年化收益率</div>
        <div class="metric-value" :class="result.metrics.annualized_return_pct >= 0 ? 'up' : 'down'">
          {{ formatPercent(result.metrics.annualized_return_pct) }}
        </div>
        <div class="metric-sub" :class="result.metrics.total_return_pct >= 0 ? 'up' : 'down'">
          总收益 {{ formatPercent(result.metrics.total_return_pct) }}
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-label">夏普比率</div>
        <div class="metric-value neutral">
          {{ result.metrics.sharpe_ratio.toFixed(2) }}
        </div>
        <div class="metric-sub down">
          最大回撤 {{ formatPercent(result.metrics.max_drawdown_pct) }}
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-label">最大回撤</div>
        <div class="metric-value down">
          {{ formatPercent(result.metrics.max_drawdown_pct) }}
        </div>
        <div class="metric-sub neutral">
          夏普 {{ result.metrics.sharpe_ratio.toFixed(2) }}
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-label">胜率</div>
        <div class="metric-value neutral">
          {{ result.metrics.win_rate.toFixed(1) }}%
        </div>
        <div class="metric-sub neutral">
          交易 {{ result.metrics.total_trades }} 笔
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-label">交易次数</div>
        <div class="metric-value neutral">
          {{ result.metrics.total_trades }}
        </div>
        <div class="metric-sub neutral">
          胜率 {{ result.metrics.win_rate.toFixed(1) }}%
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-label">Sortino比率</div>
        <div class="metric-value neutral">
          {{ result.metrics.sortino_ratio.toFixed(2) }}
        </div>
        <div class="metric-sub neutral">
          Calmar {{ result.metrics.calmar_ratio.toFixed(2) }}
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-label">盈亏比</div>
        <div class="metric-value neutral">
          {{ result.metrics.profit_factor != null ? result.metrics.profit_factor.toFixed(2) : '∞' }}
        </div>
        <div class="metric-sub neutral">
          均盈 {{ formatPercent(result.metrics.avg_win_pct) }} / 均亏 {{ formatPercent(result.metrics.avg_loss_pct) }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { BacktestResultDetail } from '@/types/backtest'
import { useFormat } from '@/composables/useFormat'

const { formatPercent } = useFormat()

defineProps<{
  result: BacktestResultDetail | null
  loading?: boolean
}>()
</script>

<style scoped lang="scss">
.backtest-metrics-cards {
  margin-bottom: 0;
}

.metrics-header {
  margin-bottom: 16px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}

@media (max-width: 1024px) {
  .metrics-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 640px) {
  .metrics-grid {
    grid-template-columns: 1fr;
  }
}

.metric-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 10px;
  padding: 20px;
  text-align: center;
  transition: border-color 0.2s, box-shadow 0.2s;

  &:hover {
    border-color: var(--color-border-hover);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
  }
}

.metric-label {
  font-size: 13px;
  color: var(--color-text-tertiary);
  margin-bottom: 8px;
}

.metric-value {
  font-size: 24px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  margin-bottom: 4px;

  &.up {
    color: #10b981;
  }

  &.down {
    color: #E5484D;
  }

  &.neutral {
    color: var(--color-text-primary);
  }
}

.metric-sub {
  font-size: 12px;
  font-weight: 500;
  font-variant-numeric: tabular-nums;

  &.up {
    color: #10b981;
  }

  &.down {
    color: #E5484D;
  }

  &.neutral {
    color: var(--color-text-tertiary);
  }
}

.metrics-skeleton {
  .skeleton-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
  }

  .skeleton-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    padding: 20px;
    text-align: center;
  }

  .skeleton-label {
    height: 14px;
    width: 60%;
    margin: 0 auto 12px;
    border-radius: 4px;
    background: linear-gradient(90deg, var(--color-surface) 25%, rgba(255,255,255,0.04) 50%, var(--color-surface) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.5s ease-in-out infinite;
  }

  .skeleton-value {
    height: 28px;
    width: 40%;
    margin: 0 auto 6px;
    border-radius: 4px;
    background: linear-gradient(90deg, var(--color-surface) 25%, rgba(255,255,255,0.04) 50%, var(--color-surface) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.5s ease-in-out infinite 0.3s;
  }

  .skeleton-sub {
    height: 12px;
    width: 50%;
    margin: 0 auto;
    border-radius: 4px;
    background: linear-gradient(90deg, var(--color-surface) 25%, rgba(255,255,255,0.04) 50%, var(--color-surface) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.5s ease-in-out infinite 0.5s;
  }
}

.metrics-empty {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 10px;
  padding: 40px;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
</style>