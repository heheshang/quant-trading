<template>
  <div class="copy-trader-detail-view">
    <div class="back-row">
      <el-button text :icon="ArrowLeft" @click="goBack">Back to list</el-button>
    </div>

    <!-- Loading -->
    <template v-if="loading && !trader">
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
        title="Failed to load trader"
        :sub-title="error"
      >
        <template #extra>
          <el-button type="primary" @click="load">Retry</el-button>
        </template>
      </el-result>
    </template>

    <!-- Body -->
    <template v-else-if="trader">
      <div class="header">
        <h1>{{ trader.display_name }}</h1>
        <p v-if="trader.bio" class="trader-bio">{{ trader.bio }}</p>
        <div class="header-meta">
          <el-tag
            :type="trader.status === 'Active' ? 'success' : 'info'"
            size="small"
          >
            {{ trader.status }}
          </el-tag>
          <span v-if="trader.username" class="muted">
            @{{ trader.username }}
          </span>
        </div>
        <div class="header-actions">
          <el-button type="primary" :icon="Plus" @click="openSubscribeDialog">
            Subscribe
          </el-button>
        </div>
      </div>

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

      <el-tabs v-model="activeTab" class="detail-tabs">
        <el-tab-pane label="Overview" name="overview">
          <el-card shadow="never" class="section-card">
            <template #header>
              <span class="section-title">Profile</span>
            </template>
            <el-descriptions :column="2" border>
              <el-descriptions-item label="Display name">
                {{ trader.display_name }}
              </el-descriptions-item>
              <el-descriptions-item label="Status">
                <el-tag
                  :type="trader.status === 'Active' ? 'success' : 'info'"
                  size="small"
                >
                  {{ trader.status }}
                </el-tag>
              </el-descriptions-item>
              <el-descriptions-item label="Total P&L">
                <span :class="pnlClass(trader.total_pnl)">
                  {{ formatNumber(trader.total_pnl) }}
                </span>
              </el-descriptions-item>
              <el-descriptions-item label="Monthly P&L">
                <span :class="pnlClass(trader.monthly_pnl)">
                  {{ formatNumber(trader.monthly_pnl) }}
                </span>
              </el-descriptions-item>
              <el-descriptions-item label="Win rate">
                {{ formatPct(trader.win_rate) }}
              </el-descriptions-item>
              <el-descriptions-item label="Followers">
                {{ trader.follower_count }}
              </el-descriptions-item>
              <el-descriptions-item label="Bio" :span="2">
                <span v-if="trader.bio">{{ trader.bio }}</span>
                <span v-else class="muted">No bio yet.</span>
              </el-descriptions-item>
              <el-descriptions-item label="Created at">
                {{ formatDate(trader.created_at) }}
              </el-descriptions-item>
              <el-descriptions-item label="Updated at">
                {{ formatDate(trader.updated_at) }}
              </el-descriptions-item>
            </el-descriptions>
          </el-card>
        </el-tab-pane>

        <el-tab-pane label="Subscribers" name="subscribers">
          <el-card shadow="never" class="section-card">
            <template #header>
              <div class="section-header">
                <span class="section-title">Active subscribers</span>
                <el-button
                  size="small"
                  text
                  :icon="Refresh"
                  @click="loadSubscribers"
                >
                  Refresh
                </el-button>
              </div>
            </template>
            <p v-if="!trader.user_id" class="muted">
              Subscriber list is only visible to the trader's owner — sign
              in as the trader to see the full list.
            </p>
            <template v-else>
              <div v-if="subscribersLoading" class="loading">Loading...</div>
              <el-empty
                v-else-if="subscribers.length === 0"
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
              </el-table>
            </template>
          </el-card>
        </el-tab-pane>

        <el-tab-pane label="Performance" name="performance">
          <el-card shadow="never" class="section-card">
            <template #header>
              <span class="section-title">Performance snapshot</span>
            </template>
            <el-descriptions :column="2" border>
              <el-descriptions-item label="Monthly P&L">
                <span :class="pnlClass(trader.monthly_pnl)">
                  {{ formatNumber(trader.monthly_pnl) }}
                </span>
              </el-descriptions-item>
              <el-descriptions-item label="Total P&L">
                <span :class="pnlClass(trader.total_pnl)">
                  {{ formatNumber(trader.total_pnl) }}
                </span>
              </el-descriptions-item>
              <el-descriptions-item label="Win rate">
                {{ formatPct(trader.win_rate) }}
              </el-descriptions-item>
              <el-descriptions-item label="Followers">
                {{ trader.follower_count }}
              </el-descriptions-item>
            </el-descriptions>
            <p class="muted perf-note">
              Time-series P&L and per-trade history live on the trader
              dashboard once you subscribe. Detailed performance charts
              are tracked separately under §6-2 charting work.
            </p>
          </el-card>
        </el-tab-pane>
      </el-tabs>
    </template>

    <!-- Subscribe dialog -->
    <el-dialog
      v-model="subscribeDialogVisible"
      :title="trader ? `Subscribe to ${trader.display_name}` : 'Subscribe'"
      width="480px"
      :close-on-click-modal="false"
    >
      <p v-if="trader" class="dialog-hint">
        The follower engine will mirror <strong>{{ trader.display_name }}</strong>'s
        orders pro-rata. Risk caps apply per-subscription and per-day; set
        0 to disable a cap.
      </p>
      <el-form
        ref="subscribeFormRef"
        :model="subscribeForm"
        :rules="subscribeRules"
        label-position="top"
      >
        <el-form-item label="Copy ratio" prop="ratio">
          <el-input
            v-model="subscribeForm.ratio"
            placeholder="0.10 (i.e. 10%)"
          />
          <span class="form-hint">0.01 to 1.00 (1.00 = full mirror)</span>
        </el-form-item>
        <el-form-item label="Max position per copied trade" prop="max_position_size">
          <el-input
            v-model="subscribeForm.max_position_size"
            placeholder="0 = no cap"
          />
        </el-form-item>
        <el-form-item label="Max loss per day" prop="max_loss_per_day">
          <el-input
            v-model="subscribeForm.max_loss_per_day"
            placeholder="0 = no cap"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="subscribeDialogVisible = false">Cancel</el-button>
        <el-button
          type="primary"
          :loading="subscribing"
          @click="submitSubscribe"
        >
          Subscribe
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { ArrowLeft, Plus, Refresh } from '@element-plus/icons-vue'
import {
  useCopyTradingStore,
  type Trader,
  type Subscription,
} from '@/stores/copyTrading'

