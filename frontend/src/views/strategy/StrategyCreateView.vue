<template>
  <div class="strategy-create-view">
    <!-- Page Header -->
    <div class="page-header">
      <div class="header-left">
        <el-button class="back-btn" text @click="goBack">
          <el-icon><ArrowLeft /></el-icon>
          返回
        </el-button>
        <h1 class="page-title">{{ isEdit ? '编辑策略' : '创建策略' }}</h1>
      </div>
    </div>

    <!-- Step Indicator (ADR D2) -->
    <div class="step-indicator">
      <div class="step" :class="{ active: currentStep >= 1, completed: currentStep > 1 }">
        <div class="step-circle">
          <el-icon v-if="currentStep > 1"><Check /></el-icon>
          <span v-else>1</span>
        </div>
        <span class="step-label">选择模板</span>
      </div>
      <div class="step-line" :class="{ completed: currentStep > 1 }" />
      <div class="step" :class="{ active: currentStep >= 2, completed: currentStep > 2 }">
        <div class="step-circle">
          <el-icon v-if="currentStep > 2"><Check /></el-icon>
          <span v-else>2</span>
        </div>
        <span class="step-label">配置参数</span>
      </div>
      <div class="step-line" :class="{ completed: currentStep > 2 }" />
      <div class="step" :class="{ active: currentStep >= 3 }">
        <div class="step-circle">
          <span>3</span>
        </div>
        <span class="step-label">确认完成</span>
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
      <!-- Step 1: Template Selection -->
      <div v-show="currentStep === 1" class="step-content">
        <el-card class="section-card" shadow="never">
          <template #header>
            <span class="section-title">选择策略模板</span>
          </template>
          <p class="section-desc">选择一个预设模板开始配置，或从空白创建</p>

          <div class="template-grid">
            <div
              v-for="tpl in templates"
              :key="tpl.id"
              class="template-card"
              :class="{ 'is-selected': selectedTemplate?.id === tpl.id }"
              @click="selectTemplate(tpl)"
            >
              <div class="template-icon">
                <el-icon :size="28"><component :is="getTemplateIcon(tpl.category)" /></el-icon>
              </div>
              <div class="template-name">{{ tpl.name }}</div>
              <div class="template-desc">{{ tpl.description }}</div>
              <div class="template-category">
                <el-tag size="small" effect="plain" :type="getCategoryTagType(tpl.category)">
                  {{ getCategoryLabel(tpl.category) }}
                </el-tag>
              </div>
            </div>
          </div>
        </el-card>

        <div class="step-actions">
          <el-button @click="goBack">取消</el-button>
          <el-button type="primary" :disabled="!selectedTemplate" @click="nextStep">
            下一步
          </el-button>
        </div>
      </div>

      <!-- Step 2: Parameter Configuration -->
      <div v-show="currentStep === 2" class="step-content">
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
            <div class="form-row">
              <el-form-item label="策略名称" prop="name" class="form-item-half">
                <el-input
                  v-model="formData.name"
                  placeholder="请输入策略名称"
                  maxlength="50"
                  show-word-limit
                  class="strategy-name-input"
                />
              </el-form-item>
            </div>

            <div class="form-row">
              <el-form-item label="策略描述" prop="description" class="form-item-full">
                <el-input
                  v-model="formData.description"
                  type="textarea"
                  placeholder="请输入策略描述（可选，最多500字符）"
                  :rows="3"
                  maxlength="500"
                  show-word-limit
                  class="strategy-desc-input"
                />
              </el-form-item>
            </div>

            <!-- T4.5: Strategy code file upload -->
            <div class="form-row">
              <el-form-item label="策略代码文件" class="form-item-full">
                <div v-if="strategyCodeFile" class="file-info">
                  <div class="file-item">
                    <el-icon class="file-icon"><Document /></el-icon>
                    <div class="file-details">
                      <span class="file-name">{{ strategyCodeFile.name }}</span>
                      <span class="file-size">{{ formatFileSize(strategyCodeFile.size) }}</span>
                    </div>
                    <el-icon class="file-remove" @click="removeStrategyFile"><Close /></el-icon>
                  </div>
                  <div v-if="strategyCodePath" class="file-path">已保存至: {{ strategyCodePath }}</div>
                </div>
                <el-upload
                  v-else
                  class="strategy-code-uploader"
                  :disabled="uploading"
                  :show-file-list="false"
                  accept=".py,.js"
                  :http-request="(options: any) => handleFileChange(options)"
                >
                  <el-button type="default" :loading="uploading">
                    <el-icon v-if="!uploading"><Upload /></el-icon>
                    上传策略文件
                  </el-button>
                  <template #tip>
                    <div class="upload-tip">支持 .py / .js 文件，最大 2MB</div>
                  </template>
                </el-upload>
              </el-form-item>
            </div>

            <!-- ADR D1: Symbol and Timeframe — required fields -->
            <div class="form-row">
              <el-form-item
                label="交易对"
                prop="symbol"
                class="form-item-half"
                :required="!isEdit"
              >
                <el-select
                  v-model="formData.symbol"
                  placeholder="选择交易对"
                  filterable
                  :disabled="!!isEdit"
                  class="symbol-select"
                >
                  <el-option
                    v-for="s in symbolOptions"
                    :key="s"
                    :label="formatSymbolOpt(s)"
                    :value="s"
                  />
                </el-select>
                <span v-if="isEdit" class="field-hint">创建后不可更改</span>
              </el-form-item>

              <el-form-item
                label="时间周期"
                prop="timeframe"
                class="form-item-third"
                :required="!isEdit"
              >
                <el-select
                  v-model="formData.timeframe"
                  placeholder="选择周期"
                  :disabled="!!isEdit"
                  class="timeframe-select"
                >
                  <el-option
                    v-for="tf in timeframeOptions"
                    :key="tf"
                    :label="tf"
                    :value="tf"
                  />
                </el-select>
                <span v-if="isEdit" class="field-hint">创建后不可更改</span>
              </el-form-item>

              <!-- T4.5: Strategy type (required) -->
              <el-form-item
                label="策略类型"
                prop="strategy_type"
                class="form-item-third"
                :required="!isEdit"
              >
                <el-select
                  v-model="formData.strategy_type"
                  placeholder="选择类型"
                  :disabled="!!isEdit"
                  class="strategy-type-select"
                >
                  <el-option
                    v-for="st in strategyTypeOptions"
                    :key="st.value"
                    :label="st.label"
                    :value="st.value"
                  />
                </el-select>
                <span v-if="isEdit" class="field-hint">创建后不可更改</span>
              </el-form-item>
            </div>
          </el-form>
        </el-card>

        <!-- ADR 6.3: Risk Parameter Group -->
        <el-card class="section-card" shadow="never">
          <template #header>
            <span class="section-title">风控参数</span>
          </template>
          <el-form label-position="top" class="risk-form">
            <div class="form-row">
              <el-form-item label="最大持仓" class="form-item-third">
                <el-input-number
                  v-model="riskConfig.max_position"
                  :min="1"
                  :max="10"
                  :step="1"
                  size="default"
                  class="risk-input-number"
                />
                <div class="param-desc">允许同时持仓的最大仓位数量</div>
              </el-form-item>

              <el-form-item label="止损比例" class="form-item-third">
                <div class="percent-input">
                  <el-input-number
                    v-model="riskConfig.stop_loss"
                    :min="0"
                    :max="1"
                    :precision="2"
                    :step="0.01"
                    size="default"
                    class="risk-input-number"
                  />
                  <span class="percent-suffix">%</span>
                </div>
                <div class="param-desc">亏损达到此比例时触发止损</div>
              </el-form-item>

              <el-form-item label="止盈比例" class="form-item-third">
                <div class="percent-input">
                  <el-input-number
                    v-model="riskConfig.stop_profit"
                    :min="0"
                    :max="1"
                    :precision="2"
                    :step="0.01"
                    size="default"
                    class="risk-input-number"
                  />
                  <span class="percent-suffix">%</span>
                </div>
                <div class="param-desc">盈利达到此比例时触发止盈</div>
              </el-form-item>
            </div>
          </el-form>
        </el-card>

        <!-- Parameter Configuration -->
        <el-card v-if="selectedTemplate" class="section-card" shadow="never">
          <template #header>
            <span class="section-title">参数配置 - {{ selectedTemplate.name }}</span>
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
        </el-card>

        <!-- Parameter Preview (sticky right panel) -->
        <div class="preview-panel">
          <el-card class="param-preview" shadow="never">
            <template #header>
              <span class="preview-title">策略摘要</span>
            </template>
            <div class="preview-content">
              <div class="preview-item" v-if="formData.name">
                <span class="preview-label">策略名称</span>
                <span class="preview-value">{{ formData.name }}</span>
              </div>
              <div class="preview-item" v-if="formData.description">
                <span class="preview-label">策略描述</span>
                <span class="preview-value">{{ formData.description }}</span>
              </div>
              <div class="preview-item" v-if="formData.symbol || selectedTemplate">
                <span class="preview-label">交易周期</span>
                <span class="preview-value">{{ formatSymbolOpt(formData.symbol) }} · {{ formData.timeframe }}</span>
              </div>
              <div class="preview-item" v-if="selectedTemplate">
                <span class="preview-label">模板类型</span>
                <span class="preview-value">{{ selectedTemplate.name }}</span>
              </div>
              <template v-if="selectedTemplate">
                <div class="preview-divider" />
                <div class="preview-item" v-for="param in selectedTemplate.parameter_schema" :key="param.name">
                  <span class="preview-label">{{ param.label }}</span>
                  <span class="preview-value">{{ formatParamValue(param, paramValues[param.name]) }}</span>
                </div>
              </template>
            </div>
          </el-card>
        </div>

        <div class="step-actions">
          <el-button @click="prevStep">
            <el-icon><ArrowLeft /></el-icon>
            返回选择模板
          </el-button>
          <div class="right-actions">
            <el-button :loading="saving" @click="handleSaveDraft">保存草稿</el-button>
            <el-button
              type="primary"
              :loading="saving"
              @click="handleSaveAndActivate"
              :disabled="!canActivate"
            >
              {{ isEdit ? '保存' : '保存并启动' }}
            </el-button>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { ArrowLeft, WarningFilled, Check, TrendCharts, Grid, DataAnalysis, Connection, Coin, Timer, Upload, Document, Close } from '@element-plus/icons-vue'
