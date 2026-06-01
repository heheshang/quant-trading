<template>
  <div class="admin-view">
    <div class="page-header">
      <h1>System Administration</h1>
    </div>

    <el-tabs v-model="activeTab" class="admin-tabs">
      <el-tab-pane label="Users" name="users">
        <!-- Search and Create -->
        <div class="toolbar">
          <el-input
            v-model="searchQuery"
            placeholder="Search username or email..."
            clearable
            class="search-input"
            :prefix-icon="Search"
          />
          <el-button type="primary" @click="showCreateDialog">
            <el-icon><Plus /></el-icon>
            Create User
          </el-button>
        </div>

        <!-- Loading State -->
        <template v-if="loading">
          <div class="skeleton-table">
            <div v-for="n in 5" :key="n" class="skeleton-row">
              <div class="skeleton-cell" style="width: 15%"></div>
              <div class="skeleton-cell" style="width: 25%"></div>
              <div class="skeleton-cell" style="width: 12%"></div>
              <div class="skeleton-cell" style="width: 10%"></div>
              <div class="skeleton-cell" style="width: 18%"></div>
              <div class="skeleton-cell" style="width: 20%"></div>
            </div>
          </div>
        </template>

        <!-- Error State -->
        <template v-else-if="error">
          <div class="error-section">
            <el-result icon="error" title="Failed to load users" sub-title="Something went wrong">
              <template #extra>
                <el-button type="primary" @click="loadUsers">Retry</el-button>
              </template>
            </el-result>
          </div>
        </template>

        <!-- Empty State -->
        <template v-else-if="filteredUsers.length === 0">
          <el-empty description="No users found" :image-size="80">
            <el-button type="primary" @click="showCreateDialog">
              <el-icon><Plus /></el-icon> Create User
            </el-button>
          </el-empty>
        </template>

        <!-- Table -->
        <template v-else>
          <el-table :data="filteredUsers" style="width: 100%" stripe @row-dblclick="editUser">
            <el-table-column prop="id" label="ID" width="60" align="center" />
            <el-table-column prop="username" label="Username" min-width="140" />
            <el-table-column prop="email" label="Email" min-width="200" />
            <el-table-column prop="role" label="Role" width="120">
              <template #default="{ row }">
                <el-tag
                  :type="roleTagType(row.role)"
                  size="small"
                  effect="dark"
                >
                  {{ roleLabel(row.role) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="Status" width="100">
              <template #default="{ row }">
                <el-switch
                  :model-value="row.status === 'active'"
                  :disabled="row.id === 1"
                  size="small"
                  @change="(val: boolean) => toggleStatus(row, val)"
                />
              </template>
            </el-table-column>
            <el-table-column prop="createdAt" label="Created At" width="140">
              <template #default="{ row }">{{ f.formatDate(row.createdAt) }}</template>
            </el-table-column>
            <el-table-column label="Actions" width="160" fixed="right">
              <template #default="{ row }">
                <el-button size="small" text @click="editUser(row)">Edit</el-button>
                <el-button
                  size="small"
                  text
                  type="danger"
                  :disabled="row.id === 1"
                  @click="confirmDelete(row)"
                >
                  Delete
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </template>
      </el-tab-pane>

      <!-- P0-F2: 风控规则 -->
      <el-tab-pane label="Risk Rules" name="risk">
        <div class="risk-panel">
          <!-- 连接状态卡片 -->
          <div class="status-cards">
            <el-card class="status-card" shadow="hover">
              <div class="status-indicator">
                <span class="dot" :class="connectionStatus.exchange_connected ? 'connected' : 'disconnected'"></span>
                <span class="status-label">{{ connectionStatus.exchange_connected ? 'Binance Connected' : 'Disconnected' }}</span>
              </div>
              <div class="status-detail" v-if="!connectionStatus.exchange_connected">
                Last heartbeat: {{ connectionStatus.disconnect_elapsed_secs }}s ago
              </div>
              <div class="status-detail" v-if="connectionStatus.strategy_paused" style="color: var(--el-color-danger)">
                ⚠ Strategy paused (auto)
              </div>
            </el-card>
          </div>

          <!-- 风控规则表单 -->
          <el-card class="rules-card">
            <template #header>
              <div class="card-header">
                <span>资金风控规则</span>
                <el-switch
                  v-model="riskForm.is_active"
                  active-text="启用"
                  inactive-text="停用"
                  @change="saveRiskRules"
                />
              </div>
            </template>

            <el-form label-width="160px" label-position="left" class="risk-form">
              <el-divider content-position="left">亏损限制</el-divider>
              <el-form-item label="当日亏损限额 (U)">
                <el-input-number
                  v-model="riskForm.daily_loss_limit"
                  :precision="2"
                  :step="100"
                  :min="0"
                  controls-position="right"
                  style="width: 200px"
                />
              </el-form-item>
              <el-form-item label="超限自动平仓">
                <el-switch v-model="riskForm.daily_loss_auto_close" />
              </el-form-item>

              <el-divider content-position="left">单笔交易</el-divider>
              <el-form-item label="单笔最大亏损比例">
                <el-input-number
                  v-model="riskForm.single_trade_loss_ratio"
                  :precision="4"
                  :step="0.001"
                  :min="0"
                  :max="1"
                  controls-position="right"
                  style="width: 200px"
                />
                <span class="form-hint">0.05 = 5%</span>
              </el-form-item>

              <el-divider content-position="left">回撤控制</el-divider>
              <el-form-item label="最大回撤比例">
                <el-input-number
                  v-model="riskForm.max_drawdown_ratio"
                  :precision="4"
                  :step="0.01"
                  :min="0"
                  :max="1"
                  controls-position="right"
                  style="width: 200px"
                />
                <span class="form-hint">0.2 = 20%</span>
              </el-form-item>
              <el-form-item label="超限自动平仓">
                <el-switch v-model="riskForm.drawdown_auto_close" />
              </el-form-item>

              <el-divider content-position="left">止损类型</el-divider>
              <el-form-item label="止损方式">
                <el-radio-group v-model="riskForm.stop_loss_type">
                  <el-radio label="fixed">固定止损</el-radio>
                  <el-radio label="atr">ATR 动态止损</el-radio>
                </el-radio-group>
              </el-form-item>
              <template v-if="riskForm.stop_loss_type === 'atr'">
                <el-form-item label="ATR 周期">
                  <el-input-number v-model="riskForm.atr_period" :min="1" :max="200" controls-position="right" style="width: 120px" />
                </el-form-item>
                <el-form-item label="ATR 倍数">
                  <el-input-number v-model="riskForm.atr_multiplier" :precision="2" :step="0.1" :min="0" controls-position="right" style="width: 120px" />
                </el-form-item>
              </template>

              <el-form-item>
                <el-button type="primary" :loading="riskSaving" @click="saveRiskRules">
                  保存规则
                </el-button>
                <el-button @click="loadRiskRules">重置</el-button>
              </el-form-item>
            </el-form>
          </el-card>

          <!-- 风控日志 -->
          <el-card class="logs-card">
            <template #header>
              <div class="card-header">
                <span>风控日志</span>
                <el-button size="small" @click="loadRiskLogs">
                  <el-icon><Refresh /></el-icon> 刷新
                </el-button>
              </div>
            </template>

            <el-table :data="riskLogs" stripe size="small" max-height="300">
              <el-table-column prop="triggered_rule" label="触发规则" width="140" />
              <el-table-column prop="severity" label="严重度" width="100">
                <template #default="{ row }">
                  <el-tag :type="severityType(row.severity)" size="small">{{ row.severity }}</el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="action" label="动作" width="100" />
              <el-table-column prop="details" label="详情" min-width="200" show-overflow-tooltip />
              <el-table-column prop="created_at" label="时间" width="160">
                <template #default="{ row }">{{ f.formatDate(row.created_at) }}</template>
              </el-table-column>
            </el-table>
            <div class="pagination" v-if="riskLogsTotal > pageSize">
              <el-pagination
                small
                layout="prev, pager, next"
                :total="riskLogsTotal"
                :page-size="pageSize"
                v-model:current-page="riskLogsPage"
                @current-change="loadRiskLogs"
              />
            </div>
          </el-card>
        </div>
      </el-tab-pane>

      <!-- P0-F3: 应急操作 -->
      <el-tab-pane label="Emergency" name="emergency">
        <div class="emergency-panel">
          <!-- 当前状态 -->
          <el-card class="emergency-status-card">
            <template #header>系统应急状态</template>
            <div class="emergency-status-grid">
              <div class="status-item">
                <span class="status-key">交易状态</span>
                <el-tag :type="tradingStatus === 'active' ? 'success' : 'danger'" size="large">
                  {{ tradingStatus === 'active' ? '正常交易' : '已暂停' }}
                </el-tag>
              </div>
              <div class="status-item">
                <span class="status-key">Binance 连接</span>
                <span class="status-val">
                  <span class="dot" :class="connectionStatus.exchange_connected ? 'connected' : 'disconnected'"></span>
                  {{ connectionStatus.exchange_connected ? '已连接' : '已断开' }}
                  <template v-if="!connectionStatus.exchange_connected">
                    ({{ connectionStatus.disconnect_elapsed_secs }}s)
                  </template>
                </span>
              </div>
              <div class="status-item">
                <span class="status-key">断线自动暂停</span>
                <span class="status-val">
                  <span class="dot" :class="connectionStatus.strategy_paused ? 'paused' : 'connected'"></span>
                  {{ connectionStatus.strategy_paused ? '已暂停' : '未触发' }}
                </span>
              </div>
            </div>
            <el-button size="small" @click="loadConnectionStatus" style="margin-top: 12px">
              <el-icon><Refresh /></el-icon> 刷新状态
            </el-button>
          </el-card>

          <!-- 手动操作 -->
          <el-card class="emergency-actions-card">
            <template #header>手动应急操作</template>
            <div class="action-buttons">
              <div class="action-item">
                <div class="action-info">
                  <h4>暂停交易</h4>
                  <p>立即禁止所有新订单开仓（已持仓不受影响）</p>
                </div>
                <el-button
                  type="warning"
                  :loading="emergencyLoading"
                  :disabled="tradingStatus !== 'active'"
                  @click="handlePause"
                >
                  暂停交易
                </el-button>
              </div>
              <el-divider />
              <div class="action-item">
                <div class="action-info">
                  <h4>恢复交易</h4>
                  <p>解除暂停，允许新订单开仓</p>
                </div>
                <el-button
                  type="success"
                  :loading="emergencyLoading"
                  :disabled="tradingStatus !== 'paused'"
                  @click="handleResume"
                >
                  恢复交易
                </el-button>
              </div>
              <el-divider />
              <div class="action-item danger-action">
                <div class="action-info">
                  <h4>紧急全平</h4>
                  <p>立即平掉所有持仓（不受风控规则限制）</p>
                </div>
                <el-button
                  type="danger"
                  :loading="emergencyLoading"
                  @click="handleEmergencyClose"
                >
                  紧急全平
                </el-button>
              </div>
            </div>
          </el-card>

          <!-- 手动风控检查 -->
          <el-card class="check-card">
            <template #header>手动风控检查</template>
            <div class="action-item">
              <div class="action-info">
                <h4>触发风控检查</h4>
                <p>立即对当前持仓和账户执行风控规则检查</p>
              </div>
              <el-button :loading="emergencyLoading" @click="handleRiskCheck">
                触发检查
              </el-button>
            </div>
          </el-card>
        </div>
      </el-tab-pane>
    </el-tabs>

    <!-- Create/Edit Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editingUser ? 'Edit User' : 'Create User'"
      width="420px"
      :close-on-click-modal="false"
    >
      <el-form
        ref="userFormRef"
        :model="userForm"
        :rules="userFormRules"
        label-position="top"
      >
        <el-form-item label="Username" prop="username">
          <el-input v-model="userForm.username" placeholder="Enter username" />
        </el-form-item>
        <el-form-item label="Email" prop="email">
          <el-input v-model="userForm.email" placeholder="Enter email" />
        </el-form-item>
        <el-form-item v-if="!editingUser" label="Password" prop="password">
          <el-input v-model="userForm.password" type="password" placeholder="Enter password" show-password />
        </el-form-item>
        <el-form-item label="Role" prop="role">
          <el-select v-model="userForm.role" style="width: 100%">
            <el-option label="Admin" value="admin" />
            <el-option label="Trader" value="trader" />
            <el-option label="Observer" value="observer" />
          </el-select>
        </el-form-item>
        <el-form-item label="Status" prop="status">
          <el-switch
            v-model="userForm.status"
            active-value="active"
            inactive-value="disabled"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">Cancel</el-button>
        <el-button type="primary" :loading="saving" @click="saveUser">
          {{ editingUser ? 'Save' : 'Create' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, reactive, watch } from 'vue'
import { useFormat } from '@/composables/useFormat'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus, Refresh } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { User } from '@/types'
import {
  getRiskRules,
  updateRiskRules,
  getRiskLogs,
  emergencyClose,
  pauseTrading,
  resumeTrading,
  manualRiskCheck,
  getConnectionStatus,
} from '@/api/risk'
import type {
  RiskRules,
  RiskLog,
  ConnectionStatus,
} from '@/types'

const f = useFormat()
const activeTab = ref('users')
const searchQuery = ref('')
const loading = ref(true)
const error = ref(false)
const saving = ref(false)
const dialogVisible = ref(false)
const editingUser = ref<User | null>(null)
const userFormRef = ref<FormInstance>()

const users = ref<User[]>([])
const nextId = ref(3)

// ─── P0-F2 / P0-F3: Risk & Emergency ──────────────────────────────────

const tradingStatus = ref<'active' | 'paused'>('active')
const emergencyLoading = ref(false)
const riskSaving = ref(false)

const connectionStatus = ref<ConnectionStatus>({
  exchange_connected: false,
  disconnect_elapsed_secs: 0,
  strategy_paused: false,
})

const riskRules = ref<RiskRules | null>(null)
const riskLogs = ref<RiskLog[]>([])
const riskLogsTotal = ref(0)
const riskLogsPage = ref(1)
const pageSize = 10

const riskForm = reactive({
  daily_loss_limit: 0,
  daily_loss_auto_close: false,
  single_trade_loss_ratio: 0.05,
  max_drawdown_ratio: 0.2,
  drawdown_auto_close: false,
  stop_loss_type: 'fixed' as 'fixed' | 'atr',
  atr_period: 14 as number | null,
  atr_multiplier: 2.0 as number | null,
  is_active: false,
})

function syncRiskForm(rules: RiskRules | null) {
  riskForm.daily_loss_limit = rules ? Number(rules.daily_loss_limit) : 0
  riskForm.daily_loss_auto_close = rules?.daily_loss_auto_close ?? false
  riskForm.single_trade_loss_ratio = rules ? Number(rules.single_trade_loss_ratio) : 0.05
  riskForm.max_drawdown_ratio = rules ? Number(rules.max_drawdown_ratio) : 0.2
  riskForm.drawdown_auto_close = rules?.drawdown_auto_close ?? false
  riskForm.stop_loss_type = (rules?.stop_loss_type as 'fixed' | 'atr') ?? 'fixed'
  riskForm.atr_period = rules?.atr_period ?? 14
  riskForm.atr_multiplier = rules?.atr_multiplier ? Number(rules.atr_multiplier) : 2.0
  riskForm.is_active = rules?.is_active ?? false
}

async function loadConnectionStatus() {
  try {
    const res = await getConnectionStatus()
    if (res.data) connectionStatus.value = res.data
  } catch {
    // ignore
  }
}

async function loadRiskRules() {
  try {
    const res = await getRiskRules()
    if (res.data) {
      riskRules.value = res.data
      syncRiskForm(res.data)
    }
  } catch {
    // ignore
  }
}

async function saveRiskRules() {
  riskSaving.value = true
  try {
    await updateRiskRules({
      daily_loss_limit: String(riskForm.daily_loss_limit),
      daily_loss_auto_close: riskForm.daily_loss_auto_close,
      single_trade_loss_ratio: String(riskForm.single_trade_loss_ratio),
      max_drawdown_ratio: String(riskForm.max_drawdown_ratio),
      drawdown_auto_close: riskForm.drawdown_auto_close,
      stop_loss_type: riskForm.stop_loss_type,
      atr_period: riskForm.atr_period,
      atr_multiplier: riskForm.atr_multiplier !== null ? String(riskForm.atr_multiplier) : null,
      is_active: riskForm.is_active,
    })
    ElMessage.success('风控规则已保存')
    await loadRiskRules()
  } catch (e: unknown) {
    ElMessage.error(`保存失败: ${e instanceof Error ? e.message : String(e)}`)
  } finally {
    riskSaving.value = false
  }
}

async function loadRiskLogs() {
  try {
    const res = await getRiskLogs({ page: riskLogsPage.value, page_size: pageSize })
    if (res.data) {
      riskLogs.value = res.data.data ?? []
      riskLogsTotal.value = res.data.total ?? 0
    }
  } catch {
    // ignore
  }
}

async function handlePause() {
  try {
    await pauseTrading()
    tradingStatus.value = 'paused'
    ElMessage.success('交易已暂停')
  } catch (e: unknown) {
    ElMessage.error(`暂停失败: ${e instanceof Error ? e.message : String(e)}`)
  }
}

async function handleResume() {
  try {
    await resumeTrading()
    tradingStatus.value = 'active'
    ElMessage.success('交易已恢复')
  } catch (e: unknown) {
    ElMessage.error(`恢复失败: ${e instanceof Error ? e.message : String(e)}`)
  }
}

async function handleEmergencyClose() {
  await ElMessageBox.confirm(
    '确认立即平掉所有持仓？此操作不可撤销。',
    '紧急全平确认',
    { confirmButtonText: '确认全平', cancelButtonText: '取消', type: 'warning' }
  )
  emergencyLoading.value = true
  try {
    await emergencyClose()
    ElMessage.success('已执行紧急全平')
    await loadConnectionStatus()
  } catch (e: unknown) {
    ElMessage.error(`全平失败: ${e instanceof Error ? e.message : String(e)}`)
  } finally {
    emergencyLoading.value = false
  }
}

async function handleRiskCheck() {
  emergencyLoading.value = true
  try {
    await manualRiskCheck()
    ElMessage.success('风控检查完成')
    await loadRiskLogs()
  } catch (e: unknown) {
    ElMessage.error(`检查失败: ${e instanceof Error ? e.message : String(e)}`)
  } finally {
    emergencyLoading.value = false
  }
}

function severityType(s: string): string {
  switch (s) {
    case 'critical': return 'danger'
    case 'high': return 'danger'
    case 'medium': return 'warning'
    case 'low': return 'info'
    default: return 'info'
  }
}

// 切换到 Risk/Emergency Tab 时加载数据
watch(activeTab, async (tab) => {
  if (tab === 'risk') {
    await Promise.all([loadRiskRules(), loadRiskLogs(), loadConnectionStatus()])
  } else if (tab === 'emergency') {
    await loadConnectionStatus()
  }
})

const userForm = reactive({
  username: '',
  email: '',
  password: '',
  role: 'trader' as User['role'],
  status: 'active' as User['status'],
})

const userFormRules: FormRules = {
  username: [
    { required: true, message: 'Username is required', trigger: 'blur' },
    { min: 3, max: 32, message: '3-32 characters', trigger: 'blur' },
  ],
  email: [
    { required: true, message: 'Email is required', trigger: 'blur' },
    { type: 'email', message: 'Invalid email', trigger: 'blur' },
  ],
  password: [
    { required: true, message: 'Password is required', trigger: 'blur' },
    { min: 6, message: 'At least 6 characters', trigger: 'blur' },
  ],
}

const filteredUsers = computed(() => {
  if (!searchQuery.value) return users.value
  const q = searchQuery.value.toLowerCase()
  return users.value.filter(
    (u) => u.username.toLowerCase().includes(q) || u.email.toLowerCase().includes(q)
  )
})

function roleTagType(role: string): string {
  switch (role) {
    case 'admin': return 'primary'
    case 'trader': return 'success'
    case 'observer': return 'warning'
    default: return 'info'
  }
}

function roleLabel(role: string): string {
  return role.charAt(0).toUpperCase() + role.slice(1)
}

function showCreateDialog() {
  editingUser.value = null
  userForm.username = ''
  userForm.email = ''
  userForm.password = ''
  userForm.role = 'trader'
  userForm.status = 'active'
  dialogVisible.value = true
}

function editUser(user: User) {
  editingUser.value = user
  userForm.username = user.username
  userForm.email = user.email
  userForm.password = ''
  userForm.role = user.role
  userForm.status = user.status
  dialogVisible.value = true
}

async function saveUser() {
  if (!userFormRef.value) return
  const valid = await userFormRef.value.validate().catch(() => false)
  if (!valid) return

  saving.value = true
  await new Promise((r) => setTimeout(r, 500))

  if (editingUser.value) {
    const idx = users.value.findIndex((u) => u.id === editingUser.value!.id)
    if (idx >= 0) {
      users.value[idx] = {
        ...users.value[idx],
        username: userForm.username,
        email: userForm.email,
        role: userForm.role,
        status: userForm.status,
      }
    }
    ElMessage.success('User updated')
  } else {
    users.value.push({
      id: nextId.value++,
      username: userForm.username,
      email: userForm.email,
      role: userForm.role,
      status: userForm.status,
      createdAt: new Date().toISOString(),
      lastLogin: null,
    })
    ElMessage.success('User created')
  }

  saving.value = false
  dialogVisible.value = false
}

function confirmDelete(user: User) {
  ElMessageBox.confirm(
    `Are you sure you want to delete user "${user.username}"? This action cannot be undone.`,
    'Confirm Delete',
    {
      confirmButtonText: 'Delete',
      cancelButtonText: 'Cancel',
      type: 'warning',
      buttonSize: 'small',
    }
  ).then(() => {
    users.value = users.value.filter((u) => u.id !== user.id)
    ElMessage.success('User deleted')
  }).catch(() => {
    // cancelled
  })
}

function toggleStatus(user: User, val: boolean) {
  const idx = users.value.findIndex((u) => u.id === user.id)
  if (idx >= 0) {
    users.value[idx].status = val ? 'active' : 'disabled'
    ElMessage.success(`User ${val ? 'enabled' : 'disabled'}`)
  }
}

function loadUsers() {
  loading.value = true
  error.value = false

  // Mock data
  setTimeout(() => {
    users.value = [
      {
        id: 1,
        username: 'admin',
        email: 'admin@quanttrade.io',
        role: 'admin',
        status: 'active',
        createdAt: '2025-06-01T00:00:00Z',
        lastLogin: '2026-05-12T08:30:00Z',
      },
      {
        id: 2,
        username: 'trader_jane',
        email: 'jane@quanttrade.io',
        role: 'trader',
        status: 'active',
        createdAt: '2025-08-15T00:00:00Z',
        lastLogin: '2026-05-11T16:45:00Z',
      },
      {
        id: 3,
        username: 'analyst_bob',
        email: 'bob@quanttrade.io',
        role: 'observer',
        status: 'active',
        createdAt: '2025-10-01T00:00:00Z',
        lastLogin: '2026-05-10T09:12:00Z',
      },
      {
        id: 4,
        username: 'algo_trader',
        email: 'algo@quanttrade.io',
        role: 'trader',
        status: 'disabled',
        createdAt: '2025-07-20T00:00:00Z',
        lastLogin: '2026-04-28T11:00:00Z',
      },
    ]
    nextId.value = 5
    loading.value = false
  }, 500)
}

onMounted(() => {
  loadUsers()
})
</script>

<style scoped lang="scss">
@import url('https://fonts.googleapis.com/css2?family=Outfit:wght@400;500;600;700&family=Work+Sans:wght@400;500;600&display=swap');

.admin-view {
  overflow: hidden;
}

.page-header {
  margin-bottom: 24px;

  h1 {
    font-size: 24px;
    font-weight: 600;
    font-family: var(--font-outfit, 'Outfit', sans-serif);
    color: var(--color-text-primary);
    margin: 0;
  }
}

.access-denied {
  display: flex;
  justify-content: center;
  padding: 60px 0;
}

.admin-tabs {
  :deep(.el-tabs__header) {
    margin-bottom: 20px;
    border-bottom: 1px solid var(--color-border);
  }
  :deep(.el-tabs__item) {
    color: var(--color-text-tertiary);
    font-size: 14px;
    font-family: var(--font-work-sans, 'Work Sans', sans-serif);
    &.is-active {
      color: var(--color-primary);
    }
  }
  :deep(.el-tabs__active-bar) {
    background: var(--color-primary);
  }
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.search-input {
  width: 240px;
}

// Skeleton
.skeleton-table {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 8px;
  padding: 16px;
}

.skeleton-row {
  display: flex;
  gap: 16px;
  padding: 12px 0;
  border-bottom: 1px solid var(--color-border);

  &:last-child { border-bottom: none; }
}

.skeleton-cell {
  height: 14px;
  border-radius: 4px;
  background: linear-gradient(90deg, var(--color-surface-elevated) 25%, rgba(255,255,255,0.03) 50%, var(--color-surface-elevated) 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s ease-in-out infinite;
}

.error-section {
  display: flex;
  justify-content: center;
  padding: 40px 0;
}

@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}

// ─── P0-F2 / P0-F3: Risk & Emergency ──────────────────────────────────

.risk-panel,
.emergency-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.status-cards {
  display: flex;
  gap: 12px;
  margin-bottom: 8px;
}

.status-card {
  flex: 0 0 auto;
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card, 0 2px 8px rgba(0,0,0,0.4));
  transition: box-shadow var(--transition-base);

  &:hover {
    box-shadow: var(--shadow-glow-primary);
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 500;
  }

  .status-detail {
    margin-top: 6px;
    font-size: 12px;
    color: var(--el-text-color-secondary);
  }
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;

  &.connected { background: var(--el-color-success); }
  &.disconnected { background: var(--el-color-danger); }
  &.paused { background: var(--el-color-warning); }
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.form-hint {
  margin-left: 12px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.pagination {
  display: flex;
  justify-content: center;
  margin-top: 12px;
}

.risk-form {
  :deep(.el-divider__text) {
    font-size: 13px;
    font-weight: 600;
    color: var(--el-text-color-primary);
  }
}

// ─── Emergency Panel ───────────────────────────────────────────

.el-card {
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-card, 0 2px 8px rgba(0,0,0,0.4));
  border: 1px solid var(--color-border);
  transition: box-shadow var(--transition-base), border-color var(--transition-base);

  &:hover {
    border-color: var(--color-border-hover);
    box-shadow: var(--shadow-glow-primary);
  }
}

.emergency-status-card {
  .emergency-status-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 16px;
  }

  .status-item {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .status-key {
    font-size: 12px;
    color: var(--el-text-color-secondary);
    font-weight: 500;
  }

  .status-val {
    font-size: 14px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
  }
}

.emergency-actions-card,
.check-card {
  .action-buttons {
    display: flex;
    flex-direction: column;
  }

  .action-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 0;

    &.danger-action .action-info h4 {
      color: var(--el-color-danger);
    }
  }

  .action-info {
    h4 {
      margin: 0 0 4px;
      font-size: 14px;
      font-weight: 600;
    }
    p {
      margin: 0;
      font-size: 12px;
      color: var(--el-text-color-secondary);
    }
  }
}
</style>
