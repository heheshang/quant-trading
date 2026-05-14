import { ref } from 'vue'
import * as portfolioApi from '@/api/portfolio'
import type { EquityCurve, EquityGranularity } from '@/types/portfolio'

export function useEquityCurve() {
  const curve = ref<EquityCurve | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const granularity = ref<EquityGranularity>('day')

  // Default: last 30 days
  const now = new Date()
  const thirtyDaysAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000)
  const dateRange = ref<[string, string]>([
    thirtyDaysAgo.toISOString().slice(0, 10),
    now.toISOString().slice(0, 10),
  ])

  async function fetch() {
    loading.value = true
    error.value = null
    try {
      curve.value = await portfolioApi.getEquityCurve({
        start_date: dateRange.value[0],
        end_date: dateRange.value[1],
        granularity: granularity.value,
      })
    } catch (err: any) {
      error.value = err?.message || '获取权益曲线失败'
    } finally {
      loading.value = false
    }
  }

  return { curve, loading, error, granularity, dateRange, fetch }
}