import { listTemplates, createStrategy, updateStrategy, getStrategy, uploadStrategyCode } from '@/api/strategies'
import type { StrategyTemplate, StrategyParamDef, StrategyFull } from '@/types'
import type { FormInstance, FormRules } from 'element-plus'

const props = defineProps<{
  strategyId?: string
}>()

const router = useRouter()

const isEdit = computed(() => !!props.strategyId)

const currentStep = ref(1)

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
  description: '',        // T4.5: 策略描述，最多500字符
  strategy_type: '',
  symbol: '',
  timeframe: '',
})

const paramValues = reactive<Record<string, any>>({})

const riskConfig = reactive({
  max_position: 1,
  stop_loss: 0.05,
  stop_profit: 0.10,
})

// T4.5: Strategy code file upload state
const strategyCodeFile = ref<File | null>(null)
const strategyCodePath = ref<string | null>(null)
const uploading = ref(false)

const symbolOptions = ['BTCUSDT', 'ETHUSDT', 'BNBUSDT', 'SOLUSDT', 'XRPUSDT', 'ADAUSDT', 'DOGEUSDT', 'MATICUSDT']
const timeframeOptions = ['1m', '5m', '15m', '30m', '1H', '4H', '1D', '1W']

/** T4.5: Strategy type options for the select field (ADR D1) */
const strategyTypeOptions: { value: string; label: string }[] = [
  { value: 'trend_following', label: '趋势跟踪' },
  { value: 'mean_reversion', label: '均值回归' },
  { value: 'grid_trading', label: '网格交易' },
  { value: 'arbitrage', label: '套利' },
  { value: 'custom', label: '自定义' },
]

