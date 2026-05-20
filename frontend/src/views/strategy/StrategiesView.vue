<template>
  <div class="strategies-view">
    <!-- Page Header -->
    <div class="page-header">
      <h1 class="page-title">策略管理</h1>
      <div class="header-actions">
        <router-link :to="{ name: 'StrategyTemplates' }" class="template-link">
          <el-button type="default" class="template-market-btn">
            <el-icon><Shop /></el-icon>
            策略模板市场
          </el-button>
        </router-link>
        <el-button type="default" @click="handleImport">
          <el-icon><Upload /></el-icon>
          导入
        </el-button>
        <el-button type="default" @click="handleExportAll">
          <el-icon><Download /></el-icon>
          导出全部
        </el-button>
        <router-link :to="{ name: 'StrategyCreate' }">
          <el-button type="primary" class="create-btn">
            <el-icon><Plus /></el-icon>
            创建策略
          </el-button>
        </router-link>
      </div>
    </div>

    <!-- Loading State -->
    <template v-if="loading">
      <el-skeleton :rows="6" animated class="skeleton-table" />
    </template>

    <!-- Error State -->
    <template v-else-if="error">
      <el-card class="state-card" shadow="never">
        <el-result icon="error" title="加载失败" sub-title="策略数据获取失败，请重试">
          <template #extra>
            <el-button type="primary" @click="fetchStrategies">重新加载</el-button>
          </template>
        </el-result>
      </el-card>
    </template>

    <!-- Content -->
    <template v-else>
      <!-- Filter Bar -->
      <div class="filter-bar">
        <div class="filter-left">
          <el-radio-group v-model="filterStatus" @change="onFilterChange" class="filter-pills">
            <el-radio-button value="" class="filter-pill">全部</el-radio-button>
            <el-radio-button value="draft" class="filter-pill">草稿</el-radio-button>
            <el-radio-button value="active" class="filter-pill">运行中</el-radio-button>
            <el-radio-button value="paused" class="filter-pill">已暂停</el-radio-button>
            <el-radio-button value="stopped" class="filter-pill">已停止</el-radio-button>
          </el-radio-group>
        </div>
        <div class="filter-right">
          <el-input
            v-model="searchQuery"
            placeholder="搜索策略名称..."
            :prefix-icon="Search"
            clearable
            class="search-input"
            @clear="onSearchClear"
            @keyup.enter="onSearchChange"
          />
          <el-select v-model="sortBy" size="default" class="sort-select" @change="onSortChange">
            <el-option value="created_at:desc" label="创建时间（最新）" />
            <el-option value="created_at:asc" label="创建时间（最早）" />
            <el-option value="name:asc" label="名称 A-Z" />
            <el-option value="name:desc" label="名称 Z-A" />
          </el-select>
        </div>
      </div>

      <!-- Bulk Action Bar -->
      <transition name="slide-down">
        <div v-if="selectedIds.length > 0" class="bulk-action-bar">
          <span class="selected-count">已选择 {{ selectedIds.length }} 项:</span>
          <el-button
            v-if="canBulkStart"
            size="small"
            type="success"
            @click="handleBulkStart"
            :loading="bulkLoading"
          >启用</el-button>
          <el-button
            v-if="canBulkPause"
            size="small"
            type="warning"
            @click="handleBulkPause"
            :loading="bulkLoading"
          >暂停</el-button>
          <el-button
            v-if="canBulkStop"
            size="small"
            type="danger"
            @click="handleBulkStop"
            :loading="bulkLoading"
          >停止</el-button>
          <el-button
            size="small"
            type="danger"
            @click="handleBulkDelete"
            :loading="bulkLoading"
          >删除</el-button>
          <el-button size="small" text @click="clearSelection">取消全选</el-button>
        </div>
      </transition>

      <!-- Empty State -->
      <template v-if="strategies.length === 0 && !isFiltered">
        <el-card class="state-card" shadow="never">
          <el-empty description="还没有创建策略" :image-size="80">
            <template #image>
              <el-icon :size="48" color="var(--color-text-tertiary)"><Cpu /></el-icon>
            </template>
            <router-link :to="{ name: 'StrategyCreate' }">
              <el-button type="primary">创建第一个策略</el-button>
            </router-link>
          </el-empty>
        </el-card>
      </template>

      <!-- Filter Empty State -->
      <template v-else-if="strategies.length === 0 && isFiltered">
        <el-card class="state-card" shadow="never">
          <el-empty description="没有匹配的策略" :image-size="80">
            <template #image>
              <el-icon :size="48" color="var(--color-text-tertiary)"><Search /></el-icon>
            </template>
            <template #default>
              <p class="empty-sub">尝试调整筛选条件或搜索关键词</p>
              <el-button text @click="clearFilters">清除筛选</el-button>
            </template>
          </el-empty>
        </el-card>
      </template>

      <!-- Strategy Card Grid -->
      <div v-else class="strategy-grid">
        <el-card
          v-for="strategy in strategies"
          :key="strategy.id"
          class="strategy-card"
          :class="{
            'is-selected': selectedIds.includes(strategy.id),
            'is-stopped': strategy.status === 'stopped',
          }"
          shadow="never"
          @click="toggleSelection(strategy)"
        >
          <!-- Selection checkbox (stops card click when clicked) -->
          <div class="card-checkbox" @click.stop>
            <el-checkbox
              :model-value="selectedIds.includes(strategy.id)"
              :disabled="strategy.status === 'stopped'"
              @change="toggleSelection(strategy)"
            />
          </div>

          <!-- Card Header: name + action menu -->
          <div class="card-header">
            <div class="card-title-row">
              <span class="strategy-name">{{ strategy.name }}</span>
              <StrategyStatusBadge :status="strategy.status" />
            </div>
            <div class="strategy-desc">{{ strategy.description || '—' }}</div>
          </div>

          <!-- Card Body: symbol/timeframe + template tag -->
          <div class="card-body">
            <div class="card-info-row">
              <span class="symbol-cell">{{ formatSymbol(strategy.symbol) }}</span>
              <span class="timeframe-cell">{{ strategy.timeframe }}</span>
              <el-tag
                class="template-type-tag"
                :type="getTemplateTagType(strategy.template_type)"
                size="small"
                effect="plain"
              >
                {{ getTemplateLabel(strategy.template_type) }}
              </el-tag>
            </div>
            <!-- Performance metrics row -->
            <div class="card-metrics-row">
              <div class="metric-item" :title="'收益率'">
                <span class="metric-label">收益率</span>
                <span
                  class="metric-value"
                  :class="getReturnClass(strategy.performance)"
                >{{ formatMetricValue(strategy.performance?.total_return_pct) }}</span>
              </div>
              <div class="metric-item" :title="'夏普率'">
                <span class="metric-label">夏普率</span>
                <span class="metric-value">{{ formatMetricValue(strategy.performance?.sharpe_ratio) }}</span>
              </div>
              <div class="metric-item" :title="'最大回撤'">
                <span class="metric-label">最大回撤</span>
                <span
                  class="metric-value down"
                >{{ formatMetricValue(strategy.performance?.max_drawdown_pct) }}</span>
              </div>
              <div class="metric-item" :title="'胜率'">
                <span class="metric-label">胜率</span>
                <span class="metric-value">{{ formatPercentValue(strategy.performance?.win_rate) }}</span>
              </div>
            </div>
          </div>

          <!-- Card Footer: last run time + action menu -->
          <div class="card-footer">
            <span class="last-run-time">上次运行: {{ formatDate(strategy.updated_at) }}</span>
            <el-dropdown trigger="click" @command="(cmd: string) => handleActionCommand(cmd, strategy)">
              <el-button class="action-btn" text @click.stop>
                <el-icon><MoreFilled /></el-icon>
              </el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <template v-if="strategy.status === 'draft'">
                    <el-dropdown-item command="edit">
                      <el-icon><Edit /></el-icon>编辑
                    </el-dropdown-item>
                    <el-dropdown-item command="export" divided>
                      <el-icon><Download /></el-icon>导出
                    </el-dropdown-item>
                    <el-dropdown-item command="delete">
                      <el-icon><Delete /></el-icon><span class="danger-text">删除</span>
                    </el-dropdown-item>
                  </template>
                  <template v-else-if="strategy.status === 'active'">
                    <el-dropdown-item command="pause">
                      <el-icon><VideoPause /></el-icon>暂停
                    </el-dropdown-item>
                    <el-dropdown-item command="stop">
                      <el-icon><CircleClose /></el-icon>停止
                    </el-dropdown-item>
                    <el-dropdown-item command="clone">
                      <el-icon><CopyDocument /></el-icon>克隆
                    </el-dropdown-item>
                    <el-dropdown-item command="export" divided>
                      <el-icon><Download /></el-icon>导出
                    </el-dropdown-item>
                  </template>
                  <template v-else-if="strategy.status === 'paused'">
                    <el-dropdown-item command="start">
                      <el-icon><VideoPlay /></el-icon>启用
                    </el-dropdown-item>
                    <el-dropdown-item command="stop">
                      <el-icon><CircleClose /></el-icon>停止
                    </el-dropdown-item>
                    <el-dropdown-item command="edit">
                      <el-icon><Edit /></el-icon>编辑
                    </el-dropdown-item>
                    <el-dropdown-item command="clone">
                      <el-icon><CopyDocument /></el-icon>克隆
                    </el-dropdown-item>
                    <el-dropdown-item command="export" divided>
                      <el-icon><Download /></el-icon>导出
                    </el-dropdown-item>
                  </template>
                  <template v-else-if="strategy.status === 'stopped'">
                    <el-dropdown-item command="clone">
                      <el-icon><CopyDocument /></el-icon>克隆
                    </el-dropdown-item>
                    <el-dropdown-item command="export" divided>
                      <el-icon><Download /></el-icon>导出
                    </el-dropdown-item>
                    <el-dropdown-item command="delete">
                      <el-icon><Delete /></el-icon><span class="danger-text">删除</span>
                    </el-dropdown-item>
                  </template>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </el-card>
      </div>

      <!-- Pagination -->
      <div v-if="strategies.length > 0" class="pagination-wrapper">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[12, 24, 48]"
          :total="totalCount"
          layout="prev, pager, next, total, sizes"
          @current-change="onPageChange"
          @size-change="onSizeChange"
        />
      </div>
    </template>

    <!-- Delete Confirm Modal -->
    <el-dialog v-model="deleteModalVisible" title="确认删除" width="420px" destroy-on-close>
      <p v-if="deleteTargets.length === 1">
        确定要删除「{{ deleteTargets[0].name }}」吗？此操作不可撤销。
      </p>
      <p v-else>
        确定要删除选中的 {{ deleteTargets.length }} 个策略吗？此操作不可撤销。<br />
        <span class="delete-targets-preview">{{ deleteTargets.map(t => t.name).join('、') }}</span>
      </p>
      <template #footer>
        <el-button @click="deleteModalVisible = false">取消</el-button>
        <el-button type="danger" :loading="deleteLoading" @click="confirmDelete">确认删除</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Search, Cpu, MoreFilled, Edit, Delete, VideoPlay, VideoPause, CircleClose, CopyDocument, Shop, Upload, Download } from '@element-plus/icons-vue'
