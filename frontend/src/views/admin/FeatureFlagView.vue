<template>
  <div class="feature-flag-view">
    <div class="page-header">
      <h1>Feature Flags</h1>
      <p class="page-subtitle">
        Toggle in-house feature flags. Changes apply to the next evaluation
        for every user (the per-user cache is invalidated on upsert).
      </p>
    </div>

    <!-- Loading state -->
    <template v-if="loading">
      <div class="skeleton-table">
        <div v-for="n in 4" :key="n" class="skeleton-row">
          <div class="skeleton-cell" style="width: 20%"></div>
          <div class="skeleton-cell" style="width: 40%"></div>
          <div class="skeleton-cell" style="width: 15%"></div>
          <div class="skeleton-cell" style="width: 15%"></div>
        </div>
      </div>
    </template>

    <!-- Error state -->
    <template v-else-if="loadError">
      <el-result icon="error" title="Failed to load feature flags" :sub-title="loadError">
        <template #extra>
          <el-button type="primary" @click="loadFlags">Retry</el-button>
        </template>
      </el-result>
    </template>

    <!-- Empty state -->
    <template v-else-if="flags.length === 0">
      <el-empty description="No feature flags defined yet" :image-size="100" />
    </template>

    <!-- Table -->
    <template v-else>
      <el-table :data="flags" style="width: 100%" stripe v-loading="saving">
        <el-table-column prop="key" label="Key" min-width="180">
          <template #default="{ row }">
            <code class="flag-key">{{ row.key }}</code>
          </template>
        </el-table-column>

        <el-table-column prop="description" label="Description" min-width="280">
          <template #default="{ row }">
            <span class="flag-desc">{{ row.description || '—' }}</span>
          </template>
        </el-table-column>

        <el-table-column prop="percentageRollout" label="Rollout %" width="120" align="center">
          <template #default="{ row }">
            <el-tag size="small" effect="plain">
              {{ row.percentageRollout ?? 0 }}%
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column prop="enabled" label="Enabled" width="100" align="center">
          <template #default="{ row }">
            <el-switch
              v-model="row.enabled"
              :loading="row._saving"
              :disabled="row._saving"
              size="default"
              @change="(val: boolean) => saveFlag(row, val)"
            />
          </template>
        </el-table-column>

        <el-table-column prop="updatedAt" label="Last updated" width="180">
          <template #default="{ row }">
            <span class="muted">{{ formatDate(row.updatedAt) }}</span>
          </template>
        </el-table-column>

        <el-table-column label="Actions" width="160" fixed="right">
          <template #default="{ row }">
            <el-button
              size="small"
              text
              type="danger"
              :disabled="row._saving"
              @click="confirmDelete(row)"
            >
              Delete
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { typedApi } from '@/api/typedClient'
import type { components } from '@/types/api-generated'

// FeatureFlag as it comes back from GET /api/v1/admin/feature-flags
// (camelCase via serde rename_all). We extend it with a per-row `_saving`
// flag to track the in-flight switch toggle without mutating the
// server-owned fields.
type FeatureFlag = components['schemas']['FeatureFlag']
type FlagRow = FeatureFlag & { _saving?: boolean }

const flags = ref<FlagRow[]>([])
const loading = ref(false)
const saving = ref(false)
const loadError = ref<string | null>(null)

async function loadFlags(): Promise<void> {
  loading.value = true
  loadError.value = null
  const { data, error } = await typedApi.GET('/api/v1/admin/feature-flags')
  if (error) {
    loadError.value = (error as unknown as { message?: string })?.message
      || `HTTP ${(error as unknown as { status?: number })?.status ?? 'unknown'}`
    flags.value = []
  } else if (data) {
    // openapi-typescript typed the response as FeatureFlag[] (the body shape).
    // Add a local `_saving` flag for per-row spinner state.
    flags.value = (data as FeatureFlag[]).map((f) => ({ ...f, _saving: false }))
  }
  loading.value = false
}

async function saveFlag(row: FlagRow, val: boolean): Promise<void> {
  // Optimistic + rollback: flip the switch, send the request, revert on
  // failure. The switch model is bound to `row.enabled` directly, so the
  // optimistic update is implicit.
  row._saving = true
  saving.value = true
  try {
    // The upsert body mirrors the admin view's editable fields. The server
    // re-derives timestamps + updatedBy from the JWT, so we don't include
    // them in the request.
    const body: components['schemas']['UpsertFeatureFlagRequest'] = {
      key: row.key,
      description: row.description,
      enabled: val,
      userWhitelist: row.userWhitelist ?? [],
      percentageRollout: row.percentageRollout ?? 0,
      metadata: row.metadata ?? {},
    }
    const { data, error } = await typedApi.POST('/api/v1/admin/feature-flags', {
      body,
    })
    if (error) {
      // Roll back the switch.
      row.enabled = !val
      const msg = (error as unknown as { message?: string })?.message
        || 'Save failed'
      ElMessage.error(msg)
    } else if (data) {
      // Refresh the row with the server's authoritative response.
      Object.assign(row, data, { _saving: false })
      ElMessage.success(`Flag "${row.key}" updated`)
    }
  } catch (e: unknown) {
    row.enabled = !val
    ElMessage.error((e as { message?: string })?.message || 'Save failed')
  } finally {
    row._saving = false
    saving.value = false
  }
}

async function confirmDelete(row: FlagRow): Promise<void> {
  try {
    await ElMessageBox.confirm(
      `Delete feature flag "${row.key}"? This cannot be undone.`,
      'Confirm delete',
      { type: 'warning', confirmButtonText: 'Delete', cancelButtonText: 'Cancel' },
    )
  } catch {
    // User cancelled
    return
  }
  row._saving = true
  try {
    const { error } = await typedApi.DELETE('/api/v1/admin/feature-flags/{key}', {
      params: { path: { key: row.key } },
    })
    if (error) {
      const msg = (error as unknown as { message?: string })?.message
        || 'Delete failed'
      ElMessage.error(msg)
    } else {
      flags.value = flags.value.filter((f) => f.key !== row.key)
      ElMessage.success(`Flag "${row.key}" deleted`)
    }
  } catch (e: unknown) {
    ElMessage.error((e as { message?: string })?.message || 'Delete failed')
  } finally {
    row._saving = false
  }
}

function formatDate(iso: string | null | undefined): string {
  if (!iso) return '—'
  try {
    const d = new Date(iso)
    if (Number.isNaN(d.getTime())) return iso
    return d.toLocaleString()
  } catch {
    return iso
  }
}

onMounted(loadFlags)
</script>

<style scoped lang="scss">
.feature-flag-view {
  padding: 24px;
  max-width: 1280px;
  margin: 0 auto;
}

.page-header {
  margin-bottom: 24px;
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

.flag-key {
  font-family: 'JetBrains Mono', 'Menlo', 'Consolas', monospace;
  font-size: 13px;
  background: var(--el-fill-color-light);
  padding: 2px 6px;
  border-radius: 4px;
}

.flag-desc {
  color: var(--el-text-color-regular);
}

.muted {
  color: var(--el-text-color-secondary);
  font-size: 13px;
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
