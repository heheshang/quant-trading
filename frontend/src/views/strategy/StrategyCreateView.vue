<template>
  <div class="strategy-create-view">
    <!-- Page Header -->
    <div class="page-header">
      <div class="header-left">
        <el-button class="back-btn" text @click="goBack">
          <el-icon><ArrowLeft /></el-icon>
          返回
        </el-button>
        <h1 class="page-title">{{ isEdit ? '编辑策略' : '新建策略' }}</h1>
      </div>
    </div>

    <!-- Loading State -->
    <template v-if="pageLoading">
      <el-skeleton :rows="8" animated class="skeleton-form" />
    </template>

    <!-- Error State -->
    <template v-else-if="pageError">
      <el-card class="state-card" shadow="never">
        <el-empty description="加载失败" :image-size="80">
          <template #image>
            <el-icon :size="48" color="var(--color-error)"><WarningFilled /></el-icon>
          </template>
          <el-button type="primary" @click="loadData">重新加载</el-button>
        </el-empty>
      </el-card>
    </template>

    <!-- Content -->
    <template v-else>
      <!-- Step 1: Basic Info -->
      <el-card class="section-card" shadow="never">
        <template #header>
          <span class="section-title">基本信息</span>
        </template>
        <el-form
          ref="basicFormRef"
          :model="formData"
          :rules="basicRules"
          label-position="top"
          class="basic-form"
        >
          <el-form-item label="策略名称" prop="name">
            <el-input
              v-model="formData.name"
              placeholder="请输入策略名称"
              maxlength="50"
              show-word-limit
              class="strategy-name-input"
            />
          </el-form-item>
        </el-form>
      </el-card>

      <!-- Step 2: Template Selection -->
      <el-card class="section-card" shadow="never">
        <template #header>
          <span class="section-title">选择策略模板</span>
        </template>

        <div class="template-grid">
          <div
            v-for="tpl in templates"
            :key="tpl.id"
            class="template-card"
            :class="{ 'is-selected': selectedTemplate?.id === tpl.id }"
            @click="selectTemplate(tpl)"
          >
            <div class="template-icon">
              <el-icon :size="28"><component :is="getTemplateIcon(tpl.id)" /></el-icon>
            </div>
            <div class="template-name">{{ tpl.name }}</div>
            <div class="template-desc">{{ tpl.description }}</div>
            <div class="template-category">
              <el-tag size="small" effect="plain">{{ tpl.category }}</el-tag>
            </div>
          </div>
        </div>
      </el-card>

      <!-- Step 3: Parameter Configuration -->
      <el-card v-if="selectedTemplate" class="section-card" shadow="never">
        <template #header>
          <div class="section-header">
            <span class="section-title">参数配置</span>
          </div>
        </template>

        <el-form
          ref="paramsFormRef"
          :model="paramValues"
          label-position="top"
          class="param-form"
        >
          <el-form-item
            v-for="param in selectedTemplate.parameter_schema"
            :key="param.name"
            :label="param.label"
            :prop="param.name"
            :rules="getParamRule(param)"
          >
            <!-- Number: integer or float → slider + input-number -->
            <template v-if="param.type === 'integer' || param.type === 'float'">
              <div class="number-param">
                <el-slider
                  v-model="paramValues[param.name]"
                  :min="param.min ?? 0"
                  :max="param.max ?? 100"
                  :step="param.type === 'float' ? 0.1 : 1"
                  :show-stops="false"
                  class="param-slider"
                />
                <el-input-number
                  v-model="paramValues[param.name]"
                  :min="param.min ?? 0"
                  :max="param.max ?? 100"
                  :precision="param.type === 'float' ? 2 : 0"
                  :step="param.type === 'float' ? 0.1 : 1"
                  size="small"
                  class="param-input-number"
                />
              </div>
              <div v-if="param.description" class="param-desc">{{ param.description }}</div>
            </template>

            <!-- Select -->
            <template v-else-if="param.type === 'select'">
              <el-select
                v-model="paramValues[param.name]"
                :placeholder="'请选择'"
                class="param-select"
              >
                <el-option
                  v-for="opt in param.options"
                  :key="opt"
                  :label="opt"
                  :value="opt"
                />
              </el-select>
              <div v-if="param.description" class="param-desc">{{ param.description }}</div>
            </template>

            <!-- Boolean -->
            <template v-else-if="param.type === 'boolean'">
              <el-switch v-model="paramValues[param.name]" />
              <div v-if="param.description" class="param-desc">{{ param.description }}</div>
            </template>

            <!-- String -->
            <template v-else>
              <el-input
                v-model="paramValues[param.name]"
                :placeholder="'请输入'"
              />
              <div v-if="param.description" class="param-desc">{{ param.description }}</div>
            </template>
          </el-form-item>
        </el-form>

        <!-- Parameter Preview -->
        <el-card class="param-preview" shadow="never">
          <template #header>
            <span class="preview-title">参数预览</span>
          </template>
          <div class="preview-content">
            <div class="preview-item" v-if="formData.name">
              <span class="preview-label">策略名称</span>
              <span class="preview-value">{{ formData.name }}</span>
            </div>
            <div class="preview-item">
              <span class="preview-label">模板类型</span>
              <span class="preview-value">{{ selectedTemplate.name }}</span>
            </div>
            <div class="preview-item" v-for="param in selectedTemplate.parameter_schema" :key="param.name">
              <span class="preview-label">{{ param.label }}</span>
              <span class="preview-value">{{ formatParamValue(param, paramValues[param.name]) }}</span>
            </div>
          </div>
        </el-card>
      </el-card>

      <!-- Actions -->
      <div class="form-actions">
        <el-button class="cancel-btn" @click="goBack">取消</el-button>
        <el-button
          class="save-btn"
          type="primary"
          :loading="saving"
          @click="handleSave"
        >
          {{ isEdit ? '保存修改' : '创建策略' }}
        </el-button>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { ArrowLeft, WarningFilled, TrendCharts, Grid, DataAnalysis, Connection, Coin } from '@element-plus/icons-vue'
