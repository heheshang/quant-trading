// KDJ Indicator Types

export type KdjSignal = 'golden_cross' | 'death_cross' | 'overbought' | 'oversold' | 'none'

export interface KdjBar {
  open_time: number
  k: number
  d: number
  j: number
  signal: KdjSignal
}

export interface KdjParams {
  n: number
  m1: number
  m2: number
}

export interface KdjResponse {
  data: KdjBar[]
  params: KdjParams
  symbol: string
  interval: string
}

export interface KdjQueryParams {
  symbol: string
  interval: string
  start_time?: number
  end_time?: number
  n?: number
  m1?: number
  m2?: number
}
