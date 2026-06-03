/**
 * portfolio.spec.ts — Portfolio summary / positions / equity curve / perf.
 *
 * Endpoints (handlers/portfolio.rs):
 *   GET /api/v1/portfolio/summary
 *   GET /api/v1/portfolio/positions
 *   GET /api/v1/portfolio/performance
 *   GET /api/v1/portfolio/equity_curve
 */
import { test, expect, type Page } from '@playwright/test';
import { api, registerAndLogin, expectStatus, expectJsonField } from './helpers';

test.describe('portfolio — summary / positions / equity', () => {

  test.beforeEach(async ({ page }: { page: Page }) => {
    await registerAndLogin(page);
  });

  test('01 — get portfolio summary', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/portfolio/summary');
    expectStatus(r, 200, 'portfolio summary');
    expectJsonField(r, 'data', 'portfolio summary data');
  });

  test('02 — list positions', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/portfolio/positions');
    expectStatus(r, 200, 'portfolio positions');
  });

  test('03 — get equity curve', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/portfolio/equity_curve?days=30');
    expectStatus(r, 200, 'equity curve');
    expectJsonField(r, 'data', 'equity curve data');
  });

  test('04 — get performance (multi-strategy)', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/portfolio/performance');
    expectStatus(r, 200, 'portfolio performance');
  });

  test('05 — paginated positions (page=1,size=5)', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/portfolio/positions?page=1&size=5');
    expectStatus(r, 200, 'paginated positions');
  });

  test('06 — equity curve with custom range', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/portfolio/equity_curve?start=2024-01-01&end=2024-12-31');
    expect([200, 400]).toContain(r.status);
  });
});