/** Derive StrategyType from template category string */
function categoryToStrategyType(category: string): string {
  const map: Record<string, string> = {
    趋势跟踪: 'trend_following',
    趋势: 'trend_following',
    均值回归: 'mean_reversion',
    网格交易: 'grid_trading',
    套利: 'arbitrage',
    自定义: 'custom',
    trend_following: 'trend_following',
    mean_reversion: 'mean_reversion',
    grid_trading: 'grid_trading',
    arbitrage: 'arbitrage',
    custom: 'custom',
  }
  return map[category] || 'custom'
}

const basicRules: FormRules = {
  name: [
    { required: true, message: '请输入策略名称', trigger: 'blur' },
    { max: 50, message: '策略名称不能超过50个字符', trigger: 'blur' },
  ],
  symbol: [
    { required: true, message: '请选择交易对', trigger: 'change' },
  ],
  timeframe: [
    { required: true, message: '请选择时间周期', trigger: 'change' },
  ],
  strategy_type: [
    { required: true, message: '请选择策略类型', trigger: 'change' },
  ],
}

const canActivate = computed(() => {
  return formData.name && formData.symbol && formData.timeframe && selectedTemplate.value
})

function formatSymbolOpt(symbol: string): string {
  if (!symbol) return ''
  if (symbol.endsWith('USDT')) {
    return symbol.slice(0, -4) + '/USDT'
  }
  return symbol
}

