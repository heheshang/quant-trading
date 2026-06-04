<template>
  <div class="pamm-fund-list-view">
    <div class="page-header">
      <h1>PAMM Funds</h1>
      <p class="page-subtitle">
        Browse active Percent Allocation Management Module funds. Click
        a fund to see KPIs, investor list, and subscribe / redeem.
      </p>
    </div>

    <div class="toolbar">
      <el-input
        v-model="filterText"
        placeholder="Filter by name or manager"
        clearable
        style="max-width: 360px"
        :prefix-icon="Search"
      />
      <el-button
        v-if="canCreate"
        type="primary"
        :icon="Plus"
        @click="openCreateDialog"
      >
        Open new fund
      </el-button>
    </div>

    <!-- Loading state -->
    <template v-if="loading && funds.length === 0">
      <div class="skeleton-table">
        <div v-for="n in 4" :key="n" class="skeleton-row">
          <div class="skeleton-cell" style="width: 25%"></div>
          <div class="skeleton-cell" style="width: 15%"></div>
          <div class="skeleton-cell" style="width: 15%"></div>
          <div class="skeleton-cell" style="width: 15%"></div>
          <div class="skeleton-cell" style="width: 15%"></div>
          <div class="skeleton-cell" style="width: 15%"></div>
        </div>
      </div>
    </template>

    <!-- Error state -->
    <template v-else-if="error">
      <el-result
        icon="error"
        title="Failed to load funds"
        :sub-title="error"
      >
        <template #extra>
          <el-button type="primary" @click="load">Retry</el-button>
        </template>
      </el-result>
    </template>

    <!-- Empty state -->
    <template v-else-if="filteredFunds.length === 0">
      <el-empty
        :description="
          filterText
            ? 'No funds match your filter'
            : 'No active funds yet — open one to get started'
        "
        :image-size="120"
      />
    </template>

    <!-- Table -->
    <template v-else>
      <el-table
        :data="filteredFunds"
        style="width: 100%"
        stripe
        @row-click="(row: Fund) => openDetail(row.id)"
      >
        <el-table-column prop="name" label="Name" min-width="180">
          <template #default="{ row }">
            <span class="fund-name">{{ row.name }}</span>
            <div v-if="row.description" class="fund-desc">
              {{ truncate(row.description, 60) }}
            </div>
          </template>
        </el-table-column>

        <el-table-column label="Manager" min-width="140">
          <template #default="{ row }">
            <span class="muted">{{
              row.manager_username || shortenId(row.manager_id)
            }}</span>
          </template>
        </el-table-column>

        <el-table-column prop="base_currency" label="Currency" width="100" align="center">
          <template #default="{ row }">
            <el-tag size="small" effect="plain">{{ row.base_currency }}</el-tag>
          </template>
        </el-table-column>

        <el-table-column label="NAV" min-width="140" align="right">
          <template #default="{ row }">
            <span class="nav-value">{{ formatNumber(row.nav) }}</span>
            <span class="nav-currency">{{ row.base_currency }}</span>
          </template>
        </el-table-column>

        <el-table-column label="Mgmt / Perf fee" width="160" align="center">
          <template #default="{ row }">
            <span class="fee">{{ formatPct(row.management_fee_pct) }}</span>
            <span class="fee-sep">/</span>
            <span class="fee">{{ formatPct(row.performance_fee_pct) }}</span>
            <el-tooltip
              v-if="row.high_water_mark"
              content="High-water mark enabled"
              placement="top"
            >
              <el-icon class="hwm-icon"><Histogram /></el-icon>
            </el-tooltip>
          </template>
        </el-table-column>

        <el-table-column label="Actions" width="160" fixed="right">
          <template #default="{ row }">
            <el-button
              size="small"
              type="primary"
              @click.stop="openDetail(row.id)"
            >
              View
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </template>

    <!-- Create-fund dialog (manager or anyone) -->
    <el-dialog
      v-model="createDialogVisible"
      title="Open a new PAMM fund"
      width="480px"
      :close-on-click-modal="false"
    >
      <el-form
        ref="createFormRef"
        :model="createForm"
        :rules="createRules"
        label-position="top"
      >
        <el-form-item label="Fund name" prop="name">
          <el-input v-model="createForm.name" placeholder="e.g. Alpha Momentum" />
        </el-form-item>
        <el-form-item label="Description">
          <el-input
            v-model="createForm.description"
            type="textarea"
            :rows="2"
            placeholder="Optional short description"
          />
        </el-form-item>
        <el-form-item label="Base currency" prop="base_currency">
          <el-select v-model="createForm.base_currency" style="width: 100%">
            <el-option label="USDT" value="USDT" />
            <el-option label="USDC" value="USDC" />
            <el-option label="BTC" value="BTC" />
            <el-option label="ETH" value="ETH" />
          </el-select>
        </el-form-item>
        <el-form-item label="Management fee (annualised)" prop="management_fee_pct">
          <el-input
            v-model="createForm.management_fee_pct"
            placeholder="0.02 (i.e. 2%)"
          />
        </el-form-item>
        <el-form-item label="Performance fee (on HWM profit)" prop="performance_fee_pct">
          <el-input
            v-model="createForm.performance_fee_pct"
            placeholder="0.20 (i.e. 20%)"
          />
        </el-form-item>
        <el-form-item label="High-water mark">
          <el-switch v-model="createForm.high_water_mark" />
          <span class="form-hint">
            If enabled, perf fee is charged only on profit above the prior peak.
          </span>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createDialogVisible = false">Cancel</el-button>
        <el-button
          type="primary"
          :loading="creating"
          @click="submitCreate"
        >
          Open fund
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Search, Histogram } from '@element-plus/icons-vue'
import { usePammStore, type Fund } from '@/stores/pamm'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const pammStore = usePammStore()
const authStore = useAuthStore()

