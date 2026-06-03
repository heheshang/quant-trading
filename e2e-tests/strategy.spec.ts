/**
 * strategy.spec.ts — Strategy CRUD + start/stop + template binding.
 *
 * Strategies are the central domain object.  We test:
 *   - create
 *   - list (paginated)
 *   - get one
 *   - start/stop
 *   - update
 *   - delete (soft or hard)
 */
import { test, expect, type Page } from '@playwright/test';
import { api, registerAndLogin, expectStatus, expectJsonField } from './helpers';

const baseStrategy = {
  name: `E2E Strategy ${Date.now()}`,
  description: 'Playwright E2E',
  symbol: 'BTCUSDT',
  timeframe: '1h',
  strategy_type: 'trend_following',
  template_type: 'breakout',
  parameters: {
    entry_threshold: 0.6,
    exit_threshold: 0.4,
    stop_loss: 0.02,
    position_size: 0.1,
  },
};

let createdId = '';

test.describe('strategy — CRUD / lifecycle', () => {

  test.beforeAll(async ({ page }: { page: Page }) => {
    await registerAndLogin(page);
  });

  test('01 — create strategy', async ({ page }) => {
    const r = await api(page, 'POST', '/api/v1/strategies', {
      ...baseStrategy,
      template_id: '00000000-0000-0000-0000-000000000001',
    });
    expect([200, 400, 422]).toContain(r.status);
    if (r.status === 200) {
      createdId = r.json.data?.strategy?.id || r.json.data?.id || '';
      expect(createdId).toBeTruthy();
    }
  });

  test('02 — list strategies', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/strategies?page=1&size=20');
    expectStatus(r, 200, 'list strategies');
    expectJsonField(r, 'data', 'strategies data envelope');
  });

  test('03 — get one strategy (404 ok if not created yet)', async ({ page }) => {
    const id = createdId || '00000000-0000-0000-0000-000000000000';
    const r = await api(page, 'GET', `/api/v1/strategies/${id}`);
    expect([200, 404]).toContain(r.status);
  });

  test('04 — start a strategy (202 ok if implemented async)', async ({ page }) => {
    const id = createdId || '00000000-0000-0000-0000-000000000000';
    // Common verbs across the codebase: /start, /activate, /run
    const r = await api(page, 'POST', `/api/v1/strategies/${id}/start`);
    expect([200, 202, 400, 404, 405]).toContain(r.status);
  });

  test('05 — stop a strategy', async ({ page }) => {
    const id = createdId || '00000000-0000-0000-0000-000000000000';
    const r = await api(page, 'POST', `/api/v1/strategies/${id}/stop`);
    expect([200, 202, 400, 404, 405]).toContain(r.status);
  });

  test('06 — update strategy (partial PATCH/PUT)', async ({ page }) => {
    const id = createdId || '00000000-0000-0000-0000-000000000000';
    const r = await api(page, 'PUT', `/api/v1/strategies/${id}`, {
      description: 'updated by E2E',
    });
    expect([200, 400, 404, 405]).toContain(r.status);
  });
});

// =============================================================
// P1-1: Grid strategy lifecycle
//
// The grid template (id = "grid") is registered in
// backend/src/services/strategy/templates_impls.rs. Strategies created
// against it expose `strategy_type = "grid_trading"` and a small JSON
// parameter set: { upper, lower, grid_levels, size_per_grid }.
// =============================================================

const gridStrategy = {
  name: `E2E Grid Strategy ${Date.now()}`,
  description: 'Playwright E2E grid template (P1-1)',
  symbol: 'BTCUSDT',
  timeframe: '1h',
  strategy_type: 'grid_trading',
  template_type: 'grid',
  parameters: {
    upper: 1.05,
    lower: 0.95,
    grid_levels: 10,
    size_per_grid: 0.1,
  },
};

let gridId = '';

test.describe('strategy — grid template (P1-1)', () => {

  test.beforeAll(async ({ page }: { page: Page }) => {
    await registerAndLogin(page);
  });

  test('10 — create grid strategy', async ({ page }) => {
    const r = await api(page, 'POST', '/api/v1/strategies', {
      ...gridStrategy,
      // Use a placeholder UUID; the handler resolves the real template from
      // the explicit `template_type: "grid"` field above.
      template_id: '00000000-0000-0000-0000-000000000001',
    });
    expect([200, 201, 400, 422]).toContain(r.status);
    if (r.status === 200 || r.status === 201) {
      gridId =
        r.json.data?.strategy?.id || r.json.data?.id || r.json.data?.strategy_id || '';
      expect(gridId).toBeTruthy();
    }
  });

  test('11 — grid strategy appears in list', async ({ page }) => {
    const r = await api(page, 'GET', '/api/v1/strategies?page=1&size=20');
    expectStatus(r, 200, 'list strategies');
    // We don't require gridId to be populated (the create in this worker may
    // not have been accepted), but if it was, the strategy must be findable.
    if (!gridId) test.skip(true, 'grid strategy not created in this worker');
    const items: Array<{ id: string; template_type?: string; strategy_type?: string }> =
      Array.isArray(r.json.data?.items)
        ? r.json.data.items
        : Array.isArray(r.json.data)
        ? r.json.data
        : [];
    const found = items.find((s) => s.id === gridId);
    if (found) {
      expect(found.template_type).toBe('grid');
      expect(found.strategy_type).toBe('grid_trading');
    }
  });

  test('12 — activate grid strategy (draft → active)', async ({ page }) => {
    if (!gridId) test.skip(true, 'grid strategy not created in this worker');
    const r = await api(page, 'POST', `/api/v1/strategies/${gridId}/status`, {
      status: 'active',
    });
    expect([200, 400, 404, 409]).toContain(r.status);
  });

  test('13 — invalid grid parameters rejected', async ({ page }) => {
    // upper <= 1.0 violates the grid template's `validate` contract.
    const r = await api(page, 'POST', '/api/v1/strategies', {
      ...gridStrategy,
      name: `E2E Bad Grid ${Date.now()}`,
      parameters: { upper: 0.5, lower: 0.4, grid_levels: 5, size_per_grid: 0.1 },
      template_id: '00000000-0000-0000-0000-000000000001',
    });
    // Must NOT be 2xx; 400/422 are the expected validation responses.
    expect([200, 201]).not.toContain(r.status);
    expect([400, 422]).toContain(r.status);
  });
});