import { listTemplates, createStrategy, updateStrategy, getStrategy } from '@/api/strategies'
import type { StrategyTemplate, StrategyParamDef, StrategyFull } from '@/types'
import type { FormInstance, FormRules } from 'element-plus'

const props = defineProps<{
  strategyId?: string
}>()

const router = useRouter()

const isEdit = computed(() => !!props.strategyId)

// State
const templates = ref<StrategyTemplate[]>([])
const selectedTemplate = ref<StrategyTemplate | null>(null)
const existingStrategy = ref<StrategyFull | null>(null)
const pageLoading = ref(false)
const pageError = ref(false)
const saving = ref(false)

const basicFormRef = ref<FormInstance>()
const paramsFormRef = ref<FormInstance>()

const formData = reactive({
  name: '',
})

const paramValues = reactive<Record<string, any>>({})

// Form validation rules
const basicRules: FormRules = {
  name: [
    { required: true, message: '请输入策略名称', trigger: 'blur' },
    { max: 50, message: '策略名称不能超过50个字符', trigger: 'blur' },
  ],
}

function getParamRule(param: StrategyParamDef) {
  const rules: any[] = []
  rules.push({ required: true, message: `请配置${param.label}`, trigger: 'change' })
  if ((param.type === 'integer' || param.type === 'float') && param.min !== undefined) {
    rules.push({ type: 'number', min: param.min, message: `最小值为${param.min}`, trigger: 'blur' })
  }
  if ((param.type === 'integer' || param.type === 'float') && param.max !== undefined) {
    rules.push({ type: 'number', max: param.max, message: `最大值为${param.max}`, trigger: 'blur' })
  }
  return rules
}

function getTemplateIcon(iconName: string): any {
  const icons: Record<string, any> = {
    TrendCharts,
    Grid,
    DataAnalysis,
    Connection,
    Coin,
  }
  return icons[iconName] || TrendCharts
}

function selectTemplate(tpl: StrategyTemplate) {
  selectedTemplate.value = tpl
  // Reset param values with defaults
  Object.keys(paramValues).forEach((k) => delete paramValues[k])
  tpl.parameter_schema.forEach((p) => {
    paramValues[p.name] = p.default ?? ((p.type === 'integer' || p.type === 'float') ? p.min ?? 0 : p.type === 'boolean' ? false : '')
  })
}

function formatParamValue(param: StrategyParamDef, value: any): string {
  if (value === undefined || value === null) return '—'
  if (param.type === 'integer' || param.type === 'float') return String(value)
  if (param.type === 'boolean') return value ? '是' : '否'
  if (param.type === 'select') {
    return String(value)
  }
  return String(value)
}

