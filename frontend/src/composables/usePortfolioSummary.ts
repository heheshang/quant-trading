import { ref } from 'vue'
import * as portfolioApi from '@/api/portfolio'
import type { PortfolioSummary } from '@/types/portfolio'

export function usePortfolioSummary() {
  const summary = ref<PortfolioSummary | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetch(userId?: string) {
    loading.value = true
    error.value = null
    try {
      summary.value = await portfolioApi.getPortfolioSummary(userId)
    } catch (err: any) {
      error.value = err?.message || '获取组合汇总失败'
    } finally {
      loading.value = false
    }
  }

  function applyWsUpdate(data: Partial<PortfolioSummary>) {
    if (summary.value) {
      Object.assign(summary.value, data)
    }
  }

  return { summary, loading, error, fetch, applyWsUpdate }
}
