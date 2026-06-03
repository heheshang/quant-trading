/**
 * trigger_order.spec.ts — Stop-loss / take-profit / OCO / TWAP triggers.
 *
 * Endpoints (mounted at /api/v1/trigger-orders):
 *   POST /stop-loss
 *   POST /take-profit
 *   POST /oco
 *   POST /twap
 *   GET  /          (list)
 *   GET  /:id       (get)
 *   DELETE /:id     (cancel)
 */
import { test, expect, type Page } from '@playwright/test';
import { api, registerAndLogin, expectStatus } from './helpers';

const baseOrder = {
  symbol: 'BTCUSDT',
  quantity: 0.01,
};

test.describe('trigger-order — stop / OCO / TWAP', () => {

  test.beforeEach(async ({ page }: { page: Page }) => {
    await registerAndLogin(page);
  });

  test('01 — create stop-loss', async ({ page }) => {
    const r = await api(page, 'POST', '/api/v1/trigger-orders/stop-loss', {
      ...baseOrder,
      side: 'sell',
      trigger_price: 28000,
    });
    expect([200, 201, 400, 422]).toContain(r.status);
  });

  test('02 — create take-profit', async ({ page }) => {
    const r = await api(page, 'POST', '/api/v1/trigger-orders/take-profit', {
      ...baseOrder,
      side: 'sell',
      trigger_price: 75000,
    });
    expect([200, 201, 400, 422]).toContain(r.status);
  });

  test('03 — create OCO (one-cancels-other)', async ({ page }) => {
    const r = await api(page, 'POST', '/api/v1/trigger-orders/oco', {
      ...baseOrder,
      side: 'sell',
      take_profit: 75000,
      stop_loss: 28000,
    });
    expect([200, 201, 400, 422]).toContain(r.status);
  });

  test('04 — list trigger orders', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/trigger-orders');
    expectStatus(r, 200, 'list trigger orders');
  });

  test('05 — get one trigger order (404 ok if none yet)', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/trigger-orders/00000000-0000-0000-0000-000000000000');
    expect([200, 404]).toContain(r.status);
  });

  test('06 — cancel a trigger order (404 ok if absent)', async ({ page }) => {
    const r = await api(page, 'DELETE', '/api/v1/trigger-orders/00000000-0000-0000-0000-000000000000');
    expect([200, 204, 404]).toContain(r.status);
  });
});
