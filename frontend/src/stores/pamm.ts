// pamm store — Pinia store for the in-house PAMM (Percent Allocation
// Management Module) system, PRD §6-1.
//
// Wire endpoints (full set lives in `backend/src/handlers/pamm.rs`):
//   GET    /api/v1/pamm/funds                  — list active funds
//   POST   /api/v1/pamm/funds                  — open a fund (any auth user)
//   GET    /api/v1/pamm/funds/{id}             — fund detail
//   GET    /api/v1/pamm/funds/{id}/investments — investor list (manager + self)
//   POST   /api/v1/pamm/funds/{id}/subscribe   — submit subscription
//   POST   /api/v1/pamm/funds/{id}/redeem      — submit redemption
//   POST   /api/v1/pamm/funds/{id}/distribute  — manager-only distribution
//   POST   /api/v1/pamm/funds/{id}/liquidate   — manager-only liquidation
//   GET    /api/v1/pamm/my-investments         — caller's investments
//
// We use the typed openapi-fetch client (`typedApi`) — see
// `frontend/src/api/typedClient.ts` — so request/response shapes are
// checked against the OpenAPI spec at compile time.
//
// Loading is intentionally coarse (a single `loading` ref) because
// per-call spinners live in the views themselves; the store only
// needs to guard against double-fire.

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { typedApi } from '@/api/typedClient'

// ─── Local types ──────────────────────────────────────────────────────
//
// We declare the shapes the views consume. Decimal values come back as
// strings (the backend uses `rust_decimal::Decimal` serialised via
// `to_string()` to preserve precision) — they are NOT JS numbers.
//
// These types mirror the backend DTOs in `backend/src/handlers/pamm.rs`
// and `backend/src/services/pamm.rs::FundView` etc. If the backend
// shape changes, `npm run gen:api` will regenerate
// `types/api-generated.ts` and these can be replaced with
// `components['schemas']['FundView']` references.

export interface Fund {
  id: string
  manager_id: string
  manager_username?: string | null
  name: string
  description?: string | null
  base_currency: string
  management_fee_pct: string
  performance_fee_pct: string
  high_water_mark: boolean
  nav: string
  share_value: string
  hwm: string
  total_shares: string
  strategy_id?: string | null
  status: string
  created_at: string
  updated_at: string
}

export interface Investment {
  id: string
  fund_id: string
  user_id: string
  user_username?: string | null
  share_pct: string
  initial_investment: string
  current_value: string
  cumulative_pnl?: string
  status: string
  created_at: string
  updated_at?: string
}

export interface Distribution {
  investor_id: string
  investor_username?: string | null
  share_pct: string
  amount: string
}

