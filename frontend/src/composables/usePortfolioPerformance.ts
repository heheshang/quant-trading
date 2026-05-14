import { ref } from 'vue'
import * as portfolioApi from '@/api/portfolio'
import type { PortfolioPerformance } from '@/types/portfolio'

export function usePortfolioPerformance() {
  const performance = ref<PortfolioPerformance | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetch(userId?: string) {
    loading.value = true
    error.value = null
    try {
      performance.value = await portfolioApi.getPortfolioPerformance(userId)
    } catch (err: any) {
      error.value = err?.message || '获取策略绩效失败'
    } finally {
      loading.value = false
    }
  }

  return { performance, loading, error, fetch }
}
