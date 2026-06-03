/**
 * risk.spec.ts — Risk rules + manual pause/resume + risk log + check.
 *
 * Endpoints (handlers/risk.rs):
 *   GET  /api/v1/risk/rules
 *   PUT  /api/v1/risk/rules
 *   GET  /api/v1/risk/logs
 *   POST /api/v1/risk/pause
 *   POST /api/v1/risk/resume
 *   POST /api/v1/risk/check
 */
import { test, expect, type Page } from '@playwright/test';
import { api, registerAndLogin, expectStatus, expectJsonField } from './helpers';

test.describe('risk — rules / pause / resume / logs / check', () => {

  test.beforeEach(async ({ page }: { page: Page }) => {
    await registerAndLogin(page);
  });

  test('01 — get current risk rules', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/risk/rules');
    expectStatus(r, 200, 'risk rules');
    expectJsonField(r, 'data', 'risk rules data');
  });

  test('02 — list risk logs', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/risk/logs?page=1&size=20');
    expect([200, 404]).toContain(r.status);
  });

  test('03 — pause trading', async ({ page }) => {
    const r = await api(page, 'POST', '/api/v1/risk/pause', { reason: 'E2E pause' });
    expect([200, 202, 400]).toContain(r.status);
  });

  test('04 — resume trading', async ({ page }) => {
    const r = await api(page, 'POST', '/api/v1/risk/resume', { reason: 'E2E resume' });
    expect([200, 202, 400]).toContain(r.status);
  });

  test('05 — manual risk check', async ({ page }) => {
    const r = await api(page, 'POST', '/api/v1/risk/check', {
      symbol: 'BTCUSDT',
      side: 'buy',
      quantity: 0.01,
      price: 60000,
    });
    expect([200, 202, 400, 422]).toContain(r.status);
  });

  test('06 — update risk rules (PUT)', async ({ page }) => {
    const r = await api(page, 'PUT', '/api/v1/risk/rules', {
      max_position_size: 0.5,
      max_daily_loss: 1000,
      max_drawdown: 0.2,
    });
    expect([200, 400, 403]).toContain(r.status);
  });
});
