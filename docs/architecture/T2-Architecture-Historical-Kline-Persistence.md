# T2: Architecture — Historical Kline Persistence

**Phase**: Phase 4
**Date**: 2026-05-17

---

## Data Flow

```
Binance WS Stream
        │
        ▼
┌───────────────────┐
│   WsHub           │  ← already exists (Phase 3)
│   - KlineEvent    │
│     variant       │
└────────┬──────────┘
         │ HubEvent::Kline(...)
         ▼
┌───────────────────┐
│  KlineWriter      │  ← NEW: background task
│  - mpsc receiver  │
│  - buffer (100)   │
│  - bulk INSERT    │
└────────┬──────────┘
         │ ON CONFLICT DO NOTHING
         ▼
┌───────────────────┐
│  PostgreSQL       │
│  klines (range)   │
└───────────────────┘
```

---

## Entity Definition (SeaORM)

Location: `backend/src/models/kline_entity.rs` (NEW)

```rust
use sea_orm::entity::prelude::*;

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

## Migration SQL

Location: `backend/migrations/YYYYMMDDHHMMSS_create_klines_table.sql`

```sql
CREATE TABLE IF NOT EXISTS klines (
    id              UUID        NOT NULL DEFAULT gen_random_uuid(),
    symbol          VARCHAR(20) NOT NULL,
    interval        VARCHAR(10) NOT NULL,
    open_time       TIMESTAMPTZ NOT NULL,
    close_time      TIMESTAMPTZ NOT NULL,
    open            DECIMAL(20,8) NOT NULL,
    high            DECIMAL(20,8) NOT NULL,
    low             DECIMAL(20,8) NOT NULL,
    close           DECIMAL(20,8) NOT NULL,
    volume          DECIMAL(20,8) NOT NULL,
    quote_volume    DECIMAL(20,8) NOT NULL,
    trades          INT         NOT NULL DEFAULT 0,
    source          VARCHAR(20) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, open_time)
) PARTITION BY RANGE (open_time);

-- Initial partitions
CREATE TABLE klines_202605 PARTITION OF klines
    FOR VALUES FROM ('2026-05-01') TO ('2026-06-01');
CREATE TABLE klines_202606 PARTITION OF klines
    FOR VALUES FROM ('2026-06-01') TO ('2026-07-01');
CREATE TABLE klines_202607 PARTITION OF klines
    FOR VALUES FROM ('2026-07-01') TO ('2026-08-01');

CREATE UNIQUE INDEX uq_klines ON klines (symbol, interval, open_time);
CREATE INDEX idx_klines_lookup ON klines (symbol, interval, open_time DESC);
```

---

## Buffer Flush Logic (Pseudocode)

```rust
const MAX_BUFFER_SIZE: usize = 100;
const FLUSH_INTERVAL: Duration = Duration::from_secs(5);

struct KlineWriter {
    buffer: Vec<KlineRecord>,
    last_flush: Instant,
}

impl KlineWriter {
    async fn run(&mut self, mut rx: Receiver<KlineRecord>) {
        loop {
            tokio::select! {
                // New KlineEvent from WsHub
                kline = rx.recv() => {
                    self.buffer.push(kline);
                    if self.buffer.len() >= MAX_BUFFER_SIZE {
                        self.flush().await;
                    }
                }
                // Periodic flush
                _ = tokio::time::sleep(FLUSH_INTERVAL) => {
                    if !self.buffer.is_empty() {
                        self.flush().await;
                    }
                }
            }
        }
    }

    async fn flush(&mut self) {
        let batch = std::mem::take(&mut self.buffer);
        // bulk_insert(batch).await; // ON CONFLICT DO NOTHING
        self.last_flush = Instant::now();
    }
}
```

---

## API Changes

### New: `GET /api/v1/kline/history`

Existing `get_klines` in `kline.rs` will be modified:

```rust
// OLD: query_klines → calls Binance REST
// NEW: get_historical_klines → DB first, fallback Binance REST
pub async fn get_historical_klines(
    db: &DatabaseConnection,
    symbol: &str,
    interval: &str,
    start: DateTime,
    end: DateTime,
) -> Result<Vec<KlineResponse>, AppError> {
    // Try DB
    let from_db = kline_entity::Entity::find()
        .filter(Column::Symbol.eq(symbol))
        .filter(Column::Interval.eq(interval))
        .filter(Column::OpenTime.gte(start))
        .filter(Column::OpenTime.lte(end))
        .order_by_desc(Column::OpenTime)
        .all(db)
        .await?;

    if !from_db.is_empty() {
        return Ok(from_db.into_iter().map(Into::into).collect());
    }

    // Fallback: Binance REST (existing code)
    get_from_binance_rest(symbol, interval, start, end).await
}
```

---

## WsHub Integration

In `ws_hub.rs`, on receiving HubEvent::Kline:

```rust
// In WsHub::start() — spawn writer task
let (tx, rx) = mpsc::channel(1000);
let writer_tx = tx.clone();

spawn(async move {
    let mut writer = KlineWriter::new(rx);
    writer.run().await;
});

// Store sender in WsHub state
self.kline_writer_tx = Some(writer_tx);

// On HubEvent::Kline(kline_data):
if let Some(ref tx) = self.kline_writer_tx {
    let _ = tx.try_send(kline_data); // non-blocking, fire-and-forget
}
```

---

## Key Design Decisions

1. **mpsc channel over direct DB write**: decouples WsHub broadcast from DB latency
2. **ON CONFLICT DO NOTHING**: idempotent inserts, safe to retry
3. **try_send vs send**: non-blocking — if buffer full, skip this candle (logged)
4. **Partition per month**: balances query performance vs partition management overhead
5. **DB-first, REST fallback**: serves from cache when DB has data, preserves Binance as source of truth for gaps