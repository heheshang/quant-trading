<template>
  <div class="pamm-my-investments-view">
    <div class="page-header">
      <h1>My PAMM Investments</h1>
      <p class="page-subtitle">
        Track the funds you've subscribed to, your current value, and
        cumulative P&L.
      </p>
    </div>

    <div v-if="loading" class="loading">
      <el-icon class="is-loading"><Loading /></el-icon>
      Loading...
    </div>

    <el-result
      v-else-if="error"
      icon="error"
      title="Failed to load investments"
      :sub-title="error"
    >
      <template #extra>
        <el-button type="primary" @click="load">Retry</el-button>
      </template>
    </el-result>

    <el-empty
      v-else-if="investments.length === 0"
      description="You have no PAMM investments yet"
      :image-size="120"
    >
      <el-button type="primary" @click="goList">Browse funds</el-button>
    </el-empty>

    <template v-else>
      <el-table :data="enrichedInvestments" style="width: 100%" stripe>
        <el-table-column label="Fund" min-width="200">
          <template #default="{ row }">
            <div class="fund-cell">
              <span class="fund-name">{{ row.fund_name }}</span>
              <span class="muted">{{ row.base_currency }}</span>
            </div>
          </template>
        </el-table-column>

        <el-table-column label="Initial" min-width="140" align="right">
          <template #default="{ row }">
            <span class="mono">{{ formatNumber(row.initial_investment) }}</span>
            <span class="kpi-unit">{{ row.base_currency }}</span>
          </template>
        </el-table-column>

        <el-table-column label="Current" min-width="140" align="right">
          <template #default="{ row }">
            <span class="mono">{{ formatNumber(row.current_value) }}</span>
            <span class="kpi-unit">{{ row.base_currency }}</span>
          </template>
        </el-table-column>

        <el-table-column label="P&L" min-width="160" align="right">
          <template #default="{ row }">
            <span
              class="mono"
              :class="row.pnl >= 0 ? 'pnl-pos' : 'pnl-neg'"
            >
              {{ row.pnl >= 0 ? '+' : '' }}{{ formatNumber(row.pnl) }}
            </span>
            <span
              class="pnl-pct"
              :class="row.pnlPct >= 0 ? 'pnl-pos' : 'pnl-neg'"
            >
              ({{ row.pnlPct >= 0 ? '+' : '' }}{{ row.pnlPct.toFixed(2) }}%)
            </span>
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

        <el-table-column label="Actions" width="160" fixed="right">
          <template #default="{ row }">
            <el-button
              size="small"
              type="primary"
              @click="goDetail(row.fund_id)"
            >
              View
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

const router = useRouter()
const pammStore = usePammStore()

const investments = computed(() => pammStore.myInvestments)
const loading = computed(() => pammStore.loading)
const error = computed(() => pammStore.error)

/**
 * Enrich each investment with the fund's display name + currency by
 * joining against the in-store `funds` list. The backend doesn't
 * currently flatten fund fields into the investment row, so we look
 * them up client-side. If the list isn't loaded yet (e.g. user lands
 * here directly), we fall back to currency-less placeholders.
 */
const enrichedInvestments = computed(() => {
  const byId = new Map(pammStore.funds.map((f) => [f.id, f]))
  return investments.value.map((i) => {
    const f = byId.get(i.fund_id)
    const init = Number(i.initial_investment) || 0
    const cur = Number(i.current_value) || 0
    const pnl = cur - init
    const pnlPct = init > 0 ? (pnl / init) * 100 : 0
    return {
      ...i,
      fund_name: f?.name ?? 'Unknown fund',
      base_currency: f?.base_currency ?? '',
      pnl,
      pnlPct,
    }
  })
})

async function load(): Promise<void> {
  await pammStore.loadMyInvestments()
  // Pre-load funds so the join in `enrichedInvestments` resolves names.
  if (pammStore.funds.length === 0) {
    pammStore.listFunds().catch(() => {/* best effort */})
  }
}

function goDetail(id: string): void {
  router.push({ name: 'PammFundDetail', params: { id } })
}

function goList(): void {
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
.pamm-my-investments-view {
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

.fund-cell {
  display: flex;
  flex-direction: column;
}

.fund-name {
  font-weight: 600;
  color: var(--el-color-primary);
  cursor: pointer;
}

.muted {
  font-size: 12px;
  color: var(--el-text-color-secondary);
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

.pnl-pct {
  font-size: 12px;
  margin-left: 6px;
}

.pnl-pos {
  color: var(--el-color-success);
}

.pnl-neg {
  color: var(--el-color-danger);
}
</style>
