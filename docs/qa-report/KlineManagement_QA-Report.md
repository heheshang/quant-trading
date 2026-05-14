# TC-A: Market Module API Testing — Deep Analysis Report

**Date:** 2026-05-14  
**Scope:** Backend handlers, services, schemas; Frontend API client, types, routes  
**Files Analyzed:**
- `backend/src/handlers/market.rs`
- `backend/src/services/market_data.rs`
- `backend/src/models/market_schemas.rs`
- `frontend/src/api/market.ts`
- `backend/src/main.rs` (routes section, lines 144–155)
- `frontend/src/types/market.ts`
- `frontend/src/api/client.ts` (response unwrapping logic)
- `backend/src/utils/response.rs` / `error.rs`

---

## Findings

### F-01: Frontend `getKline` calls non-existent backend endpoint `/market/kline`

| Field | Value |
|---|---|
| **Severity** | **P0** |
| **Description** | `frontend/src/api/market.ts:4-6` calls `client.get('/market/kline', { params: { symbol, interval } })` but the backend has **no** `/market/kline` route. The market routes in `main.rs:145-152` only register: `/market/tickers`, `/market/ticker`, `/market/depth`, `/market/ticker/history`. The kline endpoint lives under a separate router at `/kline/query` (main.rs:134). Every frontend call to `getKline()` will receive a 404. |
| **File:Line** | `frontend/src/api/market.ts:4-6`; `backend/src/main.rs:145-152` |
| **Reproduction** | 1. Call `getKline('BTCUSDT', '1d')` from frontend. 2. Observe HTTP 404 from `GET /api/v1/market/kline?symbol=BTCUSDT&interval=1d`. |
| **Fix** | Either (a) add `/market/kline` route in backend pointing to `handlers::kline::query_klines`, or (b) change frontend to call `/kline/query` with proper params. The two endpoints are semantically different (kline OHLCV vs ticker snapshots), so option (a) with a dedicated market-kline handler is recommended. |

---

### F-02: Mock depth generates negative prices for low-value symbols (DOGEUSDT, XRPUSDT)

| Field | Value |
|---|---|
| **Severity** | **P0** |
| **Description** | `market_data.rs:117` generates bids as `price = base_price - (i + 1) * 0.5`. For DOGEUSDT (base_price=0.2385), the very first bid level (i=0) is `0.2385 - 0.5 = -0.2615` — a **negative price**. For XRPUSDT (base_price=2.453), bids go negative at level 5 (`2.453 - 3.0 = -0.547`). With default levels=10, both symbols produce multiple negative-priced bid entries, which is invalid market data. |
| **File:Line** | `backend/src/services/market_data.rs:117` (bids), `:131` (asks) |
| **Reproduction** | 1. `GET /api/v1/market/depth?symbol=DOGEUSDT&levels=10`. 2. Inspect `data.bids[0].price` → `-0.2615`. 3. Same for XRPUSDT with levels≥5. |
| **Fix** | Scale the price increment by a percentage of base_price (e.g., `0.001 * base_price` per level) instead of a fixed $0.5. |

---

### F-03: `BNBUSUSDT` typo in MarketView.vue WebSocket subscription

| Field | Value |
|---|---|
| **Severity** | **P0** |
| **Description** | `frontend/src/views/market/MarketView.vue:104` subscribes to `ticker:BNBUSUSDT` (double "US"). The backend only recognizes `BNBUSDT`. The WS subscription for this symbol will either silently fail or produce a channel that never receives data. |
| **File:Line** | `frontend/src/views/market/MarketView.vue:104` |
| **Reproduction** | 1. Open MarketView, observe WS subscribe message. 2. Channel `ticker:BNBUSUSDT` receives no data. 3. BNB row stays stale. |
| **Fix** | Change `BNBUSUSDT` → `BNBUSDT`. |

---

### F-04: Frontend `getKline` vs backend `ticker/history` — semantic mismatch

