<template>
  <div class="kline-quality-view">
    <div class="page-header">
      <div class="header-left">
        <el-button class="back-btn" text @click="router.push('/klines')">
          <el-icon><ArrowLeft /></el-icon>
          返回
        </el-button>
        <h1 class="page-title">数据质量报告</h1>
      </div>
    </div>

    <!-- Selector -->
    <el-card shadow="never" class="selector-card">
      <div class="selector-bar">
        <el-select v-model="form.symbol" placeholder="选择交易对" filterable clearable class="selector-select">
          <el-option v-for="s in availableSymbols" :key="s" :label="s" :value="s" />
        </el-select>
        <el-select v-model="form.interval" placeholder="选择周期" clearable class="selector-select">
          <el-option v-for="iv in KLINE_INTERVALS" :key="iv.value" :label="iv.label" :value="iv.value" />
        </el-select>
        <el-button type="primary" :loading="loading" @click="runQualityCheck">
          <el-icon><DataAnalysis /></el-icon> 生成报告
        </el-button>
      </div>
    </el-card>

    <!-- Loading -->
    <div v-if="loading" class="loading-state">
      <el-skeleton :rows="6" animated />
    </div>

    <!-- Report Content -->
    <template v-else-if="report">
      <!-- Summary Cards — 6 metrics -->
      <div class="metrics-grid">
        <div class="metric-card">
          <div class="metric-label">总行数</div>
          <div class="metric-value">{{ formatNumber(report.total_rows) }}</div>
          <div class="metric-sub">数据总条数</div>
        </div>
        <div class="metric-card">
          <div class="metric-label">有效行</div>
          <div class="metric-value success">{{ formatNumber(report.valid_rows) }}</div>
          <div class="metric-sub">通过校验</div>
        </div>
        <div class="metric-card coverage-card">
          <div class="metric-label">数据覆盖率</div>
          <div class="coverage-ring-wrapper">
            <svg class="coverage-ring" width="80" height="80" viewBox="0 0 80 80">
              <circle class="ring-track" cx="40" cy="40" r="34" fill="none" stroke-width="6" />
              <circle
                class="ring-progress"
                cx="40" cy="40" r="34" fill="none" stroke-width="6"
                :stroke-dasharray="ringDashArray"
                :stroke-dashoffset="ringDashOffset"
                stroke-linecap="round"
                :class="coverageClass"
              />
            </svg>
            <span class="ring-text" :class="coverageClass">{{ report.coverage_rate }}%</span>
          </div>
          <div class="metric-sub">{{ report.valid_rows }} / {{ report.total_rows }} 条</div>
        </div>
        <div class="metric-card">
          <div class="metric-label">缺口数量</div>
          <div class="metric-value warning">{{ report.gap_count }}</div>
          <div class="metric-sub">处连续缺失</div>
        </div>
        <div class="metric-card">
          <div class="metric-label">异常数量</div>
          <div class="metric-value" :class="report.anomaly_count > 0 ? 'danger' : 'success'">
            {{ report.anomaly_count }}
          </div>
          <div class="metric-sub">可疑/损坏数据</div>
        </div>
        <div class="metric-card">
          <div class="metric-label">重复数量</div>
          <div class="metric-value warning">{{ report.duplicate_count }}</div>
          <div class="metric-sub">完全重复记录</div>
        </div>
      </div>

      <!-- Anomaly Details -->
      <el-card v-if="report.anomaly_rows?.length" shadow="never" class="detail-card">
        <template #header>
          <div class="card-header">
            <span class="card-title">异常数据详情</span>
            <div class="card-header-actions">
              <el-button size="small" type="warning" @click="showCleanDialog">
                全部清洗
              </el-button>
              <el-button size="small" @click="handleExportAnomalies">
                导出
              </el-button>
            </div>
          </div>
        </template>
        <el-table :data="report.anomaly_rows" stripe border :max-height="300">
          <el-table-column label="时间" prop="open_time" width="180">
            <template #default="{ row }">
              {{ formatTimestamp(row.open_time) }}
            </template>
          </el-table-column>
          <el-table-column label="类型" width="120">
            <template #default="{ row }">
              <el-tag :type="anomalyTagType(row.type)" size="small">{{ row.type }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="字段" prop="field" width="100" />
          <el-table-column label="异常值" prop="value" width="140" align="right">
            <template #default="{ row }">
              {{ row.value !== undefined ? formatNumber(row.value) : '-' }}
            </template>
          </el-table-column>
          <el-table-column label="说明" prop="expected_range" />
          <el-table-column label="操作" width="100" align="center">
            <template #default="{ row }">
              <el-button size="small" text type="primary" @click="handleFix(row)">修正</el-button>
              <el-button size="small" text type="danger" @click="handleDelete(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-card>

      <!-- Gap Positions -->
      <el-card v-if="report.gap_positions?.length" shadow="never" class="detail-card">
        <template #header>
          <span class="card-title">缺口位置 ({{ report.gap_count }} 处)</span>
        </template>
        <div class="gap-list">
          <div v-for="(pos, i) in report.gap_positions.slice(0, 20)" :key="i" class="gap-item">
            {{ formatTimestamp(pos) }}
          </div>
          <div v-if="report.gap_positions.length > 20" class="gap-more">
            还有 {{ report.gap_positions.length - 20 }} 处...
          </div>
        </div>
      </el-card>

      <!-- Cleaning Actions -->
      <el-card shadow="never" class="detail-card">
        <template #header>
          <span class="card-title">数据清洗</span>
        </template>
        <div class="clean-actions">
          <el-button type="primary" :loading="cleaning" @click="showCleanDialog">
            <el-icon><Brush /></el-icon> 自动清洗
          </el-button>
          <el-button :loading="cleaning" @click="router.push('/klines')">
            返回列表
          </el-button>
        </div>
        <p class="clean-desc">
          自动清洗将：填充缺口（线性插值）、去重（保留第一条）、标记异常数据（不自动删除）
        </p>
      </el-card>
    </template>

    <!-- Empty State -->
    <div v-else class="empty-state">
      <el-card shadow="never" class="empty-card">
        <el-empty description="请先选择交易对和周期，生成质量报告" :image-size="80">
          <template #image>
            <el-icon :size="48" color="var(--color-text-tertiary)"><DataAnalysis /></el-icon>
          </template>
        </el-empty>
      </el-card>
    </div>

    <!-- Fix Dialog -->
    <el-dialog v-model="fixDialogVisible" title="修正数据" width="400px">
      <el-form :model="fixForm" label-width="100px">
        <el-form-item label="时间">
          <span>{{ fixForm.open_time ? formatTimestamp(fixForm.open_time) : '-' }}</span>
        </el-form-item>
        <el-form-item label="类型">
          <el-tag :type="anomalyTagType(fixForm.type ?? '')">{{ fixForm.type }}</el-tag>
        </el-form-item>
        <template v-if="fixForm.type === 'corrupted'">
          <el-form-item label="最高价 (high)">
            <el-input-number v-model="fixForm.high" :precision="4" />
          </el-form-item>
          <el-form-item label="最低价 (low)">
            <el-input-number v-model="fixForm.low" :precision="4" />
          </el-form-item>
        </template>
      </el-form>
      <template #footer>
        <el-button @click="fixDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="fixing" @click="submitFix">确认修正</el-button>
      </template>
    </el-dialog>

    <!-- Clean Confirm Dialog (KlineCleanDialog) -->
    <el-dialog
      v-model="cleanDialogVisible"
      :title="`确认清洗 ${report?.anomaly_count ?? 0} 条数据？`"
      width="440px"
      :close-on-click-modal="false"
    >
      <div class="clean-dialog-content">
        <p>以下异常数据将被清洗：</p>
        <ul class="clean-type-list">
          <li v-if="report?.suspicious_count">
            <el-tag type="warning" size="small">可疑</el-tag>
            {{ report.suspicious_count }} 条
          </li>
          <li v-if="report?.corrupted_count">
            <el-tag type="danger" size="small">损坏</el-tag>
            {{ report.corrupted_count }} 条
          </li>
          <li v-if="otherAnomalyCount > 0">
            <el-tag type="info" size="small">其他</el-tag>
            {{ otherAnomalyCount }} 条
          </li>
        </ul>
        <p class="clean-warning">清洗操作将自动处理异常数据，操作不可撤销。</p>
      </div>
      <template #footer>
        <el-button @click="cleanDialogVisible = false">取消</el-button>
        <el-button type="warning" :loading="cleaning" @click="confirmCleanAll">确认清洗</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { ArrowLeft, DataAnalysis, Brush } from '@element-plus/icons-vue'
import { getQualityReport, cleanKlines, getKlineSymbols } from '@/api/kline'
import { KLINE_INTERVALS } from '@/types/kline'
import type { KlineQualityReport, KlineAnomalyRow } from '@/types/kline'

const router = useRouter()

const loading = ref(false)
const cleaning = ref(false)
const fixing = ref(false)
const report = ref<KlineQualityReport | null>(null)
const availableSymbols = ref<string[]>([])

const form = reactive({ symbol: '', interval: '' })

const fixDialogVisible = ref(false)
const fixForm = reactive({
  open_time: 0,
  type: '',
  field: '',
  high: 0,
  low: 0,
})

const cleanDialogVisible = ref(false)

// P1-07: Coverage threshold — >95% green / >=90% warning / <90% danger
const coverageClass = computed(() => {
  if (!report.value) return ''
  const r = report.value.coverage_rate
  if (r > 95) return 'success'
  if (r >= 90) return 'warning'
  return 'danger'
})

// P1-08: Ring chart math
const CIRCUMFERENCE = 2 * Math.PI * 34 // r=34

const ringDashArray = computed(() => CIRCUMFERENCE)

const ringDashOffset = computed(() => {
  if (!report.value) return CIRCUMFERENCE
  const rate = Math.max(0, Math.min(100, report.value.coverage_rate))
  return CIRCUMFERENCE * (1 - rate / 100)
})

// Other anomaly count (excluding suspicious + corrupted)
const otherAnomalyCount = computed(() => {
  if (!report.value) return 0
  const total = report.value.anomaly_count ?? 0
  const suspicious = report.value.suspicious_count ?? 0
  const corrupted = report.value.corrupted_count ?? 0
  return Math.max(0, total - suspicious - corrupted)
})

function anomalyTagType(type: string) {
  switch (type) {
    case 'suspicious': return 'warning'
    case 'corrupted': return 'danger'
    case 'zero_volume': return 'info'
    default: return 'info'
  }
}

function formatTimestamp(ts: number): string {
  return new Date(ts).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' })
}

function formatNumber(n: number): string {
  return n.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 4 })
}