// We treat any non-anonymous user as eligible to open a fund. The
// backend's service layer makes the caller the new fund's manager; no
// further role check is required in the UI.
const canCreate = computed(() => authStore.isAuthenticated)

const filterText = ref('')
const funds = computed(() => pammStore.funds)
const loading = computed(() => pammStore.loading)
const error = computed(() => pammStore.error)

const filteredFunds = computed(() => {
  const q = filterText.value.trim().toLowerCase()
  if (!q) return funds.value
  return funds.value.filter((f) => {
    const mgr = (f.manager_username ?? '').toLowerCase()
    return (
      f.name.toLowerCase().includes(q) ||
      mgr.includes(q) ||
      f.base_currency.toLowerCase().includes(q)
    )
  })
})

async function load(): Promise<void> {
  await pammStore.listFunds()
}

function openDetail(id: string): void {
  router.push({ name: 'PammFundDetail', params: { id } })
}

function shortenId(id: string): string {
  return id.length > 8 ? id.slice(0, 8) + '…' : id
}

function truncate(s: string, n: number): string {
  return s.length > n ? s.slice(0, n) + '…' : s
}

function formatNumber(s: string): string {
  const n = Number(s)
  if (!Number.isFinite(n)) return s
  return n.toLocaleString(undefined, { maximumFractionDigits: 4 })
}

function formatPct(s: string): string {
  const n = Number(s)
  if (!Number.isFinite(n)) return s
  return `${(n * 100).toFixed(2)}%`
}

// ── Create dialog ─────────────────────────────────────────────────────

const createDialogVisible = ref(false)
const createFormRef = ref<FormInstance | null>(null)
const creating = ref(false)

const createForm = reactive({
  name: '',
  description: '',
  base_currency: 'USDT',
  management_fee_pct: '0.02',
  performance_fee_pct: '0.20',
  high_water_mark: true,
})

const createRules: FormRules = {
  name: [
    { required: true, message: 'Name is required', trigger: 'blur' },
    { min: 2, max: 64, message: 'Length 2-64', trigger: 'blur' },
  ],
  base_currency: [
    { required: true, message: 'Currency is required', trigger: 'change' },
  ],
  management_fee_pct: [
    { required: true, message: 'Required', trigger: 'blur' },
    {
      validator: (_rule, value, cb) => {
        const n = Number(value)
        if (!Number.isFinite(n) || n < 0 || n > 1) {
          cb(new Error('Fraction in [0, 1] (e.g. 0.02 for 2%)'))
        } else cb()
      },
      trigger: 'blur',
    },
  ],
  performance_fee_pct: [
    { required: true, message: 'Required', trigger: 'blur' },
    {
      validator: (_rule, value, cb) => {
        const n = Number(value)
        if (!Number.isFinite(n) || n < 0 || n > 1) {
          cb(new Error('Fraction in [0, 1] (e.g. 0.20 for 20%)'))
        } else cb()
      },
      trigger: 'blur',
    },
  ],
}

function openCreateDialog(): void {
  createDialogVisible.value = true
}

async function submitCreate(): Promise<void> {
  if (!createFormRef.value) return
  try {
    await createFormRef.value.validate()
  } catch {
    return
  }
  creating.value = true
  try {
    const fund = await pammStore.createFund({
      name: createForm.name,
      description: createForm.description || undefined,
      base_currency: createForm.base_currency,
      management_fee_pct: createForm.management_fee_pct,
      performance_fee_pct: createForm.performance_fee_pct,
      high_water_mark: createForm.high_water_mark,
    })
    if (fund) {
      ElMessage.success(`Fund "${fund.name}" opened`)
      createDialogVisible.value = false
      router.push({ name: 'PammFundDetail', params: { id: fund.id } })
    } else if (pammStore.error) {
      ElMessage.error(pammStore.error)
    }
  } finally {
    creating.value = false
  }
}

onMounted(load)
</script>

<style scoped lang="scss">
.pamm-fund-list-view {
  padding: 24px;
  max-width: 1280px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: 16px;
}

.page-header h1 {
  margin: 0 0 8px 0;
  font-size: 24px;
  font-weight: 600;
}

.page-subtitle {
  margin: 0;
  color: var(--el-text-color-secondary);
  font-size: 14px;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 16px;
}

.fund-name {
  font-weight: 600;
  color: var(--el-color-primary);
  cursor: pointer;
}

.fund-desc {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-top: 2px;
}

.muted {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.nav-value {
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
  font-weight: 600;
  margin-right: 4px;
}

.nav-currency {
  font-size: 11px;
  color: var(--el-text-color-secondary);
}

.fee {
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
  font-size: 12px;
}

.fee-sep {
  margin: 0 4px;
  color: var(--el-text-color-secondary);
}

.hwm-icon {
  margin-left: 6px;
  color: var(--el-color-warning);
}

.form-hint {
  margin-left: 12px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.skeleton-table {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 0;
}

.skeleton-row {
  display: flex;
  gap: 12px;
  align-items: center;
}

.skeleton-cell {
  height: 24px;
  background: var(--el-fill-color);
  border-radius: 4px;
  animation: pulse 1.4s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
</style>