| Field | Value |
|---|---|
| **Severity** | **P1** |
| **Description** | Frontend `getKline` expects OHLCV candlestick data (type `Kline { timestamp, open, high, low, close, volume }`), but the only vaguely related backend endpoint is `/market/ticker/history` which returns **ticker snapshots** (`TickerSnapshotResponse` — has price/change/volume but NO open/high/low/close candlestick fields). Even if `ticker/history` were implemented (currently P1 stub returning 500), its response schema is incompatible with the `Kline` type. The frontend will never get the data shape it expects. |
| **File:Line** | `frontend/src/api/market.ts:4-6`; `frontend/src/types/index.ts:87-94`; `backend/src/models/market_schemas.rs:61-72` |
| **Reproduction** | 1. If `ticker/history` were implemented, call `getKline('BTCUSDT', '1d')`. 2. Response contains `TickerSnapshotResponse` objects — missing `open`, `close` fields. 3. Frontend fails to render chart. |
| **Fix** | Implement a proper `/market/kline` endpoint returning OHLCV data, distinct from `ticker/history`. |

---

### F-05: Frontend SYMBOL_NAMES (10 symbols) vs backend SUPPORTED_SYMBOLS (6 symbols) mismatch

| Field | Value |
|---|---|
| **Severity** | **P1** |
| **Description** | `frontend/src/types/market.ts:69-80` defines 10 symbols (BTCUSDT, ETHUSDT, BNBUSDT, SOLUSDT, XRPUSDT, DOGEUSDT, **ADAUSDT, AVAXUSDT, DOTUSDT, LINKUSDT**). `backend/src/services/market_data.rs:8` supports only 6 symbols (missing ADA, AVAX, DOT, LINK). Any frontend request for the 4 extra symbols will receive 404 from the backend. UI dropdowns/metadata display these symbols but they cannot be queried. |
| **File:Line** | `frontend/src/types/market.ts:76-78`; `backend/src/services/market_data.rs:8` |
| **Reproduction** | 1. Call `getTicker('ADAUSDT')`. 2. Backend returns 404 "交易对 ADAUSDT 不存在". |
| **Fix** | Synchronize symbol lists. Either add the 4 symbols to backend mock data or remove them from frontend until backend supports them. A shared configuration or API to query supported symbols would prevent drift. |

---

### F-06: Mock bid/ask spread is unrealistic for low-price symbols

| Field | Value |
|---|---|
| **Severity** | **P1** |
| **Description** | `market_data.rs:28-29` calculates `bid = price - 0.5` and `ask = price + 0.5`. For BTC at $103,250, a $1 spread (~0.001%) is reasonable. For DOGE at $0.2385, a $1 spread is **419%** of the price. For XRP at $2.45, a $1 spread is **40%**. Realistic spreads for these should be <0.1%. This makes mock data unusable for any price-sensitive frontend component (order entry, P&L). |
| **File:Line** | `backend/src/services/market_data.rs:28-29` |
| **Reproduction** | 1. `GET /api/v1/market/ticker?symbol=DOGEUSDT`. 2. Observe `bid: -0.2615`, `ask: 0.7385` — bid is negative and spread is >$1 on a $0.24 asset. |
| **Fix** | Use percentage-based spread: `bid = price * 0.9999`, `ask = price * 1.0001`. |

---

### F-07: Frontend `WsMessageType` missing 'kline' — WS kline messages unhandled

| Field | Value |
|---|---|
| **Severity** | **P1** |
| **Description** | Backend `WsOutMessage` enum (`market_schemas.rs:123-128`) includes a `Kline` variant that serializes as `{ type: "kline", symbol, data, ts }`. But frontend `WsMessageType` (`market.ts:33`) does not include `'kline'`. Any WS kline message from the backend will be received but not typed, causing TypeScript errors and likely dropped by any type-guarded message handler. |
| **File:Line** | `frontend/src/types/market.ts:33`; `backend/src/models/market_schemas.rs:123-128` |
| **Reproduction** | 1. Subscribe to `kline:BTCUSDT` via WS. 2. Backend sends `{ type: "kline", ... }`. 3. Frontend TS type doesn't match — message ignored or error. |
| **Fix** | Add `'kline'` to `WsMessageType` union and add kline data handling to `WsMessage` interface. |

