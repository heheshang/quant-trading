export interface Kline {
  timestamp: number
  open: number
  high: number
  low: number
  close: number
  volume: number
}

export interface Ticker {
  symbol: string
  price: number
  change: number
  change_percent: number
  volume: number
  high: number
  low: number
}

export interface Depth {
  bids: [number, number][]
  asks: [number, number][]
  timestamp: number
}
