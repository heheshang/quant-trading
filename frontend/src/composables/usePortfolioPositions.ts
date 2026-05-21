import { ref, reactive } from 'vue'
import * as portfolioApi from '@/api/portfolio'
import type { PaginatedPositions, PortfolioPosition, PositionSide } from '@/types/portfolio'

export function usePortfolioPositions() {
  const positions = ref<PaginatedPositions | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const filters = reactive({
    symbol: '',
    side: '' as '' | PositionSide,
  })
  const pagination = reactive({
    page: 1,
    size: 10,
  })

  async function fetch() {
    loading.value = true
    error.value = null
    try {
      const params: Record<string, unknown> = {
        page: pagination.page,
        size: pagination.size,
      }
      if (filters.symbol) params.symbol = filters.symbol
      if (filters.side) params.side = filters.side
      positions.value = await portfolioApi.listPortfolioPositions(params)
    } catch (err: unknown) {
      error.value = err instanceof Error ? err.message : String(err)
    } finally {
      loading.value = false
    }
  }

  function applyWsUpdate(position: PortfolioPosition) {
    if (!positions.value) return
    const idx = positions.value.items.findIndex(
      (p) => p.symbol === position.symbol && p.side === position.side,
    )
    if (idx >= 0) {
      positions.value.items[idx] = position
    } else {
      positions.value.items.unshift(position)
      positions.value.total++
    }
  }

  return { positions, loading, error, filters, pagination, fetch, applyWsUpdate }
}
