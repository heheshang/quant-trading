<template>
  <div class="kline-list-view">
    <!-- Page header -->
    <div class="page-header">
      <h1 class="page-title">K线数据管理</h1>
      <div class="header-actions">
        <el-button type="primary" @click="router.push('/kline/import')">导入数据</el-button>
        <el-button @click="router.push('/kline/export')">导出数据</el-button>
      </div>
    </div>

    <!-- P0-02: Pills 筛选 — accent #7170ff, border-radius 16px, "" 默认选中 -->
    <div class="filter-bar">
      <el-radio-group v-model="filterForm.interval" size="large" class="interval-pills" @change="handleIntervalChange">
        <el-radio-button value="">全部</el-radio-button>
        <el-radio-button v-for="iv in KLINE_INTERVALS" :key="iv.value" :value="iv.value">
          {{ iv.label }}
        </el-radio-button>
      </el-radio-group>
    </div>

    <!-- 筛选表单: 交易对下拉 -->
    <div class="filter-form">
      <el-select v-model="filterForm.symbol" placeholder="筛选交易对" clearable @change="handleFilterChange">
        <el-option v-for="sym in allSymbols" :key="sym" :label="sym" :value="sym" />
      </el-select>
    </div>

    <!-- P0-01: 聚合表格 — 交易对|周期|数据点数|覆盖范围|最后更新|质量|操作 -->
    <div class="table-container">
      <el-table :data="paginatedData" v-loading="loading" stripe>
        <el-table-column label="交易对" prop="symbol" min-width="120">
          <template #default="{ row }">
            <span class="symbol-label">{{ row.symbol }}</span>
          </template>
        </el-table-column>
        <el-table-column label="周期" prop="interval" min-width="80">
          <template #default="{ row }">
            <span class="interval-tag">{{ row.interval }}</span>
          </template>
        </el-table-column>
        <el-table-column label="数据点数" prop="data_points" min-width="100" align="right">
          <template #default="{ row }">
            <span class="mono-number">{{ row.data_points.toLocaleString() }}</span>
          </template>
        </el-table-column>
        <el-table-column label="覆盖范围" min-width="200">
          <template #default="{ row }">
            {{ formatCoverage(row.coverage_start, row.coverage_end) }}
          </template>
        </el-table-column>
        <el-table-column label="最后更新" prop="last_updated" min-width="160">
          <template #default="{ row }">
            {{ formatDate(row.last_updated) }}
          </template>
        </el-table-column>
        <el-table-column label="质量" prop="quality" min-width="100">
          <template #default="{ row }">
            <span class="quality-dot" :class="`quality-${row.quality}`">
              <span class="quality-dot-inner" />
              {{ qualityLabel(row.quality) }}
            </span>
          </template>
        </el-table-column>
        <!-- P0-03: 操作下拉菜单 ⋮ -->
        <el-table-column label="操作" width="60" fixed="right">
          <template #default="{ row }">
            <el-dropdown trigger="click" @command="handleCommand($event, row)">
              <span class="action-trigger">
                <el-icon><MoreFilled /></el-icon>
              </span>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="detail">
                    <el-icon><Document /></el-icon>查看详情
                  </el-dropdown-item>
                  <el-dropdown-item command="preview">
                    <el-icon><DataLine /></el-icon>预览图表
                  </el-dropdown-item>
                  <el-dropdown-item command="edit">
                    <el-icon><Edit /></el-icon>编辑标签
                  </el-dropdown-item>
                  <el-dropdown-item command="delete" divided>
                    <el-icon><Delete /></el-icon>删除数据
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <!-- Pagination -->
    <div class="pagination-bar">
      <el-pagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="filteredData.length"
        layout="total, prev, pager, next"
      />
    </div>

    <!-- 图表预览弹窗 -->
    <el-dialog v-model="chartDialogVisible" title="图表预览" width="80%">
      <div v-if="chartSymbol" style="text-align:center; padding: 40px;">
        <p>交易对: <strong>{{ chartSymbol }}</strong></p>
        <p>周期: <strong>{{ chartInterval }}</strong></p>
      </div>
    </el-dialog>

    <!-- 标签编辑弹窗 -->
    <el-dialog v-model="tagDialogVisible" title="编辑标签" width="500px">
      <el-form v-if="tagRow" :model="tagForm" label-width="80px">
        <el-form-item label="交易对">
          <span>{{ tagRow.symbol }}</span>
        </el-form-item>
        <el-form-item label="来源">
          <el-select v-model="tagForm.source">
            <el-option value="exchange" label="交易所" />
            <el-option value="api" label="API" />
            <el-option value="csv" label="CSV" />
            <el-option value="manual" label="手动" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="tagDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveTag">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { MoreFilled, Document, DataLine, Edit, Delete } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getKlineSymbols } from '@/api/kline'
