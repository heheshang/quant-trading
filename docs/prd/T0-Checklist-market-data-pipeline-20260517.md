# T0-Checklist-market-data-pipeline-20260517

## 功能名称
**市场数据管道** — 接入 Binance 实时行情，替换全量 Mock 数据

## 评分结果

| 检查项 | 分值 | 级别 | 得分 | 备注 |
|--------|------|------|------|------|
| 功能范围明确 | 25 | P0 | 25 | 4个Phase：Redis缓存→BinanceConnector→WS Hub→历史数据 |
| 用户角色明确 | 20 | P0 | 20 | 量化交易者（看盘/下单）、系统（自动推送） |
| 边界清晰 | 15 | P1 | 15 | 仅 Binance，单币种WS暂不支持多账号/多交易所 |
| 数据来源明确 | 15 | P1 | 15 | Binance WS → Redis → Axum WS → Vue，PostgreSQL 持久化 |
| 非功能需求 | 10 | P2 | 8 | 延迟<500ms，1000并发，需 Redis 高可用 |
| 依赖关系 | 10 | P2 | 10 | 依赖 Redis（已部署）、docker-compose（已就绪） |
| 验收标准可量化 | 5 | P2 | 5 | ticker API 延迟<1s，WS推送≥1条/秒，Mock降级可用 |

**总分：98 / 100**

## P0 通过项
- ✅ 功能范围：实现 ADR-005 统一数据网关，Phase 1-4 清晰
- ✅ 用户角色：量化交易者使用行情/下单，系统自动采集推送

## 风险项
- ⚠️ 非功能需求：Redis 哨兵/集群未配置，单点故障风险（建议上线前配置）

## 需求澄清记录

**Q1: 是否需要支持多交易所？**
A: Phase 1 仅 Binance，架构预留多交易所扩展（Phase 2+）

**Q2: 历史数据保留多久？**
A: 90天清理（ADR-005），1m K线保留30天，4h/1d K线永久

**Q3: Redis 故障时的降级策略？**
A: Mock 数据兜底，Redis 恢复后自动切回真实数据

**Q4: 是否需要实盘交易对接？**
A: Phase 1-4 仅行情数据，实盘交易 Broker 对接在 P1 待开发

## 归档产出物
- `docs/design/PRD-market-data-pipeline-implementation.md` — 实现计划
- `docs/architecture/ADR-005-market-data-pipeline.md` — 架构设计（已存在）
- `docs/architecture/ADR-002-websocket-realtime.md` — WS 设计（已存在）

## T0 结果
**✅ 通过（98分）** — 可进入 T1 流水线