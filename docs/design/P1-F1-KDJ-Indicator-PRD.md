# P1-F1: KDJ 指标开发 PRD

## 1. 概述

**功能名称**: KDJ 技术指标
**功能ID**: P1-F1
**所属阶段**: Phase 1
**优先级**: P1
**描述**: 在 K 线数据上计算 KDJ 随机指标，提供金叉/死叉/超买/超卖信号，支撑交易策略和回测。

---

## 2. 指标算法

### 2.1 KDJ 计算公式

- **RSV (Raw Stochastic Value)**: `RSV = (Close - Lowest(L, N)) / (Highest(H, N) - Lowest(L, N)) × 100`
  - N = 9 (默认)
  - Lowest/LowestN: N 日最低价
  - Highest/HighestN: N 日最高价
- **K**: K = SMA(RSV, M1), M1 = 3 (默认), 平滑均线
- **D**: D = SMA(K, M2), M2 = 3 (默认)
- **J**: J = 3 × K - 2 × D

### 2.2 信号定义

| 信号类型 | 条件 |
|----------|------|
| 金叉 (Golden Cross) | K 从下方向上穿越 D，且 K < 80 |
| 死叉 (Death Cross) | K 从上方向下穿越 D，且 K > 20 |
| 超买 (Overbought) | K > 80 AND D > 80 |
| 超卖 (Oversold) | K < 20 AND D < 20 |

---

## 3. 后端实现

### 3.1 Rust Module: `src/services/indicator.rs`

新增 `indicator` service module，实现 `Kdjk` 结构体：

```rust
pub struct Kdjk { n: usize, m1: usize, m2: usize }
impl Kdjk { pub fn calculate(&self, klines: &[KlineInput]) -> Vec<KdjResult> }
```

### 3.2 API Endpoint

- **GET `/api/v1/kline/kdj`** — 计算 KDJ 指标
  - Query: `symbol`, `interval`, `start_time`, `end_time`, `n` (default 9), `m1` (default 3), `m2` (default 3)
  - Response: `KdjResponse { data: Vec<KdjBar> }`

### 3.3 Response Schema

```json
{
  "data": [
    {
      "open_time": 1700000000000,
      "k": 45.5,
      "d": 42.1,
      "j": 52.3,
      "signal": "none"
    }
  ],
  "params": { "n": 9, "m1": 3, "m2": 3 },
  "symbol": "btcusdt",
  "interval": "1h"
}
```

`signal` 枚举: `"golden_cross" | "death_cross" | "overbought" | "oversold" | "none"`

---

## 4. 前端实现

### 4.1 TypeScript Types: `frontend/src/types/indicator.ts`

```typescript
export interface KdjBar {
  open_time: number
  k: number
  d: number
  j: number
  signal: 'golden_cross' | 'death_cross' | 'overbought' | 'oversold' | 'none'
}

export interface KdjResponse {
  data: KdjBar[]
  params: { n: number; m1: number; m2: number }
  symbol: string
  interval: string
}
```

### 4.2 API Client: `frontend/src/api/indicator.ts`

```typescript
export function getKdj(params: KdjParams): Promise<KdjResponse>
```

### 4.3 Chart Overlay: `KlineChart.vue` 扩展

- Props: `kdjData?: KdjBar[]`
- 在 Candlestick chart 上叠加 K/D/J line series
- 金叉/死叉用箭头 marker 标记
- 超买/超卖区域用背景色标注

---

## 5. 验收条件

1. 后端 `cargo check` 通过，无 error
2. GET `/api/v1/kline/kdj?symbol=btcusdt&interval=1h` 返回正确格式
3. 前端 `vue-tsc --noEmit` 通过
4. KDJ 值与标准算法一致（K,D 在 0-100 范围内）
5. 金叉/死叉/超买/超卖信号正确触发

---

## 6. 文件清单

### 新增文件
- `backend/src/services/indicator.rs`
- `backend/src/handlers/indicator.rs`
- `frontend/src/types/indicator.ts`
- `frontend/src/api/indicator.ts`
- `frontend/src/components/charts/KdjOverlay.vue`

### 修改文件
- `backend/src/services/mod.rs` — 添加 `indicator` module
- `backend/src/handlers/mod.rs` — 添加 `indicator` handler
- `backend/src/models/schemas.rs` — 添加 `KdjResponse` 等类型
- `backend/src/main.rs` — 注册 kdj 路由
- `frontend/src/components/charts/KlineChart.vue` — KDJ overlay 支持
- `frontend/src/types/kline.ts` — KDJ 类型引用
