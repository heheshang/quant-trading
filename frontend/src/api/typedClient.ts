// typedClient.ts — openapi-fetch-based HTTP client, fully typed against the
// generated `paths` definition from `src/types/api-generated.ts`.
//
// Why this file exists alongside `client.ts` (the legacy axios client):
//   - The frontend currently has ~115 axios call sites across 16 sibling
//     modules. A one-PR refactor of all of them is high-risk (interceptor
//     behaviour, refresh-token flow, error unwrap) and would dwarf this
//     OpenAPI close-the-loop task.
//   - The spec already exists and is consumed by `openapi-typescript`.
//     This file lets us ADOPT typed contracts incrementally:
//       • New code: `import { typedApi } from '@/api/typedClient'`
//         → `typedApi.GET('/api/v1/strategies/{id}', ...)` returns a
//           `paths[...]`-typed result with full request/response inference.
//       • Existing axios code: keeps working unchanged.
//   - When the migration completes (P3-3.5 — separate task), `client.ts`
//     can be deleted and the 19 sibling modules migrated one at a time.
//
// Auth header injection lives in middleware below; refresh-token rotation
// is intentionally NOT done here for now (existing axios code already
// owns that flow and a parallel implementation would race). The 401 →
// throw pattern is what the call sites expect, matching the legacy
// client.ts behaviour.

import createClient, { type Middleware } from 'openapi-fetch'
import type { paths } from '@/types/api-generated'

const baseUrl = (import.meta.env.VITE_API_BASE_URL as string) || '/api/v1'

// ── Token injector middleware ──────────────────────────────────────────
//
// Reads the access token from `sessionStorage` (where auth.ts writes it)
// and injects it as `Authorization: Bearer <token>`. Mirrors the request
// interceptor in `client.ts` so behaviour is identical.
const authMiddleware: Middleware = {
  async onRequest({ request }) {
    const stored = sessionStorage.getItem('auth_tokens')
    if (stored) {
      try {
        const tokens = JSON.parse(stored) as { access_token?: string }
        if (tokens.access_token) {
          // openapi-fetch's `Request` headers are immutable — build a new
          // Request with the merged header set instead of mutating in place.
          const headers = new Headers(request.headers)
          headers.set('Authorization', `Bearer ${tokens.access_token}`)
          return new Request(request, { headers })
        }
      } catch {
        // bad JSON in storage — let the server return 401
      }
    }
    return request
  },
}

export const typedApi = createClient<paths>({
  baseUrl,
})

typedApi.use(authMiddleware)

export default typedApi

// ── Usage ──────────────────────────────────────────────────────────────
//
//   import { typedApi } from '@/api/typedClient'
//
//   const { data, error } = await typedApi.GET('/api/v1/strategies/{id}', {
//     params: { path: { id: '...' } },
//   })
//
//   if (error) { ... }
//   const strategy = data  // fully typed: components['schemas']['StrategyResponse']
//
// See https://openapi-ts.dev/openapi-fetch/ for the full API surface.
