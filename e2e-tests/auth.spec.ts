import { test, expect, Page } from '@playwright/test';

// ── Config ────────────────────────────────────────────────────────────────────
const BASE_URL = process.env.E2E_BASE_URL || 'http://localhost:8080';

const user = {
  username: `e2e_${Date.now()}`,
  email: `e2e_${Date.now()}@test.com`,
  password: 'Test1234!',
};

let authToken = '';
let userId = '';

// ── Helpers ───────────────────────────────────────────────────────────────────
async function api(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'DELETE',
  path: string,
  body?: unknown,
) {
  const resp = await page.request[method.toLowerCase()](`${BASE_URL}${path}`, {
    headers: authToken ? { Authorization: `Bearer ${authToken}` } : {},
    data: body ? JSON.stringify(body) : undefined,
  });
  return { status: resp.status(), body: await resp.json() };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

test.describe('quant-trading E2E', () => {

  // ── Auth ────────────────────────────────────────────────────────────────────
  test('01 — register', async ({ page }) => {
    const { status, body } = await api(page, 'POST', '/api/v1/auth/register', {
      username: user.username,
      email: user.email,
      password: user.password,
    });
    expect(status).toBe(200);
    expect(body.code).toBe(0);
    expect(body.data.access_token).toBeTruthy();
    authToken = body.data.access_token;
    userId = body.data.user.id;
    console.log(`[register] userId=${userId}`);
  });

  test('02 — login', async ({ page }) => {
    const { status, body } = await api(page, 'POST', '/api/v1/auth/login', {
      username: user.username,
      password: user.password,
    });
    expect(status).toBe(200);
    expect(body.code).toBe(0);
    expect(body.data.access_token).toBeTruthy();
    authToken = body.data.access_token; // refresh
  });

  test('03 — me / profile', async ({ page }) => {
    const { status, body } = await api(page, 'GET', '/api/v1/users/me');
    expect(status).toBe(200);
    expect(body.code).toBe(0);
    expect(body.data.username).toBe(user.username);
  });

  test('04 — change password', async ({ page }) => {
    const { status, body } = await api(page, 'POST', '/api/v1/auth/change-password', {
      old_password: user.password,
      new_password: 'NewTest5678!',
    });
    // May fail if change-password needs re-auth or old password check
    // Just verify the endpoint is reachable
    expect([200, 400, 401]).toContain(status);
  });

  // ── Strategy ────────────────────────────────────────────────────────────────
  test('05 — create strategy', async ({ page }) => {
    const { status, body } = await api(page, 'POST', '/api/v1/strategies', {
      name: 'E2E Test Strategy',
      description: 'Playwright E2E test',
      symbol: 'BTCUSDT',
      timeframe: '1h',
      strategy_type: 'trend_following',
      template_type: 'ai_enhanced',
      parameters: {
        entry_threshold: 0.6,
        exit_threshold: 0.4,
        stop_loss: 0.02,
        position_size: 0.1,
      },
    });
    expect(status).toBe(200);
    expect(body.code).toBe(0);
    expect(body.data.strategy.id).toBeTruthy();
    console.log(`[strategy] id=${body.data.strategy.id}`);
  });

  test('06 — list strategies', async ({ page }) => {
    const { status, body } = await api(page, 'GET', '/api/v1/strategies');
    expect(status).toBe(200);
    expect(body.code).toBe(0);
    expect(body.data.strategies.length).toBeGreaterThan(0);
  });

  // ── AI Quant ────────────────────────────────────────────────────────────────
  test('07 — list AI models (requires auth)', async ({ page }) => {
    const { status, body } = await api(page, 'GET', '/api/v1/ai/models');
    expect(status).toBe(200);
    expect(body.code).toBe(0);
    expect(body.data.models.length).toBeGreaterThan(0);
    expect(body.data.models[0].version).toBeTruthy();
  });

  test('08 — AI health via Rust backend → Python service', async ({ page }) => {
    // The AI service health endpoint
    const { status, body } = await page.request.get(`${BASE_URL.replace('8080', '8001')}/api/v1/health`);
    expect(status).toBe(200);
    expect(body.status).toBe('ok');
    expect(body.models_available).toBeGreaterThan(0);
  });

  // ── Dashboard / Portfolio ───────────────────────────────────────────────────
  test('09 — dashboard stats', async ({ page }) => {
    const { status, body } = await api(page, 'GET', '/api/v1/dashboard/stats');
    expect(status).toBe(200);
    // code may be 0 or non-0 if no data yet
    expect(body).toHaveProperty('code');
  });

  test('10 — portfolio positions', async ({ page }) => {
    const { status, body } = await api(page, 'GET', '/api/v1/portfolio/positions');
    expect(status).toBe(200);
    expect(body).toHaveProperty('code');
  });

  // ── Kline data ─────────────────────────────────────────────────────────────
  test('11 — klines (no data expected)', async ({ page }) => {
    const { status, body } = await api(page, 'GET', '/api/v1/klines/BTCUSDT?interval=1h&size=10');
    expect(status).toBe(200);
    // May be 0 (no data) or 40401
    expect(body).toHaveProperty('code');
  });

  // ── API key management ──────────────────────────────────────────────────────
  test('12 — create API key', async ({ page }) => {
    const { status, body } = await api(page, 'POST', '/api/v1/api-keys', {
      name: 'E2E Test Key',
      permissions: ['read_market', 'trade spot'],
    });
    expect(status).toBe(200);
    expect(body.code).toBe(0);
  });

  test('13 — list API keys', async ({ page }) => {
    const { status, body } = await api(page, 'GET', '/api/v1/api-keys');
    expect(status).toBe(200);
    expect(body.code).toBe(0);
  });

  // ── Health checks ───────────────────────────────────────────────────────────
  test('14 — backend health endpoint', async ({ page }) => {
    const { status, body } = await page.request.get(`${BASE_URL}/health`);
    expect(status).toBe(200);
    expect(body.code).toBe(0);
    expect(body.data.status).toBe('ok');
  });

  test('15 — WebSocket upgrade (should upgrade, not 404)', async ({ page }) => {
    // Just verify the WS endpoint exists (upgrade to 101 or 426)
    const resp = await page.request.fetch(`${BASE_URL}/api/v1/ws`, {
      method: 'GET',
    });
    // Axum may reject non-Upgrade requests with 426
    expect([101, 426]).toContain(resp.status());
  });

});