async function fetchSymbols() {
  try {
    const symbols = await getKlineSymbols()
    availableSymbols.value = symbols.map((s) => s.symbol)
  } catch { /* optional */ }
}

async function runQualityCheck() {
  if (!form.symbol || !form.interval) {
    ElMessage.warning('请选择交易对和周期')
    return
  }
  loading.value = true
  try {
    report.value = await getQualityReport(form.symbol, form.interval)
  } catch (err: unknown) {
    ElMessage.error(err instanceof Error ? err.message : '生成报告失败')
  } finally {
    loading.value = false
  }
}

// P0-05: Show clean dialog instead of direct execution
function showCleanDialog() {
  cleanDialogVisible.value = true
}

async function confirmCleanAll() {
  if (!form.symbol || !form.interval) return
  cleaning.value = true
  try {
    const result = await cleanKlines({ symbol: form.symbol, interval: form.interval, mode: 'auto' })
    const total = result.filled_gaps + result.deduplicated + result.deleted + result.fixed
    ElMessage.success(`清洗完成，已处理 ${total} 条`)
    cleanDialogVisible.value = false
    await runQualityCheck()
  } catch (err: unknown) {
    ElMessage.error(err instanceof Error ? err.message : '清洗失败')
  } finally {
    cleaning.value = false
  }
}

