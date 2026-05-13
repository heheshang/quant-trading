<template>
  <div class="kline-quality-view">
    <div class="page-header">
      <div class="header-left">
        <el-button class="back-btn" text @click="router.push('/kline')">
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
      <!-- Summary Cards -->
      <div class="metrics-grid">
        <div class="metric-card">
          <div class="metric-label">数据覆盖率</div>
          <div class="metric-value" :class="coverageClass">{{ report.coverage_rate }}%</div>
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
          <span class="card-title">异常数据详情</span>
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
          <el-button type="primary" :loading="cleaning" @click="runAutoClean">
            <el-icon><Brush /></el-icon> 自动清洗
          </el-button>
          <el-button :loading="cleaning" @click="router.push('/kline')">
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

const coverageClass = computed(() => {
  if (!report.value) return ''
  const r = report.value.coverage_rate
  if (r >= 95) return 'success'
  if (r >= 80) return 'warning'
  return 'danger'
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

async function runAutoClean() {
  if (!form.symbol || !form.interval) return
  cleaning.value = true
  try {
    await cleanKlines({ symbol: form.symbol, interval: form.interval, mode: 'auto' })
    ElMessage.success('清洗完成')
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

fetchSymbols()
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

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
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

.detail-card {
  margin-bottom: 16px;
  background: var(--color-surface);
  border-color: var(--color-border);
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
