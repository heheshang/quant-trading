/**
 * useAIPredict - AI 预测 WebSocket Composable
 *
 * 连接 ws://localhost:8002/ws/predict/{symbol}?interval={interval}
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

const WS_BASE_URL = 'ws://localhost:8002'

export interface UseAIPredictOptions {
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
  connect: (symbol: string, interval: string) => void
  disconnect: () => void
  onMessage: (callback: (msg: WsAIPredictMessage) => void) => void
  onStatus: (callback: (status: AIWsStatus) => void) => void
}

export function useAIPredict(options: UseAIPredictOptions = {}): UseAIPredictReturn {
  const {
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
        ws.send(JSON.stringify({ type: 'ping' }))
      }
    }, heartbeatInterval)
  }

  function stopHeartbeat() {
    if (heartbeatTimer) {
      clearInterval(heartbeatTimer)
      heartbeatTimer = null
    }
  }

  function connect(symbol: string, interval: string): void {
    // 清理旧连接
    disconnect()

    currentSymbol = symbol
    currentInterval = interval

    const url = `${WS_BASE_URL}/ws/predict/${encodeURIComponent(symbol)}?interval=${encodeURIComponent(interval)}`
    notifyStatus('connecting')

    ws = new WebSocket(url)

    ws.onopen = () => {
      notifyStatus('connected')
      reconnectAttempts = 0
      startHeartbeat()
    }

    ws.onmessage = (event: MessageEvent) => {
      try {
        const msg: WsAIPredictMessage = JSON.parse(event.data)

        if (msg.type === 'heartbeat') {
          // 后端心跳响应
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
        connect(currentSymbol, currentInterval)
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