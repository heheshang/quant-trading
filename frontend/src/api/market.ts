import client from './client'
import type { Kline, Ticker, Depth } from '@/types'
import type { Exchange } from '@/types/apiKey'

export function getKline(symbol: string, interval: string, exchange?: Exchange): Promise<Kline[]> {
  return client.get('/market/kline', { params: { symbol, interval, exchange } })
}

/** Get all tickers (B4 P0) */
export function getTickers(exchange?: Exchange): Promise<Ticker[]> {
  return client.get('/market/tickers', { params: { exchange } })
}

/** Get single ticker (CR3: 新增单个查询) */
export function getTicker(symbol: string, exchange?: Exchange): Promise<Ticker> {
  return client.get('/market/ticker', { params: { symbol, exchange } })
}

/** Get depth data (CR3: 新增 levels 参数) */
export function getDepth(symbol: string, levels: number = 10, exchange?: Exchange): Promise<Depth> {
  return client.get('/market/depth', { params: { symbol, levels, exchange } })
}