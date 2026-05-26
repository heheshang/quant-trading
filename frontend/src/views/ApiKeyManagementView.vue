<template>
  <div class="api-key-management-view">
    <!-- Page Header -->
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">API 密钥管理</h1>
      </div>
      <div class="header-right">
        <el-button type="primary" :icon="Plus" @click="openCreateDialog">
          添加 API Key
        </el-button>
      </div>
    </div>

    <!-- API Keys Table -->
    <div class="table-container">
      <el-table
        v-loading="loading"
        :data="apiKeys"
        stripe
        style="width: 100%"
        empty-text="暂无 API 密钥，点击上方按钮添加"
      >
        <el-table-column prop="exchange" label="交易所" width="120">
          <template #default="{ row }">
            <div class="exchange-cell">
              <span class="exchange-badge">{{ EXCHANGE_INFO[row.exchange as Exchange]?.icon || row.exchange.toUpperCase() }}</span>
              <span>{{ EXCHANGE_INFO[row.exchange as Exchange]?.label || row.exchange }}</span>
            </div>
          </template>
        </el-table-column>

        <el-table-column prop="api_key" label="Key 标识" min-width="180">
          <template #default="{ row }">
            <span class="masked-key">{{ row.api_key }}</span>
          </template>
        </el-table-column>

        <el-table-column prop="permissions" label="权限" min-width="160">
          <template #default="{ row }">
            <div class="permissions-cell">
              <el-tag
                v-for="perm in row.permissions"
                :key="perm"
                size="small"
                :type="getPermissionTagType(perm)"
                class="permission-tag"
              >
                {{ PERMISSION_INFO[perm as ApiKeyPermission]?.label || perm }}
              </el-tag>
            </div>
          </template>
        </el-table-column>

        <el-table-column prop="is_active" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="row.is_active ? 'success' : 'info'" size="small">
              {{ row.is_active ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column prop="last_used_at" label="最后使用" width="180">
          <template #default="{ row }">
            <span class="time-text">{{ formatTime(row.last_used_at) }}</span>
          </template>
        </el-table-column>

        <el-table-column prop="created_at" label="创建时间" width="180">
          <template #default="{ row }">
            <span class="time-text">{{ formatTime(row.created_at) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <div class="action-buttons">
              <el-button
                type="primary"
                link
                size="small"
                :icon="Connection"
                :loading="testingId === row.id"
                @click="testConnection(row)"
              >
                测试
              </el-button>
              <el-button
                type="primary"
                link
                size="small"
                :icon="Edit"
                @click="openEditDialog(row)"
              >
                编辑
              </el-button>
              <el-button
                type="danger"
                link
                size="small"
                :icon="Delete"
                @click="confirmDelete(row)"
              >
                删除
              </el-button>
            </div>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <!-- Create/Edit Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEditing ? '编辑 API Key' : '添加 API Key'"
      width="500px"
      :close-on-click-modal="false"
      @close="resetForm"
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="formRules"
        label-width="100px"
      >
        <el-form-item label="交易所" prop="exchange">
          <el-select v-model="form.exchange" placeholder="请选择交易所" style="width: 100%">
            <el-option
              v-for="(info, key) in EXCHANGE_INFO"
              :key="key"
              :label="info.label"
              :value="key"
            />
          </el-select>
        </el-form-item>

        <el-form-item label="API Key" prop="api_key">
          <el-input
            v-model="form.api_key"
            :placeholder="isEditing ? '留空则不修改' : '请输入 API Key'"
            clearable
          />
        </el-form-item>

        <el-form-item label="Secret Key" prop="secret_key">
          <el-input
            v-model="form.secret_key"
            type="password"
            :placeholder="isEditing ? '留空则不修改' : '请输入 Secret Key'"
            show-password
            clearable
          />
        </el-form-item>

        <el-form-item label="权限" prop="permissions">
          <el-checkbox-group v-model="form.permissions">
            <el-checkbox value="read">读取 (Read)</el-checkbox>
            <el-checkbox value="trade">交易 (Trade)</el-checkbox>
            <el-checkbox value="withdraw">提现 (Withdraw)</el-checkbox>
          </el-checkbox-group>
        </el-form-item>

        <el-form-item label="状态" prop="is_active">
          <el-switch v-model="form.is_active" />
          <span class="status-hint">{{ form.is_active ? '启用' : '禁用' }}</span>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmit">
          {{ isEditing ? '保存' : '创建' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- Test Result Dialog -->
    <el-dialog
      v-model="testDialogVisible"
      title="连通性测试结果"
      width="400px"
    >
      <div class="test-result">
        <el-result
          :icon="testResult?.success ? 'success' : 'error'"
          :title="testResult?.success ? '连接成功' : '连接失败'"
        >
          <template #sub-title>
            <p>{{ testResult?.message }}</p>
            <p v-if="testResult?.server_time" class="server-time">
              服务器时间: {{ testResult.server_time }}
            </p>
          </template>
        </el-result>
      </div>
      <template #footer>
        <el-button type="primary" @click="testDialogVisible = false">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { Plus, Edit, Delete, Connection } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import {
  getApiKeys,
  createApiKey,
  updateApiKey,
  deleteApiKey,
  testApiKey,
} from '@/api/apiKey'
import type {
  ApiKey,
  Exchange,
  ApiKeyPermission,
  ApiKeyTestResult,
} from '@/types/apiKey'
import {
  EXCHANGE_INFO,
  PERMISSION_INFO,
} from '@/types/apiKey'

// ===== State =====
const loading = ref(false)
const submitting = ref(false)
const testingId = ref<string | null>(null)
const apiKeys = ref<ApiKey[]>([])
const dialogVisible = ref(false)
const testDialogVisible = ref(false)
const isEditing = ref(false)
const editingId = ref<string | null>(null)
const testResult = ref<ApiKeyTestResult | null>(null)
const formRef = ref<FormInstance>()

// Form state
const form = reactive({
  exchange: '' as Exchange | '',
  api_key: '',
  secret_key: '',
  permissions: [] as ApiKeyPermission[],
  is_active: true,
})

// Form validation rules
const formRules: FormRules = {
  exchange: [{ required: true, message: '请选择交易所', trigger: 'change' }],
  api_key: [
    {
      required: true,
      message: '请输入 API Key',
      trigger: 'blur',
      validator: (_rule, _value, callback) => {
        if (!isEditing.value && !form.api_key) {
          callback(new Error('请输入 API Key'))
        } else {
          callback()
        }
      },
    },
  ],
  secret_key: [
    {
      required: true,
      message: '请输入 Secret Key',
      trigger: 'blur',
      validator: (_rule, _value, callback) => {
        if (!isEditing.value && !form.secret_key) {
          callback(new Error('请输入 Secret Key'))
        } else {
          callback()
        }
      },
    },
  ],
  permissions: [
    {
      required: true,
      message: '请至少选择一个权限',
      trigger: 'change',
      validator: (_rule, _value, callback) => {
        if (form.permissions.length === 0) {
          callback(new Error('请至少选择一个权限'))
        } else {
          callback()
        }
      },
    },
  ],
}

// ===== Methods =====

async function loadApiKeys() {
  loading.value = true
  try {
    apiKeys.value = await getApiKeys()
  } catch (err: any) {
    ElMessage.error(err.message || '加载 API 密钥失败')
    apiKeys.value = []
  } finally {
    loading.value = false
  }
}

function openCreateDialog() {
  isEditing.value = false
  editingId.value = null
  resetForm()
  dialogVisible.value = true
}

function openEditDialog(row: ApiKey) {
  isEditing.value = true
  editingId.value = row.id
  form.exchange = row.exchange
  form.api_key = ''  // Don't pre-fill for security
  form.secret_key = ''
  form.permissions = [...row.permissions]
  form.is_active = row.is_active
  dialogVisible.value = true
}

function resetForm() {
  form.exchange = ''
  form.api_key = ''
  form.secret_key = ''
  form.permissions = []
  form.is_active = true
  formRef.value?.resetFields()
}

async function handleSubmit() {
  if (!formRef.value) return

  try {
    await formRef.value.validate()
  } catch {
    return
  }

  submitting.value = true
  try {
    if (isEditing.value && editingId.value) {
      // Build update payload (only include key fields if changed)
      const updateData: any = {
        exchange: form.exchange,
        permissions: form.permissions,
        is_active: form.is_active,
      }
      if (form.api_key) updateData.api_key = form.api_key
      if (form.secret_key) updateData.secret_key = form.secret_key

      await updateApiKey(editingId.value, updateData)
      ElMessage.success('API Key 更新成功')
    } else {
      await createApiKey({
        exchange: form.exchange as Exchange,
        api_key: form.api_key,
        secret_key: form.secret_key,
        permissions: form.permissions,
        is_active: form.is_active,
      })
      ElMessage.success('API Key 创建成功')
    }
    dialogVisible.value = false
    await loadApiKeys()
  } catch (err: any) {
    ElMessage.error(err.message || (isEditing.value ? '更新失败' : '创建失败'))
  } finally {
    submitting.value = false
  }
}

async function testConnection(row: ApiKey) {
  testingId.value = row.id
  testResult.value = null
  try {
    testResult.value = await testApiKey(row.id)
    testDialogVisible.value = true
  } catch (err: any) {
    testResult.value = {
      success: false,
      message: err.message || '测试失败，请检查 API Key 是否正确',
    }
    testDialogVisible.value = true
  } finally {
    testingId.value = null
  }
}

function confirmDelete(row: ApiKey) {
  ElMessageBox.confirm(
    `确定要删除该 API Key 吗？此操作不可恢复。`,
    '删除确认',
    {
      confirmButtonText: '删除',
      cancelButtonText: '取消',
      type: 'warning',
    }
  )
    .then(async () => {
      try {
        await deleteApiKey(row.id)
        ElMessage.success('删除成功')
        await loadApiKeys()
      } catch (err: any) {
        ElMessage.error(err.message || '删除失败')
      }
    })
    .catch(() => {
      // User cancelled
    })
}

function getPermissionTagType(perm: ApiKeyPermission): '' | 'success' | 'warning' | 'danger' | 'info' | 'primary' {
  switch (perm) {
    case 'read': return 'info'
    case 'trade': return 'success'
    case 'withdraw': return 'danger'
    default: return 'info'
  }
}

function formatTime(time: string | null): string {
  if (!time) return '-'
  const date = new Date(time)
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

// ===== Lifecycle =====
onMounted(() => {
  loadApiKeys()
})
</script>

<style scoped lang="scss">
.api-key-management-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 0 16px 16px;
  overflow-y: auto;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 56px;
  padding: 0;
  border-bottom: 1px solid var(--color-border, rgba(255, 255, 255, 0.08));
  margin-bottom: 16px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.page-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--color-text-primary, #f7f8f8);
  margin: 0;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 12px;
}

.table-container {
  flex: 1;
  background: var(--color-card-bg, #1c1e26);
  border-radius: 8px;
  padding: 16px;
}

.exchange-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.exchange-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  background: var(--color-accent, #409eff);
  color: #fff;
  font-size: 10px;
  font-weight: 700;
  border-radius: 6px;
}

.masked-key {
  font-family: 'JetBrains Mono', 'Fira Code', monospace;
  color: var(--color-text-secondary, #a1a5b5);
}

.permissions-cell {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.permission-tag {
  margin-right: 0;
}

.time-text {
  color: var(--color-text-secondary, #a1a5b5);
  font-size: 13px;
}

.action-buttons {
  display: flex;
  gap: 8px;
}

.status-hint {
  margin-left: 12px;
  color: var(--color-text-secondary, #a1a5b5);
  font-size: 13px;
}

.test-result {
  padding: 16px 0;
}

.server-time {
  color: var(--color-text-secondary, #a1a5b5);
  font-size: 13px;
  margin-top: 8px;
}
</style>
