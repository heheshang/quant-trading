<template>
  <div class="strategy-review-view">
    <!-- Page Header -->
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">策略审核</h1>
        <span class="page-subtitle">审核待上线策略，批准或拒绝</span>
      </div>
      <div class="header-right">
        <el-button :icon="Refresh" @click="loadPendingReviews" :loading="loading">
          刷新
        </el-button>
      </div>
    </div>

    <!-- Pending Reviews Table -->
    <div class="table-container">
      <el-table
        v-loading="loading"
        :data="pendingReviews"
        stripe
        style="width: 100%"
        empty-text="暂无待审核策略"
      >
        <el-table-column prop="strategy_id" label="策略 ID" width="280">
          <template #default="{ row }">
            <code class="strategy-id">{{ row.strategy_id }}</code>
          </template>
        </el-table-column>

        <el-table-column prop="submitted_at" label="提交时间" width="180">
          <template #default="{ row }">
            {{ row.submitted_at ? formatDate(row.submitted_at) : '-' }}
          </template>
        </el-table-column>

        <el-table-column prop="review_status" label="状态" width="140">
          <template #default="{ row }">
            <el-tag :type="statusTagType(row.review_status)" size="small">
              {{ statusLabel(row.review_status) }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="success" size="small" @click="handleApprove(row)">
              批准
            </el-button>
            <el-button type="danger" size="small" @click="openRejectDialog(row)">
              拒绝
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <!-- Reject Dialog -->
    <el-dialog v-model="rejectDialogVisible" title="拒绝策略" width="500px">
      <el-form :model="rejectForm" label-width="80px">
        <el-form-item label="策略 ID">
          <code>{{ rejectForm.strategy_id }}</code>
        </el-form-item>
        <el-form-item label="拒绝原因" required>
          <el-input
            v-model="rejectForm.reason"
            type="textarea"
            :rows="3"
            placeholder="请输入拒绝原因（必填）"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="rejectDialogVisible = false">取消</el-button>
        <el-button type="danger" :loading="actionLoading" @click="handleReject">
          确认拒绝
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh } from '@element-plus/icons-vue'
import type { ReviewResponse } from '@/api/review'
import { listPendingReviews, approveStrategy, rejectStrategy } from '@/api/review'

const loading = ref(false)
const actionLoading = ref(false)
const pendingReviews = ref<ReviewResponse[]>([])

const rejectDialogVisible = ref(false)
const rejectForm = ref({
  strategy_id: '',
  reason: '',
})

async function loadPendingReviews() {
  loading.value = true
  try {
    const res = await listPendingReviews()
    pendingReviews.value = res.items
  } catch (e: unknown) {
    ElMessage.error('加载待审核列表失败: ' + String(e))
  } finally {
    loading.value = false
  }
}

async function handleApprove(row: ReviewResponse) {
  try {
    await ElMessageBox.confirm(`确认批准策略 ${row.strategy_id}?`, '批准策略', {
      confirmButtonText: '批准',
      cancelButtonText: '取消',
      type: 'success',
    })
  } catch {
    return
  }

  actionLoading.value = true
  try {
    await approveStrategy({ strategy_id: row.strategy_id })
    ElMessage.success('策略已批准')
    await loadPendingReviews()
  } catch (e: unknown) {
    ElMessage.error('批准失败: ' + String(e))
  } finally {
    actionLoading.value = false
  }
}

function openRejectDialog(row: ReviewResponse) {
  rejectForm.value = { strategy_id: row.strategy_id, reason: '' }
  rejectDialogVisible.value = true
}

async function handleReject() {
  if (!rejectForm.value.reason.trim()) {
    ElMessage.warning('请输入拒绝原因')
    return
  }

  actionLoading.value = true
  try {
    await rejectStrategy({
      strategy_id: rejectForm.value.strategy_id,
      reason: rejectForm.value.reason,
    })
    ElMessage.success('策略已拒绝')
    rejectDialogVisible.value = false
    await loadPendingReviews()
  } catch (e: unknown) {
    ElMessage.error('拒绝失败: ' + String(e))
  } finally {
    actionLoading.value = false
  }
}

function statusLabel(status: string): string {
  const map: Record<string, string> = {
    pending_review: '待审核',
    approved: '已批准',
    rejected: '已拒绝',
  }
  return map[status] ?? status
}

function statusTagType(status: string): string {
  const map: Record<string, string> = {
    pending_review: 'warning',
    approved: 'success',
    rejected: 'danger',
  }
  return map[status] ?? 'info'
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

onMounted(() => {
  loadPendingReviews()
})
</script>

<style scoped>
.strategy-review-view {
  padding: 24px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}

.page-title {
  font-size: 24px;
  font-weight: 600;
  margin: 0 0 4px;
}

.page-subtitle {
  font-size: 14px;
  color: var(--el-text-color-secondary);
}

.header-right {
  display: flex;
  gap: 12px;
  align-items: center;
}

.table-container {
  background: var(--el-bg-color);
  border-radius: 8px;
  padding: 16px;
}

.strategy-id {
  font-size: 12px;
  color: var(--el-color-primary);
}
</style>
