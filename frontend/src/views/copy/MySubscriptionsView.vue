<template>
  <div class="my-subscriptions-view">
    <div class="page-header">
      <h1>My Copy Subscriptions</h1>
      <p class="page-subtitle">
        Traders you're following. Each subscription mirrors the trader's
        orders pro-rata; you can cancel at any time. Copied trades and
        profit-share settlements appear below.
      </p>
    </div>

    <div class="toolbar">
      <el-button :icon="Refresh" @click="load">Refresh</el-button>
      <el-button type="primary" :icon="Search" @click="goBrowse">
        Browse traders
      </el-button>
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
      title="Failed to load subscriptions"
      :sub-title="error"
    >
      <template #extra>
        <el-button type="primary" @click="load">Retry</el-button>
      </template>
    </el-result>

    <!-- Empty -->
    <el-empty
      v-else-if="mySubscriptions.length === 0"
      description="You're not following any traders yet"
      :image-size="120"
    >
      <el-button type="primary" @click="goBrowse">Browse traders</el-button>
    </el-empty>

    <!-- Body -->
    <template v-else>
      <el-row :gutter="16">
        <el-col
          v-for="sub in mySubscriptions"
          :key="sub.id"
          :xs="24"
          :sm="12"
          :md="8"
        >
          <el-card shadow="hover" class="sub-card">
            <template #header>
              <div class="card-header">
                <span class="trader-name">
                  {{ sub.trader_name || shortId(sub.trader_id) }}
                </span>
                <el-tag
                  :type="sub.status === 'Active' ? 'success' : 'info'"
                  size="small"
                >
                  {{ sub.status }}
                </el-tag>
              </div>
            </template>

            <el-descriptions :column="1" size="small" border>
              <el-descriptions-item label="Copy ratio">
                <span class="mono">{{ formatPct(sub.ratio) }}</span>
              </el-descriptions-item>
              <el-descriptions-item label="Max position">
                <span class="mono">{{ sub.max_position_size }}</span>
              </el-descriptions-item>
              <el-descriptions-item label="Max loss/day">
                <span class="mono">{{ sub.max_loss_per_day }}</span>
              </el-descriptions-item>
              <el-descriptions-item label="Started">
                <span class="muted">{{ formatDate(sub.started_at) }}</span>
              </el-descriptions-item>
              <el-descriptions-item v-if="sub.ended_at" label="Ended">
                <span class="muted">{{ formatDate(sub.ended_at) }}</span>
              </el-descriptions-item>
            </el-descriptions>

            <div class="card-footer">
              <el-button
                size="small"
                @click="openDetail(sub.trader_id)"
              >
                View trader
              </el-button>
              <el-button
                v-if="sub.status === 'Active'"
                size="small"
                type="danger"
                :loading="unsubscribingId === sub.id"
                @click="confirmUnsubscribe(sub)"
              >
                Unsubscribe
              </el-button>
            </div>
          </el-card>
        </el-col>
      </el-row>

      <h2 class="section-heading">Recent copied trades</h2>
      <el-card shadow="never" class="trades-card">
        <template #header>
          <div class="section-header">
            <span class="section-title">My copied trades</span>
            <el-button size="small" text :icon="Refresh" @click="loadTrades">
              Refresh
            </el-button>
          </div>
        </template>

        <div v-if="tradesLoading" class="loading">Loading trades...</div>
        <el-empty
          v-else-if="myTrades.length === 0"
          description="No copied trades yet"
          :image-size="100"
        />
        <el-table v-else :data="myTrades" stripe>
          <el-table-column label="Symbol" min-width="100">
            <template #default="{ row }">
              <span class="mono">{{ row.symbol }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Side" width="80" align="center">
            <template #default="{ row }">
              <el-tag
                :type="row.side === 'Buy' ? 'success' : 'danger'"
                size="small"
              >
                {{ row.side }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="Qty" width="120" align="right">
            <template #default="{ row }">
              <span class="mono">{{ row.qty }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Price" width="140" align="right">
            <template #default="{ row }">
              <span class="mono">{{ row.price }}</span>
            </template>
          </el-table-column>
          <el-table-column label="Status" width="100" align="center">
            <template #default="{ row }">
              <el-tag size="small">{{ row.status }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="Created" width="180">
            <template #default="{ row }">
              <span class="muted">{{ formatDate(row.created_at) }}</span>
            </template>
          </el-table-column>
        </el-table>
      </el-card>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Loading, Refresh, Search } from '@element-plus/icons-vue'
import { useCopyTradingStore } from '@/stores/copyTrading'

const router = useRouter()
const copyStore = useCopyTradingStore()

const unsubscribingId = ref<string | null>(null)
const tradesLoading = ref(false)

const mySubscriptions = computed(() => copyStore.mySubscriptions)
const myTrades = computed(() => copyStore.myTrades)
const loading = computed<boolean>(() => copyStore.loading)
const error = computed<string | null>(() => copyStore.error)

async function load(): Promise<void> {
  await Promise.all([
    copyStore.loadMySubscriptions(),
    copyStore.loadMyProfitShares().catch(() => {/* best effort */}),
  ])
}

async function loadTrades(): Promise<void> {
  tradesLoading.value = true
  try {
    await copyStore.loadMyTrades()
  } finally {
    tradesLoading.value = false
  }
}

function goBrowse(): void {
  router.push({ name: 'CopyTraderList' })
}

function openDetail(traderId: string): void {
  router.push({ name: 'CopyTraderDetail', params: { id: traderId } })
}

async function confirmUnsubscribe(sub: {
  id: string
  trader_name?: string | null
  trader_id: string
}): Promise<void> {
  try {
    await ElMessageBox.confirm(
      `Cancel the subscription to ${
        sub.trader_name || shortId(sub.trader_id)
      }? Future orders will not be copied.`,
      'Confirm unsubscribe',
      {
        confirmButtonText: 'Unsubscribe',
        cancelButtonText: 'Keep',
        type: 'warning',
      },
    )
  } catch {
    return
  }
  unsubscribingId.value = sub.id
  try {
    const ok = await copyStore.unsubscribe(sub.id)
    if (ok) {
      ElMessage.success('Unsubscribed')
    } else if (copyStore.error) {
      ElMessage.error(copyStore.error)
    }
  } finally {
    unsubscribingId.value = null
  }
}

function formatPct(value: string | number | null | undefined): string {
  if (value === null || value === undefined) return '—'
  const n = Number(value)
  if (!Number.isFinite(n)) return String(value)
  return `${(n * 100).toFixed(1)}%`
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

onMounted(async () => {
  await load()
  await loadTrades()
})
</script>

<style scoped lang="scss">
.my-subscriptions-view {
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
  gap: 8px;
  margin-bottom: 16px;
}

.sub-card {
  margin-bottom: 16px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.trader-name {
  font-weight: 600;
  font-size: 15px;
  color: var(--el-color-primary);
  cursor: pointer;
}

.card-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
}

.mono {
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
  font-size: 13px;
}

.muted {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}

.section-heading {
  margin: 24px 0 12px 0;
  font-size: 18px;
  font-weight: 600;
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

.trades-card {
  margin-bottom: 24px;
}

.loading {
  text-align: center;
  padding: 32px;
  color: var(--el-text-color-secondary);
}
</style>
