<template>
  <div class="kline-import-view">
    <div class="page-header">
      <div class="header-left">
        <el-button class="back-btn" text @click="router.push('/klines')">
          <el-icon><ArrowLeft /></el-icon>
          返回
        </el-button>
        <h1 class="page-title">导入K线数据</h1>
      </div>
    </div>

    <!-- Step Indicator -->
    <div class="step-indicator">
      <div v-for="(step, idx) in steps" :key="idx" class="step-wrapper">
        <div class="step" :class="{ active: currentStep === idx + 1, completed: currentStep > idx + 1 }">
          <div class="step-circle">
            <el-icon v-if="currentStep > idx + 1"><Check /></el-icon>
            <span v-else>{{ idx + 1 }}</span>
          </div>
          <span class="step-label">{{ step }}</span>
        </div>
        <div v-if="idx < steps.length - 1" class="step-line" :class="{ completed: currentStep > idx + 1 }" />
      </div>
    </div>

    <!-- Step 1: Select Method -->
    <div v-show="currentStep === 1" class="step-content">
      <el-card shadow="never" class="section-card">
        <template #header><span class="section-title">选择导入方式</span></template>
        <div class="method-grid">
          <div
            v-for="method in importMethods"
            :key="method.value"
            class="method-card"
            :class="{ 'is-selected': selectedMethod === method.value }"
            @click="selectedMethod = method.value as 'csv' | 'api' | 'exchange'"
          >
            <div class="method-icon">
              <el-icon :size="32"><component :is="method.icon" /></el-icon>
            </div>
            <div class="method-name">{{ method.label }}</div>
            <div class="method-desc">{{ method.desc }}</div>
          </div>
        </div>
        <div class="step-actions">
          <el-button type="primary" :disabled="!selectedMethod" @click="currentStep = 2">
            下一步 <el-icon><ArrowRight /></el-icon>
          </el-button>
        </div>
      </el-card>
    </div>

    <!-- Step 2: Configure -->
    <div v-show="currentStep === 2" class="step-content">
      <el-card shadow="never" class="section-card">
        <template #header><span class="section-title">配置导入参数</span></template>

        <!-- CSV Upload Form -->
        <template v-if="selectedMethod === 'csv'">
          <el-form :model="csvForm" label-width="120px" class="config-form">
            <el-form-item label="交易对" required>
              <el-select v-model="csvForm.symbol" placeholder="选择交易对" filterable clearable class="form-select">
                <el-option v-for="s in availableSymbols" :key="s" :label="s" :value="s" />
              </el-select>
            </el-form-item>
            <el-form-item label="时间周期" required>
              <el-select v-model="csvForm.interval" placeholder="选择周期">
                <el-option v-for="iv in KLINE_INTERVALS" :key="iv.value" :label="iv.label" :value="iv.value" />
              </el-select>
            </el-form-item>
            <el-form-item label="数据来源" required>
              <el-select v-model="csvForm.source" placeholder="选择数据来源">
                <el-option label="CSV 文件" value="csv" />
                <el-option label="API 接口" value="api" />
                <el-option label="交易所" value="exchange" />
              </el-select>
            </el-form-item>
            <el-form-item label="CSV文件" required>
              <el-upload
                ref="uploadRef"
                class="csv-uploader"
                drag
                :auto-upload="false"
                :limit="1"
                accept=".csv"
                :on-change="handleFileChange"
              >
                <el-icon><Upload /></el-icon>
                <div class="el-upload__text">拖拽文件到此处，或 <em>点击选择文件</em></div>
                <template #tip>
                  <div class="el-upload__tip">支持 10 万条/次，CSV 格式，列：open_time,open,high,low,close,volume</div>
                </template>
              </el-upload>
            </el-form-item>
          </el-form>
        </template>

        <!-- API Import Form -->
        <template v-else-if="selectedMethod === 'api'">
          <el-form :model="apiForm" label-width="120px" class="config-form">
            <el-form-item label="交易对" required>
              <el-select v-model="apiForm.symbol" placeholder="选择交易对" filterable clearable class="form-select">
                <el-option v-for="s in availableSymbols" :key="s" :label="s" :value="s" />
              </el-select>
            </el-form-item>
            <el-form-item label="时间周期" required>
              <el-select v-model="apiForm.interval" placeholder="选择周期">
                <el-option v-for="iv in KLINE_INTERVALS" :key="iv.value" :label="iv.label" :value="iv.value" />
              </el-select>
            </el-form-item>
            <el-form-item label="开始时间">
              <el-date-picker v-model="apiForm.startTime" type="datetime" placeholder="开始时间" value-format="x" />
            </el-form-item>
            <el-form-item label="结束时间">
              <el-date-picker v-model="apiForm.endTime" type="datetime" placeholder="结束时间" value-format="x" />
            </el-form-item>
          </el-form>
        </template>

        <!-- Exchange Fetch Form -->
        <template v-else-if="selectedMethod === 'exchange'">
          <el-form :model="exchangeForm" label-width="120px" class="config-form">
            <el-form-item label="交易所">
              <el-select v-model="exchangeForm.exchange" placeholder="选择交易所">
                <el-option label="Binance" value="binance" />
              </el-select>
            </el-form-item>
            <el-form-item label="交易对" required>
              <el-select v-model="exchangeForm.symbol" placeholder="选择交易对" filterable clearable class="form-select">
                <el-option v-for="s in availableSymbols" :key="s" :label="s" :value="s" />
              </el-select>
            </el-form-item>
            <el-form-item label="时间周期" required>
              <el-select v-model="exchangeForm.interval" placeholder="选择周期">
                <el-option v-for="iv in KLINE_INTERVALS" :key="iv.value" :label="iv.label" :value="iv.value" />
              </el-select>
            </el-form-item>
            <el-form-item label="开始时间">
              <el-date-picker v-model="exchangeForm.startTime" type="datetime" placeholder="开始时间" value-format="x" />
            </el-form-item>
            <el-form-item label="结束时间">
              <el-date-picker v-model="exchangeForm.endTime" type="datetime" placeholder="结束时间" value-format="x" />
            </el-form-item>
          </el-form>
          <el-alert type="warning" :closable="false" show-icon>
            <template #title>专业交易员权限</template>
            交易所直采功能仅对专业交易员开放。如需开通，请联系管理员。
          </el-alert>
        </template>

        <div class="step-actions">
          <el-button @click="currentStep = 1">上一步</el-button>
          <el-button type="primary" :loading="importing" @click="startImport">
            开始导入
          </el-button>
        </div>
      </el-card>
    </div>

    <!-- Step 3: Progress -->
    <div v-show="currentStep === 3" class="step-content">
      <el-card shadow="never" class="section-card">
        <template #header><span class="section-title">导入进度</span></template>

        <div v-if="importing" class="progress-section">
          <el-progress :percentage="progressPercent" :indeterminate="true" :stroke-width="10" />
          <p class="progress-text">{{ progressText }}</p>
        </div>

        <!-- Import Result -->
        <div v-else-if="importResult" class="result-section">
          <el-result
            :icon="importResult.failed === 0 ? 'success' : 'warning'"
            :title="importResult.failed === 0 ? '导入完成' : '导入完成（部分失败）'"
          >
            <template #sub-title>
              <div class="result-stats">
                <div class="stat-card">
                  <el-icon :size="24" class="stat-icon"><Document /></el-icon>
                  <span class="stat-number">{{ importResult.imported + importResult.duplicates + importResult.failed }}</span>
                  <span class="stat-label">总行数</span>
                </div>
                <div class="stat-card stat-success">
                  <el-icon :size="24" class="stat-icon"><CircleCheck /></el-icon>
                  <span class="stat-number">{{ importResult.imported }}</span>
                  <span class="stat-label">成功</span>
                </div>
                <div class="stat-card stat-warning">
                  <el-icon :size="24" class="stat-icon"><Warning /></el-icon>
                  <span class="stat-number">{{ importResult.duplicates }}</span>
                  <span class="stat-label">重复</span>
                </div>
                <div class="stat-card stat-danger">
                  <el-icon :size="24" class="stat-icon"><CircleClose /></el-icon>
                  <span class="stat-number">{{ importResult.failed }}</span>
                  <span class="stat-label">失败</span>
                </div>
              </div>
              <div v-if="importResult.errors?.length" class="error-list">
                <p class="error-title">失败详情：</p>
                <div v-for="(err, i) in importResult.errors.slice(0, 10)" :key="i" class="error-item">
                  {{ err }}
                </div>
                <p v-if="importResult.errors.length > 10" class="error-more">
                  还有 {{ importResult.errors.length - 10 }} 条错误...
                </p>
              </div>
            </template>
            <template #extra>
              <el-button type="primary" @click="router.push('/klines')">返回列表</el-button>
              <el-button @click="resetImport">继续导入</el-button>
            </template>
          </el-result>
        </div>
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { Upload, ArrowLeft, ArrowRight, Check, Document, Connection, ShoppingCart, CircleCheck, Warning, CircleClose } from '@element-plus/icons-vue'
import { uploadKlinesCSV, importKlines, fetchKlinesFromExchange, getKlineSymbols } from '@/api/kline'
import { KLINE_INTERVALS } from '@/types/kline'
import type { KlineImportResult, KlineFetchRequest } from '@/types/kline'
import type { UploadFile } from 'element-plus'

