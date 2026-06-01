# T8 签字记录 — P1-F5 分表存储

- Feature: P1-F5 K线按月分区
- Date: 2026-05-21
- PM: ssk
- TechLead: ssk

## 签字

| 角色 | 签字 | 日期 |
|------|------|------|
| PM | ssk | 2026-05-21 |
| TechLead | ssk | 2026-05-21 |

## 备注
- Migration 需要在 PostgreSQL 14+ 环境下执行
- 生产部署前需备份现有 klines 数据
- 首次部署后手动运行 ensure_future_partitions()