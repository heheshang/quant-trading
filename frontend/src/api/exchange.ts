// Exchange API Service — P0-F1 API Signing
// Base path: /api/v1/exchange

import type {
  ApiResponse,
  ExchangePingResponse,
  ExchangeAccountResponse,
  ExchangeOrderResponse,
  ExchangeCancelResponse,
  ExchangeRateLimitResponse,
  CreateExchangeOrderRequest,
  CancelExchangeOrderRequest,
} from '@/types/exchange'

const BASE = '/api/v1/exchange'

async function fetchJson<T>(url: string, options?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  })
  if (!res.ok) {
    const body = await res.text()
    throw new Error(`Exchange API ${res.status}: ${body}`)
  }
  const json: ApiResponse<T> = await res.json()
  return json.data
}

// GET /api/v1/exchange/ping
export async function exchangePing(): Promise<ExchangePingResponse> {
  const res = await fetchJson<ExchangePingResponse>(`${BASE}/ping`)
  return res
}

// GET /api/v1/exchange/account
export async function exchangeAccount(): Promise<ExchangeAccountResponse> {
  const res = await fetchJson<ExchangeAccountResponse>(`${BASE}/account`)
  return res
}

// POST /api/v1/exchange/order
export async function createExchangeOrder(
  payload: CreateExchangeOrderRequest,
): Promise<ExchangeOrderResponse> {
  return fetchJson<ExchangeOrderResponse>(`${BASE}/order`, {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

// DELETE /api/v1/exchange/order/:orderId
export async function cancelExchangeOrder(
  orderId: string,
  payload: CancelExchangeOrderRequest,
): Promise<ExchangeCancelResponse> {
  return fetchJson<ExchangeCancelResponse>(`${BASE}/order/${orderId}`, {
    method: 'DELETE',
    body: JSON.stringify(payload),
  })
}

// GET /api/v1/exchange/rate-limit
export async function exchangeRateLimit(): Promise<ExchangeRateLimitResponse> {
  return fetchJson<ExchangeRateLimitResponse>(`${BASE}/rate-limit`)
}
