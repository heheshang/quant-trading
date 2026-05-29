<template>
  <div class="backtest-view">
    <div class="page-header">
      <h1 class="page-title">回测</h1>
    </div>

    <div class="backtest-layout">
      <!-- Left panel: Config form (fixed 340px) -->
      <div class="config-panel">
        <BacktestConfigForm
          :loading="isRunning"
          @run="handleRun"
        />
      </div>

      <!-- Right panel: Results area (flex:1) -->
      <div class="results-panel">
        <!-- Progress Panel - shown when running -->
        <div v-if="backtestState === 'running'" class="progress-panel">
          <el-card shadow="never" class="progress-card">
            <div class="progress-content">
              <!-- Strategy summary -->
              <div v-if="lastParams" class="strategy-summary">
                {{ lastParams.strategy_id }} · {{ lastParams.symbol }} · {{ lastParams.start_date }} ~ {{ lastParams.end_date }}
              </div>
              <!-- Progress bar -->
              <div class="progress-bar-wrapper">
                <el-progress
                  :percentage="combinedProgress"
                  :stroke-width="8"
                  :show-text="true"
                  :format="(v: number) => v + '%'"
                  class="progress-bar"
                />
              </div>
              <div class="running-status-text">运行中...</div>
              <!-- Cancel button -->
              <el-button
                class="cancel-btn"
                size="small"
                @click="handleCancel"
              >
                取消
              </el-button>
            </div>
          </el-card>
        </div>

        <!-- Error Section - shown when failed -->
        <div v-if="backtestState === 'failed'" class="error-section">
          <el-alert
            :title="error || '回测执行失败'"
            type="error"
            show-icon
            closable
            @close="handleReturnToIdle"
          />
          <el-button
            class="return-params-btn"
            @click="handleReturnToIdle"
            style="margin-top: 12px"
          >
            返回修改参数
          </el-button>
        </div>

        <!-- Empty State - shown when idle -->
        <div v-if="backtestState === 'idle'" class="empty-state">
          <el-card shadow="never" class="empty-card">
            <el-empty description="配置参数后点击运行回测" :image-size="80">
              <template #image>
                <el-icon :size="48" color="var(--color-text-tertiary)">
                  <Odometer />
                </el-icon>
              </template>
            </el-empty>
          </el-card>
        </div>

        <!-- History List - shown when idle -->
        <div v-if="backtestState === 'idle' && historyItems.length > 0" class="history-section">
          <BacktestHistoryList
            :items="historyItems"
            :loading="historyLoading"
            :current-id="result?.id ?? ''"
            @select="handleHistorySelect"
            @delete="handleHistoryDelete"
            @refresh="loadHistory"
          />
        </div>

        <!-- Results Section - shown when completed -->
        <div v-if="backtestState === 'completed' && result" class="results-section">
          <!-- Top action bar -->
          <div class="result-action-bar">
            <div class="action-bar-left">
              <span class="result-title">回测结果</span>
              <span v-if="result" class="result-subtitle">
                {{ result.strategy_id }} · {{ result.config.symbol }}
              </span>
            </div>
            <div class="action-bar-right">
              <el-button size="small" @click="handleReturnToIdle">
                ✕ 返回
              </el-button>
              <el-button size="small" class="rerun-btn" @click="handleRerun">
                ↻ 重新回测
              </el-button>
              <el-button size="small" @click="handleDelete">
                🗑 删除
              </el-button>
            </div>
          </div>

          <!-- Metrics cards -->
          <div class="result-metrics">
            <BacktestMetricsCards :result="result" />
          </div>

          <!-- Tab navigation -->
          <el-tabs v-model="activeTab" class="result-tabs">
            <el-tab-pane label="权益曲线" :name="0">
              <BacktestEquityChart
                :data="result.equity_curve"
                :initial-capital="result.config.initial_capital"
              />
            </el-tab-pane>
            <el-tab-pane label="K线图" :name="1">
              <BacktestKlineChart :backtest-result="result" />
            </el-tab-pane>
            <el-tab-pane label="交易明细" :name="2">
              <BacktestTradesTable :trades="result.trades" />
            </el-tab-pane>
            <el-tab-pane label="绩效报告" :name="3">
              <BacktestPerformanceReport :result="result" />
            </el-tab-pane>
          </el-tabs>
        </div>
      </div><!-- /results-panel -->
    </div><!-- /backtest-layout -->
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { ElMessageBox } from 'element-plus'
import { Odometer } from '@element-plus/icons-vue'
import BacktestConfigForm from '@/components/backtest/BacktestConfigForm.vue'
import BacktestMetricsCards from '@/components/backtest/BacktestMetricsCards.vue'
import BacktestEquityChart from '@/components/backtest/BacktestEquityChart.vue'
import BacktestKlineChart from '@/components/backtest/BacktestKlineChart.vue'
import BacktestTradesTable from '@/components/backtest/BacktestTradesTable.vue'
import BacktestHistoryList from '@/components/backtest/BacktestHistoryList.vue'
import BacktestPerformanceReport from '@/components/backtest/BacktestPerformanceReport.vue'
import * as backtestApi from '@/api/backtest'
import type { BacktestResultResponse, BacktestParams, BacktestSummary } from '@/types/backtest'

