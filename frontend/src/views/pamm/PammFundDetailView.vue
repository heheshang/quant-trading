<template>
  <div class="pamm-fund-detail-view">
    <div class="back-row">
      <el-button text :icon="ArrowLeft" @click="goBack">Back to list</el-button>
    </div>

    <!-- Loading -->
    <template v-if="loading && !fund">
      <div class="skeleton-header">
        <div class="skeleton-cell" style="width: 30%; height: 28px"></div>
        <div class="skeleton-cell" style="width: 50%; height: 16px; margin-top: 8px"></div>
      </div>
      <div class="kpi-row">
        <div v-for="n in 4" :key="n" class="kpi-card kpi-card--skeleton">
          <div class="skeleton-cell" style="height: 14px; width: 40%"></div>
          <div class="skeleton-cell" style="height: 22px; width: 60%; margin-top: 8px"></div>
        </div>
      </div>
    </template>

    <!-- Error -->
    <template v-else-if="error">
      <el-result
        icon="error"
        title="Failed to load fund"
        :sub-title="error"
      >
        <template #extra>
          <el-button type="primary" @click="load">Retry</el-button>
        </template>
      </el-result>
    </template>

    <!-- Body -->
    <template v-else-if="fund">
      <div class="header">
        <h1>{{ fund.name }}</h1>
        <p v-if="fund.description" class="fund-desc">{{ fund.description }}</p>
        <div class="header-meta">
          <el-tag size="small" effect="plain">{{ fund.base_currency }}</el-tag>
          <span class="muted">
            Manager: <strong>{{ fund.manager_username || shortId(fund.manager_id) }}</strong>
          </span>
          <el-tag
            :type="fund.status === 'Active' ? 'success' : 'info'"
            size="small"
          >
            {{ fund.status }}
          </el-tag>
        </div>
      </div>

      <!-- KPIs -->
      <div class="kpi-row">
        <div class="kpi-card">
          <div class="kpi-label">NAV</div>
          <div class="kpi-value">
            {{ formatNumber(fund.nav) }}
            <span class="kpi-unit">{{ fund.base_currency }}</span>
          </div>
        </div>
        <div class="kpi-card">
          <div class="kpi-label">AUM (proxy)</div>
          <div class="kpi-value">
            {{ formatNumber(fund.total_shares) }}
            <span class="kpi-unit">shares × {{ formatNumber(fund.share_value) }}</span>
          </div>
        </div>
        <div class="kpi-card">
          <div class="kpi-label">HWM</div>
          <div class="kpi-value">
            {{ formatNumber(fund.hwm) }}
            <span class="kpi-unit">{{ fund.base_currency }}</span>
          </div>
        </div>
        <div class="kpi-card">
          <div class="kpi-label">Total investors</div>
          <div class="kpi-value">{{ investors.length }}</div>
        </div>
      </div>

      <!-- Actions -->
      <div class="actions">
        <el-button type="primary" :icon="Plus" @click="openSubscribeDialog">
          Subscribe
        </el-button>
        <el-button
          v-if="hasMyInvestment"
          type="warning"
          plain
          :icon="Refresh"
          @click="openRedeemDialog"
        >
          Redeem
        </el-button>
        <el-button
          v-if="isManager"
          type="success"
          :icon="Money"
          @click="openDistributeDialog"
        >
          Distribute profits
        </el-button>
        <el-button
          v-if="isManager"
          type="danger"
          plain
          :icon="Delete"
          @click="confirmLiquidate"
        >
          Liquidate
        </el-button>
      </div>

      <!-- Investor list -->
      <div class="section">
        <h3>Investors</h3>
        <el-table
          v-loading="loadingInvestments"
          :data="investors"
          style="width: 100%"
          stripe
        >
          <el-table-column label="Investor" min-width="160">
            <template #default="{ row }">
              <span>{{ row.user_username || shortId(row.user_id) }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Share %" width="120" align="right">
            <template #default="{ row }">
              <span class="mono">{{ formatPct(row.share_pct) }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Current value" min-width="160" align="right">
            <template #default="{ row }">
              <span class="mono">{{ formatNumber(row.current_value) }}</span>
              <span class="kpi-unit">{{ fund.base_currency }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Initial" min-width="160" align="right">
            <template #default="{ row }">
              <span class="mono">{{ formatNumber(row.initial_investment) }}</span>
              <span class="kpi-unit">{{ fund.base_currency }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Status" width="100" align="center">
            <template #default="{ row }">
              <el-tag size="small" :type="row.status === 'Active' ? 'success' : 'info'">
                {{ row.status }}
              </el-tag>
            </template>
          </el-table-column>
        </el-table>
      </div>
    </template>

    <!-- Subscribe dialog -->
    <el-dialog
      v-model="subscribeDialogVisible"
      title="Subscribe to fund"
      width="420px"
      :close-on-click-modal="false"
    >
      <el-form :model="amountForm" label-position="top">
        <el-form-item
          :label="`Amount (${fund?.base_currency ?? ''})`"
          required
        >
          <el-input
            v-model="amountForm.amount"
            placeholder="e.g. 1000"
            type="number"
            :min="0"
          />
        </el-form-item>
        <p v-if="fund" class="dialog-hint">
          Share price is currently
          <strong>{{ formatNumber(fund.share_value) }} {{ fund.base_currency }}</strong>
          per share.
        </p>
      </el-form>
      <template #footer>
        <el-button @click="subscribeDialogVisible = false">Cancel</el-button>
        <el-button
          type="primary"
          :loading="actionLoading"
          @click="submitSubscribe"
        >
          Subscribe
        </el-button>
      </template>
    </el-dialog>

    <!-- Redeem dialog -->
    <el-dialog
      v-model="redeemDialogVisible"
      title="Redeem from fund"
      width="420px"
      :close-on-click-modal="false"
    >
      <el-form :model="amountForm" label-position="top">
        <el-form-item
          :label="`Amount to redeem (${fund?.base_currency ?? ''})`"
          required
        >
          <el-input
            v-model="amountForm.amount"
            placeholder="e.g. 500"
            type="number"
            :min="0"
          />
        </el-form-item>
        <p v-if="myInvestment" class="dialog-hint">
          Your current value is
          <strong>{{ formatNumber(myInvestment.current_value) }} {{ fund?.base_currency }}</strong>.
        </p>
      </el-form>
      <template #footer>
        <el-button @click="redeemDialogVisible = false">Cancel</el-button>
        <el-button
          type="warning"
          :loading="actionLoading"
          @click="submitRedeem"
        >
          Redeem
        </el-button>
      </template>
    </el-dialog>

    <!-- Distribute dialog (manager-only) -->
    <el-dialog
      v-model="distributeDialogVisible"
      title="Distribute profits"
      width="520px"
      :close-on-click-modal="false"
    >
      <p class="dialog-hint">
        Mgmt fee and perf-fee (on HWM-increment profit) are deducted
        first, the remainder is split by share %.
      </p>
      <el-form :model="distributeForm" label-position="top">
        <el-form-item label="Period start" required>
          <el-date-picker
            v-model="distributeForm.periodStart"
            type="datetime"
            value-format="YYYY-MM-DDTHH:mm:ss[Z]"
            style="width: 100%"
            placeholder="UTC start"
          />
        </el-form-item>
        <el-form-item label="Period end" required>
          <el-date-picker
            v-model="distributeForm.periodEnd"
            type="datetime"
            value-format="YYYY-MM-DDTHH:mm:ss[Z]"
            style="width: 100%"
            placeholder="UTC end"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="distributeDialogVisible = false">Cancel</el-button>
        <el-button
          type="success"
          :loading="actionLoading"
          @click="submitDistribute"
        >
          Run distribution
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, reactive } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  ArrowLeft,
  Plus,
  Refresh,
  Money,
  Delete,
} from '@element-plus/icons-vue'
import { usePammStore, type Investment } from '@/stores/pamm'
import { useAuthStore } from '@/stores/auth'

const route = useRoute()
const router = useRouter()
const pammStore = usePammStore()
const authStore = useAuthStore()

const fundId = computed(() => String(route.params.id ?? ''))
const fund = computed(() => pammStore.currentFund)
const investors = computed<Investment[]>(() => pammStore.currentInvestments)
const loading = computed(() => pammStore.loading)
const error = computed(() => pammStore.error)

const isManager = computed(() => {
  if (!fund.value || !authStore.userId) return false
  return fund.value.manager_id === authStore.userId
})

const myInvestment = computed<Investment | null>(() => {
  if (!authStore.userId) return null
  return (
    pammStore.myInvestments.find(
      (i) => i.fund_id === fundId.value && i.user_id === authStore.userId,
    ) ?? null
  )
})

const hasMyInvestment = computed(() => myInvestment.value !== null)

const loadingInvestments = ref(false)
const actionLoading = ref(false)

async function load(): Promise<void> {
  if (!fundId.value) return
  loadingInvestments.value = true
  try {
    await pammStore.getFund(fundId.value)
    // Load investor list (manager OR self-investor allowed by backend).
    await pammStore.loadFundInvestments(fundId.value).catch(() => {
      // Non-manager non-investor gets 403; that's fine — they just see
      // an empty investor list.
    })
    // Best-effort refresh of caller's own investments.
    if (authStore.isAuthenticated) {
      pammStore.loadMyInvestments().catch(() => {/* best effort */})
    }
  } finally {
    loadingInvestments.value = false
  }
}

function goBack(): void {
  router.push({ name: 'PammFundList' })
}

function shortId(id: string): string {
  return id.length > 8 ? id.slice(0, 8) + '…' : id
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

// ── Subscribe ─────────────────────────────────────────────────────────

const subscribeDialogVisible = ref(false)
const redeemDialogVisible = ref(false)
const distributeDialogVisible = ref(false)

const amountForm = reactive({ amount: '' })

function openSubscribeDialog(): void {
  amountForm.amount = ''
  subscribeDialogVisible.value = true
}

async function submitSubscribe(): Promise<void> {
  if (!fundId.value) return
  const n = Number(amountForm.amount)
  if (!Number.isFinite(n) || n <= 0) {
    ElMessage.warning('Enter a positive amount')
    return
  }
  actionLoading.value = true
  try {
    const res = await pammStore.subscribe(fundId.value, n)
    if (res) {
      ElMessage.success(
        `Subscribed — ${formatNumber(res.shares)} shares awarded`,
      )
      subscribeDialogVisible.value = false
      await load()
    } else if (pammStore.error) {
      ElMessage.error(pammStore.error)
    }
  } finally {
    actionLoading.value = false
  }
}

// ── Redeem ────────────────────────────────────────────────────────────

function openRedeemDialog(): void {
  amountForm.amount = ''
  redeemDialogVisible.value = true
}

async function submitRedeem(): Promise<void> {
  if (!fundId.value) return
  const n = Number(amountForm.amount)
  if (!Number.isFinite(n) || n <= 0) {
    ElMessage.warning('Enter a positive amount')
    return
  }
  actionLoading.value = true
  try {
    const res = await pammStore.redeem(fundId.value, n)
    if (res) {
      ElMessage.success(`Redeemed — ${formatNumber(res.amount_paid)} paid out`)
      redeemDialogVisible.value = false
      await load()
    } else if (pammStore.error) {
      ElMessage.error(pammStore.error)
    }
  } finally {
    actionLoading.value = false
  }
}

// ── Distribute (manager-only) ─────────────────────────────────────────

const distributeForm = reactive({
  periodStart: '',
  periodEnd: '',
})

function openDistributeDialog(): void {
  const now = new Date()
  const monthAgo = new Date(now.getTime() - 30 * 24 * 3600 * 1000)
  distributeForm.periodEnd = now.toISOString().replace(/\.\d{3}Z$/, 'Z')
  distributeForm.periodStart = monthAgo
    .toISOString()
    .replace(/\.\d{3}Z$/, 'Z')
  distributeDialogVisible.value = true
}

async function submitDistribute(): Promise<void> {
  if (!fundId.value) return
  if (!distributeForm.periodStart || !distributeForm.periodEnd) {
    ElMessage.warning('Pick both period boundaries')
    return
  }
  actionLoading.value = true
  try {
    const dists = await pammStore.distribute(
      fundId.value,
      distributeForm.periodStart,
      distributeForm.periodEnd,
    )
    if (dists) {
      ElMessage.success(`Distributed to ${dists.length} investors`)
      distributeDialogVisible.value = false
      await load()
    } else if (pammStore.error) {
      ElMessage.error(pammStore.error)
    }
  } finally {
    actionLoading.value = false
  }
}

// ── Liquidate (manager-only) ──────────────────────────────────────────

async function confirmLiquidate(): Promise<void> {
  if (!fundId.value || !fund.value) return
  try {
    await ElMessageBox.confirm(
      `Liquidate "${fund.value.name}"? This terminates the fund and returns capital to investors. This cannot be undone.`,
      'Confirm liquidation',
      { type: 'warning', confirmButtonText: 'Liquidate', cancelButtonText: 'Cancel' },
    )
  } catch {
    return
  }
  actionLoading.value = true
  try {
    const ok = await pammStore.liquidate(fundId.value)
    if (ok) {
      ElMessage.success('Fund liquidated')
      router.push({ name: 'PammFundList' })
    } else if (pammStore.error) {
      ElMessage.error(pammStore.error)
    }
  } finally {
    actionLoading.value = false
  }
}

onMounted(load)
</script>

<style scoped lang="scss">
.pamm-fund-detail-view {
  padding: 24px;
  max-width: 1280px;
  margin: 0 auto;
}

.back-row {
  margin-bottom: 12px;
}

.header h1 {
  margin: 0 0 4px 0;
  font-size: 24px;
  font-weight: 600;
}

.fund-desc {
  margin: 0 0 8px 0;
  color: var(--el-text-color-secondary);
  font-size: 14px;
}

.header-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.muted {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.kpi-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
  margin: 16px 0 20px;
}

.kpi-card {
  background: var(--el-fill-color-light);
  border-radius: 8px;
  padding: 16px;
  border: 1px solid var(--el-border-color-lighter);

  &--skeleton {
    height: 90px;
  }
}

.kpi-label {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.kpi-value {
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
  font-size: 20px;
  font-weight: 600;
  margin-top: 6px;
}

.kpi-unit {
  font-size: 11px;
  font-weight: 400;
  color: var(--el-text-color-secondary);
  margin-left: 4px;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 24px;
}

.section h3 {
  margin: 0 0 12px 0;
  font-size: 16px;
  font-weight: 600;
}

.mono {
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
  font-size: 13px;
}

.dialog-hint {
  margin: 0 0 12px 0;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.skeleton-header {
  margin-bottom: 16px;
}

.skeleton-cell {
  background: var(--el-fill-color);
  border-radius: 4px;
  animation: pulse 1.4s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
</style>
