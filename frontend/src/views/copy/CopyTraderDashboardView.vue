<template>
  <div class="copy-trader-dashboard-view">
    <div class="page-header">
      <h1>Copy Trader Dashboard</h1>
      <p class="page-subtitle">
        Your public trader profile, your active subscribers, and a
        one-click way to trigger profit-share settlement for a period.
      </p>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="loading">
      <el-icon class="is-loading"><Loading /></el-icon>
      Loading...
    </div>

    <!-- Error -->
    <el-result
      v-else-if="error"
      icon="error"
      title="Failed to load your trader profile"
      :sub-title="error"
    >
      <template #extra>
        <el-button type="primary" @click="load">Retry</el-button>
      </template>
    </el-result>

    <!-- Not a trader -->
    <el-empty
      v-else-if="!trader"
      description="You're not registered as a trader yet"
      :image-size="140"
    >
      <el-button type="primary" @click="goRegister">Register as Trader</el-button>
    </el-empty>

    <!-- Body -->
    <template v-else>
      <el-card shadow="never" class="profile-card">
        <template #header>
          <div class="card-header">
            <div>
              <h2 class="profile-name">{{ trader.display_name }}</h2>
              <span v-if="trader.username" class="muted">
                @{{ trader.username }}
              </span>
            </div>
            <el-tag
              :type="trader.status === 'Active' ? 'success' : 'info'"
              size="default"
            >
              {{ trader.status }}
            </el-tag>
          </div>
        </template>

        <p v-if="trader.bio" class="profile-bio">{{ trader.bio }}</p>
        <p v-else class="muted">No bio yet.</p>

        <div class="kpi-row">
          <div class="kpi-card">
            <div class="kpi-label">Monthly P&L</div>
            <div :class="['kpi-value', pnlClass(trader.monthly_pnl)]">
              {{ formatNumber(trader.monthly_pnl) }}
            </div>
          </div>
          <div class="kpi-card">
            <div class="kpi-label">Total P&L</div>
            <div :class="['kpi-value', pnlClass(trader.total_pnl)]">
              {{ formatNumber(trader.total_pnl) }}
            </div>
          </div>
          <div class="kpi-card">
            <div class="kpi-label">Win rate</div>
            <div class="kpi-value">{{ formatPct(trader.win_rate) }}</div>
          </div>
          <div class="kpi-card">
            <div class="kpi-label">Followers</div>
            <div class="kpi-value">{{ trader.follower_count }}</div>
          </div>
        </div>
      </el-card>

      <el-card shadow="never" class="subscribers-card">
        <template #header>
          <div class="section-header">
            <span class="section-title">Subscribers</span>
            <el-button size="small" text :icon="Refresh" @click="load">
              Refresh
            </el-button>
          </div>
        </template>

        <el-empty
          v-if="subscribers.length === 0"
          description="No active subscribers yet"
          :image-size="100"
        />
        <el-table v-else :data="subscribers" stripe>
          <el-table-column label="Follower" min-width="220">
            <template #default="{ row }">
              <span class="muted">{{ shortId(row.follower_id) }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Ratio" width="100" align="right">
            <template #default="{ row }">
              <span class="mono">{{ formatPct(row.ratio) }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Max position" width="140" align="right">
            <template #default="{ row }">
              <span class="mono">{{ row.max_position_size }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Max loss/day" width="140" align="right">
            <template #default="{ row }">
              <span class="mono">{{ row.max_loss_per_day }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Status" width="100" align="center">
            <template #default="{ row }">
              <el-tag
                :type="row.status === 'Active' ? 'success' : 'info'"
                size="small"
              >
                {{ row.status }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="Started" width="180">
            <template #default="{ row }">
              <span class="muted">{{ formatDate(row.started_at) }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Actions" width="180" fixed="right">
            <template #default="{ row }">
              <el-button
                size="small"
                type="primary"
                @click="openCalculateDialog(row)"
              >
                Calculate shares
              </el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-card>
    </template>

    <!-- Calculate-shares dialog -->
    <el-dialog
      v-model="calculateDialogVisible"
      :title="
        calculateTarget
          ? `Settle profit share for ${
              calculateTarget.trader_name || shortId(calculateTarget.trader_id)
            }`
          : 'Settle profit share'
      "
      width="520px"
      :close-on-click-modal="false"
    >
      <p class="dialog-hint">
        Period-end settlement: the follower engine computes the trader's
        profit share for the closed window and inserts a
        <code>profit_shares</code> row. The settlement is idempotent on
        <code>(subscription, period_start, period_end)</code>.
      </p>
      <el-form
        ref="calculateFormRef"
        :model="calculateForm"
        :rules="calculateRules"
        label-position="top"
      >
        <el-form-item label="Period start" prop="period_start">
          <el-date-picker
            v-model="calculateForm.period_start"
            type="datetime"
            placeholder="Pick start"
            style="width: 100%"
            value-format="YYYY-MM-DDTHH:mm:ss[Z]"
          />
        </el-form-item>
        <el-form-item label="Period end" prop="period_end">
          <el-date-picker
            v-model="calculateForm.period_end"
            type="datetime"
            placeholder="Pick end"
            style="width: 100%"
            value-format="YYYY-MM-DDTHH:mm:ss[Z]"
          />
        </el-form-item>
        <el-form-item label="Trader fee % (optional)">
          <el-input
            v-model="calculateForm.trader_fee_pct"
            placeholder="0.20 (i.e. 20% — leave blank for default)"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="calculateDialogVisible = false">Cancel</el-button>
        <el-button
          type="primary"
          :loading="calculating"
          @click="submitCalculate"
        >
          Calculate
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Loading, Refresh } from '@element-plus/icons-vue'
import {
  useCopyTradingStore,
  type Subscription,
} from '@/stores/copyTrading'

const router = useRouter()
const copyStore = useCopyTradingStore()

const calculateDialogVisible = ref(false)
const calculating = ref(false)
const calculateFormRef = ref<FormInstance>()
const calculateTarget = ref<Subscription | null>(null)
const subscribers = ref<Subscription[]>([])

const calculateForm = ref({
  period_start: '',
  period_end: '',
  trader_fee_pct: '',
})

const calculateRules: FormRules = {
  period_start: [
    { required: true, message: 'Period start is required', trigger: 'change' },
  ],
  period_end: [
    { required: true, message: 'Period end is required', trigger: 'change' },
  ],
}

const trader = computed(() => copyStore.currentTrader)
const loading = computed<boolean>(() => copyStore.loading)
const error = computed<string | null>(() => copyStore.error)

async function load(): Promise<void> {
  const res = await copyStore.loadMyTrader()
  if (res.trader) {
    // loadMyTrader already sets the store's currentTrader via the
    // returned object — but to keep the local `subscribers` list in
    // sync with the public profile, we set both from one response.
    // The store doesn't currently expose a `setCurrentTrader` action,
    // so we go through getTrader to populate currentTrader, then read
    // subscribers from the in-memory response.
    if (res.trader.id) {
      await copyStore.getTrader(res.trader.id)
    }
    subscribers.value = res.subscribers
  } else {
    subscribers.value = []
  }
}

function goRegister(): void {
  router.push({ name: 'CopyTraderList' })
}

function openCalculateDialog(sub: Subscription): void {
  calculateTarget.value = sub
  calculateForm.value = {
    period_start: '',
    period_end: '',
    trader_fee_pct: '',
  }
  calculateDialogVisible.value = true
}

async function submitCalculate(): Promise<void> {
  const form = calculateFormRef.value
  if (!form || !calculateTarget.value) return
  try {
    await form.validate()
  } catch {
    return
  }
  if (calculateForm.value.period_end <= calculateForm.value.period_start) {
    ElMessage.error('Period end must be after period start')
    return
  }
  calculating.value = true
  try {
    const result = await copyStore.calculateShares(
      calculateTarget.value.id,
      calculateForm.value.period_start,
      calculateForm.value.period_end,
      calculateForm.value.trader_fee_pct || undefined,
    )
    if (result) {
      ElMessage.success('Profit share calculated')
      calculateDialogVisible.value = false
    } else if (copyStore.error) {
      ElMessage.error(copyStore.error)
    }
  } finally {
    calculating.value = false
  }
}

function formatNumber(value: string | number | null | undefined): string {
  if (value === null || value === undefined) return '—'
  const n = Number(value)
  if (!Number.isFinite(n)) return String(value)
  return n.toLocaleString('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

function formatPct(value: string | number | null | undefined): string {
  if (value === null || value === undefined) return '—'
  const n = Number(value)
  if (!Number.isFinite(n)) return String(value)
  return `${(n * 100).toFixed(1)}%`
}

function pnlClass(value: string | number | null | undefined): string {
  const n = Number(value)
  if (!Number.isFinite(n)) return 'pnl-neutral'
  if (n > 0) return 'pnl-positive'
  if (n < 0) return 'pnl-negative'
  return 'pnl-neutral'
}

function formatDate(value: string | null | undefined): string {
  if (!value) return '—'
  try {
    return new Date(value).toLocaleString()
  } catch {
    return value
  }
}

function shortId(id: string): string {
  if (!id) return '—'
  return id.slice(0, 8) + '…'
}

onMounted(load)
</script>

<style scoped lang="scss">
.copy-trader-dashboard-view {
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
  max-width: 720px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.profile-name {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.profile-bio {
  margin: 0 0 16px 0;
  color: var(--el-text-color-regular);
  line-height: 1.5;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.section-title {
  font-weight: 600;
}

.profile-card,
.subscribers-card {
  margin-bottom: 16px;
}

.kpi-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
}

.kpi-card {
  background: var(--el-fill-color-blank);
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
  padding: 16px;
}

.kpi-label {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  text-transform: uppercase;
  letter-spacing: 0.4px;
  margin-bottom: 6px;
}

.kpi-value {
  font-size: 22px;
  font-weight: 600;
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
}

.mono {
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
  font-size: 13px;
}

.muted {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.pnl-positive {
  color: var(--el-color-success);
  font-weight: 600;
}

.pnl-negative {
  color: var(--el-color-danger);
  font-weight: 600;
}

.pnl-neutral {
  color: var(--el-text-color-secondary);
}

.dialog-hint {
  margin: 0 0 16px 0;
  color: var(--el-text-color-secondary);
  font-size: 13px;
  line-height: 1.5;
}

.loading {
  text-align: center;
  padding: 32px;
  color: var(--el-text-color-secondary);
}
</style>
