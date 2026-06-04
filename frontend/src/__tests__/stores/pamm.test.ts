// pamm store — unit tests.
//
// We mock the openapi-fetch `typedApi` directly so we don't have to
// stand up a backend. The store's responsibility is just to translate
// store actions → HTTP calls and to reflect the response in store
// state — those are the only surfaces we assert on.
//
// Two cases per the task spec:
//   1. `listFunds` calls the API and populates the `funds` ref
//   2. `subscribe` calls the API and triggers a re-load of
//      `myInvestments` so the new row shows up in My Investments

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

// `typedApi.GET/POST/...` is a chainable object — the same identity is
// returned by every call so we can stack `.mockResolvedValueOnce()` on
// it. We expose a default mock that returns `{ data: null, error: null }`
// and let each test override per-call.
const mockTypedApi = {
  GET: vi.fn(),
  POST: vi.fn(),
  PUT: vi.fn(),
  DELETE: vi.fn(),
  use: vi.fn(),
}

vi.mock('@/api/typedClient', () => ({
  typedApi: mockTypedApi,
}))

// sessionStorage is jsdom-provided; we just clear it between tests.

import { usePammStore } from '@/stores/pamm'

const fakeFund = {
  id: '11111111-2222-3333-4444-555555555555',
  manager_id: '22222222-2222-2222-2222-222222222222',
  manager_username: 'alice',
  name: 'Alpha',
  description: 'BTC momentum',
  base_currency: 'USDT',
  management_fee_pct: '0.02',
  performance_fee_pct: '0.20',
  high_water_mark: true,
  nav: '100000.00',
  share_value: '100.00',
  hwm: '100000.00',
  total_shares: '1000.00',
  strategy_id: null,
  status: 'Active',
  created_at: '2026-06-01T00:00:00Z',
  updated_at: '2026-06-01T00:00:00Z',
}

const fakeInvestment = {
  id: '99999999-9999-9999-9999-999999999999',
  fund_id: '11111111-2222-3333-4444-555555555555',
  user_id: '33333333-3333-3333-3333-333333333333',
  user_username: 'bob',
  share_pct: '0.1',
  initial_investment: '10000.00',
  current_value: '11000.00',
  cumulative_pnl: '1000.00',
  status: 'Active',
  created_at: '2026-06-01T00:00:00Z',
  updated_at: '2026-06-01T00:00:00Z',
}

describe('usePammStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    // Default: every call returns empty success so test code that
    // doesn't care about a specific endpoint doesn't blow up.
    mockTypedApi.GET.mockResolvedValue({ data: null, error: null })
    mockTypedApi.POST.mockResolvedValue({ data: null, error: null })
  })

  it('listFunds calls the API and populates the funds ref', async () => {
    mockTypedApi.GET.mockResolvedValueOnce({
      data: { items: [fakeFund], total: 1 },
      error: null,
    })

    const store = usePammStore()
    expect(store.funds).toEqual([])

    const result = await store.listFunds()

    expect(mockTypedApi.GET).toHaveBeenCalledWith('/api/v1/pamm/funds')
    expect(result).toEqual([fakeFund])
    expect(store.funds).toEqual([fakeFund])
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
  })

  it('subscribe posts to the API, returns the share count, and refreshes myInvestments', async () => {
    // 1) subscribe call → returns shares awarded
    mockTypedApi.POST.mockResolvedValueOnce({
      data: {
        subscription_id: 'sub-1',
        status: 'Confirmed',
        shares: '10.0',
        share_value: '100.00',
      },
      error: null,
    })
    // 2) cascading loadMyInvestments call inside `subscribe` →
    //    returns the new investment row
    mockTypedApi.GET.mockResolvedValueOnce({
      data: { items: [fakeInvestment], total: 1 },
      error: null,
    })

    const store = usePammStore()
    const result = await store.subscribe(
      '11111111-2222-3333-4444-555555555555',
      '1000',
    )

    // The subscribe POST was made with the right path + body shape.
    expect(mockTypedApi.POST).toHaveBeenCalledWith(
      '/api/v1/pamm/funds/{id}/subscribe',
      {
        params: { path: { id: '11111111-2222-3333-4444-555555555555' } },
        body: { amount: '1000' },
      },
    )
    expect(result).toEqual({ subscription_id: 'sub-1', shares: '10.0' })
    // And the cascade triggered loadMyInvestments, which populated the
    // store so the UI updates without an extra round-trip.
    expect(mockTypedApi.GET).toHaveBeenCalledWith('/api/v1/pamm/my-investments')
    expect(store.myInvestments).toEqual([fakeInvestment])
  })
})
