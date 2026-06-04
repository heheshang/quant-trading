// copyTrading store — unit tests.
//
// We mock the openapi-fetch `typedApi` directly so we don't have to
// stand up a backend. The store's responsibility is just to translate
// store actions → HTTP calls and to reflect the response in store
// state — those are the only surfaces we assert on.
//
// Two cases per the task spec:
//   1. `listTraders` calls the API and populates the `traders` ref
//   2. `subscribe` calls the API, returns the new subscription id, and
//      triggers a re-load of `mySubscriptions` so the new row shows up
//      in My Subscriptions.

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

// `typedApi.GET/POST/...` is a chainable object — the same identity is
// returned by every call so we can stack `.mockResolvedValueOnce()` on
// it. We expose a default mock that returns `{ data: null, error: null }`
// and let each test override per-call.
//
// `vi.hoisted` is required because `vi.mock(...)` is hoisted to the top
// of the file (before any `const` / `let`), and the factory must
// reference the same mock object the tests use.
const { mockTypedApi } = vi.hoisted(() => {
  return {
    mockTypedApi: {
      GET: vi.fn(),
      POST: vi.fn(),
      PUT: vi.fn(),
      DELETE: vi.fn(),
      use: vi.fn(),
    },
  }
})

vi.mock('@/api/typedClient', () => ({
  typedApi: mockTypedApi,
}))

// sessionStorage is jsdom-provided; we just clear it between tests.

import { useCopyTradingStore, type Trader } from '@/stores/copyTrading'

const fakeTrader: Trader = {
  id: '11111111-2222-3333-4444-555555555555',
  user_id: '22222222-2222-2222-2222-222222222222',
  username: 'alice',
  display_name: 'Alpha Momentum',
  bio: 'BTC momentum strategy',
  total_pnl: '12500.00',
  monthly_pnl: '1840.00',
  win_rate: '0.62',
  follower_count: 12,
  status: 'Active',
  created_at: '2026-06-01T00:00:00Z',
  updated_at: '2026-06-01T00:00:00Z',
}

const fakeSubscription = {
  id: '99999999-9999-9999-9999-999999999999',
  trader_id: '11111111-2222-3333-4444-555555555555',
  trader_name: 'Alpha Momentum',
  follower_id: '33333333-3333-3333-3333-333333333333',
  ratio: '0.10',
  max_position_size: '0',
  max_loss_per_day: '0',
  status: 'Active',
  started_at: '2026-06-01T00:00:00Z',
  ended_at: null,
}

describe('useCopyTradingStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    // Default: every call returns empty success so test code that
    // doesn't care about a specific endpoint doesn't blow up.
    mockTypedApi.GET.mockResolvedValue({ data: null, error: null })
    mockTypedApi.POST.mockResolvedValue({ data: null, error: null })
  })

  it('listTraders calls the API and populates the traders ref', async () => {
    mockTypedApi.GET.mockResolvedValueOnce({
      data: { items: [fakeTrader], total: 1 },
      error: null,
    })

    const store = useCopyTradingStore()
    expect(store.traders).toEqual([])

    const result = await store.listTraders()

    expect(mockTypedApi.GET).toHaveBeenCalledWith('/api/v1/copy-trading/traders')
    expect(result).toEqual([fakeTrader])
    expect(store.traders).toEqual([fakeTrader])
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
  })

  it('subscribe posts to the API, returns the subscription id, and refreshes mySubscriptions', async () => {
    // 1) subscribe call → returns the new subscription
    mockTypedApi.POST.mockResolvedValueOnce({
      data: {
        subscription_id: '99999999-9999-9999-9999-999999999999',
        status: 'Active',
        ratio: '0.10',
        max_position_size: '0',
        max_loss_per_day: '0',
      },
      error: null,
    })
    // 2) cascading loadMySubscriptions call inside `subscribe` →
    //    returns the new subscription row
    mockTypedApi.GET.mockResolvedValueOnce({
      data: { items: [fakeSubscription], total: 1 },
      error: null,
    })

    const store = useCopyTradingStore()
    const result = await store.subscribe(
      '11111111-2222-3333-4444-555555555555',
      '0.10',
      '0',
      '0',
    )

    // The subscribe POST was made with the right path + body shape.
    expect(mockTypedApi.POST).toHaveBeenCalledWith(
      '/api/v1/copy-trading/subscribe',
      {
        body: {
          trader_id: '11111111-2222-3333-4444-555555555555',
          ratio: '0.10',
          max_position_size: '0',
          max_loss_per_day: '0',
        },
      },
    )
    expect(result).toEqual({
      subscription_id: '99999999-9999-9999-9999-999999999999',
      status: 'Active',
      ratio: '0.10',
      max_position_size: '0',
      max_loss_per_day: '0',
    })
    // And the cascade triggered loadMySubscriptions, which populated the
    // store so the UI updates without an extra round-trip.
    expect(mockTypedApi.GET).toHaveBeenCalledWith(
      '/api/v1/copy-trading/my-subscriptions',
    )
    expect(store.mySubscriptions).toEqual([fakeSubscription])
  })
})
