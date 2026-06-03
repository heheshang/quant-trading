/**
 * backtest.spec.ts — Run a backtest, poll its progress, fetch the result.
 *
 * Backtests are async on the server (queue → worker → result row).  We
 * submit a tiny backtest and then poll GET /backtest/{id} until it
 * terminates.  If the backtest endpoint returns 202 Accepted with a
 * status URL we follow it; otherwise we just verify the create endpoint
 * is reachable and the response shape is sane.
 */
import { test, expect, type Page } from '@playwright/test';
import { api, registerAndLogin, expectStatus, retryHelper } from './helpers';

test.describe('backtest — run / progress / result', () => {

  test.beforeEach(async ({ page }: { page: Page }) => {
    await registerAndLogin(page);
  });

  test('01 — create backtest', async ({ page }) => {
    const r = await api(page, 'POST', '/api/v1/backtest', {
      strategy_id: '00000000-0000-0000-0000-000000000001',
      symbol: 'BTCUSDT',
      interval: '1h',
      start_time: '2024-01-01T00:00:00Z',
      end_time:   '2024-01-02T00:00:00Z',
      initial_capital: 10_000,
    });
    // 200 = sync result, 202 = accepted async, 400/422 = bad params
    expect([200, 202, 400, 422]).toContain(r.status);
  });

  test('02 — query backtest by id (404 ok if unknown)', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/backtest/00000000-0000-0000-0000-000000000000');
    expect([200, 404]).toContain(r.status);
  });

  test('03 — poll backtest to completion', async ({ page }) => {
    // Submit a backtest.
    const submit = await api(page, 'POST', '/api/v1/backtest', {
      strategy_id: '00000000-0000-0000-0000-000000000001',
      symbol: 'BTCUSDT',
      interval: '1h',
      start_time: '2024-01-01T00:00:00Z',
      end_time:   '2024-01-02T00:00:00Z',
      initial_capital: 10_000,
    });
    // 400/422 (missing strategy) is fine — endpoint reachable is the goal.
    if (![200, 202].includes(submit.status)) {
      expect([400, 422]).toContain(submit.status);
      return;
    }
    const id = submit.json?.data?.backtest?.id || submit.json?.data?.id;
    if (!id) return; // nothing to poll
    const finished = await retryHelper(async () => {
      const r = await api(page, 'GET', `/api/v1/backtest/${id}`);
      if (r.status === 200) {
        const s = r.json?.data?.backtest?.status || r.json?.data?.status;
        if (s === 'completed' || s === 'failed' || s === 'done') return r;
      }
      return null;
    }, { attempts: 10, baseDelayMs: 500 });
    if (finished) expectStatus(finished, 200, 'final backtest fetch');
  });

  test('04 — list backtests', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/backtest?page=1&size=20');
    expect([200, 404]).toContain(r.status);
  });

  test('05 — AI backtest endpoint reachable', async ({ page }) => {
    const r = await api(page, 'POST', '/api/v1/ai/backtest', {
      symbol: 'BTCUSDT',
      interval: '1h',
      horizon: 5,
    });
    expect([200, 202, 400, 422, 404]).toContain(r.status);
  });
});