const router = useRouter()

const steps = ['选择方式', '配置参数', '导入进度']
const currentStep = ref(1)
const selectedMethod = ref<'csv' | 'api' | 'exchange' | null>(null)
const importing = ref(false)
const progressPercent = ref(0)
const progressText = ref('正在导入...')
const importResult = ref<KlineImportResult | null>(null)
const uploadRef = ref()
const selectedFile = ref<File | null>(null)
const availableSymbols = ref<string[]>([])

const csvForm = reactive({ symbol: '', interval: '', source: 'csv' as 'csv' | 'api' | 'exchange' })
const apiForm = reactive({ symbol: '', interval: '', startTime: null as number | null, endTime: null as number | null })
const exchangeForm = reactive({ exchange: 'binance', symbol: '', interval: '', startTime: null as number | null, endTime: null as number | null })

const importMethods = [
  { label: 'CSV 上传', value: 'csv', icon: Document, desc: '上传本地 CSV 文件' },
  { label: 'API 导入', value: 'api', icon: Connection, desc: '通过 API 接口批量写入' },
  { label: '交易所直采', value: 'exchange', icon: ShoppingCart, desc: '从 Binance 直接获取数据' },
]

function handleFileChange(file: UploadFile) {
  selectedFile.value = file.raw ?? null
}

