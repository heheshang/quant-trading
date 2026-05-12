<template>
  <div class="admin-view">
    <div class="page-header">
      <h1>System Administration</h1>
    </div>

    <!-- Admin-only check -->
    <div v-if="!authStore.isAdmin" class="access-denied">
      <el-result icon="warning" title="Access Denied" sub-title="You do not have permission to view this page">
        <template #extra>
          <el-button type="primary" @click="$router.push('/dashboard')">Back to Dashboard</el-button>
        </template>
      </el-result>
    </div>

    <template v-else>
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
      </el-tabs>
    </template>

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
import { ref, computed, onMounted, reactive } from 'vue'
import { useAuthStore } from '@/stores/auth'
import { useFormat } from '@/composables/useFormat'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { User } from '@/types'

const authStore = useAuthStore()
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
.admin-view {
  max-width: 1344px;
}

.page-header {
  margin-bottom: 24px;

  h1 {
    font-size: 24px;
    font-weight: 600;
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
    &.is-active {
      color: var(--color-accent);
    }
  }
  :deep(.el-tabs__active-bar) {
    background: var(--color-accent);
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
</style>
