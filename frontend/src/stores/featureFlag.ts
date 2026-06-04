// featureFlag store — Pinia store for the in-house feature flag system.
//
// Bootstrap path:
//   1. App starts. After auth succeeds, call `useFeatureFlagStore().load()`.
//   2. `load()` hits GET /api/v1/feature-flags which returns a per-user
//      `Record<string, boolean>` map. The server has already evaluated
//      each flag (whitelist, percentage rollout) for the current caller —
//      the frontend just caches the resulting `true` / `false` per key.
//   3. Components read flags via `useFeatureFlag(key)` (composable) which
//      wraps `store.isEnabled(key)` in a ComputedRef.
//
// The map is intentionally flat on the wire. The full admin shape (with
// description, percentage rollout, etc.) is only fetched by the admin UI
// from the `/api/v1/admin/feature-flags` endpoint — see
// `views/admin/FeatureFlagView.vue`.

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { typedApi } from '@/api/typedClient'

export const useFeatureFlagStore = defineStore('featureFlag', () => {
  /** Flat map of flag_key → evaluated boolean for the current user. */
  const flags = ref<Record<string, boolean>>({})

  /** True after a successful bootstrap load. Admin UI uses this to avoid
   *  flashing "no flags" before the first response lands. */
  const loaded = ref(false)

  /** Error from the last load attempt, if any. Cleared on next call. */
  const error = ref<string | null>(null)

  /** Fetch the per-user evaluation map. Safe to call multiple times — a
   *  successful call replaces the entire map (we don't merge because the
   *  server is the source of truth for "what flags exist for this user"). */
  async function load(): Promise<void> {
    error.value = null
    const { data, error: err } = await typedApi.GET('/api/v1/feature-flags')
    if (err) {
      // 401 is normal during the unauthenticated window — silently skip
      // and let the auth store trigger a fresh load after login.
      error.value = `Failed to load feature flags: ${String(err)}`
      return
    }
    if (data) {
      // data is FeatureFlagEvaluation { flags: Record<string, boolean> }
      flags.value = { ...data.flags }
      loaded.value = true
    }
  }

  /** Test a single flag. Returns false for unknown keys (closed-by-default
   *  is the safer default — turning a flag on should be a deliberate act,
   *  not a typo or stale cache hit). */
  function isEnabled(key: string): boolean {
    return flags.value[key] === true
  }

  /** Force-reload. Useful after the admin UI toggles a flag, or after a
   *  role change. */
  async function refresh(): Promise<void> {
    await load()
  }

  /** Reset on logout so the next user doesn't see the previous user's
   *  flag evaluations. Called by the auth store's logout flow. */
  function reset(): void {
    flags.value = {}
    loaded.value = false
    error.value = null
  }

  return {
    flags,
    loaded,
    error,
    load,
    refresh,
    isEnabled,
    reset,
  }
})