import { listStrategies, getStrategy, createStrategy, deleteStrategy, toggleStrategy, bulkUpdateStatus, bulkDeleteStrategies, importStrategies, exportStrategies } from '@/api/strategies'
import type { StrategyFull, CreateStrategyPayload, StrategyType } from '@/types'

// StrategyStatusBadge inline component
import { defineComponent, h } from 'vue'
const StrategyStatusBadge = defineComponent({
  props: { status: String },
  setup(props) {
    const statusMap: Record<string, { label: string; color: string }> = {
      draft: { label: '草稿', color: 'var(--color-text-tertiary)' },
      active: { label: '运行中', color: 'var(--color-success)' },
      paused: { label: '已暂停', color: 'var(--color-warning)' },
      stopped: { label: '已停止', color: 'var(--color-error)' },
      archived: { label: '已归档', color: 'var(--color-text-tertiary)' },
    }
    const info = computed(() => statusMap[props.status || ''] || { label: '未知', color: 'var(--color-text-tertiary)' })
    return () => h('span', { class: 'status-badge', style: { color: info.value.color } }, [
      h('span', { class: 'status-dot', style: { background: info.value.color } }),
      info.value.label,
    ])
  },
})

const router = useRouter()

const strategies = ref<StrategyFull[]>([])
const loading = ref(false)
const error = ref(false)
const filterStatus = ref('')
const searchQuery = ref('')
const sortBy = ref('created_at:desc')
const currentPage = ref(1)
const pageSize = ref(20)
const totalCount = ref(0)
const selectedIds = ref<string[]>([])
const tableRef = ref()