import { KLINE_INTERVALS } from '@/types/kline'
import type { KlineSymbolOverview } from '@/types/kline'

const router = useRouter()
const loading = ref(false)
const symbols = ref<KlineSymbolOverview[]>([])

// 筛选相关
const filterForm = ref({ symbol: '', interval: '' })
const pagination = ref({ page: 1, pageSize: 20 })

// 弹窗状态
const chartDialogVisible = ref(false)
const chartSymbol = ref('')
const chartInterval = ref('')
const tagDialogVisible = ref(false)
const tagRow = ref<KlineSymbolOverview | null>(null)
const tagForm = ref({ source: '' })

// 全部数据（用于筛选/删除后更新）
const allData = ref<KlineSymbolOverview[]>([])

// 所有交易对列表（去重）
const allSymbols = computed(() => [...new Set(allData.value.map(s => s.symbol))].sort())

// P0-01: 获取聚合数据
async function fetchSymbols() {
  loading.value = true
  try {
    const data = await getKlineSymbols()
    allData.value = data
    symbols.value = data
  } catch (err) {
    ElMessage.error('加载 K 线数据失败')
  } finally {
    loading.value = false
  }
}

// P0-02: 按周期筛选（保留 symbol 筛选在 filteredData 中）
const filteredData = computed(() => {
  let result = allData.value
  if (filterForm.value.interval) {
    result = result.filter(s => s.interval === filterForm.value.interval)
  }
  if (filterForm.value.symbol) {
    result = result.filter(s => s.symbol === filterForm.value.symbol)
  }
  return result
})

// 分页数据
const paginatedData = computed(() => {
  const start = (pagination.value.page - 1) * pagination.value.pageSize
  return filteredData.value.slice(start, start + pagination.value.pageSize)
})

// 筛选变化处理
function handleFilterChange() {
  pagination.value.page = 1
}

// Interval pill 切换处理（el-radio-button 在 jsdom 中 v-model 更新不及时）
function handleIntervalChange() {
  pagination.value.page = 1
}

function formatCoverage(start: number, end: number): string {
  const s = new Date(start).toLocaleDateString('zh-CN')
  const e = new Date(end).toLocaleDateString('zh-CN')
  return `${s} ~ ${e}`
}

function formatDate(iso: string): string {
  if (!iso) return '-'
  return new Date(iso).toLocaleString('zh-CN')
}

function qualityTagType(quality: string): '' | 'success' | 'warning' | 'danger' | 'info' {
  const map: Record<string, '' | 'success' | 'warning' | 'danger' | 'info'> = {
    normal: 'success',
    missing: 'warning',
    anomaly: 'danger',
    duplicate: 'info',
    suspicious: 'warning',
  }
  return map[quality] ?? 'info'
}

function qualityLabel(quality: string): string {
  const map: Record<string, string> = {
    normal: '正常',
    missing: '缺失',
    anomaly: '异常',
    duplicate: '重复',
    suspicious: '可疑',
  }
  return map[quality] ?? quality
}

// P0-03: 操作下拉菜单
async function handleCommand(command: string, row: KlineSymbolOverview) {
  switch (command) {
    case 'detail':
      router.push(`/kline/${row.symbol}/${row.interval}`)
      break
    case 'preview':
      chartSymbol.value = row.symbol
      chartInterval.value = row.interval
      chartDialogVisible.value = true
      break
    case 'edit':
    case 'editTag':
      openTagDialog(row)
      break
    case 'delete':
      await confirmDelete(row)
      break
  }
}

