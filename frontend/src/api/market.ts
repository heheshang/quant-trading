import client from './client'
import type { Kline, Ticker, Depth } from '@/types'

export function getKline(symbol: string, interval: string): Promise<Kline[]> {
  return client.get('/market/kline', { params: { symbol, interval } })
}

export function getTickers(): Promise<Ticker[]> {
  return client.get('/market/tickers')
}

export function getDepth(symbol: string): Promise<Depth> {
  return client.get('/market/depth', { params: { symbol } })
}
