# K 线数据管理 API 文档

> 版本: v0.5.0 | 基础路径: `/api/v1` | 认证: Bearer Token (JWT)

---

## 目录

- [API 总览](#api-总览)
- [1. GET /kline/query — K 线数据查询](#1-get-klinequery--k-线数据查询)
- [2. POST /kline/import — K 线数据导入](#2-post-klineimport--k-线数据导入)
- [3. GET /kline/export — K 线数据导出](#3-get-klineexport--k-线数据导出)
- [4. DELETE /kline — K 线数据删除](#4-delete-kline--k-线数据删除)
- [5. GET /kline/quality — 数据质量报告](#5-get-klinequality--数据质量报告)
- [6. POST /kline/clean — 数据清洗](#6-post-klineclean--数据清洗)
- [7. GET /kline/latest — 最新 K 线](#7-get-klinelatest--最新-k-线)
- [8. GET /kline/import-history — 导入历史](#8-get-klineimport-history--导入历史)
- [数据模型](#数据模型)
- [错误码](#错误码)

---

## API 总览

| 方法 | 路径 | 说明 | 认证 |
|------|------|------|------|
| GET | `/api/v1/kline/query` | 分页查询 K 线数据（symbol/interval/时间范围） | 必需 |
| POST | `/api/v1/kline/import` | 批量导入 K 线数据（CSV / API / 交易所直采） | 必需 |
| GET | `/api/v1/kline/export` | 导出 K 线数据为 CSV 或 JSON | 必需 |
| DELETE | `/api/v1/kline` | 删除指定范围的 K 线数据 | 必需 |
| GET | `/api/v1/kline/quality` | 获取数据质量检测报告 | 必需 |
| POST | `/api/v1/kline/clean` | 执行数据清洗（自动模式） | 必需 |
| GET | `/api/v1/kline/latest` | 获取指定交易对 + 周期的最新一根 K 线 | 必需 |
| GET | `/api/v1/kline/import-history` | 获取导入历史记录 | 必需 |

> **扩展端点（P2 阶段）**
>
> | 方法 | 路径 | 说明 |
> |------|------|------|
> | DELETE | `/api/v1/kline/clean/rollback` | 回滚至上一次清洗前（需 `backup_id`） |
> | POST | `/api/v1/kline/fetch` | 交易所直采（仅 pro-trader 角色） |

---

## 统一响应格式

### 成功响应

```json
{
  "code": 0,
  "data": { ... },
  "message": "success"
}
```

### 分页响应

```json
{
  "code": 0,
  "data": {
    "items": [...],
    "page": 1,
    "page_size": 1000,
    "total": 8760,
    "gap_detected": false
  },
  "message": "success"
}
```

### 错误响应

```json
{
  "code": 40001,
  "message": "请求参数错误: symbol 为必填项"
}
```

---

## 1. GET /kline/query — K 线数据查询

分页查询当前用户的 K 线数据，支持按交易对、周期、时间范围筛选。所有查询强制以 `user_id` 过滤（从 JWT 提取），确保多用户数据隔离。

### 请求

```
GET /api/v1/kline/query?symbol=BTCUSDT&interval=1h&start_time=1704067200000&end_time=1735689600000&page=1&page_size=1000
```

### 查询参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `symbol` | string | 是 | — | 交易对，如 `BTCUSDT`、`ETHUSDT` |
| `interval` | string | 是 | — | K 线周期：`1m` / `5m` / `15m` / `30m` / `1h` / `4h` / `1d` / `1w` |
| `start_time` | integer | 是 | — | 起始时间戳（毫秒），如 `1704067200000` |
| `end_time` | integer | 是 | — | 结束时间戳（毫秒） |
| `page` | integer | 否 | `1` | 页码，从 1 开始 |
| `page_size` | integer | 否 | `1000` | 每页条数，范围 1~5000 |

### 约束

- `end_time - start_time` 不得超过 10 年，否则返回 `400`
- 结果按 `open_time ASC` 排序
- 连续超过 8 小时无数据时，`gap_detected` 标记为 `true`
- 先查 Redis 缓存，未命中再查 PostgreSQL（Read-Through 模式）

### 响应体

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "user_id": "660e8400-e29b-41d4-a716-446655440001",
        "symbol": "BTCUSDT",
        "interval": "1h",
        "open_time": 1704067200000,
        "open": "42000.00",
        "high": "42500.50",
        "low": "41800.00",
        "close": "42300.25",
        "volume": "1234.56",
        "close_time": 1704070799999,
        "quote_volume": "52000000.00",
        "trades": 8500,
        "exchange": "binance",
        "suspicious": false,
        "corrupted": false
      }
    ],
    "page": 1,
    "page_size": 1000,
    "total": 8760,
    "gap_detected": false
  },
  "message": "success"
}
```

### curl 示例

```bash
curl -X GET "http://localhost:3000/api/v1/kline/query?symbol=BTCUSDT&interval=1h&start_time=1704067200000&end_time=1735689600000&page=1&page_size=100" \
  -H "Authorization: Bearer <access_token>"
```

---

## 2. POST /kline/import — K 线数据导入

批量导入 K 线数据，支持 CSV 文本、JSON 数组和交易所直采三种来源。`user_id` 从 JWT 令牌中提取，不通过请求体传入。

### 请求

```
POST /api/v1/kline/import
Content-Type: application/json
```

### 请求体

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `symbol` | string | 是 | 交易对，如 `BTCUSDT` |
| `interval` | string | 是 | K 线周期：`1m` / `5m` / `15m` / `30m` / `1h` / `4h` / `1d` / `1w` |
| `source` | string | 是 | 数据来源：`csv` / `api` / `exchange` |
| `data` | KlineItem[] | 是 | K 线数据数组 |

### KlineItem 结构

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `open_time` | integer | 是 | 开仓时间戳（毫秒） |
| `open` | string | 是 | 开盘价（字符串，避免浮点精度问题） |
| `high` | string | 是 | 最高价 |
| `low` | string | 是 | 最低价 |
| `close` | string | 是 | 收盘价 |
| `volume` | string | 是 | 成交量 |

### 导入策略

| 配置 | 值 | 说明 |
|------|------|------|
| 分片大小 | 5000 条/批次 | 平衡内存占用与事务粒度 |
| 写入方式 | `COPY FROM STDIN` | 绕过 SQL 解析开销，并行写入 |
| 重复策略 | `ON CONFLICT DO NOTHING` | `(user_id, symbol, interval, open_time)` 复合唯一键去重 |
| 事务 | 每批次独立事务 | 部分失败不影响已导入批次 |
| 文件限制 | 100 MB | 超过返回 `413` |

### 请求示例

```json
{
  "symbol": "BTCUSDT",
  "interval": "1h",
  "source": "api",
  "data": [
    {
      "open_time": 1704067200000,
      "open": "42000.00",
      "high": "42500.50",
      "low": "41800.00",
      "close": "42300.25",
      "volume": "1234.56"
    },
    {
      "open_time": 1704070800000,
      "open": "42300.25",
      "high": "42800.00",
      "low": "42100.00",
      "close": "42650.75",
      "volume": "987.65"
    }
  ]
}
```

### 响应体

```json
{
  "code": 0,
  "data": {
    "imported": 49850,
    "duplicates": 120,
    "failed": 30,
    "total": 50000
  },
  "message": "success"
}
```

### curl 示例

```bash
# JSON 数组导入
curl -X POST "http://localhost:3000/api/v1/kline/import" \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTCUSDT",
    "interval": "1h",
    "source": "api",
    "data": [
      {"open_time": 1704067200000, "open": "42000.00", "high": "42500.50", "low": "41800.00", "close": "42300.25", "volume": "1234.56"}
    ]
  }'

# CSV 文件上传（FormData）
curl -X POST "http://localhost:3000/api/v1/kline/import" \
  -H "Authorization: Bearer <access_token>" \
  -F "file=@btcusdt_1h.csv" \
  -F "symbol=BTCUSDT" \
  -F "interval=1h" \
  -F "source=csv"
```

---

## 3. GET /kline/export — K 线数据导出

导出指定范围的 K 线数据为 CSV 或 JSON 文件，采用流式响应以支持大数据量。

### 请求

```
GET /api/v1/kline/export?symbol=BTCUSDT&interval=1h&start_time=1704067200000&end_time=1735689600000&format=csv
```

### 查询参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `symbol` | string | 是 | — | 交易对 |
| `interval` | string | 是 | — | K 线周期 |
| `start_time` | integer | 是 | — | 起始时间戳（毫秒） |
| `end_time` | integer | 是 | — | 结束时间戳（毫秒） |
| `format` | string | 否 | `csv` | 导出格式：`csv` / `json` |
| `fields` | string | 否 | 全部字段 | 导出字段列表，逗号分隔，如 `open_time,open,close,volume` |

### 响应

- **Content-Type**: `text/csv`（CSV 格式）或 `application/json`（JSON 格式）
- **Content-Disposition**: `attachment; filename="kline_{symbol}_{interval}_{start}_{end}.{csv|json}"`
- 流式响应，大数据量不占用内存

### CSV 格式示例

```csv
timestamp,open,high,low,close,volume,close_time,quote_volume,trades,exchange
1704067200000,42000.00,42500.50,41800.00,42300.25,1234.56,1704070799999,52000000.00,8500,binance
1704070800000,42300.25,42800.00,42100.00,42650.75,987.65,1704074399999,42000000.00,7200,binance
```

### JSON 格式示例

```json
[
  {
    "open_time": 1704067200000,
    "open": "42000.00",
    "high": "42500.50",
    "low": "41800.00",
    "close": "42300.25",
    "volume": "1234.56",
    "close_time": 1704070799999,
    "quote_volume": "52000000.00",
    "trades": 8500,
    "exchange": "binance"
  }
]
```

### curl 示例

```bash
# 导出 CSV
curl -X GET "http://localhost:3000/api/v1/kline/export?symbol=BTCUSDT&interval=1h&start_time=1704067200000&end_time=1735689600000&format=csv" \
  -H "Authorization: Bearer <access_token>" \
  -o kline_BTCUSDT_1h.csv

# 导出 JSON，仅含指定字段
curl -X GET "http://localhost:3000/api/v1/kline/export?symbol=BTCUSDT&interval=1h&start_time=1704067200000&end_time=1735689600000&format=json&fields=open_time,open,close,volume" \
  -H "Authorization: Bearer <access_token>" \
  -o kline_BTCUSDT_1h.json
```

---

## 4. DELETE /kline — K 线数据删除

删除指定交易对和周期的 K 线数据。操作不可逆，建议在删除前先导出备份。

### 请求

```
DELETE /api/v1/kline?symbol=BTCUSDT&interval=1h&start_time=1704067200000&end_time=1735689600000
```

### 查询参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `symbol` | string | 是 | — | 交易对 |
| `interval` | string | 是 | — | K 线周期 |
| `start_time` | integer | 是 | — | 起始时间戳（毫秒） |
| `end_time` | integer | 是 | — | 结束时间戳（毫秒） |

### 响应体

```json
{
  "code": 0,
  "data": {
    "deleted": 8760
  },
  "message": "success"
}
```

### curl 示例

```bash
curl -X DELETE "http://localhost:3000/api/v1/kline?symbol=BTCUSDT&interval=1h&start_time=1704067200000&end_time=1735689600000" \
  -H "Authorization: Bearer <access_token>"
```

---

## 5. GET /kline/quality — 数据质量报告

对指定范围的 K 线数据执行质量检测，返回缺尖、异常值、重复、零成交量和价格反向等问题的统计报告。

### 请求

```
GET /api/v1/kline/quality?symbol=BTCUSDT&interval=1h&start_time=1704067200000&end_time=1735689600000
```

### 查询参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `symbol` | string | 是 | — | 交易对 |
| `interval` | string | 是 | — | K 线周期 |
| `start_time` | integer | 是 | — | 起始时间戳（毫秒） |
| `end_time` | integer | 是 | — | 结束时间戳（毫秒） |

### 检测规则

| 规则 | 说明 |
|------|------|
| 缺尖检测 | 超过 2 倍 interval 无数据时标记为缺口 |
| 异常值检测 | 偏离 5 日均值 ±15% 的数据 |
| 重复检测 | 相同 `open_time` 的数据条目 |
| 零成交量检测 | `volume = 0` 的数据 |
| 价格反向检测 | `high < low` 的数据 |

### 响应体

```json
{
  "code": 0,
  "data": {
    "symbol": "BTCUSDT",
    "interval": "1h",
    "start_time": 1704067200000,
    "end_time": 1735689600000,
    "total_rows": 50000,
    "valid_rows": 49850,
    "gap_count": 12,
    "anomaly_count": 28,
    "duplicate_count": 120,
    "coverage_rate": 0.997,
    "created_at": "2026-05-13T12:00:00Z"
  },
  "message": "success"
}
```

### curl 示例

```bash
curl -X GET "http://localhost:3000/api/v1/kline/quality?symbol=BTCUSDT&interval=1h&start_time=1704067200000&end_time=1735689600000" \
  -H "Authorization: Bearer <access_token>"
```

---

## 6. POST /kline/clean — 数据清洗

对指定范围的 K 线数据执行自动清洗。清洗前自动创建备份快照（7 天过期），支持通过 `backup_id` 回滚。

### 请求

```
POST /api/v1/kline/clean
Content-Type: application/json
```

### 请求体

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `symbol` | string | 是 | 交易对 |
| `interval` | string | 是 | K 线周期 |
| `start_time` | integer | 是 | 起始时间戳（毫秒） |
| `end_time` | integer | 是 | 结束时间戳（毫秒） |
| `mode` | string | 否 | 清洗模式：`auto`（默认）/ `manual`（当前仅支持 `auto`） |

### 清洗规则

| 操作 | 说明 |
|------|------|
| 缺尖填充 | 对缺口处进行线性插值填充 |
| 重复去重 | 相同 `open_time` 保留第一条 |
| 异常标记 | 标记为 `suspicious` / `corrupted`，不自动删除 |
| 备份创建 | 清洗前自动创建 `kline_backup` 快照（7 天过期） |

### 请求示例

```json
{
  "symbol": "BTCUSDT",
  "interval": "1h",
  "start_time": 1704067200000,
  "end_time": 1735689600000,
  "mode": "auto"
}
```

### 响应体

```json
{
  "code": 0,
  "data": {
    "filled_gaps": 12,
    "deduplicated": 120,
    "marked_suspicious": 18,
    "marked_corrupted": 10,
    "backup_id": "770e8400-e29b-41d4-a716-446655440002"
  },
  "message": "success"
}
```

### curl 示例

```bash
curl -X POST "http://localhost:3000/api/v1/kline/clean" \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTCUSDT",
    "interval": "1h",
    "start_time": 1704067200000,
    "end_time": 1735689600000,
    "mode": "auto"
  }'
```

---

## 7. GET /kline/latest — 最新 K 线

获取指定交易对和周期的最新一根 K 线数据。优先从 Redis 缓存读取。

### 请求

```
GET /api/v1/kline/latest?symbol=BTCUSDT&interval=1h
```

### 查询参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `symbol` | string | 是 | 交易对 |
| `interval` | string | 是 | K 线周期 |

### 响应体

```json
{
  "code": 0,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "user_id": "660e8400-e29b-41d4-a716-446655440001",
    "symbol": "BTCUSDT",
    "interval": "1h",
    "open_time": 1704067200000,
    "open": "42000.00",
    "high": "42500.50",
    "low": "41800.00",
    "close": "42300.25",
    "volume": "1234.56",
    "close_time": 1704070799999,
    "quote_volume": "52000000.00",
    "trades": 8500,
    "exchange": "binance",
    "suspicious": false,
    "corrupted": false
  },
  "message": "success"
}
```

### curl 示例

```bash
curl -X GET "http://localhost:3000/api/v1/kline/latest?symbol=BTCUSDT&interval=1h" \
  -H "Authorization: Bearer <access_token>"
```

---

## 8. GET /kline/import-history — 导入历史

分页查询当前用户的 K 线数据导入历史记录。

### 请求

```
GET /api/v1/kline/import-history?page=1&page_size=20
```

### 查询参数

| 参数 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `page` | integer | 否 | `1` | 页码，从 1 开始 |
| `page_size` | integer | 否 | `20` | 每页条数 |

### 响应体

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": "880e8400-e29b-41d4-a716-446655440003",
        "symbol": "BTCUSDT",
        "interval": "1h",
        "source": "csv",
        "total_rows": 50000,
        "imported_rows": 49850,
        "duplicate_rows": 120,
        "failed_rows": 30,
        "file_name": "btcusdt_1h_2024.csv",
        "created_at": "2026-05-13T10:30:00Z"
      }
    ],
    "total": 15
  },
  "message": "success"
}
```

### curl 示例

```bash
curl -X GET "http://localhost:3000/api/v1/kline/import-history?page=1&page_size=20" \
  -H "Authorization: Bearer <access_token>"
```

---

## 数据模型

### KlineData

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 行 ID |
| `user_id` | UUID | 数据所有者（从 JWT 注入） |
| `symbol` | string | 交易对，如 `BTCUSDT` |
| `interval` | string | K 线周期：`1m` / `5m` / `15m` / `30m` / `1h` / `4h` / `1d` / `1w` |
| `open_time` | integer | 开仓时间戳（毫秒） |
| `open` | string | 开盘价（Decimal 序列化为字符串） |
| `high` | string | 最高价 |
| `low` | string | 最低价 |
| `close` | string | 收盘价 |
| `volume` | string | 成交量 |
| `close_time` | integer? | 闭仓时间戳（毫秒，可选） |
| `quote_volume` | string? | 成交额（可选） |
| `trades` | integer? | 成交笔数（可选） |
| `exchange` | string | 数据来源交易所，如 `binance` |
| `suspicious` | boolean? | 质量标记：可疑数据 |
| `corrupted` | boolean? | 质量标记：损坏数据 |

### QualityReport

| 字段 | 类型 | 说明 |
|------|------|------|
| `symbol` | string | 交易对 |
| `interval` | string | K 线周期 |
| `start_time` | integer | 检测起始时间戳（毫秒） |
| `end_time` | integer | 检测结束时间戳（毫秒） |
| `total_rows` | integer | 总数据行数 |
| `valid_rows` | integer | 有效数据行数 |
| `gap_count` | integer | 缺口数量 |
| `anomaly_count` | integer | 异常值数量 |
| `duplicate_count` | integer | 重复数据数量 |
| `coverage_rate` | float | 覆盖率 = `valid_rows / total_rows` |
| `created_at` | string? | 报告生成时间（ISO 8601） |

### KlineImportLog

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | UUID | 导入记录 ID |
| `symbol` | string | 交易对 |
| `interval` | string | K 线周期 |
| `source` | string | 数据来源：`csv` / `api` / `exchange` |
| `total_rows` | integer | 总行数 |
| `imported_rows` | integer | 成功导入行数 |
| `duplicate_rows` | integer | 重复行数 |
| `failed_rows` | integer | 失败行数 |
| `file_name` | string? | 文件名（CSV 来源时） |
| `created_at` | string | 导入时间（ISO 8601） |

### KlineCleanResult

| 字段 | 类型 | 说明 |
|------|------|------|
| `filled_gaps` | integer | 填充的缺口数 |
| `deduplicated` | integer | 去重的重复数据条数 |
| `marked_suspicious` | integer | 标记为可疑的数据条数 |
| `marked_corrupted` | integer | 标记为损坏的数据条数 |
| `backup_id` | UUID | 备份快照 ID（用于回滚） |

### 数据库表

| 表 | 说明 |
|----|------|
| `kline_data` | K 线数据主表（TimescaleDB hypertable，按 `open_time` 分区） |
| `kline_import_log` | 导入日志（记录每次导入的统计信息） |
| `kline_quality_report` | 质量检测报告（存储历史报告） |
| `kline_backup` | 清洗前备份快照（7 天自动过期） |

### 索引

```sql
-- 复合唯一键（防重复导入）
UNIQUE (user_id, symbol, interval, open_time)

-- 查询索引
CREATE INDEX idx_kline_user ON kline_data(user_id, symbol, interval, open_time DESC);

-- TimescaleDB 超表
SELECT create_hypertable('kline_data', 'open_time');
```

### 保留策略

| 周期 | 保留期 | 压缩比 |
|------|--------|--------|
| `1m` / `5m` | 30 天 | 8:1 |
| `15m` / `30m` / `1h` | 1 年 | 4:1 |
| `4h` / `1d` / `1w` | 永久 | 2:1 |

### 缓存方案

| 配置 | 值 |
|------|------|
| 缓存 Key | `kline:{symbol}:{timeframe}:{ts}` |
| TTL（1m 周期） | 5 分钟 |
| TTL（其他周期） | 1 小时 |
| 缓存范围 | 仅缓存最近 24h 的数据 |
| 失效策略 | 导入/清洗后主动 invalidate 对应 key prefix |
| 查询流程 | 先查 Redis → 未命中查 PostgreSQL → 结果写入缓存 |

---

## 错误码

| code | HTTP | 含义 |
|------|------|------|
| `0` | 200 | 成功 |
| `40001` | 400 | 请求参数错误（缺少必填参数、类型不匹配） |
| `40002` | 400 | 校验失败（时间范围超过 10 年、interval 不合法） |
| `40003` | 400 | 数据校验失败（`high < low`、`volume < 0`） |
| `40101` | 401 | 认证失败（未提供 Token 或 Token 无效） |
| `40102` | 401 | Token 过期 |
| `40301` | 403 | 权限不足（非 pro-trader 尝试交易所直采） |
| `40401` | 404 | 数据不存在（查询的交易对/周期无数据） |
| `41301` | 413 | 请求体过大（CSV 文件超过 100 MB） |
| `42901` | 429 | 请求频率超限 |
| `50001` | 500 | 内部错误 |
| `50002` | 500 | 数据库错误 |
| `50301` | 503 | Redis 缓存不可用（降级为直接查库） |
