import client from './client'
import type { KdjQueryParams, KdjResponse } from '@/types/indicator'

/**
 * Calculate KDJ indicator for a symbol/interval.
 * GET /api/v1/kline/kdj
 */
export function getKdj(params: KdjQueryParams): Promise<KdjResponse> {
  return client.get('/kline/kdj', { params })
}