---

### F-08: No input validation on `symbol` format in any handler

| Field | Value |
|---|---|
| **Severity** | **P2** |
| **Description** | All three query-param handlers (`get_ticker`, `get_depth`, `get_ticker_history`) accept `symbol: String` without any format validation. Malicious or malformed inputs like empty string `""`, SQL injection attempts `"'; DROP TABLE"`, or overly long strings are passed directly to the service layer. While the service returns 404 for unknown symbols, there is no early rejection of clearly invalid formats (e.g., must match `/^[A-Z]{2,10}USDT$/`). |
| **File:Line** | `backend/src/handlers/market.rs:33`, `:43`, `:73` |
| **Reproduction** | 1. `GET /api/v1/market/ticker?symbol=` (empty). 2. Backend processes it, service returns 404. 3. `GET /api/v1/market/ticker?symbol=<script>alert(1)</script>` — same, no 400 Bad Request. |
| **Fix** | Add symbol format validation (regex or allowlist) in handler before calling service. Return `AppError::Validation` for invalid formats. |

---

### F-09: `TickerHistoryQueryParams` has no range/logic validation

| Field | Value |
|---|---|
| **Severity** | **P2** |
| **Description** | `TickerHistoryQueryParams` requires `start: i64` and `end: i64` but the handler does not validate that `start < end`, that they represent valid Unix millisecond timestamps, or that the range isn't unreasonably large (e.g., querying 10 years of data). `page` and `page_size` also lack bounds (page_size could be 0 or 1000000). |
| **File:Line** | `backend/src/models/market_schemas.rs:52-58`; `backend/src/handlers/market.rs:70-79` |
| **Reproduction** | 1. `GET /api/v1/market/ticker/history?symbol=BTCUSDT&start=9999999999999&end=0` (inverted range). 2. No error returned (once implemented, would return empty or incorrect results). |
| **Fix** | Add validation: `start < end`, reasonable range (e.g., max 30 days), `page_size` clamped to [1, 100]. |

---

### F-10: Depth levels RBAC uses fragile string comparison for role names

| Field | Value |
|---|---|
| **Severity** | **P2** |
| **Description** | `market.rs:56-58` checks `user.role != "pro-trader" && user.role != "admin"` using hardcoded string literals. If a new privileged role is added (e.g., "enterprise"), this check must be updated in code. There is no role hierarchy or permission system — just ad-hoc string matching. |
| **File:Line** | `backend/src/handlers/market.rs:56-58` |
| **Reproduction** | 1. Create user with role "enterprise". 2. Request `levels=50`. 3. Gets 403 Forbidden despite being a premium role. |
| **Fix** | Implement a role-based permission check (e.g., `user.has_permission("depth:50")` or a set of roles that can access deep orderbooks). |

---

### F-11: Market error code constants in `market_schemas.rs` are unused dead code

| Field | Value |
|---|---|
| **Severity** | **P2** |
| **Description** | `market_schemas.rs:150-157` defines 8 error code constants (`ERR_MARKET_SYMBOL_NOT_FOUND`, `ERR_MARKET_DEPTH_LEVELS_INVALID`, etc.) but none are referenced anywhere in the handler or service code. The handlers use `AppError` enum variants which map to different code values via the generic error infrastructure. For example, `AppError::Forbidden` always maps to code 40301 regardless of context — it could be a depth-forbidden or auth-forbidden, losing specificity. |
| **File:Line** | `backend/src/models/market_schemas.rs:150-157` |
| **Reproduction** | 1. Grep for `ERR_MARKET_DEPTH_FORBIDDEN` usage — zero results outside definition. 2. Handler line 60 uses `AppError::Forbidden(...)` which generates code 40301 — this happens to match, but only by coincidence with the generic error code, not because the market-specific constant is used. |
| **Fix** | Either use the market-specific error codes (via `AppError::with_code()` or similar) or remove the dead constants. Using market-specific codes would improve error diagnostics on the frontend. |

