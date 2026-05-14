<template>
  <div class="kline-export-view">
    <div class="page-header">
      <div class="header-left">
        <el-button class="back-btn" text @click="router.push('/klines')">
          <el-icon><ArrowLeft /></el-icon>
          返回
        </el-button>
        <h1 class="page-title">导出K线数据</h1>
      </div>
    </div>

    <el-card shadow="never" class="export-card">
      <template #header>
        <span class="card-title">导出配置</span>
      </template>

      <el-form :model="form" label-width="120px" class="export-form">
        <el-form-item label="交易对" required>
          <el-select v-model="form.symbol" placeholder="选择交易对" filterable clearable class="form-select">
            <el-option v-for="s in availableSymbols" :key="s" :label="s" :value="s" />
          </el-select>
        </el-form-item>

        <el-form-item label="时间周期" required>
          <el-select v-model="form.interval" placeholder="选择周期" class="form-select">
            <el-option v-for="iv in KLINE_INTERVALS" :key="iv.value" :label="iv.label" :value="iv.value" />
          </el-select>
        </el-form-item>

        <el-form-item label="开始时间">
          <el-date-picker
            v-model="form.startTime"
            type="datetime"
            placeholder="开始时间（可选）"
            value-format="x"
            class="form-date"
          />
        </el-form-item>

        <el-form-item label="结束时间">
          <el-date-picker
            v-model="form.endTime"
            type="datetime"
            placeholder="结束时间（可选）"
            value-format="x"
            class="form-date"
          />
        </el-form-item>

        <el-form-item label="导出格式" required>
          <div class="format-cards">
            <div
              v-for="fmt in formatOptions"
              :key="fmt.value"
              class="format-card"
              :class="{ 'is-selected': form.format === fmt.value }"
              @click="form.format = fmt.value"
            >
              <div class="format-card-icon">
                <el-icon :size="24"><component :is="fmt.icon" /></el-icon>
              </div>
              <div class="format-card-name">{{ fmt.label }}</div>
              <div class="format-card-desc">{{ fmt.desc }}</div>
            </div>
          </div>
        </el-form-item>

        <el-form-item label="选择字段">
          <div class="field-checkboxes">
            <el-checkbox
              v-for="field in allFields"
              :key="field.key"
              v-model="field.checked"
              :label="field.label"
            />
          </div>
        </el-form-item>
      </el-form>

      <div class="export-actions">
        <el-button :loading="exporting" type="primary" :disabled="!canExport" @click="handleExport">
          <el-icon><Download /></el-icon>
          导出 {{ form.format.toUpperCase() }}
        </el-button>
      </div>
    </el-card>

    <!-- Format Preview -->
    <el-card v-if="form.symbol && form.interval" shadow="never" class="preview-card">
      <template #header>
        <span class="card-title">文件预览</span>
      </template>
      <div class="preview-content">
        <div class="preview-filename">
          <el-icon><Document /></el-icon>
          <span>kline_{{ form.symbol }}_{{ form.interval }}_{{ filenameDate }}.{{ form.format }}</span>
        </div>
        <div class="preview-columns">
          <span class="preview-label">包含字段：</span>
          <el-tag v-for="f in selectedFields" :key="f.key" size="small" class="field-tag">
            {{ f.label }}
          </el-tag>
        </div>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { ArrowLeft, Download, Document, Grid } from '@element-plus/icons-vue'
import { exportKlines, getKlineSymbols } from '@/api/kline'
import { KLINE_INTERVALS } from '@/types/kline'

const router = useRouter()

const exporting = ref(false)
const availableSymbols = ref<string[]>([])

const form = reactive({
  symbol: '',
  interval: '',
  startTime: null as number | null,
  endTime: null as number | null,
  format: 'csv' as 'csv' | 'json' | 'excel',
})

const formatOptions = [
  { label: 'CSV', value: 'csv' as const, icon: Document, desc: '逗号分隔值' },
  { label: 'JSON', value: 'json' as const, icon: Grid, desc: '结构化数据' },
  { label: 'Excel', value: 'excel' as const, icon: Grid, desc: '电子表格' },
]

const allFields = reactive([
  { key: 'open_time', label: 'timestamp', checked: true },
  { key: 'open', label: 'open', checked: true },
  { key: 'high', label: 'high', checked: true },
  { key: 'low', label: 'low', checked: true },
  { key: 'close', label: 'close', checked: true },
  { key: 'volume', label: 'volume', checked: true },
])

const selectedFields = computed(() => allFields.filter((f) => f.checked))

const canExport = computed(() => form.symbol && form.interval && selectedFields.value.length > 0)

const filenameDate = computed(() => {
  const now = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}`
})

async function fetchSymbols() {
  try {
    const symbols = await getKlineSymbols()
    availableSymbols.value = symbols.map((s) => s.symbol)
  } catch { /* optional */ }
}

async function handleExport() {
  if (!canExport.value) {
    ElMessage.warning('请填写完整的导出配置')
    return
  }

  exporting.value = true
  try {
    const blob = await exportKlines({
      symbol: form.symbol,
      interval: form.interval,
      start_time: form.startTime ?? undefined,
      end_time: form.endTime ?? undefined,
      format: form.format,
      fields: selectedFields.value.map((f) => f.key),
    })

    // Trigger browser download
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `kline_${form.symbol}_${form.interval}_${filenameDate.value}.${form.format}`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)

    ElMessage.success('导出成功')
  } catch (err: unknown) {
    ElMessage.error(err instanceof Error ? err.message : '导出失败')
  } finally {
    exporting.value = false
  }
}

fetchSymbols()
</script>

<style scoped lang="scss">
.kline-export-view {
  max-width: 700px;
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

.export-card,
.preview-card {
  background: var(--color-surface);
  border-color: var(--color-border);
  margin-bottom: 16px;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.export-form {
  max-width: 500px;
}

.form-select { width: 200px; }
.form-date { width: 220px; }

.format-cards {
  display: flex;
  gap: 12px;
}

.format-card {
  border: 2px solid var(--color-border);
  border-radius: 8px;
  padding: 16px 20px;
  cursor: pointer;
  text-align: center;
  min-width: 100px;
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

.format-card-icon {
  color: var(--color-text-secondary);
  margin-bottom: 8px;
}

.format-card-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin-bottom: 4px;
}

.format-card-desc {
  font-size: 11px;
  color: var(--color-text-tertiary);
}

.field-checkboxes {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.export-actions {
  margin-top: 24px;
}

.preview-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.preview-filename {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-family: monospace;
  color: var(--color-text-secondary);
  background: var(--color-muted);
  padding: 8px 12px;
  border-radius: 4px;
}

.preview-columns {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.preview-label {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.field-tag {
  margin: 0;
}
</style>
