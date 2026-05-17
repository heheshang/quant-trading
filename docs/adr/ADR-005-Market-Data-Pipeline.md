# ADR-005: Market Data Pipeline — Historical Data Persistence

**Status**: Proposed
**Date**: 2026-05-17
**Deciders**: @heheshang

---

## Context

The market data pipeline currently supports:
- **Phase 1**: Redis cache layer + Binance REST API (live ticker caching)
- **Phase 2**: Binance WebSocket Connector (real-time stream)
- **Phase 3**: WebSocket Hub (broadcast to connected clients)

Missing: **Phase 4** — persistent storage of historical Kline/candlestick data to PostgreSQL for backtesting and historical analysis.

---

## Decision

Store Kline data in PostgreSQL with the following schema:

```
Table: klines_1m (partitioned by time range)
  - id: UUID PRIMARY KEY
  - symbol: VARCHAR NOT NULL
  - open_time: TIMESTAMPTZ NOT NULL
  - close_time: TIMESTAMPTZ NOT NULL
  - open: DECIMAL(20,8) NOT NULL
  - high: DECIMAL(20,8) NOT NULL
  - low: DECIMAL(20,8) NOT NULL
  - close: DECIMAL(20,8) NOT NULL
  - volume: DECIMAL(20,8) NOT NULL
  - quote_volume: DECIMAL(20,8) NOT NULL
  - trades: INT NOT NULL
  - interval: VARCHAR NOT NULL (e.g., "1m", "5m", "1h", "1d")
  - source: VARCHAR NOT NULL ("binance_ws" | "binance_rest")
  - created_at: TIMESTAMPTZ NOT NULL DEFAULT NOW()

Indexes:
  - UNIQUE (symbol, interval, open_time)
  - INDEX (symbol, interval, open_time DESC)
  - INDEX (open_time)
```

---

## Consequences

### Positive
- Historical data available for backtesting
- Kline query API can serve from DB instead of Binance (rate limit safe)
- Supports user-created symbol_configs for custom pairs

### Negative
- DB storage growth (mitigate: partition by month, retention policy)
- WsHub Kline events need to be written to DB (async, non-blocking)

### Technical Approach
- WsHub KlineEvent variant → spawn blocking task to write to DB
- Bulk insert buffer (accumulate 100 records or 5s timeout before flush)
- Partition management via pg_partman or manual cron