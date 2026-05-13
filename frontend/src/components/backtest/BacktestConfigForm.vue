<template>
  <div class="backtest-config-form">
    <div class="config-header">
      <h3 class="section-title">回测配置</h3>
    </div>

    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-position="top"
      class="config-form"
    >
      <div class="form-grid">
        <el-form-item label="选择策略" prop="strategy_id" class="strategy-select">
          <el-select
            v-model="form.strategy_id"
            placeholder="请选择策略"
            style="width: 100%"
            :loading="loadingStrategies"
            :disabled="loading"
            filterable
          >
            <el-option
              v-for="s in strategies"
              :key="s.id"
              :label="s.name"
              :value="s.id"
            />
          </el-select>
        </el-form-item>

        <el-form-item label="时间周期" prop="timeframe" class="timeframe-select">
          <el-select
            v-model="form.timeframe"
            placeholder="请选择时间周期"
            style="width: 100%"
            :disabled="loading"
          >
            <el-option
              v-for="tf in timeframes"
              :key="tf.value"
              :label="tf.label"
              :value="tf.value"
            />
          </el-select>
        </el-form-item>

        <el-form-item label="交易对" prop="symbol" class="symbol-input">
          <el-input
            v-model="form.symbol"
            placeholder="如: BTC/USDT"
            :disabled="loading"
            clearable
          />
        </el-form-item>

        <el-form-item label="初始资金" prop="initial_capital" class="capital-input">
          <el-input-number
            v-model="form.initial_capital"
            :min="100"
            :max="100000000"
            :step="10000"
            style="width: 100%"
            :precision="2"
            :disabled="loading"
          />
        </el-form-item>

        <el-form-item label="手续费率" prop="fee_rate" class="fee-rate-input">
          <el-input-number
            v-model="form.fee_rate"
            :min="0"
            :max="0.01"
            :step="0.0001"
            :precision="4"
            style="width: 100%"
            :disabled="loading"
          />
          <div class="field-hint">0.001 = 0.1%</div>
        </el-form-item>

        <el-form-item label="滑点率" prop="slippage_rate" class="slippage-input">
          <el-input-number
            v-model="form.slippage_rate"
            :min="0"
            :max="0.01"
            :step="0.0001"
            :precision="4"
            style="width: 100%"
            :disabled="loading"
          />
          <div class="field-hint">0.001 = 0.1%</div>
        </el-form-item>

        <el-form-item label="时间范围" prop="dateRange" class="date-range">
          <el-date-picker
            v-model="form.dateRange"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            style="width: 100%"
            value-format="YYYY-MM-DD"
            :disabled="loading"
          />
        </el-form-item>
      </div>

      <!-- Strategy Parameters Section -->
      <div v-if="form.strategy_id" class="strategy-params-section">
        <div v-if="selectedTemplate" class="params-header">
          <span class="params-title">策略参数配置</span>
          <el-tag size="small" type="info">{{ selectedTemplate.name }}</el-tag>
        </div>
        <el-form
          v-if="selectedTemplate"
          ref="paramsFormRef"
          :model="form.strategy_params"
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
            <!-- Number: integer or float -->
            <template v-if="param.type === 'integer' || param.type === 'float'">
              <div class="number-param">
                <el-slider
                  v-model="form.strategy_params[param.name]"
                  :min="param.min ?? 0"
                  :max="param.max ?? 100"
                  :step="param.type === 'float' ? 0.1 : 1"
                  :show-stops="false"
                  class="param-slider"
                  :disabled="loading"
                />
                <el-input-number
                  v-model="form.strategy_params[param.name]"
                  :min="param.min ?? 0"
                  :max="param.max ?? 100"
                  :precision="param.type === 'float' ? 2 : 0"
                  :step="param.type === 'float' ? 0.1 : 1"
                  size="small"
                  class="param-input-number"
                  :disabled="loading"
                />
              </div>
              <div v-if="param.description" class="param-desc">{{ param.description }}</div>
            </template>

            <!-- Select -->
            <template v-else-if="param.type === 'select'">
              <el-select
                v-model="form.strategy_params[param.name]"
                placeholder="请选择"
                class="param-select"
                :disabled="loading"
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
              <el-switch v-model="form.strategy_params[param.name]" :disabled="loading" />
              <div v-if="param.description" class="param-desc">{{ param.description }}</div>
            </template>

            <!-- String -->
            <template v-else>
              <el-input
                v-model="form.strategy_params[param.name]"
                placeholder="请输入"
              />
              <div v-if="param.description" class="param-desc">{{ param.description }}</div>
            </template>
          </el-form-item>
        </el-form>
      </div>

      <div class="form-actions">
        <el-button
          type="primary"
          size="large"
          class="run-btn"
          :loading="loading"
          :disabled="!isFormValid"
          @click="handleRun"
        >
          {{ loading ? '运行中...' : '运行回测' }}
        </el-button>
        <el-button
          size="large"
          class="reset-btn"
          @click="handleReset"
        >
          重置
        </el-button>
      </div>
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import { listStrategies, listTemplates } from '@/api/strategies'
import type { StrategyFull, StrategyTemplate, StrategyParamDef } from '@/types'
import type { BacktestParams } from '@/types/backtest'