const result = ref<BacktestResultResponse | null>(null)
const error = ref('')
const isRunning = ref(false)
const backtestState = ref<'idle' | 'running' | 'completed' | 'failed'>('idle')
const activeTab = ref(0)
const pollProgress = ref(0)
const currentJobId = ref<string | null>(null)
const lastParams = ref<{
  strategy_id?: string
  symbol?: string
  start_date?: string
  end_date?: string
} | null>(null)

// History state
const historyItems = ref<BacktestSummary[]>([])
const historyLoading = ref(false)

// WS real-time progress (complements polling fallback)
const wsProgress = ref(0)
const combinedProgress = computed(() => Math.max(wsProgress.value, pollProgress.value))

function onBacktestProgress(event: Event) {
  const { backtestId, progress, status } = (event as CustomEvent).detail as {
    backtestId: string
    progress: number
    status: string
  }
  // Only update if this is the current job
  if (backtestId !== currentJobId.value) return
  wsProgress.value = progress
  if (status === 'completed' || status === 'failed') {
    // Fetch final result via REST (polling fallback for final state)
    if (pollTimer) clearInterval(pollTimer)
    pollJob(currentJobId.value).then(finalResult => {
      if (finalResult) {
        if (finalResult.status === 'completed') {
          result.value = finalResult
          backtestState.value = 'completed'
        } else {
          error.value = finalResult.error || '回测执行失败'
          backtestState.value = 'failed'
        }
      }
    })
  }
}

// WS progress is received via window CustomEvent dispatched by trading.ts handleWsMessage

const MAX_POLL_ATTEMPTS = 60
const POLL_INTERVAL = 2000
let pollTimer: ReturnType<typeof setInterval> | null = null
let pollCount = 0

onMounted(() => {
  loadHistory()
  window.addEventListener('backtest-progress', onBacktestProgress)
})

onUnmounted(() => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
  window.removeEventListener('backtest-progress', onBacktestProgress)
})

async function loadHistory() {
  try {
    historyLoading.value = true
    const res = await backtestApi.listBacktestHistory({ page: 1, size: 50 })
    historyItems.value = res.items
  } catch {
    historyItems.value = []
  } finally {
    historyLoading.value = false
  }
}

onMounted(() => {
  loadHistory()
  window.addEventListener('backtest-progress', onBacktestProgress)
})

async function handleHistorySelect(item: BacktestSummary) {
  try {
    const data = await backtestApi.getBacktestResult(item.id)
    if (data.status === 'completed') {
      result.value = data
      backtestState.value = 'completed'
      activeTab.value = 0
    } else if (data.status === 'failed') {
      error.value = data.error || '回测执行失败'
      backtestState.value = 'failed'
      activeTab.value = 0
    }
  } catch (err: any) {
    error.value = err?.message || '加载回测结果失败'
    backtestState.value = 'failed'
    activeTab.value = 0
  }
}

