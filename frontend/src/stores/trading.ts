/**
 * Trading Store - Real-time market data state management (Pinia)
 *
 * Manages:
 * - WebSocket connection lifecycle
 * - Current symbol ticker (price, bid, ask)
 * - Order book depth (bids/asks)
 * - Connection status
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { WsStatus, WsMessage } from '@/types'
import { MarketWebSocket, getMarketWs, toInternalSymbol } from '@/api/ws'
import { useAuthStore } from '@/stores/auth'

export interface TickerData {
  symbol: string
  price: string
  bid: string
  ask: string
  change: string
  change_percent: string
  volume: string
  high: string
  low: string
  timestamp: number
}

export interface DepthLevel {
  price: string
  quantity: string
  total?: string
}

export interface DepthData {
  symbol: string
  bids: DepthLevel[]
  asks: DepthLevel[]
  timestamp: number
}

export const useTradingStore = defineStore('trading', () => {
  // === State ===

  /** Current selected symbol in display format (BTC/USDT) */
  const currentSymbol = ref<string>('BTC/USDT')

  /** Real-time ticker data keyed by internal symbol */
  const tickers = ref<Record<string, TickerData>>({})

  /** Real-time depth data keyed by internal symbol */
  const depths = ref<Record<string, DepthData>>({})

  /** WebSocket connection status */
  const wsStatus = ref<WsStatus>('idle')

  /** Whether auto-reconnect is enabled */
  const autoReconnect = ref(true)

  // === Getters ===

  /** Best bid price for current symbol */
  const bestBid = computed(() => tickers.value[currentSymbol.value]?.bid ?? null)

  /** Best ask price for current symbol */
  const bestAsk = computed(() => tickers.value[currentSymbol.value]?.ask ?? null)

  /** Last price for current symbol */
  const lastPrice = computed(() => tickers.value[currentSymbol.value]?.price ?? null)

  /** Current ticker data */
  const currentTicker = computed(() => tickers.value[currentSymbol.value] ?? null)

  /** Current depth data */
  const currentDepth = computed(() => depths.value[currentSymbol.value] ?? null)

  /** Whether WS is connected */
  const isConnected = computed(() => wsStatus.value === 'connected')

  // === Private ===

  let ws: MarketWebSocket | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null

  // === Actions ===

  /**
   * Connect WebSocket and subscribe to current symbol
   */
  function connect(): void {
    const auth = useAuthStore()
    if (!auth.token) return

    if (!ws) {
      ws = getMarketWs()
      ws.onMessage(handleWsMessage)
      ws.onKick(handleKick)
    }

    ws.connect(auth.token, currentSymbol.value)
    wsStatus.value = ws.status

    // Subscribe after connection
    if (ws.isConnected) {
      ws.subscribe(currentSymbol.value)
    }
  }

  /**
   * Disconnect WebSocket
   */
  function disconnect(): void {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    if (ws) {
      ws.disconnect()
      ws = null
    }
    wsStatus.value = 'idle'
  }

  /**
   * Switch to a different symbol
   */
  function switchSymbol(symbol: string): void {
    const oldSymbol = currentSymbol.value
    if (symbol === oldSymbol) return

    // Unsubscribe from old symbol
    if (ws && isConnected.value) {
      ws.unsubscribe(oldSymbol)
    }

    currentSymbol.value = symbol

    // Subscribe to new symbol
    if (ws && isConnected.value) {
      ws.subscribe(symbol)
    }
  }

  /**
   * Handle incoming WebSocket messages
   */
  function handleWsMessage(msg: WsMessage): void {
    // Parse channel to determine message type
    const channel = msg.channel ?? ''
    const symbolDisplay = msg.symbol ?? ''

    if (channel === 'market:ticker' && msg.data) {
      const data = msg.data as Record<string, any>
      const internal = toInternalSymbol(symbolDisplay)
      tickers.value[internal] = {
        symbol: symbolDisplay,
        price: String(data.price ?? 0),
        bid: String(data.bid ?? 0),
        ask: String(data.ask ?? 0),
        change: String(data.change ?? 0),
        change_percent: String(data.change_percent ?? 0),
        volume: String(data.volume ?? 0),
        high: String(data.high ?? 0),
        low: String(data.low ?? 0),
        timestamp: data.timestamp ?? Date.now(),
      }
      // Also store by display symbol
      tickers.value[symbolDisplay] = tickers.value[internal]
    }

    if (channel === 'market:depth' && msg.data) {
      const data = msg.data as Record<string, any>
      const internal = toInternalSymbol(symbolDisplay)
      depths.value[internal] = {
        symbol: symbolDisplay,
        bids: (data.bids ?? []).map((b: any) => ({
          price: String(b.price ?? 0),
          quantity: String(b.quantity ?? 0),
          total: b.total !== undefined ? String(b.total) : undefined,
        })),
        asks: (data.asks ?? []).map((a: any) => ({
          price: String(a.price ?? 0),
          quantity: String(a.quantity ?? 0),
          total: a.total !== undefined ? String(a.total) : undefined,
        })),
        timestamp: data.timestamp ?? Date.now(),
      }
      depths.value[symbolDisplay] = depths.value[internal]
    }
  }

  /**
   * Handle server kick event
   */
  function handleKick(reason?: string): void {
    wsStatus.value = 'disconnected'
    if (autoReconnect.value) {
      scheduleReconnect()
    }
  }

  /**
   * Schedule reconnection with exponential backoff
   */
  function scheduleReconnect(): void {
    if (reconnectTimer) return
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null
      connect()
    }, 3000)
  }

  /**
   * Update WS status (called by the composable watcher)
   */
  function updateStatus(status: WsStatus): void {
    wsStatus.value = status
    if (status === 'connected' && ws) {
      ws.resubscribe()
    }
  }

  return {
    // State
    currentSymbol,
    tickers,
    depths,
    wsStatus,
    autoReconnect,

    // Getters
    bestBid,
    bestAsk,
    lastPrice,
    currentTicker,
    currentDepth,
    isConnected,

    // Actions
    connect,
    disconnect,
    switchSymbol,
    updateStatus,
  }
})