async function startImport() {
  importing.value = true
  progressPercent.value = 0
  progressText.value = '正在解析数据...'

  try {
    if (selectedMethod.value === 'csv') {
      if (!csvForm.symbol) { ElMessage.warning('请选择交易对'); importing.value = false; return }
      if (!csvForm.interval) { ElMessage.warning('请选择时间周期'); importing.value = false; return }
      if (!csvForm.source) { ElMessage.warning('请选择数据来源'); importing.value = false; return }
      if (!selectedFile.value) { ElMessage.warning('请选择 CSV 文件'); importing.value = false; return }

      progressText.value = '正在上传文件...'
      const result = await uploadKlinesCSV(csvForm.symbol, csvForm.interval, selectedFile.value)
      importResult.value = result
    } else if (selectedMethod.value === 'api') {
      if (!apiForm.symbol) { ElMessage.warning('请选择交易对'); importing.value = false; return }
      if (!apiForm.interval) { ElMessage.warning('请选择时间周期'); importing.value = false; return }

      progressText.value = '正在调用 API...'
      const result = await importKlines({
        symbol: apiForm.symbol,
        interval: apiForm.interval,
        source: 'api',
        data: [],
      })
      importResult.value = result
    } else if (selectedMethod.value === 'exchange') {
      if (!exchangeForm.symbol) { ElMessage.warning('请选择交易对'); importing.value = false; return }
      if (!exchangeForm.interval) { ElMessage.warning('请选择时间周期'); importing.value = false; return }

      progressText.value = '正在从交易所获取数据...'
      const result = await fetchKlinesFromExchange({
        symbol: exchangeForm.symbol,
        interval: exchangeForm.interval,
        exchange: exchangeForm.exchange as KlineFetchRequest['exchange'],
        start_time: exchangeForm.startTime ?? undefined,
        end_time: exchangeForm.endTime ?? undefined,
      })
      importResult.value = result
    }

    progressText.value = '导入完成'
    currentStep.value = 3
  } catch (err: unknown) {
    ElMessage.error(err instanceof Error ? err.message : '导入失败')
    currentStep.value = 2
  } finally {
    importing.value = false
  }
}

function resetImport() {
  currentStep.value = 1
  selectedMethod.value = null
  importResult.value = null
  selectedFile.value = null
  csvForm.symbol = ''
  csvForm.interval = ''
  csvForm.source = 'csv'
  apiForm.symbol = ''
  apiForm.interval = ''
  exchangeForm.symbol = ''
  exchangeForm.interval = ''
}

