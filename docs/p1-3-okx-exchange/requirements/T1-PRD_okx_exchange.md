# T1 — OKX Exchange Integration PRD

## Background

The quant-trading platform currently supports **Binance** as the sole signed (HMAC-authenticated) exchange integration for live trading. P1-3 extends multi-exchange coverage by adding **OKX** as a second signed client, following the same pattern as the existing Binance `SignedBinanceClient`.

The goal is to allow users to:
- View OKX account balances
- Place market / limit orders on OKX
- Cancel open OKX orders
- Query OKX pending orders
- Verify OKX API connectivity (`ping`)

## Scope

| Feature | Priority | Endpoint | Status |
|---|---|---|---|
| Connectivity test | P0 | `GET /api/v1/exchange/okx/ping` | ✅ v1 |
| Account balance | P0 | `GET /api/v1/exchange/okx/account` | ✅ v1 |
| Place order | P0 | `POST /api/v1/exchange/okx/order` | ✅ v1 |
| Cancel order | P0 | `DELETE /api/v1/exchange/okx/order/{orderId}` | ✅ v1 |
| Pending orders | P1 | `GET /api/v1/exchange/okx/orders/pending` | ✅ v1 |

## Out of scope (v1)

- WebSocket order updates from OKX (polling only)
- Cross-exchange arbitrage (cross-exchange adapter is a separate P2 track)
- Symbol auto-conversion at the API boundary (frontend uses OKX format directly: `BTC-USDT`)
- OKX futures / derivatives / margin (spot only)

## User personas

- **Retail trader** — wants to switch between Binance and OKX for fee/coin availability
- **Arbitrage bot operator** — needs to see both exchanges' balances and place orders on either (full cross-exchange logic is P2)
- **DevOps / SRE** — uses `ping` endpoint to verify OKX connectivity post-deploy

## Success criteria

1. All 5 OKX endpoints return 200/4xx with proper structured error responses
2. Signed requests use OKX's HMAC-SHA256 signing scheme (passphrase + secret + timestamp)
3. API key storage reuses the existing `ApiKeyStore` (encrypted at rest, decrypted on demand)
4. User authentication is JWT-based (same middleware as Binance endpoints)
5. Per-user API key isolation (user A's keys cannot be used to query user B's account)
6. Symbol format follows OKX convention (`BTC-USDT` not `BTCUSDT`)

## Non-functional requirements

| Requirement | Target |
|---|---|
| Latency (p99) | < 500ms (excluding OKX network latency) |
| Throughput | 100 req/s per user (rate limiter applies) |
| Availability | 99.9% (degrade gracefully if OKX API is down) |
| Auditability | All signed requests logged with user_id, endpoint, request_id |

## Dependencies

- Existing `ApiKeyStore` (encrypted at rest with master key) — already implemented
- Existing `ExchangePingResponse` / `ExchangeAccountResponse` / `ExchangeOrderResponse` shared types
- Existing JWT auth middleware
- OKX v5 API (`https://www.okx.com`)

## Risks

| Risk | Mitigation |
|---|---|
| OKX API rate limits (20 req/s per sub-account) | Per-user rate limiter, exponential backoff on 429 |
| Clock skew on HMAC timestamp | Server clock sync via NTP; reject requests with timestamp drift > 30s |
| API key leakage in logs | Never log `api_key` / `secret` / `passphrase`; mask in error messages |
| Frontend/Backend symbol format mismatch | Documented in T3 contract; frontend uses OKX format for OKX endpoints |
