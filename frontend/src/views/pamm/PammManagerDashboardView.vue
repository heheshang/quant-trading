<template>
  <div class="pamm-manager-dashboard-view">
    <div class="page-header">
      <h1>PAMM Manager Dashboard</h1>
      <p class="page-subtitle">
        Funds you manage, with one-click access to the investor list
        and the profit-distribution flow.
      </p>
    </div>

    <div v-if="loading" class="loading">
      <el-icon class="is-loading"><Loading /></el-icon>
      Loading...
    </div>

    <el-result
      v-else-if="error"
      icon="error"
      title="Failed to load"
      :sub-title="error"
    >
      <template #extra>
        <el-button type="primary" @click="load">Retry</el-button>
      </template>
    </el-result>

    <el-empty
      v-else-if="myFunds.length === 0"
      description="You haven't opened any funds yet"
      :image-size="120"
    >
      <el-button type="primary" @click="openCreate">Open a fund</el-button>
    </el-empty>

    <template v-else>
      <div class="kpi-row">
        <div class="kpi-card">
          <div class="kpi-label">Funds managed</div>
          <div class="kpi-value">{{ myFunds.length }}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-label">Total NAV</div>
          <div class="kpi-value">{{ formatNumber(totalNav) }}</div>
        </div>
        <div class="kpi-card">
          <div class="kpi-label">Active funds</div>
          <div class="kpi-value">{{ activeCount }}</div>
        </div>
      </div>

      <h3>My funds</h3>
      <el-table :data="myFunds" style="width: 100%" stripe>
        <el-table-column prop="name" label="Name" min-width="200">
          <template #default="{ row }">
            <span class="fund-name" @click="goDetail(row.id)">
              {{ row.name }}
            </span>
          </template>
        </el-table-column>

        <el-table-column prop="base_currency" label="Currency" width="100" align="center">
          <template #default="{ row }">
            <el-tag size="small" effect="plain">{{ row.base_currency }}</el-tag>
          </template>
        </el-table-column>

        <el-table-column label="NAV" min-width="140" align="right">
          <template #default="{ row }">
            <span class="mono">{{ formatNumber(row.nav) }}</span>
            <span class="kpi-unit">{{ row.base_currency }}</span>
          </template>
        </el-table-column>

        <el-table-column label="Status" width="100" align="center">
          <template #default="{ row }">
            <el-tag
              size="small"
              :type="row.status === 'Active' ? 'success' : 'info'"
            >
              {{ row.status }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column label="Actions" width="260" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="goDetail(row.id)">View</el-button>
            <el-button
              size="small"
              type="success"
              :disabled="row.status !== 'Active'"
              @click="goDistribute(row.id)"
            >
              Distribute
            </el-button>
            <el-button
              size="small"
              type="danger"
              plain
              :disabled="row.status !== 'Active'"
              @click="goDetail(row.id, 'liquidate')"
            >
              Liquidate
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { Loading } from '@element-plus/icons-vue'
import { usePammStore } from '@/stores/pamm'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const pammStore = usePammStore()
const authStore = useAuthStore()

const loading = computed(() => pammStore.loading)
const error = computed(() => pammStore.error)

/** Funds the current user manages. */
const myFunds = computed(() => {
  if (!authStore.userId) return []
  return pammStore.funds.filter((f) => f.manager_id === authStore.userId)
})

const totalNav = computed(() =>
  myFunds.value.reduce((acc, f) => acc + (Number(f.nav) || 0), 0),
)

const activeCount = computed(
  () => myFunds.value.filter((f) => f.status === 'Active').length,
)

async function load(): Promise<void> {
  // Always load the full active-funds list — we filter client-side.
  // (There's no `?manager=me` filter exposed by the API yet.)
  await pammStore.listFunds()
}

function goDetail(id: string, _tab?: string): void {
  // Future enhancement: honor a `tab` query to auto-open the
  // distribute / liquidate dialog. For now we just deep-link.
  router.push({ name: 'PammFundDetail', params: { id } })
}

function goDistribute(id: string): void {
  // Same destination; the detail view surfaces the action.
  router.push({ name: 'PammFundDetail', params: { id } })
}

function openCreate(): void {
  router.push({ name: 'PammFundList' })
}

function formatNumber(s: string | number): string {
  const n = Number(s)
  if (!Number.isFinite(n)) return String(s)
  return n.toLocaleString(undefined, { maximumFractionDigits: 4 })
}

onMounted(load)
</script>

<style scoped lang="scss">
.pamm-manager-dashboard-view {
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

.loading {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 32px 0;
  color: var(--el-text-color-secondary);
  font-size: 14px;
}

.kpi-row {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin: 12px 0 24px;
}

.kpi-card {
  background: var(--el-fill-color-light);
  border-radius: 8px;
  padding: 16px;
  border: 1px solid var(--el-border-color-lighter);
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

.fund-name {
  font-weight: 600;
  color: var(--el-color-primary);
  cursor: pointer;
}

.mono {
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
  font-size: 13px;
}

.kpi-unit {
  font-size: 11px;
  color: var(--el-text-color-secondary);
  margin-left: 4px;
}

h3 {
  margin: 0 0 12px 0;
  font-size: 16px;
  font-weight: 600;
}
</style>
