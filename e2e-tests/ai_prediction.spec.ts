/**
 * ai_prediction.spec.ts — AI model listing, prediction, history.
 *
 * The backend exposes two AI surfaces:
 *   1) Rust ↔ Python (AiServices) via /api/v1/ai/...
 *   2) Direct Python service at :8001/api/v1/...
 *
 * We cover both: the Rust path is what end-users hit, the direct Python
 * path is what ops hits when debugging.  Either being 200/202 is success.
 */
import { test, expect, type Page } from '@playwright/test';
import { api, registerAndLogin, BASE, AI, httpReq, expectStatus, expectJsonField } from './helpers';

test.describe('ai-prediction — models / predict / history', () => {

  test.beforeEach(async ({ page }: { page: Page }) => {
    await registerAndLogin(page);
  });

  test('01 — list AI models (Rust)', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/ai/models');
    expectStatus(r, 200, 'list AI models (Rust)');
    expectJsonField(r, 'models', 'models array present');
  });

  test('02 — get prediction for symbol (Rust)', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/ai/predictions/BTCUSDT?interval=1h');
    // 200 = has kline data, 404/40401 = no kline data (still a valid response shape)
    expect([200, 404]).toContain(r.status);
  });

  test('03 — direct Python /models', async () => {
    const r = await httpReq('GET', `${AI}/api/v1/models`);
    expectStatus(r, 200, 'direct Python /models');
    expect(Array.isArray(r.json)).toBeTruthy();
  });

  test('04 — direct Python /predict', async () => {
    const r = await httpReq('POST', `${AI}/api/v1/predict`, {
      features: Array(20).fill(0.01).map((v, i) => (i === 3 ? 45 : v)),
      symbol: 'BTCUSDT',
      interval: '1h',
    });
    expectStatus(r, 200, 'direct Python /predict');
    expect(['long', 'short', 'neutral']).toContain(r.json.direction);
  });

  test('05 — direct Python /health', async () => {
    const r = await httpReq('GET', `${AI}/api/v1/health`);
    expectStatus(r, 200, 'direct Python /health');
    expect(r.json.status).toBe('ok');
  });

  test('06 — AI prediction graceful when no klines', async ({ page }) => {
    // Force an unknown symbol to provoke 404 / no-data path.
    const r = await api(page, 'GET', '/api/v1/ai/predictions/NOPE?interval=1h');
    expect([200, 404]).toContain(r.status);
    void BASE; // silence unused-import warning in some tsconfigs
  });
});
