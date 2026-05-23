# 前后端字段不匹配报告

## 概述

本报告逐项对比前端 views 调用的 API 与后端实际返回，找出字段命名、类型、嵌套结构等不匹配问题。

---

## 1. Dashboard Stats API — `GET /api/v1/dashboard/stats`

| 方向 | 字段 | 问题 |
|------|------|------|
| 后端 → 前端 | `totalPnlToday` | 前端期望 `totalPnl` 或 `dailyPnl` |
| 后端 → 前端 | `totalUsers` | 前端无此字段 |
| 后端 → 前端 | `totalOrdersToday` | 前端期望 `totalTrades` |
| 后端 → 前端 | `activePositions` | 前端有但类型不同（后端 i64，前端 number） |
| 后端缺失 | `balance` | 前端期望 `balance: number`，后端无此字段 |
| 后端缺失 | `dailyPnl` / `totalPnl` | 前端期望 `dailyPnl` 或 `totalPnl`，后端只返回 `totalPnlToday` |

**详细**:
- 后端 `DashboardStats` (`services/dashboard.rs`): `{ totalPnlToday, winRate, sharpeRatio, activePositions, totalUsers, totalOrdersToday }` (camelCase, f64/i64)
- 前端 `DashboardStats` (`types/dashboard.ts`): `{ totalPnl, dailyPnl, winRate, sharpeRatio, totalTrades, activePositions, balance }` (camelCase, number)

**结论**: 严重不匹配 — 核心字段名完全不同

---

## 2. Portfolio Summary API — `GET /api/v1/portfolio/summary`

| 方向 | 字段 | 问题 |
|------|------|------|
| 类型不匹配 | 全部字段 | 后端返回 string（`"100000.00"`），前端期望 number |

- 后端 `PortfolioSummary`: `{ total_equity: String, daily_pnl: String, daily_pnl_rate: String, cumulative_pnl: String, cumulative_pnl_rate: String, total_positions: i64, updated_at: String }`
- 前端 `PortfolioSummary`: `{ total_equity: string, daily_pnl: string, daily_pnl_rate: string, cumulative_pnl: string, cumulative_pnl_rate: string, total_positions: number, updated_at: string }`

**结论**: 字段名一致，但 `total_positions` 类型不一致（后端 i64 vs 前端 number），其余为 string vs string ✅

---

## 3. Portfolio Performance API — `GET /api/v1/portfolio/performance`

| 方向 | 字段 | 问题 |
|------|------|------|
| 后端缺失 | `max_drawdown` | 顶层无此字段 |
| 后端缺失 | `sharpe_ratio` | 顶层无此字段 |
| 后端缺失 | `win_rate` | 顶层无此字段 |
| 后端 → 前端 | `strategies[].trade_count` | 后端返回 u32，前端期望 number |

**详细**:
- 后端 `PortfolioPerformance`: `{ strategies: StrategyPerformance[] }` （仅此字段）
- 前端 `PortfolioPerformance`: `{ strategies: StrategyPerformance[], max_drawdown: string, sharpe_ratio: string, win_rate: string }`

**结论**: 严重不匹配 — 前端期望的 3 个顶层聚合字段后端未返回

---

## 4. Portfolio Equity Curve API — `GET /api/v1/portfolio/equity_curve`

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名不匹配 | `equity` | 后端返回 `equity`，前端期望 `equity` ✅ |

- 后端 `EquityCurve`: `{ points: [{ timestamp, equity }] }` (string, string)
- 前端 `EquityCurve`: `{ points: [{ timestamp, equity }] }` (string, string)

**结论**: 一致 ✅

---

## 5. Kline API — `GET /api/v1/market/kline`

| 方向 | 字段 | 问题 |
|------|------|------|
| 嵌套结构错误 | 整体结构 | 后端返回 `{ data: { data: [...], meta: {...} } }`，前端期望直接是数组 `[...]` |
| 字段缺失 | 多个字段 | 后端返回 `id, user_id, symbol, interval, open_time, close_time, quote_volume, trades, source, created_at`，前端 `Kline` 类型只有 `timestamp, open, high, low, close, volume` |

