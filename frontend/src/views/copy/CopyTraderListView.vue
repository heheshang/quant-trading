<template>
  <div class="copy-trader-list-view">
    <div class="page-header">
      <h1>Copy Trading</h1>
      <p class="page-subtitle">
        Browse active traders, inspect their KPIs, and follow them with a
        configurable copy ratio. The follower engine mirrors the trader's
        orders pro-rata, then settles profit shares at the end of each
        period.
      </p>
    </div>

    <div class="toolbar">
      <el-input
        v-model="filterText"
        placeholder="Filter by display name or username"
        clearable
        style="max-width: 360px"
        :prefix-icon="Search"
      />
      <div class="toolbar-actions">
        <el-button :icon="Refresh" @click="load">Refresh</el-button>
        <el-button type="primary" :icon="Plus" @click="openRegisterDialog">
          Register as Trader
        </el-button>
      </div>
    </div>

    <!-- Loading state -->
    <template v-if="loading && traders.length === 0">
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
        title="Failed to load traders"
        :sub-title="error"
      >
        <template #extra>
          <el-button type="primary" @click="load">Retry</el-button>
        </template>
      </el-result>
    </template>

    <!-- Empty state -->
    <template v-else-if="filteredTraders.length === 0">
      <el-empty
        :description="
          filterText
            ? 'No traders match your filter'
            : 'No active traders yet — be the first'
        "
        :image-size="120"
      />
    </template>

    <!-- Table -->
    <template v-else>
      <el-table
        :data="filteredTraders"
        style="width: 100%"
        stripe
        @row-click="(row: Trader) => openDetail(row.id)"
      >
        <el-table-column label="Trader" min-width="200">
          <template #default="{ row }">
            <span class="trader-name">{{ row.display_name }}</span>
            <div v-if="row.username" class="trader-handle">
              @{{ row.username }}
            </div>
          </template>
        </el-table-column>

        <el-table-column label="Monthly P&L" min-width="140" align="right">
          <template #default="{ row }">
            <span :class="pnlClass(row.monthly_pnl)">
              {{ formatNumber(row.monthly_pnl) }}
            </span>
          </template>
        </el-table-column>

        <el-table-column label="Win rate" width="100" align="right">
          <template #default="{ row }">
            <span class="winrate">{{ formatPct(row.win_rate) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="Followers" width="100" align="right">
          <template #default="{ row }">
            <span class="follower-count">{{ row.follower_count }}</span>
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

        <el-table-column label="Actions" width="180" fixed="right">
          <template #default="{ row }">
            <el-button
              size="small"
              type="primary"
              @click.stop="openSubscribeDialog(row)"
            >
              Subscribe
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </template>

    <!-- Register-as-trader dialog -->
    <el-dialog
      v-model="registerDialogVisible"
      title="Register as a Copy Trader"
      width="480px"
      :close-on-click-modal="false"
    >
      <p class="dialog-hint">
        Becoming a trader lets you publish a profile and have followers
        mirror your orders. You can update your bio and KPIs at any time.
      </p>
      <el-form
        ref="registerFormRef"
        :model="registerForm"
        :rules="registerRules"
        label-position="top"
      >
        <el-form-item label="Display name" prop="display_name">
          <el-input
            v-model="registerForm.display_name"
            placeholder="e.g. Alpha Momentum"
          />
        </el-form-item>
        <el-form-item label="Bio">
          <el-input
            v-model="registerForm.bio"
            type="textarea"
            :rows="3"
            placeholder="Optional short bio shown on your trader profile"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="registerDialogVisible = false">Cancel</el-button>
        <el-button
          type="primary"
          :loading="registering"
          @click="submitRegister"
        >
          Register
        </el-button>
      </template>
    </el-dialog>

    <!-- Subscribe dialog -->
    <el-dialog
      v-model="subscribeDialogVisible"
      :title="subscribeTarget ? `Subscribe to ${subscribeTarget.display_name}` : 'Subscribe'"
      width="480px"
      :close-on-click-modal="false"
    >
      <p v-if="subscribeTarget" class="dialog-hint">
        You're following <strong>{{ subscribeTarget.display_name }}</strong>.
        The follower engine will mirror their orders pro-rata. Risk caps
        apply per-subscription and per-day; set 0 to disable a cap.
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
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Search, Plus, Refresh } from '@element-plus/icons-vue'
import { useCopyTradingStore, type Trader } from '@/stores/copyTrading'

const router = useRouter()
const copyStore = useCopyTradingStore()

// ── State ───────────────────────────────────────────────────────────
const filterText = ref('')
const registerDialogVisible = ref(false)
const subscribeDialogVisible = ref(false)
const registering = ref(false)
const subscribing = ref(false)
const registerFormRef = ref<FormInstance>()
const subscribeFormRef = ref<FormInstance>()
const subscribeTarget = ref<Trader | null>(null)

const registerForm = ref({
  display_name: '',
  bio: '',
})

const subscribeForm = ref({
  ratio: '0.10',
  max_position_size: '0',
  max_loss_per_day: '0',
})

const registerRules: FormRules = {
  display_name: [
    { required: true, message: 'Display name is required', trigger: 'blur' },
    { min: 2, max: 64, message: '2–64 characters', trigger: 'blur' },
  ],
}

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

// ── Computed ────────────────────────────────────────────────────────
const traders = computed<Trader[]>(() => copyStore.traders)
const loading = computed<boolean>(() => copyStore.loading)
const error = computed<string | null>(() => copyStore.error)

const filteredTraders = computed<Trader[]>(() => {
  const q = filterText.value.trim().toLowerCase()
  if (!q) return traders.value
  return traders.value.filter((t) => {
    if (t.display_name.toLowerCase().includes(q)) return true
    if (t.username && t.username.toLowerCase().includes(q)) return true
    return false
  })
})

// ── Actions ─────────────────────────────────────────────────────────
async function load(): Promise<void> {
  await copyStore.listTraders()
}

function openDetail(id: string): void {
  router.push({ name: 'CopyTraderDetail', params: { id } })
}

function openRegisterDialog(): void {
  registerForm.value = { display_name: '', bio: '' }
  registerDialogVisible.value = true
}

async function submitRegister(): Promise<void> {
  const form = registerFormRef.value
  if (!form) return
  try {
    await form.validate()
  } catch {
    return
  }
  registering.value = true
  try {
    const trader = await copyStore.registerAsTrader(
      registerForm.value.display_name,
      registerForm.value.bio || undefined,
    )
    if (trader) {
      ElMessage.success(`Registered as ${trader.display_name}`)
      registerDialogVisible.value = false
      // Jump to dashboard so the user can see their new profile.
      router.push({ name: 'CopyTraderDashboard' })
    } else if (copyStore.error) {
      ElMessage.error(copyStore.error)
    }
  } finally {
    registering.value = false
  }
}

function openSubscribeDialog(row: Trader): void {
  subscribeTarget.value = row
  subscribeForm.value = {
    ratio: '0.10',
    max_position_size: '0',
    max_loss_per_day: '0',
  }
  subscribeDialogVisible.value = true
}

async function submitSubscribe(): Promise<void> {
  const form = subscribeFormRef.value
  if (!form || !subscribeTarget.value) return
  try {
    await form.validate()
  } catch {
    return
  }
  subscribing.value = true
  try {
    const result = await copyStore.subscribe(
      subscribeTarget.value.id,
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

// ── Helpers ─────────────────────────────────────────────────────────
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

onMounted(load)
</script>

<style scoped lang="scss">
.copy-trader-list-view {
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

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 16px;
}

.toolbar-actions {
  display: flex;
  gap: 8px;
}

.trader-name {
  font-weight: 600;
  color: var(--el-color-primary);
  cursor: pointer;
}

.trader-handle {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  margin-top: 2px;
}

.winrate,
.follower-count {
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
}

.pnl-positive {
  color: var(--el-color-success);
  font-weight: 600;
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
}

.pnl-negative {
  color: var(--el-color-danger);
  font-weight: 600;
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
}

.pnl-neutral {
  color: var(--el-text-color-secondary);
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
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
  height: 16px;
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
</style>
