// ===== Kline Data Management Types — Aligned with PRD & ADR =====

/** Kline data record (B2 aligned) */
export interface KlineData {
  open_time: number      // milliseconds timestamp
  open: number
  high: number
  low: number
  close: number
  volume: number
  close_time?: number
  quote_volume?: number  // optional USDT volume
  trades?: number         // number of trades
  is_gap?: boolean        // gap detection marker
}

/** B2: KlineResponse — GET /kline/query response */
export interface KlineResponse {
  data: KlineData[]
  meta: {
    total: number
    page: number
    page_size: number
    gap_detected?: boolean
  }
}

/** B1: KlineImportRequest — POST /kline/import body */
export interface KlineImportRequest {
  symbol: string
  interval: string
  source: 'csv' | 'api' | 'exchange'
  data?: KlineData[]   // for API import
  file?: File           // for CSV upload (browser only)
}

/** B1: KlineImportResponse — POST /kline/import response */
export interface KlineImportResult {
  imported: number
  duplicates: number
  failed: number
  errors?: string[]   // detailed error messages for failed rows
}

/** B3: KlineImportLogResponse — GET /kline/import-history */
export interface KlineImportLog {
  id: string
  symbol: string
  interval: string
  source: 'csv' | 'api' | 'exchange'
  total_rows: number
  imported_rows: number
  duplicate_rows: number
  failed_rows: number
  file_name?: string
  created_at: string
}

/** B4: KlineQualityReport — GET /kline/quality */
export interface KlineQualityReport {
  symbol: string
  interval: string
  start_time: number
  end_time: number
  total_rows: number
  valid_rows: number
  gap_count: number
  gap_positions?: number[]   // open_time list
  anomaly_count: number
  anomaly_rows?: KlineAnomalyRow[]
  duplicate_count: number
  coverage_rate: number      // 0-100 percentage
  suspicious_count: number
  corrupted_count: number
  created_at: string
}

/** Anomaly row detail */
export interface KlineAnomalyRow {
  open_time: number
  type: 'suspicious' | 'corrupted' | 'zero_volume'
  field?: string
  value?: number
  expected_range?: string
}

/** Kline query params */
export interface KlineQueryParams {
  symbol?: string
  interval?: string
  start_time?: number
  end_time?: number
  page?: number
  page_size?: number
}

/** Kline export params */
export interface KlineExportParams {
  symbol: string
  interval: string
  start_time?: number
  end_time?: number
  format: 'csv' | 'json' | 'excel'
  fields?: string[]   // which columns to include
}

/** Kline clean request */
export interface KlineCleanRequest {
  symbol: string
  interval: string
  mode: 'auto' | 'manual'
  actions?: KlineCleanAction[]
}

export interface KlineCleanAction {
  open_time: number
  action: 'delete' | 'fix'
  field?: string
  value?: number
}

/** Kline clean result */
export interface KlineCleanResult {
  filled_gaps: number
  deduplicated: number
  deleted: number
  fixed: number
  backup_id?: string
}

/** Kline fetch (exchange) request */
export interface KlineFetchRequest {
  symbol: string
  interval: string
  exchange: 'binance'
  start_time?: number
  end_time?: number
}

/** Aggregated overview row for kline symbols list (GET /kline/symbols) */
export interface KlineSymbolOverview {
  symbol: string
  interval: string
  data_points: number
  coverage_start: number   // ms timestamp
  coverage_end: number     // ms timestamp
  last_updated: string     // ISO datetime or relative time
  quality: 'normal' | 'missing' | 'anomaly' | 'duplicate' | 'suspicious'
  source?: 'csv' | 'api' | 'exchange'
}

/** Kline interval options */
export const KLINE_INTERVALS = [
  { label: '1m', value: '1m' },
  { label: '5m', value: '5m' },
  { label: '15m', value: '15m' },
  { label: '30m', value: '30m' },
  { label: '1h', value: '1h' },
  { label: '4h', value: '4h' },
  { label: '1d', value: '1d' },
  { label: '1w', value: '1w' },
] as const

/** Kline import source options */
export const KLINE_SOURCES = [
  { label: 'CSV 上传', value: 'csv' },
  { label: 'API 导入', value: 'api' },
  { label: '交易所直采', value: 'exchange' },
] as const