const props = withDefaults(defineProps<{
  loading?: boolean
}>(), {
  loading: false,
})

const emit = defineEmits<{
  run: [params: BacktestParams]
}>()

const formRef = ref<FormInstance>()
const paramsFormRef = ref<FormInstance>()
const strategies = ref<StrategyFull[]>([])
const templates = ref<StrategyTemplate[]>([])
const selectedTemplate = ref<StrategyTemplate | null>(null)
const loadingStrategies = ref(false)
const loadingTemplates = ref(false)

const timeframes = [
  { label: '1 分钟', value: '1m' },
  { label: '5 分钟', value: '5m' },
  { label: '15 分钟', value: '15m' },
  { label: '30 分钟', value: '30m' },
  { label: '1 小时', value: '1h' },
  { label: '4 小时', value: '4h' },
  { label: '1 天', value: '1d' },
  { label: '1 周', value: '1w' },
]

const form = reactive({
  strategy_id: '',
  symbol: '',
  timeframe: '1h',
  initial_capital: 100000,
  fee_rate: 0.001,
  slippage_rate: 0.001,
  dateRange: [] as string[],
  strategy_params: {} as Record<string, any>,
})

const rules: FormRules = {
  strategy_id: [{ required: true, message: '请选择策略', trigger: 'change' }],
  symbol: [{ required: true, message: '请输入交易对', trigger: 'blur' }],
  timeframe: [{ required: true, message: '请选择时间周期', trigger: 'change' }],
  initial_capital: [{ required: true, message: '请输入初始资金', trigger: 'blur' }],
  fee_rate: [{ required: true, message: '请输入手续费率', trigger: 'blur' }],
  slippage_rate: [{ required: true, message: '请输入滑点率', trigger: 'blur' }],
  dateRange: [{ required: true, message: '请选择时间范围', trigger: 'change' }],
}

const isFormValid = computed(() => {
  return (
    form.strategy_id &&
    form.symbol &&
    form.timeframe &&
    form.initial_capital > 0 &&
    form.fee_rate >= 0 &&
    form.slippage_rate >= 0 &&
    form.dateRange.length === 2
  )
})

function getParamRule(param: StrategyParamDef) {
  const rulesArr: any[] = []
  rulesArr.push({ required: true, message: `请配置${param.label}`, trigger: 'change' })
  if ((param.type === 'integer' || param.type === 'float') && param.min !== undefined) {
    rulesArr.push({ type: 'number', min: param.min, message: `最小值为${param.min}`, trigger: 'blur' })
  }
  if ((param.type === 'integer' || param.type === 'float') && param.max !== undefined) {
    rulesArr.push({ type: 'number', max: param.max, message: `最大值为${param.max}`, trigger: 'blur' })
  }
  return rulesArr
}

