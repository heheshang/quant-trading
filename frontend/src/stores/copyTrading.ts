// copyTrading store — Pinia store for the in-house Copy Trading system,
// PRD §6-2 commercialization-2.
//
// Wire endpoints (full set lives in `backend/src/handlers/copy_trading.rs`):
//   GET    /api/v1/copy-trading/traders           — list active traders
//   GET    /api/v1/copy-trading/traders/{id}      — trader detail
//   POST   /api/v1/copy-trading/register          — become a trader
//   POST   /api/v1/copy-trading/subscribe         — follow a trader
//   POST   /api/v1/copy-trading/unsubscribe       — cancel a subscription
//   GET    /api/v1/copy-trading/my-subscriptions  — caller's subscriptions
//   GET    /api/v1/copy-trading/my-trader         — caller's own trader profile + subscribers
//   GET    /api/v1/copy-trading/trades            — caller's copied trades
//   GET    /api/v1/copy-trading/profit-shares     — caller's profit-share payouts
//   POST   /api/v1/copy-trading/calculate-shares  — manager/trader triggers profit settlement
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
// These types mirror the backend DTOs in
// `backend/src/handlers/copy_trading.rs` and
// `backend/src/services/copy_trading.rs::TraderView` etc. If the
// backend shape changes, `npm run gen:api` will regenerate
// `types/api-generated.ts` and these can be replaced with
// `components['schemas']['TraderView']` references.

export interface Trader {
  id: string
  user_id: string
  /** Username of the trader (helpful for at-a-glance display). */
  username?: string | null
  display_name: string
  bio?: string | null
  total_pnl: string
  monthly_pnl: string
  /** Win rate as decimal string, e.g. "0.62" = 62%. */
  win_rate: string
  follower_count: number
  status: string
  created_at: string
  updated_at: string
}

export interface Subscription {
  id: string
  trader_id: string
  /** Optional human-friendly trader name from the join. */
  trader_name?: string | null
  follower_id: string
  ratio: string
  max_position_size: string
  max_loss_per_day: string
  status: string
  started_at: string
  ended_at?: string | null
}

export interface CopyTrade {
  id: string
  subscription_id: string
  original_order_id: string
  copied_order_id: string
  symbol: string
  side: string
  qty: string
  price: string
  status: string
  created_at: string
}

export interface ProfitShare {
  id: string
  subscription_id: string
  period_start: string
  period_end: string
  gross_pnl: string
  trader_share: string
  follower_share: string
  status: string
  created_at: string
}

export interface FanOutSummary {
  trader_id: string
  triggered_at: string
  subscriptions_processed: number
  orders_placed: number
  orders_skipped: number
  skipped_reasons: Record<string, number>
}

export interface CalculateSharesResult {
  share: ProfitShare
}