async function handleHistoryDelete(id: string) {
  try {
    await ElMessageBox.confirm(
      '确定要删除该回测记录吗？删除后不可恢复。',
      '确认删除',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'warning',
      }
    )
    await backtestApi.deleteBacktestResult(id)
    historyItems.value = historyItems.value.filter((h) => h.id !== id)
    // If currently viewing this result, return to idle
    if (result.value?.id === id) {
      cleanupState()
    }
  } catch {
    // User cancelled or delete failed
  }
}

async function handleRun(params: BacktestParams) {
  // Reset all state
  result.value = null
  error.value = ''
  isRunning.value = true
  backtestState.value = 'running'
  pollCount = 0
  pollProgress.value = 0
  activeTab.value = 0

  // Store params for progress panel display
  lastParams.value = {
    strategy_id: params.strategy_id,
    symbol: params.symbol,
    start_date: params.start_date,
    end_date: params.end_date,
  }

  try {
    // Step 1: Submit backtest job — returns BacktestResultResponse with status
    const initialResult = await backtestApi.runBacktest(params)
    currentJobId.value = initialResult.id

    // Step 2: Check if already completed/failed
    if (initialResult.status === 'completed') {
      result.value = initialResult
      backtestState.value = 'completed'
      return
    }

    if (initialResult.status === 'failed') {
      throw new Error(initialResult.error || '回测执行失败')
    }

    // Step 3: Poll for completion using GET /backtest/{id}
    const finalResult = await pollJob(initialResult.id)

    if (!finalResult) {
      throw new Error('回测超时，请稍后重试')
    }

    if (finalResult.status === 'failed') {
      throw new Error(finalResult.error || '回测执行失败')
    }

    // Step 4: Use the result
    result.value = finalResult
    backtestState.value = 'completed'
  } catch (err: any) {
    error.value = err?.message || '回测执行失败'
    backtestState.value = 'failed'
  } finally {
    isRunning.value = false
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }
}

async function handleCancel() {
  if (currentJobId.value) {
    try {
      await backtestApi.cancelBacktest(currentJobId.value)
    } catch {
      // Silently ignore cancel errors
    }
  }
  cleanupState()
}

function handleReturnToIdle() {
  cleanupState()
}

function handleRerun() {
  cleanupState()
  // User will click run from the config form again
}

async function handleDelete() {
  if (!result.value) return
  try {
    await ElMessageBox.confirm(
      '确定要删除该回测结果吗？删除后不可恢复。',
      '确认删除',
      {
        confirmButtonText: '删除',
        cancelButtonText: '取消',
        type: 'warning',
      }
    )
    await backtestApi.deleteBacktestResult(result.value.id)
    cleanupState()
  } catch {
    // User cancelled or delete failed — silently ignore
  }
}

function cleanupState() {
  cleanupPolling()
  wsProgress.value = 0
  result.value = null
  error.value = ''
  isRunning.value = false
  backtestState.value = 'idle'
  currentJobId.value = null
  lastParams.value = null
  activeTab.value = 0
}

function cleanupPolling() {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
  pollCount = 0
  pollProgress.value = 0
}

function pollJob(id: string): Promise<BacktestResultResponse | null> {
  return new Promise((resolve, reject) => {
    pollCount = 0
    pollTimer = setInterval(async () => {
      pollCount++
      pollProgress.value = Math.min((pollCount / MAX_POLL_ATTEMPTS) * 100, 95)

      try {
        const data = await backtestApi.getBacktestResult(id)

        if (data.status === 'completed') {
          clearInterval(pollTimer!)
          pollTimer = null
          pollProgress.value = 100
          resolve(data)
        } else if (data.status === 'failed') {
          clearInterval(pollTimer!)
          pollTimer = null
          resolve(data)
        } else if (pollCount >= MAX_POLL_ATTEMPTS) {
          clearInterval(pollTimer!)
          pollTimer = null
          resolve(null)
        }
        // else: still running, continue polling
      } catch (err) {
        clearInterval(pollTimer!)
        pollTimer = null
        reject(err)
      }
    }, POLL_INTERVAL)
  })
}

onMounted(() => {
  loadHistory()
})

