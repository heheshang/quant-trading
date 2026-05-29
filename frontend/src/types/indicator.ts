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

// ─── MA (Moving Average) ─────────────────────────────────────────────────────

export interface MaBar {
  open_time: number
  ma: number
}

export interface MaResponse {
  data: MaBar[]
  period: number
  symbol: string
  interval: string
}

export interface MaQueryParams {
  symbol: string
  interval: string
  start_time?: number
  end_time?: number
  period?: number
}

// ─── MACD ─────────────────────────────────────────────────────────────────────

export interface MacdBar {
  open_time: number
  macd: number
  signal: number
  histogram: number
}

export interface MacdParams {
  fast_period: number
  slow_period: number
  signal_period: number
}

export interface MacdResponse {
  data: MacdBar[]
  params: MacdParams
  symbol: string
  interval: string
}

export interface MacdQueryParams {
  symbol: string
  interval: string
  start_time?: number
  end_time?: number
  fast_period?: number
  slow_period?: number
  signal_period?: number
}

// ─── RSI ─────────────────────────────────────────────────────────────────────

export interface RsiBar {
  open_time: number
  rsi: number
}

export interface RsiResponse {
  data: RsiBar[]
  period: number
  symbol: string
  interval: string
}

export interface RsiQueryParams {
  symbol: string
  interval: string
  start_time?: number
  end_time?: number
  period?: number
}

// ─── Bollinger Bands ────────────────────────────────────────────────────

export interface BollingerBar {
  open_time: number
  upper: number
  middle: number
  lower: number
}

export interface BollingerParams {
  period: number
  std_dev: number
}

export interface BollingerResponse {
  data: BollingerBar[]
  params: BollingerParams
  symbol: string
  interval: string
}

export interface BollingerQueryParams {
  symbol: string
  interval: string
  start_time?: number
  end_time?: number
  period?: number
  std_dev?: number
}

// ─── EMA (Exponential Moving Average) ────────────────────────────────

export interface EmaBar {
  open_time: number
  ema: number
}

export interface EmaResponse {
  data: EmaBar[]
  period: number
  symbol: string
  interval: string
}

export interface EmaQueryParams {
  symbol: string
  interval: string
  start_time?: number
  end_time?: number
  period?: number
}

// ─── ATR (Average True Range) ────────────────────────────────────────

export interface AtrBar {
  open_time: number
  atr: number
}

export interface AtrResponse {
  data: AtrBar[]
  period: number
  symbol: string
  interval: string
}

export interface AtrQueryParams {
  symbol: string
  interval: string
  start_time?: number
  end_time?: number
  period?: number
}

// ─── Stochastic ─────────────────────────────────────────────────────

export interface StochasticBar {
  open_time: number
  k: number
  d: number
}

export interface StochasticResponse {
  data: StochasticBar[]
  params: { k_period: number; d_period: number; smooth_k: number }
  symbol: string
  interval: string
}

export interface StochasticQueryParams {
  symbol: string
  interval: string
  start_time?: number
  end_time?: number
  k_period?: number
  d_period?: number
  smooth_k?: number
}
