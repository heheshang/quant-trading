# ADR-007: Historical Kline Persistence

**Status**: Proposed
**Date**: 2026-05-17
**Deciders**: @heheshang

---

## Context

Phases 1-3 built a real-time market data pipeline (Redis cache → Binance WS → WsHub broadcast). Phase 4 adds persistent storage of Kline/candlestick data to PostgreSQL for backtesting and historical analysis.

---

## Decision

### Schema

```sql
CREATE TABLE klines (
    id          UUID        NOT NULL DEFAULT gen_random_uuid(),
    symbol      VARCHAR(20) NOT NULL,
    interval    VARCHAR(10) NOT NULL,  -- "1m", "5m", "15m", "1h", "4h", "1d"
    open_time   TIMESTAMPTZ NOT NULL,
    close_time  TIMESTAMPTZ NOT NULL,
    open        DECIMAL(20,8) NOT NULL,
    high        DECIMAL(20,8) NOT NULL,
    low         DECIMAL(20,8) NOT NULL,
    close       DECIMAL(20,8) NOT NULL,
    volume      DECIMAL(20,8) NOT NULL,
    quote_volume DECIMAL(20,8) NOT NULL,
    trades      INT         NOT NULL DEFAULT 0,
    source      VARCHAR(20) NOT NULL, -- "binance_ws", "binance_rest"
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, open_time)
) PARTITION BY RANGE (open_time);

CREATE UNIQUE INDEX uq_klines ON klines (symbol, interval, open_time);
CREATE INDEX idx_klines_lookup ON klines (symbol, interval, open_time DESC);
```

### Write Path

```
WsHub KlineEvent
    → mpsc::Sender<KlineRecord>
    → Background writer task (spawned on server start)
    → Accumulate buffer (max 100 records OR 5s timeout)
    → bulk_insert into klines table (ON CONFLICT DO NOTHING)
    → On error: log error, do NOT block WsHub broadcast
```

### Read Path

```
get_historical_klines(db, symbol, interval, start, end)
    → SELECT FROM klines WHERE symbol=? AND interval=? AND open_time BETWEEN ? AND ?
    → If rows == 0: fallback to Binance REST
    → Return Vec<KlineResponse>
```

### Partition Strategy

- Monthly partitions: `klines_YYYYMM`
- Auto-create partition 3 months ahead via cron job
- Retention: DELETE old partitions via `DROP TABLE klines_YYYYMM`

### SeaORM Entity

```rust
#[derive(Clone, DeriveEntityModel)]
#[sea_orm(table_name = "klines")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub symbol: String,
    pub interval: String,
    pub open_time: ChronoDateTime,
    pub close_time: ChronoDateTime,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub quote_volume: Decimal,
    pub trades: i32,
    pub source: String,
    pub created_at: ChronoDateTime,
}
```

---

## Consequences

### Positive
- Historical data available for backtesting without hitting Binance rate limits
- Kline query API can serve from DB with sub-ms latency
- WsHub broadcast remains unblocked regardless of DB write health

### Negative
- DB storage growth (mitigate: monthly partitions + retention cron)
- Bulk insert complexity: need to handle partial failures, buffer overflow

### Risks
- KlineEvent frequency: 1m candles update every second per symbol — buffer must drain faster than production rate
- Partition creation lag: gap between "now" and next partition start time