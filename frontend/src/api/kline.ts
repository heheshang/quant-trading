import client from './client'
import type {
  KlineData,
  KlineQueryParams,
  KlineImportRequest,
  KlineImportResult,
  KlineImportLog,
  KlineQualityReport,
  KlineExportParams,
  KlineCleanRequest,
  KlineCleanResult,
  KlineFetchRequest,
  KlineResponse,
  KlineSymbolOverview,
} from '@/types/kline'
import type { PaginatedResponse } from '@/types'

/**
 * Query kline data with pagination and time range filter.
 * GET /api/v1/kline/query
 */
export function queryKlines(params: KlineQueryParams): Promise<KlineResponse> {
  return client.get('/kline/query', { params })
}

/**
 * Get latest kline data point.
 * GET /api/v1/kline/latest
 */
export function getLatestKline(symbol: string, interval: string): Promise<KlineData | null> {
  return client.get('/kline/latest', { params: { symbol, interval } })
}

/**
 * Import kline data via JSON body (API import).
 * POST /api/v1/kline/import
 */
export function importKlines(payload: KlineImportRequest): Promise<KlineImportResult> {
  return client.post('/kline/import', payload)
}

/**
 * Upload CSV file for kline import.
 * POST /api/v1/kline/import/csv  (multipart/form-data)
 */
export function uploadKlinesCSV(symbol: string, interval: string, file: File): Promise<KlineImportResult> {
  const formData = new FormData()
  formData.append('symbol', symbol)
  formData.append('interval', interval)
  formData.append('file', file)
  return client.post('/kline/import/csv', formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
  })
}

/**
 * Fetch kline data from exchange (pro-trader only).
 * POST /api/v1/kline/fetch
 */
export function fetchKlinesFromExchange(payload: KlineFetchRequest): Promise<KlineImportResult> {
  return client.post('/kline/fetch', payload)
}

/**
 * Get import history logs.
 * GET /api/v1/kline/import-history
 */
export function getImportHistory(params?: {
  symbol?: string
  interval?: string
  page?: number
  size?: number
}): Promise<PaginatedResponse<KlineImportLog>> {
  return client.get('/kline/import-history', { params })
}

/**
 * Get data quality report.
 * GET /api/v1/kline/quality
 */
export function getQualityReport(symbol: string, interval: string): Promise<KlineQualityReport> {
  return client.get('/kline/quality', { params: { symbol, interval } })
}

/**
 * Execute data cleaning.
 * POST /api/v1/kline/clean
 */
export function cleanKlines(payload: KlineCleanRequest): Promise<KlineCleanResult> {
  return client.post('/kline/clean', payload)
}

/**
 * Rollback to pre-clean snapshot.
 * DELETE /api/v1/kline/clean/rollback
 */
export function rollbackClean(symbol: string, interval: string, backupId: string): Promise<{ restored_rows: number }> {
  return client.delete('/kline/clean/rollback', {
    params: { symbol, interval, backup_id: backupId },
  })
}

/**
 * Export kline data as CSV or JSON.
 * GET /api/v1/kline/export — returns a download blob
 */
export function exportKlines(params: KlineExportParams): Promise<Blob> {
  return client.get('/kline/export', { params, responseType: 'blob' })
}

/**
 * List available symbols for current user.
 * GET /api/v1/kline/symbols
 */
export function getKlineSymbols(): Promise<KlineSymbolOverview[]> {
  return client.get('/kline/symbols')
}