async function fetchSymbols() {
  try {
    const symbols = await getKlineSymbols()
    availableSymbols.value = symbols.map((s) => s.symbol)
  } catch { /* optional */ }
}

onMounted(() => {
  fetchSymbols()
})
</script>

<style scoped lang="scss">
.kline-import-view {
  max-width: 900px;
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

.back-btn {
  color: var(--color-text-secondary);
}

.page-title {
  font-size: 24px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.step-indicator {
  display: flex;
  align-items: center;
  margin-bottom: 32px;
}

.step-wrapper {
  display: flex;
  align-items: center;
}

.step {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.step-circle {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid var(--color-border);
  color: var(--color-text-tertiary);
  font-weight: 600;
  font-size: 14px;
  background: var(--color-surface);
  transition: all 0.2s;
}

.step.active .step-circle {
  border-color: var(--el-color-primary);
  background: var(--el-color-primary);
  color: #fff;
}

.step.completed .step-circle {
  border-color: var(--el-color-success);
  background: var(--el-color-success);
  color: #fff;
}

.step-label {
  font-size: 13px;
  color: var(--color-text-tertiary);
}

.step.active .step-label,
.step.completed .step-label {
  color: var(--color-text-primary);
}

.step-line {
  flex: 1;
  height: 2px;
  background: var(--color-border);
  margin: 0 16px;
  min-width: 60px;
  transition: background 0.2s;
}

.step-line.completed {
  background: var(--el-color-success);
}

.section-card {
  background: var(--color-surface);
  border-color: var(--color-border);
  margin-bottom: 16px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.method-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  margin-bottom: 24px;
}

.method-card {
  border: 2px solid var(--color-border);
  border-radius: 8px;
  padding: 20px;
  cursor: pointer;
  text-align: center;
  transition: border-color 0.2s, background 0.2s;

  &:hover {
    border-color: var(--el-color-primary-light-5);
    background: var(--color-surface-hover);
  }

  &.is-selected {
    border-color: var(--el-color-primary);
    background: rgba(59, 130, 246, 0.05);
  }
}

.method-icon {
  color: var(--color-text-secondary);
  margin-bottom: 12px;
}

.method-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin-bottom: 4px;
}

.method-desc {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.config-form {
  max-width: 500px;
  margin-bottom: 24px;
}

.form-select { width: 200px; }

.csv-uploader {
  width: 100%;

  :deep(.el-upload-dragger) {
    width: 320px;
    height: 240px;
    border-radius: 12px;
    border: 2px dashed var(--color-border);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    transition: border-color 0.2s;

    &:hover {
      border-color: var(--el-color-primary);
    }
  }

  :deep(.el-upload) {
    width: auto;
  }
}

.step-actions {
  display: flex;
  gap: 12px;
  margin-top: 24px;
}

.progress-section {
  padding: 40px 20px;
  text-align: center;
}

.progress-text {
  margin-top: 16px;
  color: var(--color-text-secondary);
}

.result-section {
  padding: 20px 0;
}

.result-stats {
  display: flex;
  gap: 16px;
  justify-content: center;
  margin: 16px 0;
}

.stat-card {
  width: 160px;
  height: 80px;
  border-radius: 8px;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;

  .stat-icon {
    color: var(--color-text-secondary);
  }

  .stat-number {
    font-size: 20px;
    font-weight: 700;
    color: var(--color-text-primary);
    line-height: 1.2;
  }

  .stat-label {
    font-size: 12px;
    color: var(--color-text-tertiary);
  }

  &.stat-success {
    .stat-icon { color: var(--el-color-success); }
    .stat-number { color: var(--el-color-success); }
  }

  &.stat-warning {
    .stat-icon { color: var(--el-color-warning); }
    .stat-number { color: var(--el-color-warning); }
  }

  &.stat-danger {
    .stat-icon { color: var(--el-color-danger); }
    .stat-number { color: var(--el-color-danger); }
  }
}

.error-list {
  margin-top: 16px;
  text-align: left;
  max-width: 400px;
  margin-left: auto;
  margin-right: auto;
}

.error-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-color-danger);
  margin-bottom: 8px;
}

.error-item {
  font-size: 12px;
  color: var(--color-text-secondary);
  padding: 2px 0;
  font-family: monospace;
}

.error-more {
  font-size: 12px;
  color: var(--color-text-tertiary);
  margin-top: 4px;
}
</style>
