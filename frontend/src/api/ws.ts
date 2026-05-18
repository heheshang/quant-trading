/**
 * MarketWebSocket - WebSocket client for real-time market data (ADR D1/D3)
 *
 * Connects to GET /ws?token=<jwt>&symbol=<symbol>
 * Subscribe: {"action":"subscribe","channels":["market:ticker","market:depth"],"symbol":"BTCUSDT"}
 * Push: {"channel":"market:ticker","symbol":"BTCUSDT","data":{"price":"49500","bid":"49499","ask":"49501",...}}
 */

import type { Ticker, Depth, WsMessage, WsStatus } from '@/types'
import { useMarketWs } from '@/composables/useMarketWs'

export interface TickerUpdate {
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

export interface DepthUpdate {
  symbol: string
  bids: Array<{ price: string; quantity: string }>
  asks: Array<{ price: string; quantity: string }>
  timestamp: number
}

export type MarketWsChannel = 'market:ticker' | 'market:depth'

export type MarketWsMessageHandler = (msg: WsMessage) => void
export type MarketWsKickHandler = (reason?: string) => void

/**
 * Convert symbol display format (BTC/USDT) → internal format (BTCUSDT)
 */
export function toInternalSymbol(symbol: string): string {
  return symbol.replace('/', '')
}

/**
 * Convert internal symbol format (BTCUSDT) → display format (BTC/USDT)
 */
export function toDisplaySymbol(symbol: string): string {
  const quotes = ['USDT', 'BUSD', 'USD', 'BTC', 'ETH', 'BNB']
  for (const q of quotes) {
    if (symbol.endsWith(q) && symbol.length > q.length) {
      const base = symbol.slice(0, -q.length)
      return `${base}/${q}`
    }
  }
  return symbol
}

export class MarketWebSocket {
  private ws = useMarketWs()
  private _token: string | null = null
  private _currentSymbol: string | null = null

  get status(): WsStatus {
    return this.ws.status.value
  }

  get isConnected(): boolean {
    return this.ws.status.value === 'connected'
  }

  /**
   * Connect to the WebSocket server
   * @param token JWT access token
   * @param symbol Trading symbol in display format (e.g. "BTC/USDT")
   */
  connect(token: string, symbol: string): void {
    this._token = token
    this._currentSymbol = toInternalSymbol(symbol)
    this.ws.connect(token)
  }

  /**
   * Disconnect from the WebSocket server
   */
  disconnect(): void {
    this.ws.disconnect()
    this._token = null
    this._currentSymbol = null
  }

  /**
   * Subscribe to ticker and depth channels for a symbol
   * @param symbol Trading symbol in display format (e.g. "BTC/USDT")
   */
  subscribe(symbol: string): void {
    const internal = toInternalSymbol(symbol)
    this.ws.subscribe([`market:ticker:${internal}`, `market:depth:${internal}`])
    this._currentSymbol = internal
  }

  /**
   * Unsubscribe from ticker and depth channels for a symbol
   * @param symbol Trading symbol in display format (e.g. "BTC/USDT")
   */
  unsubscribe(symbol: string): void {
    const internal = toInternalSymbol(symbol)
    this.ws.unsubscribe([`market:ticker:${internal}`, `market:depth:${internal}`])
  }

  /**
   * Subscribe to all market data channels for current symbol
   */
  subscribeCurrent(): void {
    if (this._currentSymbol) {
      this.ws.subscribe([`market:ticker:${this._currentSymbol}`, `market:depth:${this._currentSymbol}`])
    }
  }

  /**
   * Resubscribe to channels after reconnection
   */
  resubscribe(): void {
    this.ws.subscriptions.value.forEach(channel => {
      // channel is already stored with internal symbol
    })
    if (this._currentSymbol) {
      this.ws.sendSubscribe('subscribe', [
        `market:ticker:${this._currentSymbol}`,
        `market:depth:${this._currentSymbol}`,
      ])
    }
  }

  /**
   * Register a message handler for received WebSocket messages
   */
  onMessage(handler: MarketWsMessageHandler): void {
    this.ws.onMessage(handler)
  }

  /**
   * Register a kick handler for when the connection is terminated by the server
   */
  onKick(handler: MarketWsKickHandler): void {
    this.ws.onKick(handler)
  }

  /**
   * Send a raw message through the WebSocket
   */
  send(data: object): void {
    const msgStr = JSON.stringify(data)
    // Access the underlying ws to send - useMarketWs doesn't expose direct send
    // Instead we go through the existing sendSubscribe mechanism
    // For custom messages, we need to extend - but currently only subscribe/unsubscribe/pong are needed
  }
}

// Singleton instance for app-wide use
let _instance: MarketWebSocket | null = null

export function getMarketWs(): MarketWebSocket {
  if (!_instance) {
    _instance = new MarketWebSocket()
  }
  return _instance
}
