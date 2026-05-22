import client from './client'
import type { PaginatedResponse } from '@/types'

/** 套利对类型 */
export type ArbitragePairType = 'calendar_spread' | 'cross_pair' | 'spot_futures'

/** 套利对状态 */
export type ArbitragePairStatus = 'active' | 'paused' | 'stopped'

/** 计算模式 */
export type CalculationMode = 'ratio' | 'percentage' | 'zscore'

/** 信号方向 */
export type SpreadSignal = 'long_spread' | 'short_spread' | 'neutral'

/** 仓位方向 */
export type PositionDirection = 'long_spread' | 'short_spread'

/** 仓位状态 */
export type PositionStatus = 'open' | 'closed' | 'liquidated'

/** 信号类型 */
export type SignalType = 'entry_long' | 'entry_short' | 'exit' | 'stop_loss'

/** 套利对 */
export interface ArbitragePair {
  id: number
  pair_type: ArbitragePairType
  symbol_a: string
  symbol_b: string
  exchange: string
  status: ArbitragePairStatus
  spread_entry_threshold: number
  spread_exit_threshold: number
  max_position_size: number
  calculation_mode: CalculationMode
  correlation_threshold?: number
  z_score_entry?: number
  z_score_exit?: number
  created_at: string
  updated_at: string
}

/** 创建套利对请求体 */
export interface CreateArbitragePairPayload {
  pair_type: ArbitragePairType
  symbol_a: string
  symbol_b: string
  exchange: string
  spread_entry_threshold: number
  spread_exit_threshold: number
  max_position_size: number
  calculation_mode: CalculationMode
  correlation_threshold?: number
  z_score_entry?: number
  z_score_exit?: number
}

/** 更新套利对请求体 */
export type UpdateArbitragePairPayload = Partial<CreateArbitragePairPayload>

/** 价差数据 */
export interface SpreadData {
  pair_id: number
  spread: number
  spread_pct: number
  z_score: number
  historical_mean: number
  historical_std: number
  signal: SpreadSignal
  timestamp: string
}

/** 套利持仓 */
export interface ArbitragePosition {
  id: number
  pair_id: number
  direction: PositionDirection
  size_a: number
  size_b: number
  entry_spread: number
  current_spread: number
  unrealized_pnl: number
  status: PositionStatus
  opened_at: string
  closed_at?: string
}

/** 套利信号 */
export interface ArbitrageSignal {
  id: number
  pair_id: number
  signal_type: SignalType
  spread: number
  z_score: number
  confidence: number
  executed: boolean
  created_at: string
}

// ---------------------------------------------------------------------------
// Endpoints
// ---------------------------------------------------------------------------

/** 创建套利对 */
export function createArbitragePair(
  data: CreateArbitragePairPayload,
): Promise<ArbitragePair> {
  return client.post('/arbitrage/pairs', data)
}

/** 套利对列表 */
export function listArbitragePairs(
  params?: { page?: number; size?: number; status?: ArbitragePairStatus },
): Promise<PaginatedResponse<ArbitragePair>> {
  return client.get('/arbitrage/pairs', { params })
}

/** 套利对详情 */
export function getArbitragePair(id: number): Promise<ArbitragePair> {
  return client.get(`/arbitrage/pairs/${id}`)
}

/** 更新套利对 */
export function updateArbitragePair(
  id: number,
  data: UpdateArbitragePairPayload,
): Promise<ArbitragePair> {
  return client.put(`/arbitrage/pairs/${id}`, data)
}

/** 删除套利对 */
export function deleteArbitragePair(id: number): Promise<void> {
  return client.delete(`/arbitrage/pairs/${id}`)
}

/** 当前价差 */
export function getSpread(pairId: number): Promise<SpreadData> {
  return client.get(`/arbitrage/spread/${pairId}`)
}

/** 套利持仓列表 */
export function listArbitragePositions(
  params?: { pair_id?: number; status?: PositionStatus },
): Promise<PaginatedResponse<ArbitragePosition>> {
  return client.get('/arbitrage/positions', { params })
}

/** 套利信号列表 */
export function listArbitrageSignals(
  params?: { pair_id?: number; signal_type?: SignalType },
): Promise<PaginatedResponse<ArbitrageSignal>> {
  return client.get('/arbitrage/signals', { params })
}
