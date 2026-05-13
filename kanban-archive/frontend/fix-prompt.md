# Fix Backtest Frontend Schema/Contract Bugs (Bug#2,3,4,8,9,10,11,12,13,14,15)

## Project Location
`/home/ssk/workspace/quant-trading/frontend`

## Overview
QA found 11 frontend bugs where the frontend types, API calls, and component logic don't match the backend schema. The backtest feature is 100% non-functional because of these mismatches. Fix all bugs below.

## Bug Details and Required Changes

### 1. types/backtest.ts — Complete restructure (Bug#3,8,9,11,12,14,15)

Current file at `src/types/backtest.ts` must be rewritten to match the backend:

**BacktestRunRequest** — Backend expects NESTED structure:
```typescript
/** Backend: BacktestRunRequest — POST /backtest body */
export interface BacktestRunRequest {
  strategy_id: string
  config: {
    symbol: string
    interval: string        // was: timeframe
    start_date: string
    end_date: string
    initial_capital: number
    fee_rate: number        // [0, 0.01] as decimal, NOT percentage
    slippage_rate: number   // was: slippage, [0, 0.01] as decimal
  }
}
```

**BacktestResultResponse** — Backend has nested metrics + config + status (Bug#11):
```typescript
/** Backend: BacktestResultResponse — GET /backtest/{id} response */
export interface BacktestResultResponse {
  id: string              // UUID string, NOT number (Bug#15)
  strategy_id: string
  status: 'pending' | 'running' | 'completed' | 'failed'  // Bug#11: was missing
  progress: number         // 0-100 (Bug#11: was missing)
  error: string | null     // Bug#11: was missing
  config: {                // Bug#11: was flat, now nested
    symbol: string
    interval: string
    start_date: string
    end_date: string
    initial_capital: number
    fee_rate: number
    slippage_rate: number
  }
  metrics: {               // Bug#11: was flat, now nested with 14 sub-fields
    total_return_pct: number    // was: total_return
    annualized_return_pct: number  // was: annual_return
    sharpe_ratio: number
    sortino_ratio: number       // was missing
    max_drawdown_pct: number    // was: max_drawdown
    calmar_ratio: number        // was missing
    win_rate: number
    profit_factor: number | null  // can be null when INFINITY (Bug#6)
    avg_win_pct: number          // was missing
    avg_loss_pct: number         // was missing
    total_trades: number
    total_fees: number           // was missing
    total_slippage: number       // was missing
    duration_ms: number          // was missing
  }
  equity_curve: EquityPoint[]
  trades: TradeRecord[]
  created_at: string
}
```

**TradeRecord** — Rename all fields to match backend (Bug#9,14):
```typescript
/** Backend: TradeRecord */
export interface TradeRecord {
  // NO id field in backend (Bug#9: frontend had id:number)
  direction: 'long' | 'short'
  entry_time: string       // was: open_time
  exit_time: string        // was: close_time
  entry_price: number      // was: open_price
  exit_price: number       // was: close_price
  quantity: number         // Bug#14: was missing
  pnl_usdt: number         // was: pnl
  pnl_pct: number          // was: pnl_percent
  fee: number              // Bug#14: was missing
  slippage: number         // Bug#14: was missing
  exit_reason: 'take_profit' | 'stop_loss' | 'signal'
  holding_period_ms: number  // Bug#14: was missing
}
```

**EquityPoint** — time is i64 ms timestamp, not string (Bug#8):
```typescript
/** Backend: EquityPoint */
export interface EquityPoint {
  time: number             // i64 millisecond timestamp (was: date: string)
  equity: number
  drawdown_pct: number     // was missing (Bug#7 — backend currently always 0 but field exists)
}
```

**BacktestHistoryItem** — Handle PaginatedResponse (Bug#12):
```typescript
/** Backend: BacktestSummary (item in paginated list) */
export interface BacktestSummary {
  id: string               // UUID string (Bug#15)
  strategy_id: string
  // NO strategy_name in backend (Bug#11)
  symbol: string
  status: string
  total_return_pct: number  // was: total_return (flat)
  sharpe_ratio: number
  created_at: string
}

/** Backend: PaginatedResponse<T> */
export interface PaginatedResponse<T> {
  items: T[]
  total: number
  page: number
  size: number
}
```

**BacktestJob** — Remove result_id, polling is via GET /backtest/{id} (Bug#4):
```typescript
/** Simplified — no separate job endpoint; use GET /backtest/{id} with status field */
export type BacktestStatus = 'pending' | 'running' | 'completed' | 'failed'
```

**Remove old types:** BacktestParams (replaced by BacktestRunRequest), BacktestJob (replaced by BacktestResultResponse.status), BacktestHistoryItem (replaced by BacktestSummary), BacktestTrade (replaced by TradeRecord), old BacktestResultDetail (replaced by BacktestResultResponse).

**Keep backward compat alias** for components that import BacktestResultDetail:
```typescript
/** @deprecated Use BacktestResultResponse */
export type BacktestResultDetail = BacktestResultResponse
```

### 2. api/backtest.ts — Fix request/response format (Bug#3,4,12,15)

Current file at `src/api/backtest.ts`:

- **runBacktest**: Must transform flat params to nested `BacktestRunRequest` format:
  ```typescript
  export function runBacktest(params: {
    strategy_id: string
    symbol: string
    interval: string
    start_date: string
    end_date: string
    initial_capital: number
    fee_rate: number
    slippage_rate: number
    strategy_params?: Record<string, any>
  }): Promise<BacktestResultResponse> {
    // Transform to nested structure, omit strategy_params (backend gets from strategy model)
    const { strategy_id, strategy_params, ...config } = params
    return client.post('/backtest', { strategy_id, config })
  }
  ```

- **getBacktestJob**: REMOVE this function entirely (Bug#4). The endpoint `/backtest/jobs/{id}` doesn't exist.

- **Polling**: Use `getBacktestResult(id)` to poll — it returns `BacktestResultResponse` with `status` field.

- **listBacktestHistory**: Must handle `PaginatedResponse<BacktestSummary>`:
  ```typescript
  export function listBacktestHistory(params?: {
    strategy_id?: string
    page?: number
    size?: number
  }): Promise<PaginatedResponse<BacktestSummary>> {
    return client.get('/backtest/history', { params })
  }
  ```

- **deleteBacktestResult**: Change id type from `number` to `string` (Bug#15):
  ```typescript
  export function deleteBacktestResult(id: string): Promise<void> {
    return client.delete(`/backtest/${id}`)
  }
  ```

- **getBacktestResult**: Change id type from `number` to `string`:
  ```typescript
  export function getBacktestResult(id: string): Promise<BacktestResultResponse> {
    return client.get(`/backtest/${id}`)
  }
  ```

- **cancelBacktest**: The cancel endpoint uses the backtest id, not job id:
  ```typescript
  export function cancelBacktest(id: string): Promise<void> {
    return client.post(`/backtest/${id}/cancel`)
  }
  ```

- **getBacktestTrades/getBacktestEquity**: Remove these — trades and equity are embedded in BacktestResultResponse.

### 3. BacktestConfigForm.vue — Fix fee_rate/slippage, emit format, concurrency (Bug#2,3,13)

Current file at `src/components/backtest/BacktestConfigForm.vue`:

**Bug#2 — fee_rate/slippage range:**
- Current: `fee_rate` default=0.1, max=1, step=0.01 (percentage mode, 0.1 = 10%)
- Fix: Change to decimal mode: default=0.001, max=0.01, min=0, step=0.0001, precision=4
- Current: `slippage` default=0.05, max=0.5, step=0.01 (percentage mode)
- Fix: Change to `slippage_rate`: default=0.001, max=0.01, min=0, step=0.0001, precision=4
- Remove the `%` suffix from both inputs (they are now decimals, not percentages)
- Add a hint text explaining the unit: e.g., "0.001 = 0.1%"

**Bug#3 — Emit structure:**
- The `run` event must emit the flat params that `api/backtest.ts:runBacktest()` expects (the API function handles the nesting transform):
  ```typescript
  emit('run', {
    strategy_id: form.strategy_id,
    symbol: form.symbol,
    interval: form.timeframe,  // rename: timeframe → interval
    start_date: form.dateRange[0],
    end_date: form.dateRange[1],
    initial_capital: form.initial_capital,
    fee_rate: form.fee_rate,
    slippage_rate: form.slippage,  // rename: slippage → slippage_rate
    strategy_params: { ...form.strategy_params },
  })
  ```
- Change `form.timeframe` → keep as internal UI name but emit as `interval`
- Change `form.slippage` → keep as internal UI name but emit as `slippage_rate`

**Bug#13 — Concurrency hint:**
- Replace hardcoded `"0/5 运行中"` with either:
  - Remove the hint entirely (simplest, since there's no API for running count), OR
  - Make it dynamic with a prop `runningCount`
- Simplest fix: remove the `concurrency-hint` div entirely.

**Bug#1 — initial_capital min:**
- Change `:min="1000"` to `:min="100"` to match backend validation.

### 4. BacktestView.vue — Fix polling, result parsing (Bug#4,11)

Current file at `src/views/backtest/BacktestView.vue`:

**Bug#4 — Polling endpoint:**
- Remove `pollJob()` that calls `getBacktestJob()` (deleted API)
- After `runBacktest()` returns, the response IS a `BacktestResultResponse` with `status` field
- Poll using `getBacktestResult(id)` — check `status` field:
  - `status === 'completed'` → done, use the result directly
  - `status === 'failed'` → show error from `error` field
  - `status === 'pending' | 'running'` → continue polling

New flow:
```typescript
async function handleRun(params) {
  // ... reset state ...
  const job = await backtestApi.runBacktest(params)
  currentJobId.value = job.id  // job is already a BacktestResultResponse

  if (job.status === 'completed') {
    result.value = job
    backtestState.value = 'completed'
  } else if (job.status === 'failed') {
    error.value = job.error || '回测执行失败'
    backtestState.value = 'failed'
  } else {
    // Poll for completion
    const finalResult = await pollJob(job.id)
    // ... handle finalResult ...
  }
}

function pollJob(id: string): Promise<BacktestResultResponse | null> {
  return new Promise((resolve) => {
    pollTimer = setInterval(async () => {
      try {
        const data = await backtestApi.getBacktestResult(id)
        if (data.status === 'completed') {
          clearInterval(pollTimer!)
          resolve(data)
        } else if (data.status === 'failed') {
          clearInterval(pollTimer!)
          resolve(data)
        } else if (pollCount >= MAX_POLL_ATTEMPTS) {
          clearInterval(pollTimer!)
          resolve(null)
        }
        pollCount++
      } catch (err) {
        clearInterval(pollTimer!)
        resolve(null)
      }
    }, POLL_INTERVAL)
  })
}
```

**Bug#11 — Result parsing:**
- `result.strategy_name` → doesn't exist in backend. Use `result.strategy_id` or remove
- `result.symbol` → `result.config.symbol`
- `result.initial_capital` → `result.config.initial_capital`
- `result.total_return` → `result.metrics.total_return_pct`
- `result.annual_return` → `result.metrics.annualized_return_pct`
- `result.max_drawdown` → `result.metrics.max_drawdown_pct`
- `result.sharpe_ratio` → `result.metrics.sharpe_ratio`
- `result.win_rate` → `result.metrics.win_rate`
- `result.total_trades` → `result.metrics.total_trades`
- `result.id` is now string (UUID), not number

**Bug#15 — delete:**
- `deleteBacktestResult(result.value.id)` — id is now string, works fine

**Progress display:**
- Use `result.metrics.progress` if available (from poll response), else use indeterminate

### 5. BacktestTradesTable.vue — Fix field names, direction tag, new columns (Bug#9,10,14)

Current file at `src/components/backtest/BacktestTradesTable.vue`:

**Bug#9 — Field name mapping:**
- `open_time` → `entry_time`
- `close_time` → `exit_time`
- `open_price` → `entry_price`
- `close_price` → `exit_price`
- `pnl` → `pnl_usdt`
- `pnl_percent` → `pnl_pct`
- Remove `id` column (doesn't exist in backend)
- Change `default-sort` from `{ prop: 'close_time' }` to `{ prop: 'exit_time' }`

**Bug#10 — Direction tag color:**
- Current: `row.direction === 'long' ? 'danger' : 'success'` (WRONG: long=red, short=green)
- Fix: `row.direction === 'long' ? 'success' : 'danger'` (long=green, short=red)

**Bug#14 — Add missing columns:**
Add these columns after `pnl_pct`:
- `quantity` — label: "数量", align: right, width: 100
- `fee` — label: "手续费", align: right, width: 100, format as currency
- `slippage` — label: "滑点", align: right, width: 100, format as currency  
- `holding_period_ms` — label: "持仓时长", width: 120, format ms to human-readable (e.g., "2h 30m")

**Update export CSV** to include new fields and use new field names.

**Update type import** from `BacktestTrade` to `TradeRecord`.

### 6. BacktestEquityChart.vue — Fix timestamp handling (Bug#8)

Current file at `src/components/backtest/BacktestChart.vue`:

**Bug#8 — time vs date:**
- `EquityPoint.date` (string) → `EquityPoint.time` (i64 ms timestamp)
- In `updateChart()`, convert timestamps to formatted dates for X-axis:
  ```typescript
  const dates = props.data.map((d) => {
    const dt = new Date(d.time)
    return dt.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })
  })
  ```
- Tooltip formatter should also format the timestamp properly

### 7. BacktestMetricsCards.vue — Use nested metrics (Bug#11)

Current file at `src/components/backtest/BacktestMetricsCards.vue`:

- `result.total_return` → `result.metrics.total_return_pct`
- `result.annual_return` → `result.metrics.annualized_return_pct`
- `result.sharpe_ratio` → `result.metrics.sharpe_ratio`
- `result.max_drawdown` → `result.metrics.max_drawdown_pct`
- `result.win_rate` → `result.metrics.win_rate`
- `result.total_trades` → `result.metrics.total_trades`
- Add new metric cards for: sortino_ratio, calmar_ratio, profit_factor (handle null/Infinity), avg_win_pct, avg_loss_pct

## Test Updates Required

All 5 backtest test files need updating:

1. `src/__tests__/api/backtest.test.ts`:
   - `runBacktest` test: verify it sends nested `{ strategy_id, config: {...} }` structure
   - `getBacktestJob` test: REMOVE (endpoint deleted)
   - `listBacktestHistory` test: expect `PaginatedResponse` shape
   - `deleteBacktestResult` test: id is string, not number
   - `getBacktestResult` test: id is string, response has nested metrics/config
   - Remove `getBacktestTrades` and `getBacktestEquity` tests (endpoints removed)

2. `src/__tests__/components/BacktestConfigForm.test.ts`:
   - `fee_rate` default: 0.001 (not 0.1)
   - `slippage` default: 0.001 (not 0.05)
   - emitted `run` event should have `interval` (not `timeframe`) and `slippage_rate` (not `slippage`)
   - Remove test for concurrency hint text (or update it)
   - initial_capital min should be 100

3. `src/__tests__/components/BacktestTradesTable.test.ts`:
   - Use `TradeRecord` type with new field names
   - Direction tag: long=success, short=danger

4. `src/__tests__/components/BacktestEquityChart.test.ts`:
   - Use `EquityPoint` with `time: number` (ms timestamp)

5. `src/__tests__/components/BacktestMetricsCards.test.ts`:
   - Use `BacktestResultResponse` with nested `metrics` and `config`

6. `src/__tests__/views/BacktestView.test.ts`:
   - Polling uses `getBacktestResult(id)` not `getBacktestJob(id)`
   - Result uses nested metrics/config

## Important Notes
- Run `./node_modules/.bin/vitest` (NOT `npx vitest` which times out)
- ECharts needs mock in tests (no Canvas in jsdom) — check existing test for the mock pattern
- TypeScript strict mode — no `any` types unless absolutely necessary
- All existing test patterns should be preserved (mock patterns, mount patterns, etc.)
- After all changes, run `./node_modules/.bin/vitest` and ensure ALL tests pass (currently 145 tests)