- 后端 `KlineListResponse`: `{ data: Vec<KlineResponse>, meta: KlineListMeta }`
- 前端 `Kline[]`: `[{ timestamp, open, high, low, close, volume }]`

**结论**: 严重不匹配 — 响应结构完全不同

---

## 6. Ticker API — `GET /api/v1/market/tickers` & `GET /api/v1/market/ticker`

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名一致 | 所有字段 | 后端 `TickerResponse` 与前端 `Ticker` 接口字段完全匹配 ✅ |
| 类型一致 | 所有字段 | 均为 number ✅ |

**结论**: 一致 ✅

---

## 7. Depth API — `GET /api/v1/market/depth`

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名一致 | 所有字段 | 后端 `DepthResponse` 与前端 `Depth` 接口字段完全匹配 ✅ |
| 类型一致 | 所有字段 | 均为 number ✅ |

**结论**: 一致 ✅

---

## 8. Risk Rules API — `GET|PUT /api/v1/risk/rules`

| 方向 | 字段 | 问题 |
|------|------|------|
| 类型不匹配 | `user_id` | 后端返回 Uuid（string 格式），前端类型定义为 `number \| undefined` |
| 类型不匹配 | `atr_period` | 后端返回 `Option<i32>`，前端类型为 `number \| null` |
| 类型不匹配 | `atr_multiplier` | 后端返回 `Option<String>`，前端类型为 `string \| null` |

- 后端 `RiskRulesResponse`: 所有字段为 String（`daily_loss_limit: String` 等），`atr_period: Option<i32>`, `atr_multiplier: Option<String>`, `is_active: bool`
- 前端 `RiskRules`: `daily_loss_limit: string`, `atr_period: number | null`, `atr_multiplier: string | null`, `is_active: boolean`

**结论**: 部分不匹配 — 类型定义风格不同（后端用 String 对象包装，前端用原始类型）

---

## 9. Risk Logs API — `GET /api/v1/risk/logs`

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名不匹配 | `triggered_rule` vs `rule_type` | 后端 Response 用 `#[serde(rename = "triggered_rule")]`，但实际映射的是 DB 的 `rule_type` |
| 字段名不匹配 | `equity_snapshot` vs `equity_snapshot` | 前端期望 `equity_snapshot`，但后端映射的是 `account_equity` |
| 字段名不匹配 | `threshold_snapshot` vs `threshold_snapshot` | 同上 |
| 字段名不匹配 | `triggered_at` vs `created_at` | 后端实际用 `created_at` 别名 |
| 后端缺失 | `severity` | 后端硬编码为 `"medium"`，不是前端期望的 `low\|medium\|high\|critical` 类型 |
| 类型不匹配 | `id` | 后端返回 Uuid（string），前端类型为 `string` ✅ |

**结论**: 严重不匹配 — 字段映射混乱，使用 serde rename 但映射到了错误的源字段

---

## 10. Emergency Close API — `POST /api/v1/risk/emergency-close`

| 方向 | 字段 | 问题 |
|------|------|------|
| 后端缺失 | `success` | 后端无此布尔字段 |
| 后端缺失 | `message` | 后端无此消息字段 |
| 字段类型不匹配 | `closed_orders` vs `details` | 后端返回 `closed_orders: Vec<EmergencyCloseOrder>`，前端期望 `details: EmergencyCloseOrder[]` |
| 类型不匹配 | `total_pnl` | 后端返回 `String`，前端期望 `string` ✅ |

**详细**:
- 后端 `EmergencyCloseResult`: `{ closed_positions: number, total_pnl: String, closed_orders: Vec<...> }`
- 前端 `EmergencyCloseResponse`: `{ success: boolean, message: string, closed_positions: number, total_pnl: string, details: [...] }`

**结论**: 严重不匹配 — 字段名和字段都不一致

---

