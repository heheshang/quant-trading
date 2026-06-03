/**
 * E2E shared helpers — re-exported for Playwright .spec.ts files.
 *
 * The pure-Node `runner.mjs` carries the actual implementations
 * (httpReq / expectStatus / expectJsonField / retryHelper).  This file
 * re-exports them via a lazy dynamic import so spec files can:
 *
 *   import { httpReq, expectStatus, expectJsonField, retryHelper, BASE, AI } from './helpers';
 *
 * Lazy import is necessary because runner.mjs is plain ESM and cannot be
 * pulled in via TypeScript's static CJS resolution.  Using `await` at
 * module top-level would break the CJS-require context that Playwright
 * uses, so we resolve BASE/AI from the same env vars that runner.mjs
 * uses (which is the single source of truth for "where the backend is").
 */
import type { Page, APIRequestContext } from '@playwright/test';

// ─── Base URLs ──────────────────────────────────────────────────────────────
// Same defaults as runner.mjs.  Keep in sync.
export const BASE = process.env.E2E_BASE_URL   || 'http://localhost:8080';
export const AI   = process.env.AI_SERVICE_URL || 'http://localhost:8001';

// ─── Runner.mjs lazy import ────────────────────────────────────────────────
interface RunnerModule {
  httpReq: <T = unknown>(
    method: string,
    url: string,
    body?: unknown,
    token?: string,
  ) => Promise<{ status: number; json: T; text: string }>;
  expectStatus: (
    resp: { status: number; json: unknown },
    allowed: number | number[],
    label: string,
  ) => boolean;
  expectJsonField: (
    resp: { status: number; json: any },
    dottedPath: string,
    label: string,
    predicate?: ((v: unknown) => boolean) | RegExp | unknown,
  ) => boolean;
  retryHelper: <T>(
    fn: (attempt: number) => Promise<T | null | undefined> | T | null | undefined,
    opts?: { attempts?: number; baseDelayMs?: number; factor?: number; maxDelayMs?: number },
  ) => Promise<T | null | undefined>;
}

let _runner: RunnerModule | null = null;
function getRunner(): RunnerModule {
  if (_runner) return _runner;
  // Use require() (synchronous) — Playwright compiles TS to CJS by default,
  // and runner.mjs exports are pure functions we only need to call later.
  // We use a top-level ts-ignore because TS doesn't know about Node's
  // CJS<->ESM interop for .mjs files; runtime works fine.
  // @ts-ignore — runtime CJS interop is fine
  _runner = require('./runner.mjs') as RunnerModule;
  return _runner;
}

export interface ApiResponse<T = unknown> {
  status: number;
  json: T;
  text: string;
}

export async function httpReq<T = unknown>(
  method: string,
  url: string,
  body?: unknown,
  token?: string,
): Promise<ApiResponse<T>> {
  return getRunner().httpReq<T>(method, url, body, token) as Promise<ApiResponse<T>>;
}

export async function expectStatus(
  resp: { status: number; json: unknown },
  allowed: number | number[],
  label: string,
): Promise<boolean> {
  return getRunner().expectStatus(resp, allowed, label);
}

export async function expectJsonField(
  resp: { status: number; json: any },
  dottedPath: string,
  label: string,
  predicate?: ((v: unknown) => boolean) | RegExp | unknown,
): Promise<boolean> {
  return getRunner().expectJsonField(resp, dottedPath, label, predicate);
}

export async function retryHelper<T>(
  fn: (attempt: number) => Promise<T | null | undefined> | T | null | undefined,
  opts?: { attempts?: number; baseDelayMs?: number; factor?: number; maxDelayMs?: number },
): Promise<T | null | undefined> {
  return getRunner().retryHelper(fn, opts);
}

interface CachedAuth {
  username: string;
  email: string;
  password: string;
  token: string;
  userId: string;
}
let _auth: CachedAuth | null = null;

/**
 * Register a fresh user (once per worker) and return the bearer token.
 * Subsequent calls return the cached token.  Throws on failure.
 */
export async function registerAndLogin(page: Page): Promise<CachedAuth> {
  void page;
  if (_auth) return _auth;
  const username = `e2e_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  const email    = `${username}@test.com`;
  const password = 'Test1234!';
  const r = await httpReq<{ code: number; data: { access_token: string; user: { id: string } } }>(
    'POST',
    `${BASE}/api/v1/auth/register`,
    { username, email, password },
  );
  if (r.status !== 200 || r.json.code !== 0) {
    throw new Error(`register failed: ${r.status} ${JSON.stringify(r.json)}`);
  }
  _auth = {
    username,
    email,
    password,
    token: r.json.data.access_token,
    userId: r.json.data.user.id,
  };
  return _auth;
}

export function getAuth(): CachedAuth {
  if (!_auth) throw new Error('auth not initialised; call registerAndLogin first');
  return _auth;
}

/**
 * Playwright-flavored wrapper that returns the same shape as the
 * auth.spec.ts helper.  Used by files that prefer to call
 * `api(page, 'POST', '/path', body)` exactly like the original spec.
 */
export async function api(
  page: Page,
  method: 'GET' | 'POST' | 'PUT' | 'DELETE',
  path: string,
  body?: unknown,
): Promise<ApiResponse> {
  const ctx: APIRequestContext = page.request;
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  if (_auth?.token) headers['Authorization'] = `Bearer ${_auth.token}`;
  const resp = await ctx.fetch(`${BASE}${path}`, {
    method,
    headers,
    data: body ? JSON.stringify(body) : undefined,
  });
  let json: unknown;
  try { json = await resp.json(); } catch { json = { _raw: await resp.text() }; }
  return { status: resp.status(), json, text: '' };
}
