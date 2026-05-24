<template>
  <div class="arbitrage-view">
    <!-- Page Header -->
    <div class="page-header">
      <h1 class="page-title">套利管理</h1>
      <div class="header-actions">
        <el-button type="primary" @click="openCreateDialog">
          <el-icon><Plus /></el-icon>
          创建套利对
        </el-button>
      </div>
    </div>

    <!-- Loading State -->
    <template v-if="loading && pairs.length === 0">
      <el-skeleton :rows="6" animated class="skeleton-table" />
    </template>

    <!-- Error State -->
    <template v-else-if="error">
      <el-card class="state-card" shadow="never">
        <el-result icon="error" title="加载失败" sub-title="套利数据获取失败，请重试">
          <template #extra>
            <el-button type="primary" @click="fetchAll">重新加载</el-button>
          </template>
        </el-result>
      </el-card>
    </template>

    <!-- Content -->
    <template v-else>
      <!-- Summary Cards -->
      <div class="summary-cards">
        <el-card class="summary-card" shadow="never">
          <div class="summary-item">
            <span class="summary-label">套利对总数</span>
            <span class="summary-value">{{ pairs.length }}</span>
          </div>
        </el-card>
        <el-card class="summary-card" shadow="never">
          <div class="summary-item">
            <span class="summary-label">当前持仓</span>
            <span class="summary-value">{{ positions.length }}</span>
          </div>
        </el-card>
        <el-card class="summary-card" shadow="never">
          <div class="summary-item">
            <span class="summary-label">今日信号</span>
            <span class="summary-value">{{ todaySignals }}</span>
          </div>
        </el-card>
        <el-card class="summary-card" shadow="never">
          <div class="summary-item">
            <span class="summary-label">总未实现盈亏</span>
            <span class="summary-value" :class="totalPnl >= 0 ? 'is-positive' : 'is-negative'">
              {{ formatPnl(totalPnl) }}
            </span>
          </div>
        </el-card>
      </div>

      <!-- Filter Bar -->
      <div class="filter-bar">
        <div class="filter-left">
          <el-radio-group v-model="filterStatus" @change="onFilterChange" class="filter-pills">
            <el-radio-button value="" class="filter-pill">全部</el-radio-button>
            <el-radio-button value="active" class="filter-pill">活跃</el-radio-button>
            <el-radio-button value="inactive" class="filter-pill">已停用</el-radio-button>
          </el-radio-group>
        </div>
        <div class="filter-right">
          <el-input
            v-model="searchQuery"
            placeholder="搜索交易对..."
            :prefix-icon="Search"
            clearable
            class="search-input"
            @clear="onSearchClear"
            @keyup.enter="onSearchChange"
          />
        </div>
      </div>

      <!-- Empty State -->
      <template v-if="filteredPairs.length === 0">
        <el-card class="state-card" shadow="never">
          <el-empty description="暂无套利对" :image-size="80">
            <template #image>
              <el-icon :size="48" color="var(--color-text-tertiary)"><Connection /></el-icon>
            </template>
            <el-button type="primary" @click="openCreateDialog">创建第一个套利对</el-button>
          </el-empty>
        </el-card>
      </template>

      <!-- Pairs Grid -->
      <div v-else class="pairs-grid">
        <el-card
          v-for="pair in filteredPairs"
          :key="pair.id"
          class="pair-card"
          shadow="never"
        >
          <!-- Card Header -->
          <div class="card-header">
            <div class="pair-title">
              <span class="pair-symbols">{{ pair.symbolA }} / {{ pair.symbolB }}</span>
              <el-tag
                class="pair-type-tag"
                :type="getPairTypeTagType(pair.pairType)"
                size="small"
                effect="plain"
              >
                {{ getPairTypeLabel(pair.pairType) }}
              </el-tag>
            </div>
            <div class="pair-status">
              <span class="status-badge" :class="`status--${pair.status}`">
                {{ pair.status === 'active' ? '活跃' : '停用' }}
              </span>
            </div>
          </div>

          <!-- Card Body: Spread Thresholds -->
          <div class="card-body">
            <div class="info-row">
              <span class="info-label">交易所</span>
              <span class="info-value">{{ pair.exchange }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">入场价差阈值</span>
              <span class="info-value">{{ formatThreshold(pair.spreadEntryThreshold) }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">出场价差阈值</span>
              <span class="info-value">{{ formatThreshold(pair.spreadExitThreshold) }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">计算模式</span>
              <span class="info-value">{{ getCalcModeLabel(pair.calculationMode) }}</span>
            </div>
            <div v-if="pair.zScoreEntry" class="info-row">
              <span class="info-label">Z-Score 入场</span>
              <span class="info-value">{{ pair.zScoreEntry }}</span>
            </div>
            <div v-if="pair.correlationThreshold" class="info-row">
              <span class="info-label">相关性阈值</span>
              <span class="info-value">{{ pair.correlationThreshold }}</span>
            </div>
            <div class="info-row">
              <span class="info-label">最大仓位</span>
              <span class="info-value">{{ formatMoney(pair.maxPositionSize) }}</span>
            </div>
          </div>

          <!-- Card Footer -->
          <div class="card-footer">
            <span class="update-time">更新: {{ formatDate(pair.updatedAt) }}</span>
            <el-dropdown trigger="click" @command="(cmd: string) => handleActionCommand(cmd, pair)">
              <el-button class="action-btn" text>
                <el-icon><MoreFilled /></el-icon>
              </el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="edit">
                    <el-icon><Edit /></el-icon>编辑
                  </el-dropdown-item>
                  <el-dropdown-item v-if="pair.status === 'active'" command="disable">
                    <el-icon><VideoPause /></el-icon>停用
                  </el-dropdown-item>
                  <el-dropdown-item v-else command="enable">
                    <el-icon><VideoPlay /></el-icon>启用
                  </el-dropdown-item>
                  <el-dropdown-item command="delete" divided>
                    <el-icon><Delete /></el-icon><span class="danger-text">删除</span>
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </el-card>
      </div>

      <!-- Positions Section -->
      <div v-if="positions.length > 0" class="section">
        <div class="section-header">
          <h2 class="section-title">当前持仓</h2>
        </div>
        <el-table :data="positions" stripe class="positions-table" empty-text="暂无持仓">
          <el-table-column label="交易对" prop="pairId" width="100" />
          <el-table-column label="方向" width="140">
            <template #default="{ row }">
              <el-tag :type="row.direction === 'long_spread' ? 'success' : 'danger'" size="small">
                {{ row.direction === 'long_spread' ? '做多价差' : '做空价差' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="数量 (A/B)" width="160">
            <template #default="{ row }">
              {{ formatQty(row.sizeA) }} / {{ formatQty(row.sizeB) }}
            </template>
          </el-table-column>
          <el-table-column label="入场价差" prop="entrySpread" width="120">
            <template #default="{ row }">
              {{ formatPnl(row.entrySpread) }}
            </template>
          </el-table-column>
          <el-table-column label="当前价差" prop="currentSpread" width="120">
            <template #default="{ row }">
              {{ formatPnl(row.currentSpread ?? 0) }}
            </template>
          </el-table-column>
          <el-table-column label="未实现盈亏" width="130">
            <template #default="{ row }">
              <span :class="(row.unrealizedPnl ?? 0) >= 0 ? 'is-positive' : 'is-negative'">
                {{ formatPnl(row.unrealizedPnl ?? 0) }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <span class="status-badge" :class="`status--${row.status}`">
                {{ getPositionStatusLabel(row.status) }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="开仓时间" prop="openedAt" min-width="160">
            <template #default="{ row }">
              {{ formatDate(row.openedAt) }}
            </template>
          </el-table-column>
        </el-table>
      </div>

      <!-- Signals Section -->
      <div v-if="signals.length > 0" class="section">
        <div class="section-header">
          <h2 class="section-title">最近信号</h2>
          <el-button size="small" text @click="fetchSignals">刷新</el-button>
        </div>
        <el-table :data="signals" stripe class="signals-table" empty-text="暂无信号">
          <el-table-column label="信号类型" width="130">
            <template #default="{ row }">
              <el-tag :type="getSignalTagType(row.signalType)" size="small">
                {{ getSignalLabel(row.signalType) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="交易对" prop="pairId" width="100" />
          <el-table-column label="价差" prop="spread" width="120">
            <template #default="{ row }">
              {{ formatPnl(row.spread) }}
            </template>
          </el-table-column>
          <el-table-column label="Z-Score" prop="zScore" width="100">
            <template #default="{ row }">
              {{ row.zScore ?? '—' }}
            </template>
          </el-table-column>
          <el-table-column label="置信度" prop="confidence" width="100">
            <template #default="{ row }">
              {{ row.confidence != null ? `${(row.confidence * 100).toFixed(1)}%` : '—' }}
            </template>
          </el-table-column>
          <el-table-column label="是否执行" width="100">
            <template #default="{ row }">
              <el-tag :type="row.executed ? 'success' : 'info'" size="small">
                {{ row.executed ? '已执行' : '未执行' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="时间" prop="createdAt" min-width="160">
            <template #default="{ row }">
              {{ formatDate(row.createdAt) }}
            </template>
          </el-table-column>
        </el-table>
      </div>
    </template>

    <!-- Create / Edit Pair Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editingPair ? '编辑套利对' : '创建套利对'"
      width="520px"
      destroy-on-close
    >
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="130" class="pair-form">
        <el-form-item label="套利类型" prop="pairType">
          <el-select v-model="form.pairType" class="full-width">
            <el-option value="calendar_spread" label="日历价差 (Calendar Spread)" />
            <el-option value="cross_pair" label="跨品种价差 (Cross Pair)" />
            <el-option value="spot_futures" label="现货期货价差 (Spot-Futures)" />
          </el-select>
        </el-form-item>
        <el-form-item label="交易品种 A" prop="symbolA">
          <el-input v-model="form.symbolA" placeholder="如 BTCUSDT" />
        </el-form-item>
        <el-form-item label="交易品种 B" prop="symbolB">
          <el-input v-model="form.symbolB" placeholder="如 ETHUSDT" />
        </el-form-item>
        <el-form-item label="交易所" prop="exchange">
          <el-select v-model="form.exchange" class="full-width">
            <el-option v-for="(info, key) in EXCHANGE_INFO" :key="key" :value="key" :label="info.label" />
          </el-select>
        </el-form-item>
        <el-form-item label="计算模式" prop="calculationMode">
          <el-select v-model="form.calculationMode" class="full-width">
            <el-option value="percentage" label="百分比 (Percentage)" />
            <el-option value="ratio" label="比率 (Ratio)" />
            <el-option value="zscore" label="Z-Score" />
          </el-select>
        </el-form-item>
        <el-form-item label="入场价差阈值" prop="spreadEntryThreshold">
          <el-input-number v-model="form.spreadEntryThreshold" :min="0" :precision="6" class="full-width" />
        </el-form-item>
        <el-form-item label="出场价差阈值" prop="spreadExitThreshold">
          <el-input-number v-model="form.spreadExitThreshold" :min="0" :precision="6" class="full-width" />
        </el-form-item>
        <el-form-item label="最大仓位" prop="maxPositionSize">
          <el-input-number v-model="form.maxPositionSize" :min="0" :precision="4" class="full-width" />
        </el-form-item>
        <el-form-item v-if="form.calculationMode === 'zscore'" label="Z-Score 入场">
          <el-input-number v-model="form.zScoreEntry" :step="0.1" :precision="4" class="full-width" />
        </el-form-item>
        <el-form-item v-if="form.calculationMode === 'zscore'" label="Z-Score 出场">
          <el-input-number v-model="form.zScoreExit" :step="0.1" :precision="4" class="full-width" />
        </el-form-item>
        <el-form-item v-if="form.pairType === 'cross_pair'" label="相关性阈值">
          <el-input-number v-model="form.correlationThreshold" :min="0" :max="1" :precision="4" class="full-width" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="dialogLoading" @click="submitForm">
          {{ editingPair ? '保存' : '创建' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- Delete Confirm Dialog -->
    <el-dialog v-model="deleteDialogVisible" title="确认删除" width="400px" destroy-on-close>
      <p>
        确定要删除套利对「<strong>{{ deleteTargetPair?.symbolA }} / {{ deleteTargetPair?.symbolB }}</strong>」吗？<br />
        此操作不可撤销。
      </p>
      <template #footer>
        <el-button @click="deleteDialogVisible = false">取消</el-button>
        <el-button type="danger" :loading="deleteLoading" @click="confirmDelete">确认删除</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Plus, Search, MoreFilled, Edit, Delete,
  VideoPlay, VideoPause, Connection,
} from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listArbitragePairs,
  createArbitragePair,
  updateArbitragePair,
  deleteArbitragePair,
  listArbitragePositions,
  listArbitrageSignals,
  type ArbitragePair,
  type ArbitragePosition,
  type ArbitrageSignal,
} from '@/api/arbitrage'
import { EXCHANGE_INFO } from '@/types/apiKey'

// ============================================================
// Refs
// ============================================================
const loading = ref(false)
const error = ref(false)
const pairs = ref<ArbitragePair[]>([])
const positions = ref<ArbitragePosition[]>([])
const signals = ref<ArbitrageSignal[]>([])
const filterStatus = ref('')
const searchQuery = ref('')

// Dialog state
const dialogVisible = ref(false)
const dialogLoading = ref(false)
const editingPair = ref<ArbitragePair | null>(null)
const formRef = ref<FormInstance>()

// Delete dialog state
const deleteDialogVisible = ref(false)
const deleteTargetPair = ref<ArbitragePair | null>(null)
const deleteLoading = ref(false)

// Form
const form = ref({
  pairType: 'cross_pair',
  symbolA: '',
  symbolB: '',
  exchange: 'binance',
  calculationMode: 'percentage',
  spreadEntryThreshold: 0.02,
  spreadExitThreshold: 0.005,
  maxPositionSize: 10000,
  zScoreEntry: 2,
  zScoreExit: 0.5,
  correlationThreshold: 0.9,
})

const formRules: FormRules = {
  pairType: [{ required: true, message: '请选择套利类型', trigger: 'change' }],
  symbolA: [{ required: true, message: '请输入交易品种 A', trigger: 'blur' }],
  symbolB: [{ required: true, message: '请输入交易品种 B', trigger: 'blur' }],
  exchange: [{ required: true, message: '请输入交易所', trigger: 'blur' }],
  spreadEntryThreshold: [{ required: true, message: '请输入入场阈值', trigger: 'blur' }],
  spreadExitThreshold: [{ required: true, message: '请输入出场阈值', trigger: 'blur' }],
  maxPositionSize: [{ required: true, message: '请输入最大仓位', trigger: 'blur' }],
}

// ============================================================
// Computed
// ============================================================
const filteredPairs = computed(() => {
  return pairs.value.filter((p) => {
    const matchStatus = !filterStatus.value || p.status === filterStatus.value
    const q = searchQuery.value.toLowerCase()
    const matchSearch = !q ||
      p.symbolA.toLowerCase().includes(q) ||
      p.symbolB.toLowerCase().includes(q) ||
      p.pairType.toLowerCase().includes(q)
    return matchStatus && matchSearch
  })
})

const todaySignals = computed(() => {
  const today = new Date().toDateString()
  return signals.value.filter((s) => new Date(s.createdAt).toDateString() === today).length
})

const totalPnl = computed(() => {
  return positions.value.reduce((sum, p) => sum + (p.unrealizedPnl ?? 0), 0)
})

// ============================================================
// Methods
// ============================================================
async function fetchAll() {
  loading.value = true
  error.value = false
  try {
    await Promise.all([fetchPairs(), fetchPositions(), fetchSignals()])
  } catch {
    error.value = true
  } finally {
    loading.value = false
  }
}

async function fetchPairs() {
  pairs.value = await listArbitragePairs()
}

async function fetchPositions() {
  positions.value = await listArbitragePositions()
}

async function fetchSignals() {
  signals.value = await listArbitrageSignals(undefined, 50)
}

function onFilterChange() {
  // filter is applied via computed, no re-fetch needed
}

function onSearchChange() {
  // filter is applied via computed
}

function onSearchClear() {
  searchQuery.value = ''
}

function openCreateDialog() {
  editingPair.value = null
  form.value = {
    pairType: 'cross_pair',
    symbolA: '',
    symbolB: '',
    exchange: 'binance',
    calculationMode: 'percentage',
    spreadEntryThreshold: 0.02,
    spreadExitThreshold: 0.005,
    maxPositionSize: 10000,
    zScoreEntry: 2,
    zScoreExit: 0.5,
    correlationThreshold: 0.9,
  }
  dialogVisible.value = true
}

function openEditDialog(pair: ArbitragePair) {
  editingPair.value = pair
  form.value = {
    pairType: pair.pairType,
    symbolA: pair.symbolA,
    symbolB: pair.symbolB,
    exchange: pair.exchange,
    calculationMode: pair.calculationMode,
    spreadEntryThreshold: pair.spreadEntryThreshold,
    spreadExitThreshold: pair.spreadExitThreshold,
    maxPositionSize: pair.maxPositionSize,
    zScoreEntry: pair.zScoreEntry ?? 2,
    zScoreExit: pair.zScoreExit ?? 0.5,
    correlationThreshold: pair.correlationThreshold ?? 0.9,
  }
  dialogVisible.value = true
}

async function submitForm() {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    dialogLoading.value = true
    try {
      const payload = {
        pair_type: form.value.pairType,
        symbol_a: form.value.symbolA,
        symbol_b: form.value.symbolB,
        exchange: form.value.exchange,
        spread_entry_threshold: form.value.spreadEntryThreshold,
        spread_exit_threshold: form.value.spreadExitThreshold,
        max_position_size: form.value.maxPositionSize,
        calculation_mode: form.value.calculationMode,
        z_score_entry: form.value.calculationMode === 'zscore' ? form.value.zScoreEntry : undefined,
        z_score_exit: form.value.calculationMode === 'zscore' ? form.value.zScoreExit : undefined,
        correlation_threshold: form.value.pairType === 'cross_pair' ? form.value.correlationThreshold : undefined,
      }
      if (editingPair.value) {
        await updateArbitragePair(editingPair.value.id, payload)
        ElMessage.success('更新成功')
      } else {
        await createArbitragePair(payload)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      await fetchPairs()
    } catch (err: any) {
      ElMessage.error(err?.message || '操作失败')
    } finally {
      dialogLoading.value = false
    }
  })
}

function handleActionCommand(cmd: string, pair: ArbitragePair) {
  if (cmd === 'edit') openEditDialog(pair)
  else if (cmd === 'delete') openDeleteDialog(pair)
  else if (cmd === 'enable') toggleStatus(pair, 'active')
  else if (cmd === 'disable') toggleStatus(pair, 'inactive')
}

async function toggleStatus(pair: ArbitragePair, status: string) {
  try {
    await updateArbitragePair(pair.id, { status })
    ElMessage.success(status === 'active' ? '已启用' : '已停用')
    await fetchPairs()
  } catch (err: any) {
    ElMessage.error(err?.message || '操作失败')
  }
}

function openDeleteDialog(pair: ArbitragePair) {
  deleteTargetPair.value = pair
  deleteDialogVisible.value = true
}

async function confirmDelete() {
  if (!deleteTargetPair.value) return
  deleteLoading.value = true
  try {
    await deleteArbitragePair(deleteTargetPair.value.id)
    ElMessage.success('删除成功')
    deleteDialogVisible.value = false
    await fetchPairs()
  } catch (err: any) {
    ElMessage.error(err?.message || '删除失败')
  } finally {
    deleteLoading.value = false
  }
}

// ============================================================
// Formatters
// ============================================================
function formatDate(dateStr: string): string {
  if (!dateStr) return '—'
  return new Date(dateStr).toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' })
}

function formatPnl(v: number): string {
  if (v == null) return '—'
  return `${v >= 0 ? '+' : ''}${v.toFixed(4)}`
}

function formatMoney(v: number): string {
  if (v == null) return '—'
  return v.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 4 })
}

function formatQty(v: number): string {
  if (v == null) return '—'
  return v.toFixed(4)
}

function formatThreshold(v: number): string {
  return `${(v * 100).toFixed(4)}%`
}

function getPairTypeLabel(t: string): string {
  const map: Record<string, string> = {
    calendar_spread: '日历价差',
    cross_pair: '跨品种',
    spot_futures: '现货期货',
  }
  return map[t] ?? t
}

function getPairTypeTagType(t: string): string {
  const map: Record<string, string> = {
    calendar_spread: 'primary',
    cross_pair: 'success',
    spot_futures: 'warning',
  }
  return map[t] ?? 'info'
}

function getCalcModeLabel(m: string): string {
  const map: Record<string, string> = {
    percentage: '百分比',
    ratio: '比率',
    zscore: 'Z-Score',
  }
  return map[m] ?? m
}

function getSignalLabel(t: string): string {
  const map: Record<string, string> = {
    entry_long: '入场(做多)',
    entry_short: '入场(做空)',
    exit: '出场',
    stop_loss: '止损',
  }
  return map[t] ?? t
}

function getSignalTagType(t: string): string {
  const map: Record<string, string> = {
    entry_long: 'success',
    entry_short: 'danger',
    exit: 'info',
    stop_loss: 'warning',
  }
  return map[t] ?? 'info'
}

function getPositionStatusLabel(s: string): string {
  const map: Record<string, string> = {
    open: '开仓中',
    closed: '已平仓',
    liquidated: '已清算',
  }
  return map[s] ?? s
}

// ============================================================
// Lifecycle
// ============================================================
onMounted(fetchAll)
</script>

<style scoped>
.arbitrage-view {
  padding: 24px;
  max-width: 1400px;
  margin: 0 auto;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 24px;
}

.page-title {
  font-size: 24px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 12px;
}

.skeleton-table {
  margin-top: 16px;
}

.state-card {
  margin-top: 16px;
}

/* Summary Cards */
.summary-cards {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
  margin-bottom: 24px;
}

.summary-card {
  border-radius: 8px;
}

.summary-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.summary-label {
  font-size: 13px;
  color: var(--color-text-secondary);
}

.summary-value {
  font-size: 24px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.summary-value.is-positive {
  color: var(--color-success);
}

.summary-value.is-negative {
  color: var(--color-error);
}

/* Filter Bar */
.filter-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
  gap: 16px;
}

.filter-pills {
  display: flex;
  gap: 8px;
}

.search-input {
  width: 280px;
}

/* Pairs Grid */
.pairs-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
  margin-bottom: 32px;
}

.pair-card {
  border-radius: 8px;
  cursor: default;
}

.card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 12px;
}

.pair-title {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.pair-symbols {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
}

.pair-type-tag {
  align-self: flex-start;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 4px;
}

.status--active {
  background: rgba(34, 197, 94, 0.1);
  color: var(--color-success);
}

.status--inactive {
  background: rgba(100, 116, 139, 0.1);
  color: var(--color-text-tertiary);
}

.status--open {
  background: rgba(59, 130, 246, 0.1);
  color: #3b82f6;
}

.status--closed {
  background: rgba(100, 116, 139, 0.1);
  color: var(--color-text-tertiary);
}

.status--liquidated {
  background: rgba(239, 68, 68, 0.1);
  color: var(--color-error);
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}

.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 13px;
}

.info-label {
  color: var(--color-text-secondary);
}

.info-value {
  color: var(--color-text-primary);
  font-weight: 500;
}

.card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 12px;
  border-top: 1px solid var(--color-border);
}

.update-time {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.action-btn {
  padding: 4px;
}

/* Sections */
.section {
  margin-bottom: 32px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.positions-table,
.signals-table {
  border-radius: 8px;
}

/* Dialog form */
.pair-form .full-width {
  width: 100%;
}

.danger-text {
  color: var(--color-error);
}

.is-positive {
  color: var(--color-success);
}

.is-negative {
  color: var(--color-error);
}
</style>