// Expose internal state and methods for testing
defineExpose({
  result,
  error,
  isRunning,
  backtestState,
  activeTab,
  pollProgress,
  currentJobId,
  lastParams,
  historyItems,
  historyLoading,
  handleRun,
  handleCancel,
  handleReturnToIdle,
  handleRerun,
  handleDelete,
  handleHistorySelect,
  handleHistoryDelete,
  cleanupState,
  pollJob,
  loadHistory,
})
</script>

<style scoped lang="scss">
.backtest-view {
  max-width: var(--content-max-width);
  overflow: hidden;
}

.page-header {
  margin-bottom: 24px;

  .page-title {
    font-size: 24px;
    font-weight: 600;
    font-family: var(--font-ui);
    color: var(--color-text-primary);
    margin: 0;
    letter-spacing: -0.3px;
  }
}

// Left-right split layout
.backtest-layout {
  display: flex;
  gap: 20px;
  align-items: flex-start;
}

.config-panel {
  width: 340px;
  flex-shrink: 0;
}

.results-panel {
  flex: 1;
  min-width: 0;
}

@media (max-width: 900px) {
  .backtest-layout {
    flex-direction: column;
  }
  .config-panel {
    width: 100%;
  }
}

// Progress Panel
.progress-panel {
  margin-bottom: 20px;
  max-width: 480px;
  margin-left: auto;
  margin-right: auto;

  .progress-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 28px 24px;
    box-shadow: var(--shadow-card);
    transition: all var(--transition-base);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;

    &:hover {
      box-shadow: var(--shadow-card-hover);
      border-color: var(--color-border-hover);
    }
  }

  .strategy-summary {
    font-size: 13px;
    color: var(--color-text-tertiary);
    text-align: center;
  }

  .progress-bar-wrapper {
    width: 100%;
    max-width: 320px;
  }

  .running-status-text {
    font-size: 14px;
    font-weight: 600;
    color: var(--color-text-primary);
  }

  .cancel-btn {
    min-width: 100px;
    border-radius: var(--radius-sm);
  }
}

// Error Section
.error-section {
  margin-bottom: 20px;

  .return-params-btn {
    display: block;
    margin-top: 12px;
    border-radius: var(--radius-sm);
  }
}

// Empty State
.empty-state {
  margin-bottom: 20px;

  .empty-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 48px 24px;
    box-shadow: var(--shadow-card);
    transition: all var(--transition-base);

    &:hover {
      box-shadow: var(--shadow-card-hover);
      border-color: var(--color-border-hover);
    }
  }
}

// History Section
.history-section {
  margin-top: 20px;
}

// Results Section
.results-section {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.result-action-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 48px;
  padding: 0 16px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card);
  transition: all var(--transition-base);

  &:hover {
    box-shadow: var(--shadow-card-hover);
    border-color: var(--color-border-hover);
  }

  .action-bar-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .result-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--color-text-primary);
    font-family: var(--font-ui);
  }

  .result-subtitle {
    font-size: 13px;
    color: var(--color-text-tertiary);
  }

  .action-bar-right {
    display: flex;
    align-items: center;
    gap: 8px;

    .el-button {
      border-radius: var(--radius-sm);
      font-size: 13px;
      font-weight: 500;
    }

    .rerun-btn {
      background: var(--color-primary);
      border-color: var(--color-primary);
      color: #fff;
      box-shadow: var(--shadow-glow-primary);

      &:hover {
        background: var(--color-primary-hover);
        border-color: var(--color-primary-hover);
      }
    }
  }
}

.result-metrics {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
}

.result-tabs {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 16px 20px;
  box-shadow: var(--shadow-card);
  transition: all var(--transition-base);

  &:hover {
    box-shadow: var(--shadow-card-hover);
    border-color: var(--color-border-hover);
  }

  :deep(.el-tabs__header) {
    margin-bottom: 16px;
  }

  :deep(.el-tabs__item) {
    font-size: 14px;
    font-weight: 500;
    color: var(--color-text-tertiary);
    padding: 0 16px;

    &.is-active {
      color: var(--color-primary);
    }
  }

  :deep(.el-tabs__active-bar) {
    background: var(--color-primary);
  }

  :deep(.el-tabs__nav-wrap::after) {
    background: var(--color-border);
  }
}
</style>