## 11. Pause/Resume API — `POST /api/v1/risk/pause` & `POST /api/v1/risk/resume`

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名不匹配 | `reason` | 后端 `PauseResponse` 有 `reason`，前端 `PauseResponse` 期望 `reason` ✅ |
| 类型一致 | `paused` | 均为 boolean ✅ |

- 后端 `PauseResponse`: `{ paused: bool, reason: String }`
- 前端 `PauseResponse`: `{ paused: boolean, reason: string }`

**结论**: 一致 ✅

---

## 12. Connection Status API — `GET /api/v1/risk/connection-status`

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名不匹配 | `disconnect_elapsed_secs` | 前端期望 `disconnect_elapsed_secs`，后端 `ConnectionStatus` 有此字段 ✅ |
| 类型一致 | 所有字段 | ✅ |

- 后端 `ConnectionStatus`: `{ exchange_connected: bool, disconnect_elapsed_secs: u64, strategy_paused: bool }`
- 前端 `ConnectionStatus`: `{ exchange_connected: boolean, disconnect_elapsed_secs: number, strategy_paused: boolean }`

**结论**: 一致 ✅

---

## 13. Order APIs — `GET /api/v1/orders`, `POST /api/v1/orders`, etc.

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名一致 | 所有字段 | 后端 `OrderResponse` 与前端 `Order` 接口字段完全匹配 ✅ |
| 类型一致 | 所有字段 | ✅ |

**结论**: 一致 ✅

---

## 14. Cancel Order Response — `POST /api/v1/orders/:id/cancel`

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名一致 | 所有字段 | 后端 `CancelResult` 与前端 `CancelOrderResponse` 字段匹配 ✅ |

**结论**: 一致 ✅

---

## 15. Symbol Config API — `GET /api/v1/symbols`

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名一致 | 所有字段 | 后端 `SymbolConfigResponse` 与前端 `SymbolConfig` 字段匹配 ✅ |
| 类型一致 | 所有字段 | ✅ |

**结论**: 一致 ✅

---

## 16. Account API — `GET /api/v1/account`

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名一致 | 所有字段 | 后端 `AccountResponse` 与前端 `PaperAccount` 字段匹配 ✅ |
| 类型一致 | 所有字段 | ✅ |

**结论**: 一致 ✅

---

## 17. Position APIs — `GET /api/v1/positions`

| 方向 | 字段 | 问题 |
|------|------|------|
| 字段名一致 | 所有字段 | 后端 `PositionResponse` 与前端 `Position` 字段匹配 ✅ |
| 类型一致 | 所有字段 | ✅ |

**结论**: 一致 ✅

---

## 18. Strategy APIs — `GET /api/v1/strategies`, etc.

| 方向 | 字段 | 问题 |
|------|------|------|
| 嵌套结构 | `parameters` | 后端返回 `serde_json::Value`，前端期望 `Record<string, unknown>`，兼容 ✅ |
| 字段名一致 | 基本字段 | ✅ |

**结论**: 基本一致 ✅

---

## 总结

### 🔴 严重不匹配（需立即修复）

1. **Dashboard Stats** — 字段名完全不同
2. **Portfolio Performance** — 缺少 3 个顶层聚合字段
3. **Kline** — 响应结构完全不同（对象嵌套 vs 数组）
4. **Risk Logs** — 字段映射混乱，使用 rename 但映射到错误源字段
5. **Emergency Close** — 字段名和字段都不同

### 🟡 中等不匹配

1. **Portfolio Summary** — `total_positions` 类型不一致
2. **Risk Rules** — `user_id` 类型定义不匹配（Uuid vs number）

### 🟢 一致

1. Ticker APIs
2. Depth API
3. Order APIs
4. Position APIs
5. Symbol Config API
6. Account API
7. Portfolio Summary（除类型外）
8. Equity Curve
9. Connection Status
10. Pause/Resume
11. Strategy APIs

---

## 修复优先级建议

1. **P0（必须修复）**: Dashboard Stats, Portfolio Performance, Kline, Risk Logs, Emergency Close
2. **P1（建议修复）**: Portfolio Summary 的 total_positions 类型, Risk Rules 的 user_id 类型