function handleFix(row: KlineAnomalyRow) {
  fixForm.open_time = row.open_time
  fixForm.type = row.type
  fixForm.field = row.field ?? ''
  fixForm.high = 0
  fixForm.low = 0
  fixDialogVisible.value = true
}

async function submitFix() {
  fixing.value = true
  try {
    await cleanKlines({
      symbol: form.symbol,
      interval: form.interval,
      mode: 'manual',
      actions: [{
        open_time: fixForm.open_time,
        action: 'fix',
        field: fixForm.field,
        value: fixForm.high,
      }],
    })
    ElMessage.success('修正成功')
    fixDialogVisible.value = false
    await runQualityCheck()
  } catch (err: unknown) {
    ElMessage.error(err instanceof Error ? err.message : '修正失败')
  } finally {
    fixing.value = false
  }
}

function handleDelete(row: KlineAnomalyRow) {
  cleanKlines({
    symbol: form.symbol,
    interval: form.interval,
    mode: 'manual',
    actions: [{ open_time: row.open_time, action: 'delete' }],
  })
    .then(() => {
      ElMessage.success('删除成功')
      return runQualityCheck()
    })
    .catch((err: unknown) => {
      ElMessage.error(err instanceof Error ? err.message : '删除失败')
    })
}

// P0-05: Export anomalies as CSV
function handleExportAnomalies() {
  if (!report.value?.anomaly_rows?.length) return
  const headers = '时间,类型,字段,异常值,说明'
  const rows = report.value.anomaly_rows.map(r =>
    `${formatTimestamp(r.open_time)},${r.type},${r.field ?? ''},${r.value ?? ''},${r.expected_range ?? ''}`
  )
  const csv = [headers, ...rows].join('\n')
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `anomaly_${form.symbol}_${form.interval}.csv`
  a.click()
  URL.revokeObjectURL(url)
}