---

### F-12: Hardcoded symbol lists in multiple places — no single source of truth

| Field | Value |
|---|---|
| **Severity** | **P2** |
| **Description** | Symbol lists are hardcoded in at least 5 locations: (1) `backend/services/market_data.rs:8` — `SUPPORTED_SYMBOLS` (6 items), (2) `frontend/types/market.ts:69-80` — `SYMBOL_NAMES` (10 items), (3) `frontend/views/market/MarketView.vue:91` — WS subscription (3 items), (4) `frontend/views/market/DepthView.vue:86` — depth UI dropdown (5 items), (5) `frontend/views/strategy/StrategyCreateView.vue:480` — strategy symbol options (8 items). These lists are inconsistent with each other and with the backend. |
| **File:Line** | Multiple (see above) |
| **Reproduction** | 1. Compare symbol counts: backend=6, SYMBOL_NAMES=10, MarketView=3, DepthView=5, StrategyCreate=8. 2. Add a new symbol — must update 5+ files. |
| **Fix** | Add a `/market/symbols` API endpoint returning the authoritative list. Frontend should fetch and cache it. |

---

### F-13: Mock depth `quantity` and `total` use unrealistic fract-based generation

| Field | Value |
|---|---|
| **Severity** | **P2** |
| **Description** | `market_data.rs:118` generates `quantity = 0.1 + (i as f64 * 0.15).fract() * 5.0`. The `.fract()` function returns the fractional part, creating a sawtooth pattern in quantities that is not realistic market behavior. Similarly, `total` is computed by summing all quantities from level 0 to i using the same fract pattern, which is O(n²) and produces cumulative totals that oscillate unrealistically. |
| **File:Line** | `backend/src/services/market_data.rs:118-119`, `:132-133` |
| **Reproduction** | 1. Generate depth for BTCUSDT levels=50. 2. Observe quantities follow a sawtooth pattern rather than realistic distribution (larger near best bid/ask, tapering off). |
| **Fix** | Use a more realistic depth model: exponential decay from best price, or import real depth data snapshots. |

---

### F-14: `DepthView.vue` hardcoded symbol list missing DOGEUSDT

| Field | Value |
|---|---|
| **Severity** | **P2** |
| **Description** | `frontend/src/views/market/DepthView.vue:86` defines `symbols = ['BTCUSDT', 'ETHUSDT', 'BNBUSDT', 'SOLUSDT', 'XRPUSDT']` — 5 symbols. Backend supports 6 (missing DOGEUSDT). Users cannot view DOGEUSDT depth from this component. |
| **File:Line** | `frontend/src/views/market/DepthView.vue:86` |
| **Reproduction** | 1. Open DepthView. 2. Symbol dropdown has no DOGEUSDT option. |
| **Fix** | Add DOGEUSDT to the list or source from API. |

---

### F-15: `get_tickers` response has no pagination — scalability concern

| Field | Value |
|---|---|
| **Severity** | **P3** |
| **Description** | `GET /api/v1/market/tickers` returns all tickers in a flat array. With 6 symbols this is fine, but if the platform scales to 100+ symbols, the unpaginated response becomes large. The `TickerHistoryResponse` has proper pagination (`meta: { total, page, page_size }`), but the tickers endpoint does not. |
| **File:Line** | `backend/src/handlers/market.rs:21-27` |
| **Reproduction** | N/A — currently works fine with 6 symbols. |
| **Fix** | Consider adding optional pagination params for future-proofing, or document that tickers is intended as a small "all symbols" endpoint. |

---

### F-16: Frontend `getDepth` always sends `levels` parameter even when default suffices

