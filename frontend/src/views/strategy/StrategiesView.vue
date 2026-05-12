<template>
  <div class="strategies-view">
    <!-- Page Header -->
    <div class="page-header">
      <h1 class="page-title">策略管理</h1>
      <router-link :to="{ name: 'StrategyCreate' }">
        <el-button type="primary" class="create-btn">
          <el-icon><Plus /></el-icon>
          新建策略
        </el-button>
      </router-link>
    </div>

    <!-- Loading State -->
    <template v-if="loading">
      <el-skeleton :rows="6" animated class="skeleton-table" />
    </template>

    <!-- Error State -->
    <template v-else-if="error">
      <el-card class="state-card" shadow="never">
        <el-empty description="加载失败" :image-size="80">
          <template #image>
            <el-icon :size="48" color="var(--color-error)"><WarningFilled /></el-icon>
          </template>
          <el-button type="primary" class="retry-btn" @click="fetchStrategies">重新加载</el-button>
        </el-empty>
      </el-card>
    </template>

    <!-- Content -->
    <template v-else>
      <!-- Filter Pills -->
      <div class="filter-bar">
        <el-radio-group v-model="filterStatus" @change="onFilterChange" class="filter-pills">
          <el-radio-button value="" class="filter-pill">全部</el-radio-button>
          <el-radio-button value="draft" class="filter-pill">草稿</el-radio-button>
          <el-radio-button value="active" class="filter-pill">运行中</el-radio-button>
          <el-radio-button value="paused" class="filter-pill">已暂停</el-radio-button>
          <el-radio-button value="stopped" class="filter-pill">已停止</el-radio-button>
        </el-radio-group>
      </div>

      <!-- Empty State -->
      <template v-if="strategies.length === 0">
        <el-card class="state-card" shadow="never">
          <el-empty description="暂无策略" :image-size="80">
            <template #image>
              <el-icon :size="48" color="var(--color-text-tertiary)"><Cpu /></el-icon>
            </template>
            <router-link :to="{ name: 'StrategyCreate' }">
              <el-button type="primary">创建第一个策略</el-button>
            </router-link>
          </el-empty>
        </el-card>
      </template>

      <!-- Strategy Table -->
      <el-card v-else class="table-card" shadow="never">
        <el-table :data="strategies" stripe style="width: 100%">
          <el-table-column label="名称" min-width="180">
            <template #default="{ row }">
              <div class="strategy-name">{{ row.name }}</div>
            </template>
          </el-table-column>

          <el-table-column label="模板类型" width="130">
            <template #default="{ row }">
              <el-tag
                class="template-type-tag"
                :type="getTemplateTagType(row.template_type)"
                size="small"
                effect="plain"
              >
                {{ getTemplateLabel(row.template_type) }}
              </el-tag>
            </template>
          </el-table-column>

          <el-table-column label="参数摘要" min-width="200">
            <template #default="{ row }">
              <div class="param-summary">{{ formatParamSummary(row.parameters) }}</div>
            </template>
          </el-table-column>

          <el-table-column label="创建时间" width="170">
            <template #default="{ row }">
              <span class="time-cell">{{ formatDate(row.created_at) }}</span>
            </template>
          </el-table-column>

          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag
                class="status-tag"
                :type="getStatusTagType(row.status)"
                size="small"
              >
                {{ getStatusLabel(row.status) }}
              </el-tag>
            </template>
          </el-table-column>

          <el-table-column label="操作" width="220" fixed="right">
            <template #default="{ row }">
              <el-button
                class="toggle-btn"
                :type="row.status === 'active' ? 'warning' : 'success'"
                size="small"
                link
                @click="handleToggle(row)"
              >
                {{ row.status === 'active' ? '暂停' : '启用' }}
              </el-button>
              <el-button
                class="edit-btn"
                size="small"
                link
                @click="handleEdit(row)"
              >
                编辑
              </el-button>
              <el-popconfirm
                title="确认删除此策略？"
                confirm-button-text="确认删除"
                cancel-button-text="取消"
                @confirm="handleDelete(row)"
              >
                <template #reference>
                  <el-button
                    class="delete-btn"
                    type="danger"
                    size="small"
                    link
                  >
                    删除
                  </el-button>
                </template>
              </el-popconfirm>
            </template>
          </el-table-column>
        </el-table>
      </el-card>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, WarningFilled, Cpu } from '@element-plus/icons-vue'
import { listStrategies, deleteStrategy, toggleStrategy } from '@/api/strategies'
import type { StrategyFull } from '@/types'

