# PRD: Phase 4 历史K线数据持久化

**项目**: 行情数据管道 - 历史K线持久化
**日期**: 2026-05-17
**状态**: Proposed
**负责人**: @heheshang

---

## 背景

Phase 1-3 已实现：
- Phase 1: Redis 缓存 + Binance REST（实时行情缓存）
- Phase 2: Binance WebSocket Connector（实时行情流）
- Phase 3: WebSocket Hub（广播到已连接客户端）

**缺失**: 历史K线数据持久化（PostgreSQL），用于回测和历史分析。

---

## 需求列表

| # | 需求 | 优先级 | 状态 |
|---|------|--------|------|
| 1 | 存储 Binance WS 推送的 KlineEvent 到 PostgreSQL | P0 | 待开发 |
| 2 | 分区表：`klines`（按月分区），UNIQUE(symbol, interval, open_time） | P0 | 待开发 |
| 3 | 批量写入缓冲：积累 100 条或 5 秒超时后 flush | P0 | 待开发 |
| 4 | WsHub KlineEvent → 异步非阻塞写入（不阻塞广播） | P0 | 待开发 |
| 5 | 历史K线查询 API（从 DB 而非 Binance） | P1 | 待开发 |
| 6 | 写入失败时记录 error log，不阻塞 WsHub | P0 | 待开发 |
| 7 | 分区管理：按月分区，手动清理 cron | P2 | 待开发 |
| 8 | 支持多种周期：1m, 5m, 15m, 1h, 4h, 1d | P0 | 待开发 |
| 9 | symbol_configs 支持自定义交易对 | P2 | 待开发 |

---

## T0 需求门禁

| 需求 | 通过 | 备注 |
|------|------|------|
| 1. 存储 Kline 到 PG | ✅ |  |
| 2. 分区表设计 | ✅ |  |
| 3. 批量缓冲 | ✅ |  |
| 4. 异步非阻塞写入 | ✅ |  |
| 5. 历史查询 API | ✅ |  |
| 6. 写入失败不阻塞 | ✅ |  |
| 7. 分区管理 | ⚠️ | 可后期追加 |
| 8. 多周期支持 | ✅ |  |
| 9. 自定义交易对 | ⚠️ | 依赖 symbol_configs |

**评分**: 7/9 ✅ **77.8/100**

---

## 技术约束

- PostgreSQL 分区表（每月一个分区）
- SeaORM Entity 类型安全访问
- WsHub KlineEvent → mpsc channel → 后台 writer task → bulk INSERT
- Rust 2021 edition, async/await + tokio
- 非阻塞写入：写入失败降级日志，不影响实时推送