export const usePammStore = defineStore('pamm', () => {
  // ── State ───────────────────────────────────────────────────────────
  /** All active funds from `listFunds()`. Source of truth for the list view. */
  const funds = ref<Fund[]>([])

  /** The caller's investments from `loadMyInvestments()`. */
  const myInvestments = ref<Investment[]>([])

  /** Investor list for the currently-viewed fund (PammFundDetailView). */
  const currentInvestments = ref<Investment[]>([])

  /** Currently-viewed fund detail. Cleared by `clearCurrentFund()`. */
  const currentFund = ref<Fund | null>(null)

  /** Coarse loading flag — views track their own per-action spinners. */
  const loading = ref(false)

  /** Last error message, surfaced to a view-level el-alert. Cleared on next call. */
  const error = ref<string | null>(null)

  // ── Actions ─────────────────────────────────────────────────────────

  /** Load the list of active funds. Replaces the entire `funds` array. */
  async function listFunds(): Promise<Fund[]> {
    loading.value = true
    error.value = null
    try {
      const { data, error: err } = await typedApi.GET('/api/v1/pamm/funds')
      if (err) {
        error.value = extractMessage(err) ?? 'Failed to load funds'
        funds.value = []
        return []
      }
      // Backend wraps as `{ items: [...], total: N }`; see
      // `handlers::pamm::ListFundsResponse`.
      const body = (data as unknown as { items?: Fund[]; data?: Fund[] }) ?? {}
      const items = (body.items ?? body.data ?? []) as Fund[]
      funds.value = items
      return items
    } finally {
      loading.value = false
    }
  }

  /** Fetch a single fund. Returns the fund view, or `null` on error. */
  async function getFund(id: string): Promise<Fund | null> {
    error.value = null
    const { data, error: err } = await typedApi.GET(
      '/api/v1/pamm/funds/{id}',
      { params: { path: { id } } },
    )
    if (err) {
      error.value = extractMessage(err) ?? `Failed to load fund ${id}`
      return null
    }
    const fund = (data as unknown as Fund) ?? null
    currentFund.value = fund
    return fund
  }

  /** Open a new fund. The caller becomes its manager. */
  async function createFund(body: {
    name: string
    description?: string
    base_currency: string
    management_fee_pct: string
    performance_fee_pct: string
    high_water_mark: boolean
    strategy_id?: string | null
  }): Promise<Fund | null> {
    error.value = null
    const { data, error: err } = await typedApi.POST('/api/v1/pamm/funds', {
      body,
    })
    if (err) {
      error.value = extractMessage(err) ?? 'Failed to create fund'
      return null
    }
    const wrapped = (data as unknown as { fund?: Fund; data?: Fund }) ?? {}
    const fund = (wrapped.fund ?? wrapped.data ?? null) as Fund | null
    if (fund) {
      // Optimistically prepend to the funds list (the user just opened it).
      funds.value = [fund, ...funds.value]
    }
    return fund
  }

  /** Submit a subscription. Returns the share count awarded, or `null`. */
  async function subscribe(
    fundId: string,
    amount: number | string,
  ): Promise<{ subscription_id: string; shares: string } | null> {
    error.value = null
    const { data, error: err } = await typedApi.POST(
      '/api/v1/pamm/funds/{id}/subscribe',
      {
        params: { path: { id: fundId } },
        body: { amount: String(amount) },
      },
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Subscription failed'
      return null
    }
    // Refresh the caller's investments so My Investments reflects the new row.
    await loadMyInvestments().catch(() => {/* best effort */})
    return (data as unknown as {
      subscription_id: string
      shares: string
    }) ?? null
  }

  /** Submit a redemption. Returns the amount paid out, or `null`. */
  async function redeem(
    fundId: string,
    amount: number | string,
  ): Promise<{ redemption_id: string; amount_paid: string } | null> {
    error.value = null
    const { data, error: err } = await typedApi.POST(
      '/api/v1/pamm/funds/{id}/redeem',
      {
        params: { path: { id: fundId } },
        body: { amount: String(amount) },
      },
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Redemption failed'
      return null
    }
    await loadMyInvestments().catch(() => {/* best effort */})
    return (data as unknown as {
      redemption_id: string
      amount_paid: string
    }) ?? null
  }

  /** Manager-only: trigger a profit distribution for a period. */
  async function distribute(
    fundId: string,
    periodStart: string,
    periodEnd: string,
  ): Promise<Distribution[] | null> {
    error.value = null
    const { data, error: err } = await typedApi.POST(
      '/api/v1/pamm/funds/{id}/distribute',
      {
        params: { path: { id: fundId } },
        body: {
          period_start: periodStart,
          period_end: periodEnd,
        },
      },
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Distribution failed'
      return null
    }
    const wrapped = (data as unknown as { distributions?: Distribution[] }) ?? {}
    return (wrapped.distributions ?? []) as Distribution[]
  }

  /** Manager-only: liquidate the fund (204 No Content on success). */
  async function liquidate(fundId: string): Promise<boolean> {
    error.value = null
    const { error: err } = await typedApi.POST(
      '/api/v1/pamm/funds/{id}/liquidate',
      { params: { path: { id: fundId } } },
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Liquidation failed'
      return false
    }
    // Remove from local list.
    funds.value = funds.value.filter((f) => f.id !== fundId)
    return true
  }

  /** Load the caller's investments. */
  async function loadMyInvestments(): Promise<Investment[]> {
    error.value = null
    const { data, error: err } = await typedApi.GET(
      '/api/v1/pamm/my-investments',
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Failed to load my investments'
      myInvestments.value = []
      return []
    }
    const body = (data as unknown as { items?: Investment[]; data?: Investment[] }) ?? {}
    const items = (body.items ?? body.data ?? []) as Investment[]
    myInvestments.value = items
    return items
  }

  /** Load the investor list for a fund (manager or self only). */
  async function loadFundInvestments(fundId: string): Promise<Investment[]> {
    error.value = null
    const { data, error: err } = await typedApi.GET(
      '/api/v1/pamm/funds/{id}/investments',
      { params: { path: { id: fundId } } },
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Failed to load investments'
      currentInvestments.value = []
      return []
    }
    const body = (data as unknown as { items?: Investment[]; data?: Investment[] }) ?? {}
    const items = (body.items ?? body.data ?? []) as Investment[]
    currentInvestments.value = items
    return items
  }

  /** Clear the currently-viewed fund (used when leaving PammFundDetailView). */
  function clearCurrentFund(): void {
    currentFund.value = null
    currentInvestments.value = []
  }

  /** Wipe the entire store — used on logout so the next user doesn't see stale data. */
  function reset(): void {
    funds.value = []
    myInvestments.value = []
    currentInvestments.value = []
    currentFund.value = null
    loading.value = false
    error.value = null
  }

  return {
    // state
    funds,
    myInvestments,
    currentInvestments,
    currentFund,
    loading,
    error,
    // actions
    listFunds,
    getFund,
    createFund,
    subscribe,
    redeem,
    distribute,
    liquidate,
    loadMyInvestments,
    loadFundInvestments,
    clearCurrentFund,
    reset,
  }
})

// ── Helpers ───────────────────────────────────────────────────────────

/**
 * Extract a human-readable message from the openapi-fetch error union.
 * The error shape is `{ error: { message?: string; value?: ... } }` per
 * the openapi-fetch contract; we degrade gracefully if neither field is
 * present so the UI never shows `undefined`.
 */
function extractMessage(err: unknown): string | null {
  if (!err) return null
  if (typeof err === 'string') return err
  if (typeof err !== 'object') return null
  const e = err as { message?: unknown; value?: unknown; status?: unknown }
  if (typeof e.message === 'string' && e.message.length > 0) return e.message
  if (typeof e.status === 'number') return `HTTP ${e.status}`
  return null
}
