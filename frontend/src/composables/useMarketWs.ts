import { ref, readonly } from 'vue'
import type { WsMessage, WsSubscribe, WsPong, WsStatus } from '@/types'

/**
 * useMarketWs - WebSocket 连接生命周期管理 (Design §7.1)
 * 
 * 职责：连接/心跳/重连/订阅
 * - 指数退避重连 1s→2s→4s→...→30s（封顶 30s）
 * - 重连成功后重新订阅所有频道
 * - 收到 heartbeat → 发送 pong
 * - 收到 kick → 提示 + 断开
 */

const WS_BASE_URL = import.meta.env.VITE_WS_URL || 
  `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/api/v1/ws`

export function useMarketWs() {
  const status = ref<WsStatus>('idle')
  const subscriptions = ref<Set<string>>(new Set())
  let ws: WebSocket | null = null
  let reconnectAttempts = 0
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let messageHandler: ((msg: WsMessage) => void) | null = null
  let kickHandler: ((reason?: string) => void) | null = null

  function getReconnectDelay(): number {
    // 指数退避 1s → 2s → 4s → 8s → 16s → 30s（封顶）
    const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000)
    return delay
  }

  function connect(token: string): void {
    if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
      return
    }

    status.value = 'reconnecting'
    const url = `${WS_BASE_URL}?token=${encodeURIComponent(token)}`
    ws = new WebSocket(url)

    ws.onopen = () => {
      status.value = 'connected'
      reconnectAttempts = 0
      // 重连成功后重新订阅所有频道
      if (subscriptions.value.size > 0) {
        sendSubscribe('subscribe', Array.from(subscriptions.value))
      }
    }

    ws.onmessage = (event: MessageEvent) => {
      try {
        const msg: WsMessage = JSON.parse(event.data)

        // 心跳响应
        if (msg.type === 'heartbeat') {
          const pong: WsPong = { action: 'pong' }
          ws?.send(JSON.stringify(pong))
          return
        }

        // 被踢出
        if (msg.type === 'kick') {
          status.value = 'disconnected'
          kickHandler?.(msg.reason)
          ws?.close()
          ws = null
          return
        }

        // 订阅确认
        if (msg.type === 'subscribed' || msg.type === 'unsubscribed') {
          return
        }

        // 业务消息
        messageHandler?.(msg)
      } catch {
        // 忽略解析错误
      }
    }

    ws.onclose = () => {
      if (status.value !== 'idle') {
        status.value = 'disconnected'
        scheduleReconnect(token)
      }
    }

    ws.onerror = () => {
      status.value = 'disconnected'
      scheduleReconnect(token)
    }
  }

  function scheduleReconnect(token: string): void {
    if (reconnectTimer) return
    const delay = getReconnectDelay()
    reconnectAttempts++
    status.value = 'reconnecting'
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null
      connect(token)
    }, delay)
  }

  function disconnect(): void {
    status.value = 'idle'
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    if (ws) {
      ws.onclose = null
      ws.onerror = null
      ws.close()
      ws = null
    }
    subscriptions.value.clear()
    reconnectAttempts = 0
  }

  function sendSubscribe(action: 'subscribe' | 'unsubscribe', channels: string[]): void {
    if (!ws || ws.readyState !== WebSocket.OPEN) return
    const msg: WsSubscribe = { action, channels }
    ws.send(JSON.stringify(msg))

    if (action === 'subscribe') {
      channels.forEach(ch => subscriptions.value.add(ch))
    } else {
      channels.forEach(ch => subscriptions.value.delete(ch))
    }
  }

  function subscribe(channels: string[]): void {
    sendSubscribe('subscribe', channels)
  }

  function unsubscribe(channels: string[]): void {
    sendSubscribe('unsubscribe', channels)
  }

  function onMessage(handler: (msg: WsMessage) => void): void {
    messageHandler = handler
  }

  function onKick(handler: (reason?: string) => void): void {
    kickHandler = handler
  }

  return {
    status: readonly(status),
    subscriptions: readonly(subscriptions),
    connect,
    disconnect,
    subscribe,
    unsubscribe,
    onMessage,
    onKick,
  }
}