const router = useRouter()

const strategies = ref<StrategyFull[]>([])
const loading = ref(false)
const error = ref(false)
const filterStatus = ref('')

async function fetchStrategies() {
  loading.value = true
  error.value = false
  try {
    const params = filterStatus.value ? { status: filterStatus.value, page: 1, size: 100 } : { page: 1, size: 100 }
    strategies.value = await listStrategies(params)
  } catch {
    error.value = true
  } finally {
    loading.value = false
  }
}

function onFilterChange() {
  fetchStrategies()
}

async function handleToggle(row: StrategyFull) {
  try {
    const newStatus = row.status === 'active' ? 'paused' : 'active'
    await toggleStrategy(row.id, newStatus)
    ElMessage.success(newStatus === 'active' ? '策略已启用' : '策略已暂停')
    await fetchStrategies()
  } catch {
    ElMessage.error('操作失败，请重试')
  }
}

function handleEdit(row: StrategyFull) {
  router.push({ name: 'StrategyEdit', params: { id: row.id } })
}

async function handleDelete(row: StrategyFull) {
  try {
    await deleteStrategy(row.id)
    ElMessage.success('策略已删除')
    await fetchStrategies()
  } catch {
    ElMessage.error('删除失败，请重试')
  }
}

function getTemplateTagType(type: string | undefined): string {
  const map: Record<string, string> = {
    ma_crossover: 'primary',
    triple_ma: 'primary',
    macd: 'warning',
    bollinger: 'success',
    rsi: 'warning',
    keltner: '',
    atr_stop: 'danger',
    mean_reversion: 'info',
    ichimoku: '',
    double_bollinger: 'success',
  }
  return map[type || ''] || 'info'
}

function getTemplateLabel(type: string | undefined): string {
  const map: Record<string, string> = {
    ma_crossover: 'MA交叉',
    triple_ma: '三均线',
    macd: 'MACD',
    bollinger: '布林带',
    rsi: 'RSI',
    keltner: '肯特纳通道',
    atr_stop: 'ATR止损',
    mean_reversion: '均值回归',
    ichimoku: '一目均衡',
    double_bollinger: '双布林带',
  }
  return map[type || ''] || type || '自定义'
}

function getStatusTagType(status: string): string {
  const map: Record<string, string> = {
    draft: 'info',
    active: 'success',
    paused: 'warning',
    stopped: 'danger',
  }
  return map[status] || 'info'
}

function getStatusLabel(status: string): string {
  const map: Record<string, string> = {
    draft: '草稿',
    active: '运行中',
    paused: '已暂停',
    stopped: '已停止',
  }
  return map[status] || status
}

function formatParamSummary(params: Record<string, any> | undefined): string {
  if (!params) return '—'
  return Object.entries(params)
    .map(([key, value]) => `${key}: ${value}`)
    .join(', ')
}

function formatDate(dateStr: string): string {
  if (!dateStr) return '—'
  const d = new Date(dateStr)
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

onMounted(() => {
  fetchStrategies()
})
</script>

<style scoped lang="scss">
.strategies-view {
  max-width: var(--content-max-width);

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;

    .page-title {
      font-size: 24px;
      font-weight: 600;
      color: var(--color-text-primary);
      margin: 0;
    }
  }

  .filter-bar {
    margin-bottom: 16px;

    .filter-pills {
      :deep(.el-radio-button__inner) {
        background: transparent;
        border-color: var(--color-border);
        color: var(--color-text-tertiary);
        font-size: 13px;
        padding: 8px 16px;
      }

      :deep(.el-radio-button__original-radio:checked + .el-radio-button__inner) {
        background: var(--color-accent);
        border-color: var(--color-accent);
        color: #fff;
        box-shadow: none;
      }
    }
  }

  .skeleton-table {
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

  .table-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    overflow: hidden;

    :deep(.el-table) {
      border: none;
    }

    :deep(.el-table__header-wrapper) {
      border-bottom: 1px solid var(--color-border);
    }
  }

  .strategy-name {
    font-size: 14px;
    font-weight: 510;
    color: var(--color-text-primary);
    margin-bottom: 2px;
  }

  .strategy-desc {
    font-size: 12px;
    color: var(--color-text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 240px;
  }

  .param-summary {
    font-size: 12px;
    color: var(--color-text-secondary);
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 260px;
  }

  .time-cell {
    font-size: 12px;
    color: var(--color-text-secondary);
    font-family: var(--font-mono);
  }
}
</style>