async function loadData() {
  pageLoading.value = true
  pageError.value = false

  try {
    // Load templates
    templates.value = await listTemplates()

    // If editing, load existing strategy data
    if (props.strategyId) {
      const strategy = await getStrategy(props.strategyId)
      existingStrategy.value = strategy
      formData.name = strategy.name

      // Find and preselect template
      const tpl = templates.value.find((t) => t.id === strategy.template_type)
      if (tpl) {
        selectTemplate(tpl)
        // Override defaults with existing parameters
        if (strategy.parameters) {
          Object.entries(strategy.parameters).forEach(([key, value]) => {
            paramValues[key] = value
          })
        }
      }
    }
  } catch {
    pageError.value = true
  } finally {
    pageLoading.value = false
  }
}

async function handleSave() {
  // Validate basic form
  const basicValid = await basicFormRef.value?.validate().catch(() => false)
  if (!basicValid) return

  // Validate params if template selected
  if (selectedTemplate.value) {
    const paramsValid = await paramsFormRef.value?.validate().catch(() => false)
    if (!paramsValid) return
  }

  saving.value = true

  try {
    if (isEdit.value && props.strategyId) {
      await updateStrategy(props.strategyId, {
        name: formData.name,
        parameters: { ...paramValues },
      })
      ElMessage.success('策略已更新')
    } else {
      await createStrategy({
        name: formData.name,
        template_type: selectedTemplate.value?.id ?? '',
        parameters: { ...paramValues },
      })
      ElMessage.success('策略创建成功')
    }
    router.push({ name: 'Strategies' })
  } catch {
    ElMessage.error(isEdit.value ? '保存失败，请重试' : '创建失败，请重试')
  } finally {
    saving.value = false
  }
}

function goBack() {
  router.push({ name: 'Strategies' })
}

onMounted(() => {
  loadData()
})
</script>

<style scoped lang="scss">
.strategy-create-view {
  max-width: var(--content-max-width);

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;

    .header-left {
      display: flex;
      align-items: center;
      gap: 12px;
    }

    .page-title {
      font-size: 24px;
      font-weight: 600;
      color: var(--color-text-primary);
      margin: 0;
    }
  }

  .skeleton-form {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 20px;
  }

  .state-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
  }

  .section-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    margin-bottom: 16px;

    :deep(.el-card__header) {
      padding: 16px 20px;
      border-bottom: 1px solid var(--color-border);
    }

    :deep(.el-card__body) {
      padding: 20px;
    }
  }

  .section-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--color-text-primary);
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  // Template Grid
  .template-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
  }

  .template-card {
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    padding: 20px;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;

    &:hover {
      border-color: var(--color-accent);
      background: rgba(113, 112, 255, 0.03);
    }

    &.is-selected {
      border-color: var(--color-accent);
      background: rgba(113, 112, 255, 0.08);
      box-shadow: 0 0 0 1px var(--color-accent);
    }

    .template-icon {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 48px;
      height: 48px;
      background: rgba(113, 112, 255, 0.1);
      border-radius: 12px;
      color: var(--color-accent);
    }

    .template-name {
      font-size: 15px;
      font-weight: 600;
      color: var(--color-text-primary);
    }

    .template-desc {
      font-size: 12px;
      color: var(--color-text-tertiary);
      line-height: 1.4;
    }

    .template-category {
      margin-top: 4px;
    }
  }

  // Parameter Form
  .param-form {
    max-width: 600px;
  }

  .number-param {
    display: flex;
    align-items: center;
    gap: 16px;

    .param-slider {
      flex: 1;
    }

    .param-input-number {
      width: 120px;
    }
  }

  .param-select {
    width: 100%;
    max-width: 400px;
  }

  .param-desc {
    font-size: 12px;
    color: var(--color-text-tertiary);
    margin-top: 4px;
  }

  // Parameter Preview
  .param-preview {
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    margin-top: 16px;

    :deep(.el-card__header) {
      padding: 12px 16px;
      border-bottom: 1px solid var(--color-border);
    }

    :deep(.el-card__body) {
      padding: 12px 16px;
    }
  }

  .preview-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-text-secondary);
  }

  .preview-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .preview-item {
    display: flex;
    align-items: center;
    gap: 12px;

    .preview-label {
      font-size: 12px;
      color: var(--color-text-tertiary);
      min-width: 80px;
    }

    .preview-value {
      font-size: 13px;
      color: var(--color-text-primary);
      font-family: var(--font-mono);
    }
  }

  // Actions
  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 24px 0;
  }
}
</style>