function getCategoryTagType(category: string): string {
  const map: Record<string, string> = {
    趋势跟踪: 'primary',
    均值回归: '',
    网格交易: 'success',
    套利: 'warning',
    自定义: 'info',
    趋势: 'primary',
    震荡: 'success',
  }
  return map[category] || 'info'
}

function getCategoryLabel(category: string): string {
  const map: Record<string, string> = {
    ma_crossover: 'MA交叉',
    triple_ma: '三均线',
    macd: 'MACD',
    bollinger: '布林带',
    rsi: 'RSI',
    keltner: '肯特纳通道',
    atr_stop: 'ATR止损',
    mean_reversion: '均值回归',
    trend_following: '趋势跟踪',
    grid_trading: '网格交易',
    arbitrage: '套利',
    custom: '自定义',
    趋势跟踪: '趋势跟踪',
    趋势: '趋势跟踪',
    均值回归: '均值回归',
    网格交易: '网格交易',
    震荡: '震荡',
    套利: '套利',
    自定义: '自定义',
  }
  return map[category] || category
}

function getTemplateIcon(category: string): any {
  const icons: Record<string, any> = {
    趋势跟踪: TrendCharts,
    趋势: TrendCharts,
    均值回归: DataAnalysis,
    网格交易: Grid,
    套利: Connection,
    自定义: Coin,
  }
  return icons[category] || TrendCharts
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

function selectTemplate(tpl: StrategyTemplate) {
  selectedTemplate.value = tpl
  Object.keys(paramValues).forEach((k) => delete paramValues[k])
  tpl.parameter_schema.forEach((p) => {
    paramValues[p.name] = p.default ?? ((p.type === 'integer' || p.type === 'float') ? p.min ?? 0 : p.type === 'boolean' ? false : '')
  })
  // T4.5: auto-derive strategy_type from template category
  formData.strategy_type = tpl.strategy_type || categoryToStrategyType(tpl.category)
}

function formatParamValue(param: StrategyParamDef, value: any): string {
  if (value === undefined || value === null) return '—'
  if (param.type === 'integer' || param.type === 'float') return String(value)
  if (param.type === 'boolean') return value ? '是' : '否'
  return String(value)
}

function nextStep() {
  if (!selectedTemplate.value) return
  currentStep.value = 2
}

function prevStep() {
  currentStep.value = 1
}

async function loadData() {
  pageLoading.value = true
  pageError.value = false

  try {
    templates.value = await listTemplates()

    if (props.strategyId) {
      const strategy = await getStrategy(props.strategyId)
      existingStrategy.value = strategy
      formData.name = strategy.name
      formData.description = strategy.description || ''
      formData.symbol = strategy.symbol || ''
      formData.timeframe = strategy.timeframe || ''
      formData.strategy_type = strategy.strategy_type || ''

      const tpl = templates.value.find((t) => t.id === (strategy.template_id || strategy.template_type))
      if (tpl) {
        selectTemplate(tpl)
        if (strategy.parameters) {
          Object.entries(strategy.parameters).forEach(([key, value]) => {
            paramValues[key] = value
          })
        }
      }
      if (strategy.risk_config) {
        riskConfig.max_position = strategy.risk_config.max_position ?? 1
        riskConfig.stop_loss = strategy.risk_config.stop_loss ?? 0.05
        riskConfig.stop_profit = strategy.risk_config.stop_profit ?? 0.10
      }
      // T4.5: Load existing strategy code file info
      if (strategy.strategy_code) {
        strategyCodePath.value = strategy.strategy_code
      }
      currentStep.value = 2
    }
  } catch {
    pageError.value = true
  } finally {
    pageLoading.value = false
  }
}

async function handleSaveDraft() {
  const basicValid = await basicFormRef.value?.validate().catch(() => false)
  if (!basicValid) {
    currentStep.value = 2
    return
  }
  await doSave('draft')
}

async function handleSaveAndActivate() {
  const basicValid = await basicFormRef.value?.validate().catch(() => false)
  if (!basicValid) {
    currentStep.value = 2
    return
  }
  const paramsValid = selectedTemplate.value
    ? await paramsFormRef.value?.validate().catch(() => false)
    : true
  if (!paramsValid) return

  await doSave(isEdit.value ? 'paused' : 'active')
}

async function doSave(targetStatus: 'active' | 'paused' | 'draft') {
  saving.value = true

  try {
    // T4.5: Build risk_config first (avoid temporal dead zone)
    const riskConfigPayload = {
      max_position: riskConfig.max_position,
      stop_loss: riskConfig.stop_loss,
      stop_profit: riskConfig.stop_profit,
    }

    const payload: Record<string, any> = {
      name: formData.name,
      symbol: formData.symbol,
      timeframe: formData.timeframe,
      strategy_type: formData.strategy_type as any,
      description: formData.description || undefined,
      parameters: { ...paramValues },
      risk_config: riskConfigPayload,
    }

    if (selectedTemplate.value) {
      payload.template_id = selectedTemplate.value.id
      payload.template_type = selectedTemplate.value.id
    }

    if (isEdit.value && props.strategyId) {
      await updateStrategy(props.strategyId, {
        name: formData.name,
        parameters: { ...paramValues },
        risk_config: riskConfigPayload,
        ...(strategyCodePath.value ? { strategy_code: strategyCodePath.value } : {}),
      })
      ElMessage.success('策略已保存')
    } else {
      await createStrategy(payload as any)
      ElMessage.success(targetStatus === 'active' ? '策略创建并启动成功' : '策略已保存为草稿')
    }
    router.push({ name: 'Strategies' })
  } catch (e: any) {
    ElMessage.error(e?.message || (isEdit.value ? '保存失败，请重试' : '创建失败，请重试'))
  } finally {
    saving.value = false
  }
}

function goBack() {
  router.push({ name: 'Strategies' })
}

// T4.5: Handle strategy code file selection and upload
async function handleFileChange(uploadFile: any) {
  const file = uploadFile.raw as File

  // Validate file extension
  const allowedExts = ['.py', '.js']
  const ext = file.name.substring(file.name.lastIndexOf('.')).toLowerCase()
  if (!allowedExts.includes(ext)) {
    ElMessage.error('仅支持 .py 或 .js 文件')
    return false
  }

  // Validate file size (max 2MB)
  if (file.size > 2 * 1024 * 1024) {
    ElMessage.error('文件大小不能超过 2MB')
    return false
  }

  strategyCodeFile.value = file
  uploading.value = true

  try {
    const result = await uploadStrategyCode(file)
    strategyCodePath.value = result.path
    ElMessage.success('策略文件上传成功')
    return true
  } catch {
    ElMessage.error('文件上传失败，请重试')
    strategyCodeFile.value = null
    return false
  } finally {
    uploading.value = false
  }
}

function removeStrategyFile() {
  strategyCodeFile.value = null
  strategyCodePath.value = null
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
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

  // Step Indicator
  .step-indicator {
    display: flex;
    align-items: center;
    margin-bottom: 32px;

    .step {
      display: flex;
      align-items: center;
      gap: 8px;

      .step-circle {
        width: 28px;
        height: 28px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 13px;
        font-weight: 600;
        border: 2px solid var(--color-border);
        color: var(--color-text-tertiary);
        background: var(--color-bg);
        transition: all 0.2s;
      }

      .step-label {
        font-size: 13px;
        color: var(--color-text-tertiary);
        font-weight: 500;
        transition: color 0.2s;
      }

      &.active {
        .step-circle {
          border-color: var(--color-accent);
          color: var(--color-accent);
        }
        .step-label {
          color: var(--color-text-primary);
        }
      }

      &.completed {
        .step-circle {
          background: var(--color-accent);
          border-color: var(--color-accent);
          color: #fff;
        }
        .step-label {
          color: var(--color-text-primary);
        }
      }
    }

    .step-line {
      flex: 1;
      height: 2px;
      background: var(--color-border);
      margin: 0 12px;
      transition: background 0.2s;

      &.completed {
        background: var(--color-accent);
      }
    }
  }

  .step-content {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .step-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 24px 0;

    .right-actions {
      display: flex;
      gap: 12px;
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

    .section-title {
      font-size: 15px;
      font-weight: 600;
      color: var(--color-text-primary);
    }

    .section-desc {
      font-size: 13px;
      color: var(--color-text-tertiary);
      margin: 8px 0 16px;
    }
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
      border-color: var(--color-border-hover, rgba(255,255,255,0.16));
      transform: translateY(-2px);
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
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

  // Basic Form
  .basic-form {
    max-width: 600px;
  }

  .form-row {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }

  .form-item-half {
    flex: 1;
    min-width: 200px;
  }

  .form-item-third {
    flex: 1;
    min-width: 160px;
  }

  .form-item-full {
    flex: 1 1 100%;
    min-width: 200px;
  }

  .field-hint {
    display: block;
    font-size: 11px;
    color: var(--color-text-tertiary);
    margin-top: 4px;
  }

  .symbol-select,
  .timeframe-select,
  .strategy-type-select {
    width: 100%;
  }

  // T4.5: File upload styles
  .strategy-code-uploader {
    display: block;

    .upload-tip {
      font-size: 12px;
      color: var(--color-text-tertiary);
      margin-top: 6px;
    }
  }

  .file-info {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    background: var(--color-surface-elevated);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    max-width: 400px;

    .file-icon {
      font-size: 18px;
      color: var(--color-accent);
      flex-shrink: 0;
    }

    .file-details {
      display: flex;
      flex-direction: column;
      gap: 2px;
      flex: 1;
      min-width: 0;

      .file-name {
        font-size: 13px;
        font-weight: 500;
        color: var(--color-text-primary);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .file-size {
        font-size: 11px;
        color: var(--color-text-tertiary);
      }
    }

    .file-remove {
      font-size: 14px;
      color: var(--color-text-tertiary);
      cursor: pointer;
      flex-shrink: 0;
      transition: color 0.2s;

      &:hover {
        color: var(--color-error);
      }
    }
  }

  .file-path {
    font-size: 11px;
    color: var(--color-text-tertiary);
    font-family: var(--font-mono);
  }

  // Risk Form
  .risk-form {
    max-width: 800px;

    .form-row {
      display: flex;
      gap: 16px;
      flex-wrap: wrap;
    }

    .form-item-third {
      flex: 1;
      min-width: 160px;
    }

    .risk-input-number {
      width: 100%;
    }

    .percent-input {
      display: flex;
      align-items: center;
      gap: 4px;

      .risk-input-number {
        flex: 1;
      }

      .percent-suffix {
        font-size: 14px;
        color: var(--color-text-secondary);
        min-width: 16px;
      }
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

  // Parameter Preview (sidebar style)
  .preview-panel {
    position: sticky;
    top: 80px;
    float: right;
    width: 280px;
    margin-top: -16px;
  }

  .param-preview {
    background: var(--color-surface-elevated);
    border: 1px solid var(--color-border);
    border-radius: 8px;

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
    flex-direction: column;
    gap: 2px;

    .preview-label {
      font-size: 11px;
      color: var(--color-text-tertiary);
    }

    .preview-value {
      font-size: 13px;
      color: var(--color-text-primary);
      font-family: var(--font-mono);
    }
  }

  .preview-divider {
    height: 1px;
    background: var(--color-border);
    margin: 4px 0;
  }
}
</style>