const route = useRoute()
const router = useRouter()
const copyStore = useCopyTradingStore()

const activeTab = ref<'overview' | 'subscribers' | 'performance'>('overview')
const subscribeDialogVisible = ref(false)
const subscribing = ref(false)
const subscribeFormRef = ref<FormInstance>()
const subscribersLoading = ref(false)
const subscribers = ref<Subscription[]>([])

const subscribeForm = ref({
  ratio: '0.10',
  max_position_size: '0',
  max_loss_per_day: '0',
})

const subscribeRules: FormRules = {
  ratio: [
    { required: true, message: 'Copy ratio is required', trigger: 'blur' },
    {
      validator: (_rule, value, cb) => {
        const n = Number(value)
        if (!Number.isFinite(n) || n <= 0 || n > 1) {
          cb(new Error('Ratio must be in (0, 1]'))
        } else {
          cb()
        }
      },
      trigger: 'blur',
    },
  ],
  max_position_size: [
    { required: true, message: 'Required (use 0 for no cap)', trigger: 'blur' },
  ],
  max_loss_per_day: [
    { required: true, message: 'Required (use 0 for no cap)', trigger: 'blur' },
  ],
}

const trader = computed<Trader | null>(() => copyStore.currentTrader)
const loading = computed<boolean>(() => copyStore.loading)
const error = computed<string | null>(() => copyStore.error)

async function load(): Promise<void> {
  const id = String(route.params.id)
  await copyStore.getTrader(id)
  await loadSubscribers()
}

async function loadSubscribers(): Promise<void> {
  // The /my-trader endpoint returns the caller's own subscribers. From a
  // public trader detail page the caller is just a visitor, so we just
  // hit the same endpoint and let the backend decide if the caller is
  // the owner. The backend will return trader=null + subscribers=[] if not.
  subscribersLoading.value = true
  try {
    const res = await copyStore.loadMyTrader()
    // Only show subscribers for the trader we're viewing.
    if (trader.value && res.trader && res.trader.id === trader.value.id) {
      subscribers.value = res.subscribers
    } else {
      subscribers.value = []
    }
  } catch {
    subscribers.value = []
  } finally {
    subscribersLoading.value = false
  }
}

function goBack(): void {
  router.push({ name: 'CopyTraderList' })
}

function openSubscribeDialog(): void {
  subscribeForm.value = {
    ratio: '0.10',
    max_position_size: '0',
    max_loss_per_day: '0',
  }
  subscribeDialogVisible.value = true
}

async function submitSubscribe(): Promise<void> {
  const form = subscribeFormRef.value
  if (!form || !trader.value) return
  try {
    await form.validate()
  } catch {
    return
  }
  subscribing.value = true
  try {
    const result = await copyStore.subscribe(
      trader.value.id,
      subscribeForm.value.ratio,
      subscribeForm.value.max_position_size,
      subscribeForm.value.max_loss_per_day,
    )
    if (result) {
      ElMessage.success(
        `Subscribed (ratio ${result.ratio}, status ${result.status})`,
      )
      subscribeDialogVisible.value = false
    } else if (copyStore.error) {
      ElMessage.error(copyStore.error)
    }
  } finally {
    subscribing.value = false
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
  return id.slice(0, 8)
}

// Reload on route param change (navigating from one trader to another).
watch(
  () => route.params.id,
  async (newId) => {
    if (newId) await load()
  },
)

onMounted(load)
onUnmounted(() => copyStore.clearCurrentTrader())
</script>

<style scoped lang="scss">
.copy-trader-detail-view {
  padding: 24px;
  max-width: 1280px;
  margin: 0 auto;
}

.back-row {
  margin-bottom: 12px;
}

.header {
  margin-bottom: 16px;
  position: relative;
}

.header h1 {
  margin: 0 0 8px 0;
  font-size: 26px;
  font-weight: 600;
}

.trader-bio {
  margin: 0 0 12px 0;
  color: var(--el-text-color-regular);
  max-width: 720px;
  line-height: 1.5;
}

.header-meta {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-actions {
  position: absolute;
  top: 0;
  right: 0;
}

.muted {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.mono {
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
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

.kpi-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
  margin: 16px 0 24px 0;
}

.kpi-card {
  background: var(--el-fill-color-blank);
  border: 1px solid var(--el-border-color-light);
  border-radius: 8px;
  padding: 16px;
}

.kpi-card--skeleton {
  height: 86px;
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

.detail-tabs {
  margin-top: 8px;
}

.section-card {
  margin-bottom: 16px;
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

.perf-note {
  margin: 16px 0 0 0;
  font-size: 12px;
  font-style: italic;
}

.dialog-hint {
  margin: 0 0 16px 0;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.form-hint {
  margin-left: 12px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

.skeleton-cell {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  animation: pulse 1.4s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 0.6;
  }
  50% {
    opacity: 1;
  }
}

.loading {
  text-align: center;
  padding: 32px;
  color: var(--el-text-color-secondary);
}
</style>