fetchSymbols()

// Expose internals for testing
defineExpose({
  form,
  report,
  loading,
  cleanDialogVisible,
  fixDialogVisible,
  runQualityCheck,
  showCleanDialog,
  confirmCleanAll,
  handleExportAnomalies,
  handleFix,
  submitFix,
  ringDashOffset,
  coverageClass,
  otherAnomalyCount,
})
</script>

<style scoped lang="scss">
.kline-quality-view {
  max-width: 1100px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 24px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.back-btn { color: var(--color-text-secondary); }

.page-title {
  font-size: 24px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.selector-card {
  background: var(--color-surface);
  border-color: var(--color-border);
  margin-bottom: 16px;
}

.selector-bar {
  display: flex;
  gap: 12px;
  align-items: center;
}

.selector-select { width: 160px; }

// P0-06: 6-column metrics grid
.metrics-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 16px;
  margin-bottom: 16px;
}

.metric-card {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 20px;
  text-align: center;

  .metric-label {
    font-size: 13px;
    color: var(--color-text-tertiary);
    margin-bottom: 8px;
  }

  .metric-value {
    font-size: 32px;
    font-weight: 700;
    color: var(--color-text-primary);
    &.success { color: var(--el-color-success); }
    &.warning { color: var(--el-color-warning); }
    &.danger { color: var(--el-color-danger); }
  }

  .metric-sub {
    font-size: 12px;
    color: var(--color-text-tertiary);
    margin-top: 4px;
  }
}

// P1-08: Coverage ring chart
.coverage-card {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.coverage-ring-wrapper {
  position: relative;
  width: 80px;
  height: 80px;
  margin: 4px 0;
}

.coverage-ring {
  transform: rotate(-90deg);
}

.ring-track {
  stroke: var(--color-surface-elevated, #212223);
}

.ring-progress {
  transition: stroke-dashoffset 0.6s ease;
  &.success { stroke: var(--el-color-success); }
  &.warning { stroke: var(--el-color-warning); }
  &.danger { stroke: var(--el-color-danger); }
}

.ring-text {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  font-size: 16px;
  font-weight: 700;
  font-family: 'JetBrains Mono', 'SF Mono', Menlo, monospace;
  &.success { color: var(--el-color-success); }
  &.warning { color: var(--el-color-warning); }
  &.danger { color: var(--el-color-danger); }
}

.detail-card {
  margin-bottom: 16px;
  background: var(--color-surface);
  border-color: var(--color-border);
}

// P0-05: Card header with actions
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.card-header-actions {
  display: flex;
  gap: 8px;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.gap-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.gap-item {
  font-size: 12px;
  color: var(--color-text-secondary);
  background: var(--color-muted);
  padding: 2px 8px;
  border-radius: 4px;
  font-family: monospace;
}

.gap-more {
  font-size: 12px;
  color: var(--color-text-tertiary);
  padding: 2px 8px;
}

.clean-actions {
  display: flex;
  gap: 12px;
  margin-bottom: 12px;
}

.clean-desc {
  font-size: 12px;
  color: var(--color-text-tertiary);
  margin: 0;
}

// P0-05: Clean dialog styles
.clean-dialog-content {
  p {
    margin: 0 0 8px;
    color: var(--color-text-secondary);
  }
}

.clean-type-list {
  list-style: none;
  padding: 0;
  margin: 0 0 12px;

  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    color: var(--color-text-primary);
  }
}

.clean-warning {
  font-size: 12px;
  color: var(--el-color-warning) !important;
}

.empty-state {
  .empty-card {
    background: var(--color-surface);
    border-color: var(--color-border);
  }
}

.loading-state {
  padding: 20px 0;
}
</style>
