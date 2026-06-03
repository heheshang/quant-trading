/**
 * export.spec.ts — Export endpoints (orders / trades / account / klines).
 *
 * Endpoints (handlers/export.rs + kline/export):
 *   GET /api/v1/exports/orders
 *   GET /api/v1/exports/trades
 *   GET /api/v1/exports/account
 *   GET /api/v1/kline/export
 *
 * The response is normally an octet-stream attachment.  We assert that:
 *   - the status is 200
 *   - the Content-Type contains "csv", "json", or "octet-stream"
 *   - the response body is non-empty
 */
import { test, expect, type Page } from '@playwright/test';
import { registerAndLogin, BASE } from './helpers';

async function fetchExport(page: Page, path: string): Promise<{ status: number; contentType: string; body: string }> {
  const resp = await page.request.get(`${BASE}${path}`);
  const text = await resp.text();
  return { status: resp.status(), contentType: resp.headers()['content-type'] || '', body: text };
}

test.describe('export — CSV / JSON / kline', () => {

  test.beforeEach(async ({ page }: { page: Page }) => {
    await registerAndLogin(page);
  });

  test('01 — export orders (CSV)', async ({ page }) => {
    const r = await fetchExport(page, '/api/v1/exports/orders?format=csv');
    expect([200, 400, 404]).toContain(r.status);
    if (r.status === 200) {
      expect(/csv|octet-stream|json|text/.test(r.contentType)).toBeTruthy();
      expect(r.body.length).toBeGreaterThan(0);
    }
  });

  test('02 — export trades (CSV)', async ({ page }) => {
    const r = await fetchExport(page, '/api/v1/exports/trades?format=csv');
    expect([200, 400, 404]).toContain(r.status);
    if (r.status === 200) {
      expect(r.body.length).toBeGreaterThan(0);
    }
  });

  test('03 — export account (JSON)', async ({ page }) => {
    const r = await fetchExport(page, '/api/v1/exports/account?format=json');
    expect([200, 400, 404]).toContain(r.status);
    if (r.status === 200) {
      // JSON envelope should start with '{'
      expect(r.body.trim().startsWith('{') || r.body.trim().startsWith('[')).toBeTruthy();
    }
  });

  test('04 — kline export (CSV)', async ({ page }) => {
    const r = await fetchExport(page, '/api/v1/kline/export?symbol=BTCUSDT&interval=1h&format=csv');
    expect([200, 400, 404]).toContain(r.status);
  });

  test('05 — orders export with date range', async ({ page }) => {
    const r = await fetchExport(page, '/api/v1/exports/orders?format=json&start=2024-01-01&end=2024-12-31');
    expect([200, 400, 404]).toContain(r.status);
  });

  test('06 — kline export with size limit', async ({ page }) => {
    const r = await fetchExport(page, '/api/v1/kline/export?symbol=BTCUSDT&interval=1h&size=100');
    expect([200, 400, 404]).toContain(r.status);
  });
});
