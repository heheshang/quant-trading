<template>
  <div class="strategy-templates-view">
    <!-- Page Header -->
    <div class="page-header">
      <div class="header-left">
        <el-button class="back-btn" text @click="router.push({ name: 'Strategies' })">
          <el-icon><ArrowLeft /></el-icon>
          返回
        </el-button>
        <h1 class="page-title">策略模板市场</h1>
      </div>
    </div>

    <p class="page-desc">浏览社区模板，快速创建量化策略</p>

    <!-- Loading State -->
    <template v-if="loading">
      <div class="template-grid">
        <el-skeleton v-for="i in 6" :key="i" animated class="template-skeleton" />
      </div>
    </template>

    <!-- Error State -->
    <template v-else-if="error">
      <el-card class="state-card" shadow="never">
        <el-empty description="加载失败，请重试">
          <el-button type="primary" @click="loadTemplates">重新加载</el-button>
        </el-empty>
      </el-card>
    </template>

    <!-- Content -->
    <template v-else>
      <!-- Category Filter -->
      <div class="category-filter">
        <el-radio-group v-model="filterCategory" class="category-pills">
          <el-radio-button value="">全部</el-radio-button>
          <el-radio-button value="趋势跟踪">趋势跟踪</el-radio-button>
          <el-radio-button value="均值回归">均值回归</el-radio-button>
          <el-radio-button value="网格交易">网格交易</el-radio-button>
          <el-radio-button value="套利">套利</el-radio-button>
        </el-radio-group>
      </div>

      <!-- Templates Grid -->
      <div class="template-grid">
        <el-card
          v-for="tpl in filteredTemplates"
          :key="tpl.id"
          class="template-card"
          shadow="hover"
          @click="useTemplate(tpl)"
        >
          <div class="tpl-icon">
            <el-icon :size="32"><component :is="getTemplateIcon(tpl.category)" /></el-icon>
          </div>
          <div class="tpl-name">{{ tpl.name }}</div>
          <div class="tpl-desc">{{ tpl.description }}</div>
          <div class="tpl-meta">
            <el-tag size="small" effect="plain" :type="getCategoryTagType(tpl.category)">
              {{ tpl.category }}
            </el-tag>
            <span class="tpl-param-count">{{ tpl.parameter_schema?.length || 0 }} 参数</span>
          </div>
          <div class="tpl-footer">
            <el-button type="primary" size="small" @click.stop="useTemplate(tpl)">
              使用此模板
            </el-button>
          </div>
        </el-card>
      </div>

      <!-- Empty Category State -->
      <template v-if="filteredTemplates.length === 0">
        <el-card class="state-card" shadow="never">
          <el-empty :description="`暂无${filterCategory}类模板`" />
        </el-card>
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowLeft, TrendCharts, Grid, DataAnalysis, Connection, Coin } from '@element-plus/icons-vue'
import { listTemplates } from '@/api/strategies'
import type { StrategyTemplate } from '@/types'

const router = useRouter()

const templates = ref<StrategyTemplate[]>([])
const loading = ref(false)
const error = ref(false)
const filterCategory = ref('')

const filteredTemplates = computed(() => {
  if (!filterCategory.value) return templates.value
  return templates.value.filter(t => t.category === filterCategory.value)
})

function getCategoryTagType(category: string): string {
  const map: Record<string, string> = {
    趋势跟踪: 'primary',
    均值回归: '',
    网格交易: 'success',
    套利: 'warning',
    自定义: 'info',
    趋势: 'primary',
    震荡: 'success',
  }
  return map[category] || 'info'
}

function getTemplateIcon(category: string): any {
  const icons: Record<string, any> = {
    趋势跟踪: TrendCharts,
    趋势: TrendCharts,
    均值回归: DataAnalysis,
    网格交易: Grid,
    套利: Connection,
    自定义: Coin,
  }
  return icons[category] || TrendCharts
}

function useTemplate(tpl: StrategyTemplate) {
  // Navigate to create with pre-selected template
  router.push({ name: 'StrategyCreate', query: { template: tpl.id } })
}

async function loadTemplates() {
  loading.value = true
  error.value = false
  try {
    templates.value = await listTemplates()
  } catch {
    error.value = true
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadTemplates()
})
</script>

<style scoped>
.strategy-templates-view {
  padding: 0 0 40px;
}

.page-header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 8px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.back-btn {
  color: var(--color-text-secondary);
}

.page-title {
  font-size: 22px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0;
}

.page-desc {
  color: var(--color-text-secondary);
  margin: 0 0 24px;
  font-size: 14px;
}

.category-filter {
  margin-bottom: 24px;
}

.category-pills {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.template-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 20px;
}

.template-skeleton {
  height: 220px;
  border-radius: 8px;
}

.template-card {
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
  border-radius: 8px;
}

.template-card:hover {
  transform: translateY(-2px);
}

.tpl-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  background: var(--color-surface-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 16px;
  color: var(--color-primary);
}

.tpl-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin-bottom: 8px;
}

.tpl-desc {
  font-size: 13px;
  color: var(--color-text-secondary);
  margin-bottom: 16px;
  line-height: 1.5;
  min-height: 40px;
}

.tpl-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.tpl-param-count {
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.tpl-footer {
  padding-top: 12px;
  border-top: 1px solid var(--color-border);
}

.state-card {
  border-radius: 8px;
  margin-top: 16px;
}
</style>