| Field | Value |
|---|---|
| **Severity** | **P3** |
| **Description** | `frontend/src/api/market.ts:19-21` always includes `levels` in query params (default=10). The backend defaults to 10 if omitted. Not a bug, but slightly redundant and couples frontend to backend defaults. |
| **File:Line** | `frontend/src/api/market.ts:19` |
| **Reproduction** | 1. Call `getDepth('BTCUSDT')`. 2. Observe request includes `?symbol=BTCUSDT&levels=10`. |
| **Fix** | Low priority; consider only sending `levels` when user explicitly selects a non-default value. |

---

### F-17: `ApiResponse` success shape `{code, data, message}` vs error shape `{code, message}` — no `data` on error

| Field | Value |
|---|---|
| **Severity** | **P3** |
| **Description** | Success responses use `ApiResponse<T> { code: 0, data: T, message }`. Error responses use `ApiError { code: N, message }` with **no `data` field**. The frontend client interceptor (`client.ts:52-56`) checks `body.code !== 0` and accesses `body.data` only on success. The error interceptor (`client.ts:121`) accesses `error.response?.data?.message` — which is the `message` field of the JSON error body. This works but could confuse developers expecting a uniform response envelope. |
| **File:Line** | `backend/src/utils/response.rs:6-10` vs `:37-42`; `frontend/src/api/client.ts:50-56` |
| **Reproduction** | 1. Trigger a 404 on `/market/ticker?symbol=INVALID`. 2. Response: `{ code: 40401, message: "交易对 INVALID 不存在" }`. 3. No `data` field present. |
| **Fix** | Low priority; document the dual response shape or unify with `data: null` on errors. |

---

### F-18: `get_ticker_history` handler serializes `TickerHistoryResponse` to `serde_json::Value` then back — unnecessary round-trip

| Field | Value |
|---|---|
| **Severity** | **P3** |
| **Description** | `market.rs:74-78` calls `serde_json::to_value(result)?` on a `TickerHistoryResponse` then wraps it in `ApiResponse::success(value)`. This double-serialization is unnecessary since `ApiResponse<TickerHistoryResponse>` would serialize correctly directly. The intermediate `serde_json::Value` also loses type safety. |
| **File:Line** | `backend/src/handlers/market.rs:74-78` |
| **Reproduction** | Code inspection. |
| **Fix** | Change return type to `Json<ApiResponse<TickerHistoryResponse>>` and return `ApiResponse::success(result)` directly. |

---

## Summary Statistics

| Severity | Count | Finding IDs |
|---|---|---|
| **P0** | 3 | F-01, F-02, F-03 |
| **P1** | 4 | F-04, F-05, F-06, F-07 |
| **P2** | 7 | F-08, F-09, F-10, F-11, F-12, F-13, F-14 |
| **P3** | 4 | F-15, F-16, F-17, F-18 |
| **Total** | **18** | |

### Category Breakdown

| Category | Count | Findings |
|---|---|---|
| API endpoint mismatch (frontend↔backend) | 2 | F-01, F-04 |
| Mock data quality / realism | 3 | F-02, F-06, F-13 |
| Typo / symbol error | 1 | F-03 |
| Schema / type inconsistency | 2 | F-05, F-07 |
| Missing input validation | 2 | F-08, F-09 |
| RBAC / auth design weakness | 1 | F-10 |
| Dead code / unused constants | 1 | F-11 |
| Hardcoded values / no config | 2 | F-12, F-14 |
| API response format | 2 | F-15, F-17 |
| Code quality | 2 | F-16, F-18 |

### Critical Path Issues (must fix before launch)

1. **F-01** (P0): `getKline` → 404 on every call — frontend chart broken
2. **F-02** (P0): Negative prices in DOGEUSDT/XRPUSDT depth — invalid market data
3. **F-03** (P0): `BNBUSUSDT` typo — BNB ticker never updates via WS
4. **F-04** (P1): Even if `ticker/history` is implemented, it returns wrong schema for `getKline`
5. **F-06** (P1): Negative bid on DOGEUSDT ticker — `bid = 0.2385 - 0.5 = -0.2615`