// 确认删除（自己内部处理确认对话框）
async function confirmDelete(row: KlineSymbolOverview) {
  try {
    await ElMessageBox.confirm(
      `确定删除 ${row.symbol} ${row.interval} 的所有数据吗？此操作不可恢复。`,
      '删除确认',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' }
    )
  } catch {
    return // 用户取消
  }
  allData.value = allData.value.filter(
    r => !(r.symbol === row.symbol && r.interval === row.interval)
  )
  ElMessage.success('数据已删除')
}

// 打开标签编辑弹窗
function openTagDialog(row: KlineSymbolOverview) {
  tagRow.value = row
  tagForm.value = { source: row.source ?? '' }
  tagDialogVisible.value = true
}

// 保存标签
function saveTag() {
  if (!tagRow.value) return
  const idx = allData.value.findIndex(
    r => r.symbol === tagRow.value!.symbol && r.interval === tagRow.value!.interval
  )
  if (idx !== -1) {
    allData.value[idx] = { ...allData.value[idx], source: tagForm.value.source as any }
  }
  tagDialogVisible.value = false
  ElMessage.success('标签已保存')
}

onMounted(fetchSymbols)

// Expose internals for testing
defineExpose({
  filterForm,
  allData,
  filteredData,
  paginatedData,
  pagination,
  chartDialogVisible,
  chartSymbol,
  chartInterval,
  tagDialogVisible,
  tagRow,
  tagForm,
  handleCommand,
  confirmDelete,
  openTagDialog,
  saveTag,
  handleFilterChange,
  handleIntervalChange,
})
</script>

<style scoped>
.kline-list-view {
  padding: 24px;
}

/* Page header */
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}

.header-actions {
  display: flex;
  gap: 8px;
}

/* Filter bar */
.filter-bar {
  margin-bottom: 16px;
}

.filter-form {
  margin-bottom: 16px;
}

.filter-form .el-select {
  width: 200px;
}

/* Pills 样式: accent #7170ff, 圆角 16px */
.interval-pills :deep(.el-radio-button__inner) {
  border-radius: 16px;
  border-left: 1px solid var(--el-border-color);
  margin-right: 8px;
  background: transparent;
  color: var(--el-text-color-regular);
  transition: all 0.2s;
}

.interval-pills :deep(.el-radio-button__original-radio:checked + .el-radio-button__inner) {
  background-color: #7170ff;
  border-color: #7170ff;
  color: #fff;
  box-shadow: none;
}

.interval-pills :deep(.el-radio-button:first-child .el-radio-button__inner) {
  border-radius: 16px;
}

.interval-pills :deep(.el-radio-button:last-child .el-radio-button__inner) {
  border-radius: 16px;
}

.table-container {
  background: var(--el-bg-color);
  border-radius: 8px;
  padding: 16px;
}

.interval-tag {
  font-family: monospace;
  font-weight: 600;
}

/* P0-01: symbol-label */
.symbol-label {
  font-weight: 600;
  color: var(--el-text-color-primary);
}

/* P0-01: mono-number */
.mono-number {
  font-family: monospace;
  font-size: 13px;
}

/* P0-01: quality-dot */
.quality-dot {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}

.quality-dot-inner {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.quality-normal .quality-dot-inner { background: #67c23a; }
.quality-anomaly .quality-dot-inner { background: #f56c6c; }
.quality-missing .quality-dot-inner { background: #e6a23c; }
.quality-duplicate .quality-dot-inner { background: #909399; }
.quality-suspicious .quality-dot-inner { background: #e6a23c; }

.quality-normal { color: #67c23a; }
.quality-anomaly { color: #f56c6c; }
.quality-missing { color: #e6a23c; }
.quality-duplicate { color: #909399; }
.quality-suspicious { color: #e6a23c; }

/* Action trigger */
.action-trigger {
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 4px;
  transition: background 0.2s;
}

.action-trigger:hover {
  background: var(--el-fill-color-light);
}

:deep(.el-dropdown-menu__item) {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Pagination */
.pagination-bar {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