async function loadStrategies() {
  loadingStrategies.value = true
  try {
    strategies.value = await listStrategies()
  } catch {
    strategies.value = []
  } finally {
    loadingStrategies.value = false
  }
}

async function loadTemplates() {
  loadingTemplates.value = true
  try {
    const result = await listTemplates()
    templates.value = result ?? []
  } catch {
    templates.value = []
  } finally {
    loadingTemplates.value = false
  }
}

async function loadTemplateParams() {
  // Ensure templates are loaded
  if (!templates.value || templates.value.length === 0) {
    await loadTemplates()
  }

  const strategy = strategies.value.find((s) => s.id === form.strategy_id)
  if (!strategy) {
    selectedTemplate.value = null
    return
  }

  const tpl = (templates.value ?? []).find((t) => t.id === strategy.template_type)
  if (tpl) {
    selectedTemplate.value = tpl
    // Reset and populate params with defaults
    const defaults: Record<string, any> = {}
    tpl.parameter_schema.forEach((p) => {
      defaults[p.name] = p.default ?? (
        p.type === 'integer' || p.type === 'float' ? p.min ?? 0
        : p.type === 'boolean' ? false
        : ''
      )
    })
    form.strategy_params = defaults
  } else {
    selectedTemplate.value = null
  }
}

function handleReset() {
  form.strategy_id = ''
  form.symbol = ''
  form.timeframe = '1h'
  form.initial_capital = 100000
  form.fee_rate = 0.001
  form.slippage_rate = 0.001
  form.dateRange = []
  form.strategy_params = {}
  selectedTemplate.value = null
  formRef.value?.clearValidate()
  paramsFormRef.value?.clearValidate()
}

function handleRun() {
  if (!formRef.value) return

  formRef.value.validate((valid) => {
    if (valid) {
      if (paramsFormRef.value) {
        paramsFormRef.value.validate((paramsValid) => {
          if (paramsValid) {
            emit('run', {
              strategy_id: form.strategy_id,
              symbol: form.symbol,
              interval: form.timeframe,
              start_date: form.dateRange[0],
              end_date: form.dateRange[1],
              initial_capital: form.initial_capital,
              fee_rate: form.fee_rate,
              slippage_rate: form.slippage_rate,
              strategy_params: { ...form.strategy_params },
            } as BacktestParams)
          }
        })
      } else {
        emit('run', {
          strategy_id: form.strategy_id,
          symbol: form.symbol,
          interval: form.timeframe,
          start_date: form.dateRange[0],
          end_date: form.dateRange[1],
          initial_capital: form.initial_capital,
          fee_rate: form.fee_rate,
          slippage_rate: form.slippage_rate,
          strategy_params: { ...form.strategy_params },
        } as BacktestParams)
      }
    }
  })
}

watch(() => form.strategy_id, () => {
  if (form.strategy_id) {
    loadTemplateParams()
  } else {
    selectedTemplate.value = null
    form.strategy_params = {}
  }
})

onMounted(() => {
  loadStrategies()
  loadTemplates()
})
</script>

<style scoped lang="scss">
.backtest-config-form {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 24px;
}

.config-header {
  margin-bottom: 20px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.config-form {
  max-width: 100%;
}

.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

@media (max-width: 768px) {
  .form-grid {
    grid-template-columns: 1fr;
  }
}

.strategy-params-section {
  margin-top: 24px;
  padding-top: 20px;
  border-top: 1px solid var(--color-border);

  .params-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 16px;

    .params-title {
      font-size: 14px;
      font-weight: 600;
      color: var(--color-text-primary);
    }
  }

  .param-form {
    max-width: 100%;
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
  }

  .param-desc {
    font-size: 12px;
    color: var(--color-text-tertiary);
    margin-top: 4px;
    line-height: 1.4;
  }
}

.form-actions {
  margin-top: 24px;
  display: flex;
  justify-content: flex-end;
  gap: 12px;

  .run-btn {
    min-width: 160px;
  }

  .reset-btn {
    min-width: 80px;
  }
}

.field-hint {
  font-size: 11px;
  color: var(--color-text-tertiary);
  margin-top: 2px;
}
</style>
