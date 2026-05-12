export interface Order {
  id: number
  symbol: string
  side: 'buy' | 'sell'
  type: 'limit' | 'market' | 'stop'
  price: number
  quantity: number
  filled_quantity: number
  status: 'pending' | 'filled' | 'cancelled' | 'rejected'
  created_at: string
}

export interface Position {
  id: number
  symbol: string
  side: 'long' | 'short'
  quantity: number
  entry_price: number
  current_price: number
  pnl: number
  pnl_percent: number
  liquidation_price?: number
}

export interface Trade {
  id: number
  order_id: number
  symbol: string
  side: 'buy' | 'sell'
  price: number
  quantity: number
  fee: number
  timestamp: string
}

export interface Portfolio {
  total_equity: number
  available_balance: number
  frozen_balance: number
  total_pnl: number
  positions: Position[]
}
