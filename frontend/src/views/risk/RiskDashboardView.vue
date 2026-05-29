<template>
  <div class="risk-dashboard">
    <!-- Header -->
    <div class="risk-header">
      <h1 class="page-title">风控面板</h1>
      <div class="header-controls">
        <!-- F15: Connection Status Indicator -->
        <div class="connection-indicator" :class="`connection-indicator--${disconnectLevel}`">
          <span class="connection-dot" />
          <span class="connection-label">{{ disconnectLabel }}</span>
          <el-tag v-if="connectionStatus?.strategy_paused" type="warning" size="small">
            策略已暂停
          </el-tag>
        </div>
        <el-button circle plain :icon="Refresh" @click="handleRefresh" />
      </div>
    </div>

    <!-- Top Action Bar -->
    <div class="action-bar">
      <!-- F14: Emergency Close Button -->
      <el-button
        type="danger"
        size="large"
        :loading="emergencyLoading"
        class="btn-emergency"
        @click="handleEmergencyClose"
      >
        <el-icon class="el-icon--left"><WarningFilled /></el-icon>
        紧急全平 (F14)
      </el-button>

      <div class="action-info">
        <span v-if="emergencyError" class="error-text">{{ emergencyError }}</span>
      </div>
    </div>

    <!-- Rules Overview Cards -->
    <el-row :gutter="16" class="overview-cards">
      <el-col :xs="24" :sm="12" :lg="6">
        <div class="metric-card" :class="{ 'metric-card--loading': rulesLoading }">
          <div class="metric-card__header">
            <span class="metric-card__label">当日亏损限额</span>
            <el-icon class="metric-card__icon" :size="24"><Wallet /></el-icon>
          </div>
          <template v-if="rulesLoading">
            <div class="metric-card__skeleton">
              <div class="skeleton-line" style="width: 100px; height: 28px;" />
            </div>
          </template>
          <template v-else>
            <div class="metric-card__value">{{ rules?.daily_loss_limit ?? '--' }} U</div>
            <div class="metric-card__sub">
              <span v-if="rules?.daily_loss_auto_close" class="tag tag--active">自动平仓</span>
              <span v-else class="tag tag--inactive">自动平仓</span>
            </div>
          </template>
        </div>
      </el-col>

      <el-col :xs="24" :sm="12" :lg="6">
        <div class="metric-card" :class="{ 'metric-card--loading': rulesLoading }">
          <div class="metric-card__header">
            <span class="metric-card__label">单笔最大亏损比例</span>
            <el-icon class="metric-card__icon" :size="24"><Connection /></el-icon>
          </div>
          <template v-if="rulesLoading">
            <div class="metric-card__skeleton">
              <div class="skeleton-line" style="width: 80px; height: 28px;" />
            </div>
          </template>
          <template v-else>
            <div class="metric-card__value">
              {{ rules ? `${(parseFloat(rules.single_trade_loss_ratio) * 100).toFixed(1)}%` : '--' }}
            </div>
            <div class="metric-card__sub">单笔交易</div>
          </template>
        </div>
      </el-col>

      <el-col :xs="24" :sm="12" :lg="6">
        <div class="metric-card" :class="{ 'metric-card--loading': rulesLoading }">
          <div class="metric-card__header">
            <span class="metric-card__label">最大回撤比例</span>
            <el-icon class="metric-card__icon" :size="24"><DataLine /></el-icon>
          </div>
          <template v-if="rulesLoading">
            <div class="metric-card__skeleton">
              <div class="skeleton-line" style="width: 80px; height: 28px;" />
            </div>
          </template>
          <template v-else>
            <div class="metric-card__value">
              {{ rules ? `${(parseFloat(rules.max_drawdown_ratio) * 100).toFixed(1)}%` : '--' }}
            </div>
            <div class="metric-card__sub">
              <span v-if="rules?.drawdown_auto_close" class="tag tag--active">自动平仓</span>
              <span v-else class="tag tag--inactive">自动平仓</span>
            </div>
          </template>
        </div>
      </el-col>

      <el-col :xs="24" :sm="12" :lg="6">
        <div class="metric-card" :class="{ 'metric-card--loading': rulesLoading }">
          <div class="metric-card__header">
            <span class="metric-card__label">止损类型</span>
            <el-icon class="metric-card__icon" :size="24"><Position /></el-icon>
          </div>
          <template v-if="rulesLoading">
            <div class="metric-card__skeleton">
              <div class="skeleton-line" style="width: 60px; height: 28px;" />
            </div>
          </template>
          <template v-else>
            <div class="metric-card__value">{{ rules?.stop_loss_type ?? '--' }}</div>
            <div class="metric-card__sub">
              <template v-if="rules?.stop_loss_type === 'atr'">
                ATR × {{ rules.atr_multiplier }} ({{ rules.atr_period }}期)
              </template>
              <template v-else>固定止损</template>
            </div>
          </template>
        </div>
      </el-col>
    </el-row>

    <!-- Risk Rules Detail Card -->
    <div class="rules-section">
      <div class="section-card">
        <div class="section-header">
          <span class="section-title">风控规则详情</span>
          <el-tag v-if="rules?.is_active" type="success" size="small">已激活</el-tag>
          <el-tag v-else type="info" size="small">未激活</el-tag>
        </div>
        <el-descriptions v-if="rules" :column="2" border size="small">
          <el-descriptions-item label="当日亏损限额">
            {{ rules.daily_loss_limit }} U
          </el-descriptions-item>
          <el-descriptions-item label="当日亏损自动平仓">
            {{ rules.daily_loss_auto_close ? '是' : '否' }}
          </el-descriptions-item>
          <el-descriptions-item label="单笔交易最大亏损比例">
            {{ `${(parseFloat(rules.single_trade_loss_ratio) * 100).toFixed(1)}%` }}
          </el-descriptions-item>
          <el-descriptions-item label="最大回撤比例">
            {{ `${(parseFloat(rules.max_drawdown_ratio) * 100).toFixed(1)}%` }}
          </el-descriptions-item>
          <el-descriptions-item label="回撤超限自动平仓">
            {{ rules.drawdown_auto_close ? '是' : '否' }}
          </el-descriptions-item>
          <el-descriptions-item label="止损类型">
            {{ rules.stop_loss_type === 'atr' ? `ATR (${rules.atr_period}期 × ${rules.atr_multiplier})` : '固定止损' }}
          </el-descriptions-item>
        </el-descriptions>
        <div v-else-if="rulesLoading" class="skeleton-block">
          <el-skeleton :rows="3" animated />
        </div>
        <el-empty v-else description="暂无风控规则" />
      </div>
    </div>

    <!-- Risk Logs -->
    <div class="logs-section">
      <div class="section-card">
        <div class="section-header">
          <span class="section-title">风控日志</span>
          <span class="log-count">共 {{ logsTotal }} 条</span>
        </div>

        <el-table
          v-loading="logsLoading"
          :data="logs"
          style="width: 100%"
          :header-cell-style="{ background: 'var(--color-surface)', color: 'var(--color-text-secondary)' }"
          empty-text="暂无风控日志"
        >
          <el-table-column prop="triggered_rule" label="触发规则" min-width="140" />
          <el-table-column prop="rule_type" label="规则类型" width="120">
            <template #default="{ row }">
              <el-tag size="small" :type="ruleTypeTagType(row.rule_type)">
                {{ row.rule_type }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="action" label="执行动作" width="120" />
          <el-table-column prop="severity" label="严重程度" width="100">
            <template #default="{ row }">
              <el-tag size="small" :type="severityTagType(row.severity)" :hit="true">
                {{ row.severity }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="details" label="详情" min-width="200" show-overflow-tooltip />
          <el-table-column prop="equity_snapshot" label="权益快照" width="130">
            <template #default="{ row }">{{ row.equity_snapshot }} U</template>
          </el-table-column>
          <el-table-column prop="threshold_snapshot" label="阈值快照" width="130">
            <template #default="{ row }">{{ row.threshold_snapshot }} U</template>
          </el-table-column>
          <el-table-column prop="created_at" label="触发时间" width="170">
            <template #default="{ row }">{{ formatTime(row.created_at) }}</template>
          </el-table-column>
        </el-table>

        <div v-if="logsTotal > 0" class="pagination-bar">
          <el-pagination
            v-model:current-page="logPage"
            :page-size="logPageSize"
            :total="logsTotal"
            layout="prev, pager, next"
            @current-change="handleLogPageChange"
          />
        </div>
      </div>
    </div>

    <!-- Emergency Close Result Dialog -->
    <el-dialog v-model="emergencyDialogVisible" title="紧急全平结果" width="500px" destroy-on-close>
      <div v-if="emergencyResult">
        <el-alert
          :title="emergencyResult.message"
          :type="emergencyResult.success ? 'success' : 'error'"
          :closable="false"
          style="margin-bottom: 16px;"
        />
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="平仓持仓数">{{ emergencyResult.closed_positions }}</el-descriptions-item>
          <el-descriptions-item label="总盈亏">{{ emergencyResult.total_pnl }} U</el-descriptions-item>
        </el-descriptions>
        <el-table
          v-if="emergencyResult.details?.length"
          :data="emergencyResult.details"
          size="small"
          style="margin-top: 12px;"
        >
          <el-table-column prop="order_id" label="订单ID" width="120" show-overflow-tooltip />
          <el-table-column prop="symbol" label="交易对" width="100" />
          <el-table-column prop="side" label="方向" width="70" />
          <el-table-column prop="executed_qty" label="成交数量" width="100" />
          <el-table-column prop="pnl" label="盈亏" width="90">
            <template #default="{ row }">
              <span :style="{ color: parseFloat(row.pnl) >= 0 ? 'var(--color-positive)' : 'var(--color-negative)' }">
                {{ row.pnl }}
              </span>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { ElMessageBox, ElMessage } from 'element-plus'
import { Refresh, WarningFilled, Wallet, Connection, DataLine, Position } from '@element-plus/icons-vue'
import { useRiskDashboard } from '@/composables/useRiskDashboard'
import type { EmergencyCloseResponse } from '@/types/risk'

const {
  rules,
  logs,
  logsTotal,
  connectionStatus,
  rulesLoading,
  logsLoading,
  emergencyLoading,
  emergencyError,
  isDisconnected,
  disconnectLevel,
  disconnectLabel,
  fetchRules,
  fetchLogs,
  fetchConnectionStatus,
  triggerEmergencyClose,
  refresh,
} = useRiskDashboard()

const logPage = ref(1)
const logPageSize = ref(20)
const emergencyDialogVisible = ref(false)
const emergencyResult = ref<EmergencyCloseResponse | null>(null)

let pollTimer: ReturnType<typeof setInterval> | null = null

// ─── Helpers ──────────────────────────────────────────────────
function formatTime(iso: string): string {
  if (!iso) return '--'
  return new Date(iso).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' })
}

function ruleTypeTagType(type: string): '' | 'success' | 'warning' | 'danger' | 'info' {
  const map: Record<string, '' | 'success' | 'warning' | 'danger' | 'info'> = {
    daily_loss: 'warning',
    single_trade: 'info',
    drawdown: 'danger',
  }
  return map[type] ?? ''
}

function severityTagType(severity: string): '' | 'success' | 'warning' | 'danger' | 'info' {
  const map: Record<string, '' | 'success' | 'warning' | 'danger' | 'info'> = {
    critical: 'danger',
    high: 'warning',
    medium: 'info',
    low: 'success',
  }
  return map[severity] ?? ''
}

// ─── Handlers ──────────────────────────────────────────────────
async function handleRefresh() {
  await refresh()
}

async function handleEmergencyClose() {
  try {
    await ElMessageBox.confirm(
      '确定执行紧急全平？所有持仓将被市价平掉。',
      '紧急全平确认',
      {
        confirmButtonText: '确认全平',
        cancelButtonText: '取消',
        type: 'warning',
        confirmButtonClass: 'el-button--danger',
      },
    )
  } catch {
    return // user cancelled
  }

  const result = await triggerEmergencyClose()
  if (result) {
    emergencyResult.value = result
    emergencyDialogVisible.value = true
    ElMessage({ type: result.success ? 'success' : 'error', message: result.message })
  } else if (emergencyError.value) {
    ElMessage({ type: 'error', message: emergencyError.value })
  }
}

function handleLogPageChange(page: number) {
  logPage.value = page
  fetchLogs(page, logPageSize.value)
}

// ─── Lifecycle ─────────────────────────────────────────────────
onMounted(async () => {
  await refresh()
  // poll connection status every 10s
  pollTimer = setInterval(fetchConnectionStatus, 10_000)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<style scoped lang="scss">
// Font imports — Outfit (UI) + Work Sans (body/mono fallback)
@import url('https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Work+Sans:wght@400;500;600;700&display=swap');

.risk-dashboard {
  padding: var(--layout-padding-desktop);
  display: flex;
  flex-direction: column;
  gap: 16px;

  @media (max-width: 767px) {
    padding: var(--layout-padding-mobile);
  }
}

// ─── Header ─────────────────────────────────────────────────
.risk-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 4px;
}

.page-title {
  font-family: 'Outfit', var(--font-ui), sans-serif;
  font-size: 22px;
  font-weight: 700;
  color: var(--color-text-primary);
  margin: 0;
  letter-spacing: -0.3px;
}

.header-controls {
  display: flex;
  align-items: center;
  gap: 12px;
}

// ─── F15: Connection Indicator ───────────────────────────────
.connection-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border-radius: var(--radius-lg);
  border: 1px solid transparent;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.3s ease;

  &--connected {
    background: rgba(59, 130, 246, 0.08);
    border-color: rgba(59, 130, 246, 0.3);
    color: var(--color-primary);

    .connection-dot {
      background: var(--color-primary);
      box-shadow: 0 0 8px var(--color-primary);
    }
  }

  &--warning {
    background: rgba(245, 166, 35, 0.08);
    border-color: rgba(245, 166, 35, 0.3);
    color: var(--color-warning);

    .connection-dot {
      background: var(--color-warning);
      animation: pulse 1.5s ease-in-out infinite;
    }
  }

  &--critical {
    background: rgba(229, 72, 77, 0.08);
    border-color: rgba(229, 72, 77, 0.3);
    color: var(--color-error);

    .connection-dot {
      background: var(--color-error);
      animation: pulse 0.8s ease-in-out infinite;
    }
  }

  &--unknown {
    background: var(--color-surface);
    border-color: var(--color-border);
    color: var(--color-text-tertiary);

    .connection-dot {
      background: var(--color-text-tertiary);
    }
  }
}

.connection-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.connection-label {
  font-family: 'Work Sans', var(--font-mono), monospace;
  font-weight: 500;
  letter-spacing: 0.3px;
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(1.3); }
}

// ─── Action Bar ──────────────────────────────────────────────
.action-bar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 14px 20px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
}

