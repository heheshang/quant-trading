/**
 * AI 预测面板类型定义
 * 对齐 Python ai-service/features.py 输出的指标字段名
 */

// ===== Signal Types =====

export type AIPredictSignal = 'strong_buy' | 'buy' | 'neutral' | 'sell' | 'strong_sell'
export type AIDirection = 'long' | 'short' | 'neutral'

// ===== Indicator Types =====
// 对齐 ai-service/features.py compute_indicators() 输出的 dict 结构

export interface AIIndicatorSnapshot {
  // RSI(14)
  rsi_14?: number
  // MACD (12, 26, 9)
  macd?: number           // MACD 快线值
  macd_signal?: number   // MACD 信号线
  macd_hist?: number     // MACD 柱状图
  // SMA / EMA
  sma_7?: number
  sma_25?: number
  ema_20?: number
  // Bollinger Bands (20, 2)
  bb_upper?: number
  bb_middle?: number
  bb_lower?: number
  // ATR(14)
  atr_14?: number
  // Volume
  volume_ma_20?: number
  // Current price
  current_price?: number
}

// ===== REST Response (POST /api/v1/predict) =====

export interface AIPredictResponse {
  symbol: string
  interval: string
  direction: AIDirection
  confidence: number          // 0-1
  signal: AIPredictSignal
  analysis: string
  price_target: number | null
  indicators: AIIndicatorSnapshot
  generated_at: string        // ISO datetime
}

// ===== WebSocket Message (ws://localhost:8002/ws/predict/{symbol}) =====

export type WsAIPredictMessageType = 'prediction' | 'heartbeat' | 'error'

export interface WsAIPredictMessage {
  type: WsAIPredictMessageType
  symbol?: string
  interval?: string
  direction?: AIDirection
  confidence?: number        // 0-1
  signal?: AIPredictSignal
  analysis?: string
  price_target?: number | null
  indicators?: AIIndicatorSnapshot
  kline_close_time?: number  // Unix timestamp ms
  generated_at?: string      // ISO datetime
  error?: string              // present when type='error'
}

// ===== WebSocket Connection Status =====

export type AIWsStatus = 'idle' | 'connecting' | 'connected' | 'disconnected' | 'reconnecting' | 'error'