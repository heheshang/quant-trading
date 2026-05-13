import client from './client'
import type {
  StrategyFull,
  StrategyTemplate,
  CreateStrategyPayload,
  UpdateStrategyPayload,
  StrategyQueryParams,
} from '@/types'

export function listStrategies(params?: StrategyQueryParams): Promise<{ data: StrategyFull[]; meta: { page: number; size: number; total: number } }> {
  return client.get('/strategies', { params })
}

export function getStrategy(id: string): Promise<StrategyFull> {
  return client.get(`/strategies/${id}`)
}

export function createStrategy(data: CreateStrategyPayload): Promise<StrategyFull> {
  return client.post('/strategies', data)
}

export function updateStrategy(id: string, data: UpdateStrategyPayload): Promise<StrategyFull> {
  return client.put(`/strategies/${id}`, data)
}

export function deleteStrategy(id: string): Promise<void> {
  return client.delete(`/strategies/${id}`)
}

export function listTemplates(params?: { category?: string; search?: string }): Promise<StrategyTemplate[]> {
  return client.get('/strategies/templates', { params })
}

export function toggleStrategy(id: string, status: 'active' | 'paused' | 'stopped' | 'archived'): Promise<StrategyFull> {
  return client.post(`/strategies/${id}/status`, { status })
}

/** Bulk update strategy status (ADR D2) */
export function bulkUpdateStatus(ids: string[], status: 'active' | 'paused' | 'stopped' | 'archived'): Promise<{ updated: number }> {
  return client.post('/strategies/bulk/status', { ids, status })
}

/** Bulk delete strategies (ADR D2) */
export function bulkDeleteStrategies(ids: string[]): Promise<{ deleted: number }> {
  return client.post('/strategies/bulk/delete', { ids })
}

/** Export strategies as JSON (ADR D2) */
export function exportStrategies(ids?: string[]): Promise<{ filename: string; data: CreateStrategyPayload[] }> {
  return client.get('/strategies/export', { params: ids ? { ids: ids.join(',') } : undefined })
}

/** Import strategies from JSON (ADR D2) */
export function importStrategies(file: File): Promise<{ imported: number; errors: string[] }> {
  const formData = new FormData()
  formData.append('file', file)
  return client.post('/strategies/import', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
  })
}

/** T4.5: Upload strategy code file (.py/.js), returns file path */
export function uploadStrategyCode(file: File): Promise<{ path: string }> {
  const formData = new FormData()
  formData.append('file', file)
  return client.post('/strategies/code/upload', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
  })
}