.btn-emergency {
  font-family: 'Outfit', var(--font-ui), sans-serif;
  font-weight: 600;
  box-shadow: 0 4px 16px rgba(229, 72, 77, 0.3);

  &:hover {
    box-shadow: 0 6px 24px rgba(229, 72, 77, 0.4);
  }
}

.error-text {
  color: var(--color-error);
  font-size: 13px;
  font-family: 'Work Sans', var(--font-ui), sans-serif;
}

// ─── Overview Cards ─────────────────────────────────────────
.overview-cards {
  margin-bottom: 0;
}

.metric-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 20px;
  transition:
    box-shadow 0.2s ease,
    border-color 0.2s ease,
    transform 0.2s ease;

  &:hover {
    box-shadow: 0 4px 20px rgba(59, 130, 246, 0.12);
    border-color: rgba(59, 130, 246, 0.3);
    transform: translateY(-2px);
  }

  &--loading {
    opacity: 0.7;
  }

  &__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  &__label {
    font-family: 'Work Sans', var(--font-ui), sans-serif;
    font-size: 13px;
    font-weight: 500;
    color: var(--color-text-tertiary);
    letter-spacing: 0.2px;
  }

  &__icon {
    color: var(--color-primary);
    opacity: 0.8;
  }

  &__value {
    font-family: 'Outfit', var(--font-ui), sans-serif;
    font-size: 26px;
    font-weight: 700;
    font-feature-settings: 'tnum';
    color: var(--color-text-primary);
    margin-bottom: 4px;
    letter-spacing: -0.5px;
  }

  &__sub {
    font-family: 'Work Sans', var(--font-ui), sans-serif;
    font-size: 12px;
    color: var(--color-text-tertiary);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  &__skeleton {
    padding: 4px 0;
  }
}

