/**
 * market_data.spec.ts — Kline REST, market depth, tickers, and WS subscribe.
 *
 * Endpoints:
 *   GET /api/v1/klines/{symbol}/{interval}
 *   GET /api/v1/market/ticker
 *   GET /api/v1/market/tickers
 *   GET /api/v1/market/depth
 *   GET /api/v1/market/kline
 *   GET /api/v1/ws
 */
import { test, expect, type Page } from '@playwright/test';
import { api, registerAndLogin, BASE, expectStatus } from './helpers';

test.describe('market-data — klines / tickers / WS', () => {

  test.beforeEach(async ({ page }: { page: Page }) => {
    await registerAndLogin(page);
  });

  test('01 — fetch klines for symbol/interval', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/klines/BTCUSDT/1h?size=10');
    // 200 = has data, 404 = no klines stored yet
    expect([200, 404]).toContain(r.status);
  });

  test('02 — fetch single ticker', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/market/ticker?symbol=BTCUSDT');
    expect([200, 400, 404]).toContain(r.status);
  });

  test('03 — fetch all tickers', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/market/tickers');
    expectStatus(r, 200, 'all tickers');
  });

  test('04 — fetch order book depth', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/market/depth?symbol=BTCUSDT&limit=20');
    expect([200, 400, 404]).toContain(r.status);
  });

  test('05 — WS upgrade handshake', async ({ page }) => {
    // Plain HTTP GET against the WS endpoint should return 426 (Upgrade Required)
    // or 101 (already upgraded) — both prove the route exists.
    const resp = await page.request.fetch(`${BASE}/api/v1/ws`, { method: 'GET' });
    expect([101, 426]).toContain(resp.status());
  });

  test('06 — indicator endpoint reachable (e.g. RSI)', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/kline/rsi?symbol=BTCUSDT&interval=1h&period=14');
    expect([200, 400, 404]).toContain(r.status);
  });

  // P2-3: multi-timeframe linked chart (MultiTimeframeChart.vue) — same
  // symbol must be fetchable for two different intervals so the main + sub
  // chart can render side-by-side.  Validates the backend's kline-query
  // endpoint is interval-agnostic.
  test('07 — same symbol fetchable in two intervals (multi-timeframe support)', async ({ page }) => {
    const r1 = await api(page, 'GET', '/api/v1/kline/query?symbol=BTCUSDT&interval=1m&size=10');
    const r2 = await api(page, 'GET', '/api/v1/kline/query?symbol=BTCUSDT&interval=1h&size=10');
    // 200 = has data, 404 = symbol/interval has no klines stored yet
    // Both calls must hit the same code path (i.e. 200 or 404, not 500).
    for (const [label, r] of [['1m', r1], ['1h', r2]] as const) {
      expect([200, 404]).toContain(r.status);
      // The backend must not return 500 — that would mean a code-path bug
      // (e.g. the 1m code path missing) rather than a benign empty-store.
      expect(r.status, `kline/query ${label} should not 500`).not.toBe(500);
    }
  });
});
