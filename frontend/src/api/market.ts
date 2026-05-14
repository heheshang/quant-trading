import client from './client'
import type { Kline, Ticker, Depth } from '@/types'

export function getKline(symbol: string, interval: string): Promise<Kline[]> {
  return client.get('/market/kline', { params: { symbol, interval } })
}

/** Get all tickers (B4 P0) */
export function getTickers(): Promise<Ticker[]> {
  return client.get('/market/tickers')
}

/** Get single ticker (CR3: 新增单个查询) */
export function getTicker(symbol: string): Promise<Ticker> {
  return client.get('/market/ticker', { params: { symbol } })
}

/** Get depth data (CR3: 新增 levels 参数) */
export function getDepth(symbol: string, levels: number = 10): Promise<Depth> {
  return client.get('/market/depth', { params: { symbol, levels } })
}
