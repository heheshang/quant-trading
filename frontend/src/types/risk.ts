// Risk & Emergency Management Types — P0-F2 / P0-F3

import type { PaginatedResponse } from './index'

// ─── P0-F2: 资金风控规则 ────────────────────────────────────────

export interface RiskRules {
  /** 主键 ID */
  id: string
  /** 用户 ID */
  user_id: string
  /** 当日亏损限额（绝对值，如 1000 表示亏 1000 U） */
  daily_loss_limit: string
  /** 当日亏损超限自动平仓 */
  daily_loss_auto_close: boolean
  /** 单笔交易最大亏损比例（0.05 = 5%） */
  single_trade_loss_ratio: string
  /** 最大回撤比例（0.2 = 20%） */
  max_drawdown_ratio: string
  /** 回撤超限自动平仓 */
  drawdown_auto_close: boolean
  /** 止损类型：fixed | atr */
  stop_loss_type: string
  /** ATR 周期（stop_loss_type=atr 时生效） */
  atr_period: number | null
  /** ATR 倍数（stop_loss_type=atr 时生效） */
  atr_multiplier: string | null
  /** 规则是否激活 */
  is_active: boolean
  created_at?: string
  updated_at?: string
}

export interface UpdateRiskRulesRequest {
  daily_loss_limit?: string
  daily_loss_auto_close?: boolean
  single_trade_loss_ratio?: string
  max_drawdown_ratio?: string
  drawdown_auto_close?: boolean
  stop_loss_type?: string
  atr_period?: number | null
  atr_multiplier?: string | null
  is_active?: boolean
}

// ─── P0-F2: 风控日志 ────────────────────────────────────────────

export interface RiskLog {
  id: string
  user_id: string
  triggered_rule: string
  rule_type: string
  action: string
  severity: 'low' | 'medium' | 'high' | 'critical'
  details: string
  equity_snapshot: string
  threshold_snapshot: string
  created_at: string
}

export interface RiskLogsResponse {
  data: RiskLog[]
  total?: number
  page?: number
  page_size?: number
}

// ─── P0-F3: 应急操作 ────────────────────────────────────────────

export interface EmergencyCloseResult {
  closed_positions: number
  total_pnl: string
  closed_orders: EmergencyCloseOrder[]
}

export interface EmergencyCloseOrder {
  order_id: string
  symbol: string
  side: string
  executed_qty: string
  pnl: string
}

export interface EmergencyCloseResponse {
  success: boolean
  message: string
  closed_positions: number
  total_pnl: string
  details: EmergencyCloseOrder[]
}

export interface PauseResponse {
  paused: boolean
  reason: string
}

// ─── P0-F3: 连接状态 ────────────────────────────────────────────

export interface ConnectionStatus {
  /** Binance WebSocket 是否已连接 */
  exchange_connected: boolean
  /** 距离上次收到消息的秒数（0 = 刚收到） */
  disconnect_elapsed_secs: number
  /** 策略是否因断线被自动暂停 */
  strategy_paused: boolean
}
