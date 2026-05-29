import client from './client'
import type { KdjQueryParams, KdjResponse, MaQueryParams, MaResponse, MacdQueryParams, MacdResponse, RsiQueryParams, RsiResponse, BollingerQueryParams, BollingerResponse, EmaQueryParams, EmaResponse, AtrQueryParams, AtrResponse, StochasticQueryParams, StochasticResponse } from '@/types/indicator'

/**
 * Calculate KDJ indicator for a symbol/interval.
 * GET /api/v1/kline/kdj
 */
export function getKdj(params: KdjQueryParams): Promise<KdjResponse> {
  return client.get('/kline/kdj', { params })
}

/**
 * Calculate MA (Moving Average) for a symbol/interval.
 * GET /api/v1/kline/ma
 */
export function getMa(params: MaQueryParams): Promise<MaResponse> {
  return client.get('/kline/ma', { params })
}

/**
 * Calculate MACD for a symbol/interval.
 * GET /api/v1/kline/macd
 */
export function getMacd(params: MacdQueryParams): Promise<MacdResponse> {
  return client.get('/kline/macd', { params })
}

/**
 * Calculate RSI for a symbol/interval.
 * GET /api/v1/kline/rsi
 */
export function getRsi(params: RsiQueryParams): Promise<RsiResponse> {
  return client.get('/kline/rsi', { params })
}

/**
 * Calculate Bollinger Bands for a symbol/interval.
 * GET /api/v1/kline/bollinger
 */
export function getBollinger(params: BollingerQueryParams): Promise<BollingerResponse> {
  return client.get('/kline/bollinger', { params })
}

/**
 * Calculate EMA (Exponential Moving Average) for a symbol/interval.
 * GET /api/v1/kline/ema
 */
export function getEma(params: EmaQueryParams): Promise<EmaResponse> {
  return client.get('/kline/ema', { params })
}

/**
 * Calculate ATR (Average True Range) for a symbol/interval.
 * GET /api/v1/kline/atr
 */
export function getAtr(params: AtrQueryParams): Promise<AtrResponse> {
  return client.get('/kline/atr', { params })
}

/**
 * Calculate Stochastic Oscillator for a symbol/interval.
 * GET /api/v1/kline/stochastic
 */
export function getStochastic(params: StochasticQueryParams): Promise<StochasticResponse> {
  return client.get('/kline/stochastic', { params })
}
