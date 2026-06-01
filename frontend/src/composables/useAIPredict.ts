/**
 * useAIPredict - AI 预测 WebSocket Composable
 *
 * 连接后端统一 WebSocket /api/v1/ws?token=xxx，订阅 ai:predict:{symbol}:{interval} 频道
 * - 自动重连（指数退避 1s→2s→4s→...→30s）
 * - 心跳 ping/pong
 * - 消息状态回调注册
 */

import { ref, readonly } from 'vue'
import type {
  WsAIPredictMessage,
  AIWsStatus,
  AIPredictResponse,
} from '@/types/ai'

// 后端统一 WS 地址（通过参数注入或环境变量）
const DEFAULT_WS_URL = import.meta.env.VITE_WS_URL ||
  `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/api/v1/ws`

export interface UseAIPredictOptions {
  /** 后端 WebSocket URL（默认使用 VITE_WS_URL 或相对路径 /api/v1/ws） */
  wsUrl?: string
  /** 自动重连（默认 true） */
  useAutoReconnect?: boolean
  /** 重连最大间隔（默认 30000ms） */
  maxReconnectDelay?: number
  /** 心跳间隔（默认 30000ms） */
  heartbeatInterval?: number
}

export interface UseAIPredictReturn {
  status: Readonly<ReturnType<typeof ref<AIWsStatus>>>
  latestPrediction: Readonly<ReturnType<typeof ref<AIPredictResponse | null>>>
  connect: (symbol: string, interval: string, token?: string) => void
  disconnect: () => void
  onMessage: (callback: (msg: WsAIPredictMessage) => void) => void
  onStatus: (callback: (status: AIWsStatus) => void) => void
}

export function useAIPredict(options: UseAIPredictOptions = {}): UseAIPredictReturn {
  const {
    wsUrl = DEFAULT_WS_URL,
    useAutoReconnect = true,
    maxReconnectDelay = 30000,
    heartbeatInterval = 30000,
  } = options

  const status = ref<AIWsStatus>('idle')
  const latestPrediction = ref<AIPredictResponse | null>(null)

  let ws: WebSocket | null = null
  let reconnectAttempts = 0
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let heartbeatTimer: ReturnType<typeof setInterval> | null = null
  let currentSymbol = ''
  let currentInterval = '1h'
  let currentToken = ''

  let messageCallbacks: ((msg: WsAIPredictMessage) => void)[] = []
  let statusCallbacks: ((s: AIWsStatus) => void)[] = []

  function notifyStatus(s: AIWsStatus) {
    status.value = s
    statusCallbacks.forEach(cb => cb(s))
  }

  function notifyMessage(msg: WsAIPredictMessage) {
    // 缓存最新预测
    if (msg.type === 'prediction' && msg.symbol && msg.interval) {
      const pred: AIPredictResponse = {
        symbol: msg.symbol,
        interval: msg.interval,
        direction: msg.direction ?? 'neutral',
        confidence: msg.confidence ?? 0,
        signal: msg.signal ?? 'neutral',
        analysis: msg.analysis ?? '',
        price_target: msg.price_target ?? null,
        indicators: msg.indicators ?? {},
        generated_at: msg.generated_at ?? new Date().toISOString(),
      }
      latestPrediction.value = pred
    }
    messageCallbacks.forEach(cb => cb(msg))
  }

  function getReconnectDelay(): number {
    const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), maxReconnectDelay)
    return delay
  }

  function startHeartbeat() {
    stopHeartbeat()
    heartbeatTimer = setInterval(() => {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ action: 'pong' }))
      }
    }, heartbeatInterval)
  }

  function stopHeartbeat() {
    if (heartbeatTimer) {
      clearInterval(heartbeatTimer)
      heartbeatTimer = null
    }
  }

  function connect(symbol: string, interval: string, token?: string): void {
    // 清理旧连接
    disconnect()

    currentSymbol = symbol
    currentInterval = interval
    if (token) currentToken = token

    // 构建订阅频道名
    const channel = `ai:predict:${symbol}:${interval}`
    // 若 wsUrl 中已包含 token 参数则直接使用，否则在 query 中追加 token
    let url = wsUrl
    if (token && !wsUrl.includes('token=')) {
      url = `${wsUrl}${wsUrl.includes('?') ? '&' : '?'}token=${encodeURIComponent(token)}`
    }

    notifyStatus('connecting')
    ws = new WebSocket(url)

    ws.onopen = () => {
      notifyStatus('connected')
      reconnectAttempts = 0
      // 订阅 ai:predict 频道
      ws?.send(JSON.stringify({ action: 'subscribe', channels: [channel] }))
      startHeartbeat()
    }

    ws.onmessage = (event: MessageEvent) => {
      try {
        const msg: WsAIPredictMessage & { channel?: string } = JSON.parse(event.data)

        if (msg.type === 'heartbeat') {
          // 后端心跳响应
          return
        }

        // 过滤非 ai:predict 频道的消息（支持多频道 WS 的情况）
        if (msg.channel && !msg.channel.startsWith('ai:predict:')) {
          return
        }

        notifyMessage(msg)
      } catch {
        // 忽略解析错误
      }
    }

    ws.onclose = () => {
      stopHeartbeat()
      if (status.value !== 'idle') {
        notifyStatus('disconnected')
        if (useAutoReconnect) {
          scheduleReconnect()
        }
      }
    }

    ws.onerror = () => {
      notifyStatus('error')
    }
  }

  function scheduleReconnect(): void {
    if (reconnectTimer) return
    const delay = getReconnectDelay()
    reconnectAttempts++
    notifyStatus('reconnecting')
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null
      if (currentSymbol) {
        connect(currentSymbol, currentInterval, currentToken)
      }
    }, delay)
  }

  function disconnect(): void {
    notifyStatus('idle')
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    stopHeartbeat()
    if (ws) {
      ws.onclose = null
      ws.onerror = null
      ws.close()
      ws = null
    }
    reconnectAttempts = 0
  }

  function onMessage(callback: (msg: WsAIPredictMessage) => void): void {
    messageCallbacks.push(callback)
  }

  function onStatus(callback: (status: AIWsStatus) => void): void {
    statusCallbacks.push(callback)
  }

  return {
    status: readonly(status),
    latestPrediction: readonly(latestPrediction),
    connect,
    disconnect,
    onMessage,
    onStatus,
  }
}
