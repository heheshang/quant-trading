// ===== Market Types (ADR D1/D2/D3 + Contract-Review CR1/CR2/CR3) =====

/** Ticker type - ADR D1: PRD field naming + numeric types */
export interface Ticker {
  symbol: string          // 交易对（内部格式 BTCUSDT）
  price: number           // 最新价 f64
  change: number          // 24h涨跌额 f64
  change_percent: number  // 24h涨跌幅 f64
  volume: number          // 24h成交量 f64
  high: number            // 24h最高 f64
  low: number             // 24h最低 f64
  bid: number             // 买一价 f64  [CR1: 前端缺失]
  ask: number             // 卖一价 f64  [CR1: 前端缺失]
  timestamp: number       // 毫秒时间戳 i64  [CR1: 前端缺失]
}

/** Depth level - ADR D2: 结构化对象，非二维数组 */
export interface DepthLevel {
  price: number       // 价格
  quantity: number    // 数量
  total: number       // 累计量
}

/** Depth type - ADR D2: bids/asks 为 DepthLevel[] (CR2) */
export interface Depth {
  bids: DepthLevel[]  // 买盘（价格降序）
  asks: DepthLevel[]  // 卖盘（价格升序）
  timestamp: number
}

// ===== WS Message Types (B3 对齐) =====

export type WsMessageType = 'ticker' | 'depth' | 'depth_update' | 'kline' | 'heartbeat' | 'subscribed' | 'unsubscribed' | 'kick' | 'error' | 'trade_executed'

export interface WsMessage {
  type: WsMessageType
  symbol?: string
  data?: Ticker | Depth | TradeExecutedData
  ts?: number
  channel?: string
  reason?: string
  code?: number
  message?: string
}

export interface TradeExecutedData {
  order_id: string
  side: string
  filled_quantity: number
  avg_fill_price: number
  is_fully_filled: boolean
  realized_pnl: number | null
}

export interface WsSubscribe {
  action: 'subscribe' | 'unsubscribe'
  channels: string[]  // e.g. ['ticker:BTCUSDT', 'depth:ETHUSDT']
}

export interface WsPong {
  action: 'pong'
}

/** WS connection status */
export type WsStatus = 'connected' | 'disconnected' | 'reconnecting' | 'idle'

// ===== Symbol metadata for precision =====

export interface SymbolMetadata {
  symbol: string
  price_precision: number
  quantity_precision: number
  display_name: string  // e.g. "BTC/USDT"
}

// ===== Known symbol names mapping =====

export const SYMBOL_NAMES: Record<string, string> = {
  BTCUSDT: 'Bitcoin',
  ETHUSDT: 'Ethereum',
  BNBUSDT: 'BNB',
  SOLUSDT: 'Solana',
  XRPUSDT: 'XRP',
  DOGEUSDT: 'Dogecoin',
  ADAUSDT: 'Cardano',
  AVAXUSDT: 'Avalanche',
  DOTUSDT: 'Polkadot',
  LINKUSDT: 'Chainlink',
}

/** Format internal symbol to display format: BTCUSDT → BTC/USDT (D3) */
export function formatSymbol(symbol: string): string {
  // Common quote currencies
  const quotes = ['USDT', 'BUSD', 'USD', 'BTC', 'ETH', 'BNB']
  for (const q of quotes) {
    if (symbol.endsWith(q) && symbol.length > q.length) {
      const base = symbol.slice(0, -q.length)
      return `${base}/${q}`
    }
  }
  return symbol
}