.skeleton-line {
  background: linear-gradient(
    90deg,
    var(--color-surface-elevated) 0%,
    rgba(255, 255, 255, 0.08) 50%,
    var(--color-surface-elevated) 100%
  );
  background-size: 200% 100%;
  border-radius: var(--radius-sm);
  animation: skeleton-shimmer 1.5s ease-in-out infinite;
}

@keyframes skeleton-shimmer {
  0% { background-position: 200% 0; opacity: 0.6; }
  50% { background-position: 0% 0; opacity: 1; }
  100% { background-position: -200% 0; opacity: 0.6; }
}

.tag {
  font-family: 'Work Sans', var(--font-ui), sans-serif;
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: var(--radius-sm);
  letter-spacing: 0.3px;
  text-transform: uppercase;

  &--active {
    background: var(--color-primary-light);
    color: var(--color-primary);
    border: 1px solid rgba(59, 130, 246, 0.2);
  }

  &--inactive {
    background: var(--color-surface-elevated);
    color: var(--color-text-tertiary);
    border: 1px solid var(--color-border);
  }
}

// ─── Section Card ─────────────────────────────────────────────
.section-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 20px;
  transition: all 0.2s ease;

  &:hover {
    border-color: var(--color-border-hover);
  }
}

.section-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
}

.section-title {
  font-family: 'Outfit', var(--font-ui), sans-serif;
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-primary);
  letter-spacing: -0.2px;
}

.log-count {
  font-family: 'Work Sans', var(--font-ui), sans-serif;
  font-size: 12px;
  color: var(--color-text-tertiary);
  margin-left: auto;
  font-weight: 500;
}

.skeleton-block {
  padding: 12px 0;
}

// ─── Pagination ───────────────────────────────────────────────
.pagination-bar {
  display: flex;
  justify-content: flex-end;
  margin-top: 12px;
}

// ─── Responsive ───────────────────────────────────────────────
@media (max-width: 767px) {
  .risk-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .action-bar {
    flex-direction: column;
    align-items: flex-start;
  }

  .metric-card__value {
    font-size: 22px;
  }
}
</style>