// Delete modal
const deleteModalVisible = ref(false)
const deleteTargets = ref<StrategyFull[]>([])
const deleteLoading = ref(false)
const bulkLoading = ref(false)
const importLoading = ref(false)

// Import/Export handlers
const handleImport = async () => {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.json'
  input.onchange = async (e: Event) => {
    const file = (e.target as HTMLInputElement).files?.[0]
    if (!file) return
    importLoading.value = true
    try {
      const text = await file.text()
      const strategies = JSON.parse(text) as CreateStrategyPayload[]
      if (!Array.isArray(strategies)) throw new Error('JSON must be an array of strategies')
      const result = await importStrategies(strategies)
      ElMessage.success(`成功导入 ${result.imported} 个策略`)
      if (result.errors.length > 0) {
        ElMessage.warning(`部分失败: ${result.errors.join('; ')}`)
      }
      await fetchStrategies()
    } catch (err: any) {
      ElMessage.error(err?.message === 'JSON must be an array of strategies'
        ? '文件格式错误：JSON 必须是策略数组'
        : '导入失败，请检查文件格式')
    } finally {
      importLoading.value = false
    }
  }
  input.click()
}

const handleExportAll = async () => {
  try {
    const result = await exportStrategies()
    const blob = new Blob([JSON.stringify(result.data, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = result.filename || `strategies_backup_${new Date().toISOString().slice(0, 10)}.json`
    a.click()
    URL.revokeObjectURL(url)
    ElMessage.success('导出成功')
  } catch {
    ElMessage.error('导出失败，请重试')
  }
}

const isFiltered = computed(() => filterStatus.value !== '' || searchQuery.value !== '')

const canBulkStart = computed(() => {
  const selected = strategies.value.filter(s => selectedIds.value.includes(s.id))
  return selected.some(r => r.status === 'paused' || r.status === 'draft')
})
const canBulkPause = computed(() => {
  const selected = strategies.value.filter(s => selectedIds.value.includes(s.id))
  return selected.some(r => r.status === 'active')
})
const canBulkStop = computed(() => {
  const selected = strategies.value.filter(s => selectedIds.value.includes(s.id))
  return selected.some(r => r.status === 'active' || r.status === 'paused')
})

async function fetchStrategies() {
  loading.value = true
  error.value = false
  try {
    const [sortField, sortOrder] = sortBy.value.split(':') as ['created_at' | 'name', 'asc' | 'desc']
    const result = await listStrategies({
      status: filterStatus.value || undefined,
      page: currentPage.value,
      size: pageSize.value,
      search: searchQuery.value || undefined,
      sort_by: sortField,
      sort_order: sortOrder,
    })
    const r = result as any
    strategies.value = r?.items ?? r ?? []
    totalCount.value = r?.total ?? 0
  } catch {
    error.value = true
  } finally {
    loading.value = false
  }
}

function onFilterChange() {
  currentPage.value = 1
  fetchStrategies()
}

function onSearchChange() {
  currentPage.value = 1
  fetchStrategies()
}

function onSearchClear() {
  currentPage.value = 1
  fetchStrategies()
}

function onSortChange() {
  currentPage.value = 1
  fetchStrategies()
}

function onPageChange() {
  fetchStrategies()
}

function onSizeChange() {
  currentPage.value = 1
  fetchStrategies()
}

function toggleSelection(strategy: StrategyFull) {
  if (strategy.status === 'stopped') return
  const id = strategy.id
  const idx = selectedIds.value.indexOf(id)
  if (idx === -1) {
    selectedIds.value.push(id)
  } else {
    selectedIds.value.splice(idx, 1)
  }
}

function clearSelection() {
  selectedIds.value = []
}

function clearFilters() {
  filterStatus.value = ''
  searchQuery.value = ''
  currentPage.value = 1
  fetchStrategies()
}

function formatSymbol(symbol: string): string {
  if (!symbol) return '—'
  // BTCUSDT -> BTC/USDT
  if (symbol.endsWith('USDT')) {
    return symbol.slice(0, -4) + '/USDT'
  }
  return symbol
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
    mean_reversion: '',
    trend_following: 'info',
    grid_trading: 'success',
    arbitrage: 'warning',
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
    trend_following: '趋势跟踪',
    grid_trading: '网格交易',
    arbitrage: '套利',
    ichimoku: '一目均衡',
    double_bollinger: '双布林带',
  }
  return map[type || ''] || type || '自定义'
}

function formatDate(dateStr: string): string {
  if (!dateStr) return '—'
  const d = new Date(dateStr)
  if (isNaN(d.getTime())) return dateStr
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

function formatMetricValue(value: number | null | undefined): string {
  if (value == null) return '—'
  if (Number.isInteger(value)) return value.toLocaleString()
  return value.toFixed(2)
}

function formatPercentValue(value: number | null | undefined): string {
  if (value == null) return '—'
  return value.toFixed(1) + '%'
}

function getReturnClass(performance: StrategyFull['performance'] | undefined): string {
  if (!performance?.total_return_pct) return ''
  return performance.total_return_pct >= 0 ? 'up' : 'down'
}

async function handleActionCommand(cmd: string, row: StrategyFull) {
  switch (cmd) {
    case 'edit':
      router.push({ name: 'StrategyEdit', params: { id: row.id } })
      break
    case 'clone':
      // Clone by creating a new strategy with same params but new name (draft status)
      try {
        const original = await getStrategy(row.id)
        const cloneData: CreateStrategyPayload = {
          name: `${original.name}_clone`,
          description: original.description,
          symbol: original.symbol,
          timeframe: original.timeframe,
          strategy_type: (original.strategy_type ?? 'ma_crossover') as StrategyType,
          template_id: original.template_id,
          template_type: original.template_type,
          parameters: original.parameters,
        }
        await createStrategy(cloneData)
        ElMessage.success(`策略「${original.name}」已克隆为草稿`)
        await fetchStrategies()
      } catch {
        ElMessage.error('克隆失败，请重试')
      }
      break
    case 'export':
      try {
        const result = await exportStrategies([row.id])
        const blob = new Blob([JSON.stringify(result.data, null, 2)], { type: 'application/json' })
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = `${row.name}_${new Date().toISOString().slice(0, 10)}.json`
        a.click()
        URL.revokeObjectURL(url)
        ElMessage.success('导出成功')
      } catch {
        ElMessage.error('导出失败，请重试')
      }
      break
    case 'start':
      try {
        await toggleStrategy(row.id, 'active')
        ElMessage.success('策略已启用')
        await fetchStrategies()
      } catch {
        ElMessage.error('操作失败，请重试')
      }
      break
    case 'pause':
      try {
        await toggleStrategy(row.id, 'paused')
        ElMessage.success('策略已暂停')
        await fetchStrategies()
      } catch {
        ElMessage.error('操作失败，请重试')
      }
      break
    case 'stop':
      try {
        await ElMessageBox.confirm(`确定要停止「${row.name}」吗？停止后将无法自动交易。`, '确认停止', {
          confirmButtonText: '确认停止',
          cancelButtonText: '取消',
          type: 'warning',
        })
        await toggleStrategy(row.id, 'stopped')
        ElMessage.success('策略已停止')
        await fetchStrategies()
      } catch {
        // user cancelled
      }
      break
    case 'delete':
      try {
        await ElMessageBox.confirm(`确定要删除「${row.name}」吗？此操作不可撤销。`, '确认删除', {
          confirmButtonText: '确认删除',
          cancelButtonText: '取消',
          type: 'warning',
        })
        await deleteStrategy(row.id)
        ElMessage.success('策略已删除')
        await fetchStrategies()
      } catch {
        // user cancelled
      }
      break
  }
}

async function handleBulkStart() {
  const ids = strategies.value
    .filter(s => selectedIds.value.includes(s.id) && (s.status === 'paused' || s.status === 'draft'))
    .map(s => s.id)
  if (ids.length === 0) return
  bulkLoading.value = true
  try {
    await bulkUpdateStatus(ids, 'active')
    ElMessage.success(`已启用 ${ids.length} 个策略`)
    clearSelection()
    await fetchStrategies()
  } catch {
    ElMessage.error('批量启用失败，请重试')
  } finally {
    bulkLoading.value = false
  }
}

async function handleBulkPause() {
  const ids = strategies.value
    .filter(s => selectedIds.value.includes(s.id) && s.status === 'active')
    .map(s => s.id)
  if (ids.length === 0) return
  bulkLoading.value = true
  try {
    await bulkUpdateStatus(ids, 'paused')
    ElMessage.success(`已暂停 ${ids.length} 个策略`)
    clearSelection()
    await fetchStrategies()
  } catch {
    ElMessage.error('批量暂停失败，请重试')
  } finally {
    bulkLoading.value = false
  }
}

async function handleBulkStop() {
  const ids = strategies.value
    .filter(s => selectedIds.value.includes(s.id) && (s.status === 'active' || s.status === 'paused'))
    .map(s => s.id)
  if (ids.length === 0) return
  bulkLoading.value = true
  try {
    await bulkUpdateStatus(ids, 'stopped')
    ElMessage.success(`已停止 ${ids.length} 个策略`)
    clearSelection()
    await fetchStrategies()
  } catch {
    ElMessage.error('批量停止失败，请重试')
  } finally {
    bulkLoading.value = false
  }
}

function handleBulkDelete() {
  deleteTargets.value = strategies.value.filter(s => selectedIds.value.includes(s.id))
  deleteModalVisible.value = true
}

async function confirmDelete() {
  const ids = deleteTargets.value.map(r => r.id)
  deleteLoading.value = true
  try {
    await bulkDeleteStrategies(ids)
    ElMessage.success(`已删除 ${ids.length} 个策略`)
    deleteModalVisible.value = false
    clearSelection()
    await fetchStrategies()
  } catch {
    ElMessage.error('批量删除失败，请重试')
  } finally {
    deleteLoading.value = false
  }
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

    .header-actions {
      display: flex;
      align-items: center;
      gap: 12px;
    }

    .template-link {
      text-decoration: none;
    }

    .template-market-btn {
      display: flex;
      align-items: center;
      gap: 6px;
    }
  }

  .filter-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
    flex-wrap: wrap;
    gap: 12px;

    .filter-left {
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

    .filter-right {
      display: flex;
      align-items: center;
      gap: 12px;

      .search-input {
        width: 240px;
      }

      .sort-select {
        width: 160px;
      }
    }
  }

  .bulk-action-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: var(--color-surface-elevated);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    margin-bottom: 16px;

    .selected-count {
      font-size: 13px;
      color: var(--color-text-secondary);
      margin-right: 4px;
    }

    .danger-text {
      color: var(--color-error);
    }
  }

  .slide-down-enter-active,
  .slide-down-leave-active {
    /* no animation — static show/hide */
  }

  .slide-down-enter-from,
  .slide-down-leave-to {
    opacity: 0;
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

    .empty-sub {
      color: var(--color-text-tertiary);
      font-size: 13px;
      margin-bottom: 8px;
    }
  }

  .strategy-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
    margin-bottom: 16px;

    @media (max-width: 1200px) {
      grid-template-columns: repeat(2, 1fr);
    }

    @media (max-width: 768px) {
      grid-template-columns: 1fr;
    }
  }

  .strategy-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 8px;
    cursor: pointer;
    position: relative;
    /* no animation — static hover state */

    &:hover {
      border-color: var(--color-accent);
      transform: translateY(-2px);
    }

    &.is-selected {
      border-color: var(--color-accent);
      background: rgba(233, 130, 66, 0.04);
    }

    &.is-stopped {
      opacity: 0.7;
    }

    .card-header {
      margin-bottom: 12px;

      .card-title-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        margin-bottom: 4px;
      }
    }

    .strategy-name {
      font-size: 14px;
      font-weight: 510;
      color: var(--color-text-primary);
    }

    .strategy-desc {
      font-size: 12px;
      color: var(--color-text-tertiary);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .card-body {
      display: flex;
      flex-direction: column;
      gap: 8px;
      margin-bottom: 12px;
    }

    .card-info-row {
      display: flex;
      align-items: center;
      gap: 8px;

      .info-label {
        font-size: 12px;
        color: var(--color-text-tertiary);
        min-width: 40px;
      }

      .symbol-cell {
        font-size: 13px;
        color: var(--color-text-primary);
        font-family: var(--font-mono);
        letter-spacing: 0.5px;
      }

      .timeframe-cell {
        font-size: 12px;
        color: var(--color-text-secondary);
        font-family: var(--font-mono);
      }
    }

    .card-metrics-row {
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 8px 0 0;
      border-top: 1px solid var(--color-border);
      margin-top: 8px;

      .metric-item {
        flex: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 2px;
        padding: 4px 4px;
        border-radius: 4px;
        min-width: 0;

        .metric-label {
          font-size: 10px;
          color: var(--color-text-tertiary);
          white-space: nowrap;
        }

        .metric-value {
          font-size: 12px;
          font-family: var(--font-mono);
          color: var(--color-text-primary);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
          width: 100%;
          text-align: center;

          &.up {
            color: var(--color-success);
          }

          &.down {
            color: var(--color-error);
          }
        }
      }
    }

    .card-footer {
      display: flex;
      justify-content: space-between;
      align-items: center;

      .last-run-time {
        font-size: 11px;
        color: var(--color-text-tertiary);
      }

      .action-btn {
        width: 28px;
        height: 28px;
        border-radius: 6px;
        background: transparent;
        border: none;
        display: flex;
        align-items: center;
        justify-content: center;

        &:hover {
          background: rgba(255, 255, 255, 0.04);
        }
      }
    }

    .card-checkbox {
      position: absolute;
      top: 12px;
      left: 12px;
    }
  }

  .pagination-wrapper {
    display: flex;
    justify-content: center;
    padding: 16px 0 4px;
  }
}

:deep(.status-badge) {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 500;

  .status-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
}

.delete-targets-preview {
  display: block;
  margin-top: 8px;
  font-size: 13px;
  color: var(--color-text-secondary);
  word-break: break-all;
  line-height: 1.5;
}

.metric-value {
  font-size: 13px;
  font-family: var(--font-mono);
  color: var(--color-text-primary);

  &.up {
    color: var(--color-success);
  }

  &.down {
    color: var(--color-error);
  }
}
</style>