export const useCopyTradingStore = defineStore('copyTrading', () => {
  // ── State ───────────────────────────────────────────────────────────

  /** All active traders from `listTraders()`. */
  const traders = ref<Trader[]>([])

  /** The caller's subscriptions from `loadMySubscriptions()`. */
  const mySubscriptions = ref<Subscription[]>([])

  /** The caller's audit trail of copied trades from `loadMyTrades()`. */
  const myTrades = ref<CopyTrade[]>([])

  /** The caller's profit-share payouts from `loadMyProfitShares()`. */
  const profitShares = ref<ProfitShare[]>([])

  /** Currently-viewed trader detail. Cleared by `clearCurrentTrader()`. */
  const currentTrader = ref<Trader | null>(null)

  /** Coarse loading flag — views track their own per-action spinners. */
  const loading = ref(false)

  /** Last error message, surfaced to a view-level el-alert. Cleared on next call. */
  const error = ref<string | null>(null)

  // ── Actions ─────────────────────────────────────────────────────────

  /** Load the list of active traders (sorted by monthly_pnl DESC). */
  async function listTraders(): Promise<Trader[]> {
    loading.value = true
    error.value = null
    try {
      const { data, error: err } = await typedApi.GET(
        '/api/v1/copy-trading/traders',
      )
      if (err) {
        error.value = extractMessage(err) ?? 'Failed to load traders'
        traders.value = []
        return []
      }
      // Backend wraps as `{ items: [...], total: N }`; see
      // `handlers::copy_trading::ListTradersResponse`.
      const body = (data as unknown as { items?: Trader[]; data?: Trader[] }) ?? {}
      const items = (body.items ?? body.data ?? []) as Trader[]
      traders.value = items
      return items
    } finally {
      loading.value = false
    }
  }

  /** Fetch a single trader. Returns the trader view, or `null` on error. */
  async function getTrader(id: string): Promise<Trader | null> {
    error.value = null
    const { data, error: err } = await typedApi.GET(
      '/api/v1/copy-trading/traders/{id}',
      { params: { path: { id } } },
    )
    if (err) {
      error.value = extractMessage(err) ?? `Failed to load trader ${id}`
      return null
    }
    const trader = (data as unknown as Trader) ?? null
    currentTrader.value = trader
    return trader
  }

  /** Register the caller as a trader. */
  async function registerAsTrader(
    displayName: string,
    bio?: string,
  ): Promise<Trader | null> {
    error.value = null
    const { data, error: err } = await typedApi.POST(
      '/api/v1/copy-trading/register',
      {
        body: {
          display_name: displayName,
          bio: bio ?? null,
        },
      },
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Failed to register as trader'
      return null
    }
    // Backend wraps the new trader as `{ trader: TraderView }`.
    const wrapped =
      (data as unknown as { trader?: Trader; data?: Trader }) ?? {}
    const trader = (wrapped.trader ?? wrapped.data ?? null) as Trader | null
    if (trader) {
      // Optimistically prepend to the traders list (the user just registered).
      traders.value = [trader, ...traders.value]
    }
    return trader
  }

  /** Subscribe to a trader. Triggers a refresh of mySubscriptions. */
  async function subscribe(
    traderId: string,
    ratio: number | string,
    maxPosition: number | string = '0',
    maxLoss: number | string = '0',
  ): Promise<{
    subscription_id: string
    status: string
    ratio: string
    max_position_size: string
    max_loss_per_day: string
  } | null> {
    error.value = null
    const { data, error: err } = await typedApi.POST(
      '/api/v1/copy-trading/subscribe',
      {
        body: {
          trader_id: traderId,
          ratio: String(ratio),
          max_position_size: String(maxPosition),
          max_loss_per_day: String(maxLoss),
        },
      },
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Subscription failed'
      return null
    }
    // Refresh the caller's subscriptions so My Subscriptions reflects the new row.
    await loadMySubscriptions().catch(() => {/* best effort */})
    return (
      (data as unknown as {
        subscription_id: string
        status: string
        ratio: string
        max_position_size: string
        max_loss_per_day: string
      }) ?? null
    )
  }

  /** Cancel a subscription. Removes the row from mySubscriptions locally. */
  async function unsubscribe(subscriptionId: string): Promise<boolean> {
    error.value = null
    const { error: err } = await typedApi.POST(
      '/api/v1/copy-trading/unsubscribe',
      {
        body: { subscription_id: subscriptionId },
      },
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Unsubscribe failed'
      return false
    }
    // Remove the cancelled row from the local list.
    mySubscriptions.value = mySubscriptions.value.filter(
      (s) => s.id !== subscriptionId,
    )
    return true
  }

  /** Load the caller's subscriptions. */
  async function loadMySubscriptions(): Promise<Subscription[]> {
    error.value = null
    const { data, error: err } = await typedApi.GET(
      '/api/v1/copy-trading/my-subscriptions',
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Failed to load my subscriptions'
      mySubscriptions.value = []
      return []
    }
    const body =
      (data as unknown as { items?: Subscription[]; data?: Subscription[] }) ?? {}
    const items = (body.items ?? body.data ?? []) as Subscription[]
    mySubscriptions.value = items
    return items
  }

  /** Load the caller's copied-trade audit trail. */
  async function loadMyTrades(): Promise<CopyTrade[]> {
    error.value = null
    const { data, error: err } = await typedApi.GET(
      '/api/v1/copy-trading/trades',
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Failed to load my trades'
      myTrades.value = []
      return []
    }
    const body =
      (data as unknown as { items?: CopyTrade[]; data?: CopyTrade[] }) ?? {}
    const items = (body.items ?? body.data ?? []) as CopyTrade[]
    myTrades.value = items
    return items
  }

  /** Load the caller's profit-share payouts. */
  async function loadMyProfitShares(): Promise<ProfitShare[]> {
    error.value = null
    const { data, error: err } = await typedApi.GET(
      '/api/v1/copy-trading/profit-shares',
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Failed to load profit shares'
      profitShares.value = []
      return []
    }
    const body =
      (data as unknown as { items?: ProfitShare[]; data?: ProfitShare[] }) ?? {}
    const items = (body.items ?? body.data ?? []) as ProfitShare[]
    profitShares.value = items
    return items
  }

  /** Load the caller's own trader profile (returns null if they're not a trader). */
  async function loadMyTrader(): Promise<{
    trader: Trader | null
    subscribers: Subscription[]
  }> {
    error.value = null
    const { data, error: err } = await typedApi.GET(
      '/api/v1/copy-trading/my-trader',
    )
    if (err) {
      // 404 means the caller isn't a trader — not really an error.
      error.value = extractMessage(err) ?? 'Failed to load my trader profile'
      return { trader: null, subscribers: [] }
    }
    const wrapped =
      (data as unknown as {
        trader?: Trader | null
        subscribers?: Subscription[]
      }) ?? {}
    return {
      trader: (wrapped.trader ?? null) as Trader | null,
      subscribers: (wrapped.subscribers ?? []) as Subscription[],
    }
  }

  /** Manager/trader-only: trigger a profit-share settlement for one subscription. */
  async function calculateShares(
    subscriptionId: string,
    periodStart: string,
    periodEnd: string,
    traderFeePct?: string,
  ): Promise<CalculateSharesResult | null> {
    error.value = null
    const body: {
      subscription_id: string
      period_start: string
      period_end: string
      trader_fee_pct?: string
    } = {
      subscription_id: subscriptionId,
      period_start: periodStart,
      period_end: periodEnd,
    }
    if (traderFeePct !== undefined) body.trader_fee_pct = traderFeePct
    const { data, error: err } = await typedApi.POST(
      '/api/v1/copy-trading/calculate-shares',
      { body },
    )
    if (err) {
      error.value = extractMessage(err) ?? 'Profit-share calculation failed'
      return null
    }
    return (data as unknown as CalculateSharesResult) ?? null
  }

  /** Clear the currently-viewed trader (used when leaving the detail view). */
  function clearCurrentTrader(): void {
    currentTrader.value = null
  }

  /** Wipe the entire store — used on logout so the next user doesn't see stale data. */
  function reset(): void {
    traders.value = []
    mySubscriptions.value = []
    myTrades.value = []
    profitShares.value = []
    currentTrader.value = null
    loading.value = false
    error.value = null
  }

  return {
    // state
    traders,
    mySubscriptions,
    myTrades,
    profitShares,
    currentTrader,
    loading,
    error,
    // actions
    listTraders,
    getTrader,
    registerAsTrader,
    subscribe,
    unsubscribe,
    loadMySubscriptions,
    loadMyTrades,
    loadMyProfitShares,
    loadMyTrader,
    calculateShares,
    clearCurrentTrader,
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
