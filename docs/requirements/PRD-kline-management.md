# PRD: K线数据管理 — 导入、查询、清洗、存储、导出

> 版本: v1.0
> 状态: Draft
> 作者: PM
> 关联: ADR-005 (市场数据管道), ADR-004 (回测引擎), data-model.md, PRD-backtest-engine.md

---

## 1. 背景

主 PRD 中 K线数据是回测引擎的燃料。回测引擎 ADR-004 已实现信号生成和绩效计算，但依赖 `kline_data` 表的数据。当前 `db/backtest.rs` 中 `load_klines` 为 TODO 占位。本 PRD 定义 K线数据的完整管理链路，确保回测引擎有可靠数据源。

### 当前状态 (2026-05-13)

- `kline_data` 表 schema 已定义（symbol, interval, open_time, open, high, low, close, volume, exchange）
- 回测引擎 `load_klines` 接口为 TODO（PRD-backtest-engine.md 标注 P0 ❌）
- 市场数据管道 ADR-005 提及实时数据写入 PostgreSQL，但批量导入和清洗功能未规划
- 用户无法自行导入历史数据进行回测

### 为何这个功能重要

K线数据质量直接决定回测结果的可靠性。GIGO（Garbage In, Garbage Out）原则在量化领域尤为严格：
- 缺失数据导致信号跳空、绩效虚高
- 错误价格导致 SL/TP 误触发
- 重复数据导致仓位 double-counting
- 非连续数据（如交易所有维护窗口）导致策略逻辑异常

用户需要自主管理数据，而不是完全依赖系统实时采集。

---

## 2. 目标

| 目标 | 指标 | 当前状态 |
|------|------|---------|
| 多源导入 | 支持 CSV 上传 / REST API / 交易所直采三种方式 | TODO |
| 时间范围查询 | 按 symbol + interval + 时间范围返回分页结果，P99 < 500ms | TODO |
| 数据质量检测 | 缺尖 / 异常值 / 重复检测，覆盖率 ≥ 95% | TODO |
| 自动清洗 | 缺失插值、异常标记/剔除、重复去重，无需人工干预 | TODO |
| 存储效率 | TimescaleDB 自动压缩，存储空间节省 ≥ 60% | TODO |
| 数据导出 | CSV / JSON 格式，支持自定义字段和范围 | TODO |
| 权限隔离 | 用户只能访问自己导入或授权的 K线数据 | TODO |
| 回测集成 | `load_klines` 实现，供回测引擎调用 | TODO（阻塞 PRD-backtest-engine P0） |

### 非目标 (Non-Goals)

- ❌ K线数据实时推送（已有 ADR-005 WebSocket 方案）
- ❌ 交易所直采功能（P2，后续阶段）
- ❌ 多品种组合数据管理（P2+）
- ❌ K线数据订阅计费系统（商业化阶段）
- ❌ 用户间数据分享市场（P2+）

---

## 3. 用户角色与权限

| 角色 | K线数据权限 | 说明 |
|------|-----------|------|
| **trader** (普通用户) | 导入/查询/导出/清洗自有数据 | 默认角色 |
| **pro-trader** (专业交易员) | 同 trader，额外享有交易所直采权限 | 需管理员开通 |
| **admin** (管理员) | 查看所有用户数据，可强制清洗任意数据 | 运维/审计用途 |

> 注意：用户只能访问自己导入的数据（user_id 隔离）；admin 可跨用户审计但不可修改。

---

## 4. 用户故事 (User Stories)

### US-KM-01: K线数据导入 (P0)

> **As a** 交易者
> **I want to** 将历史K线数据导入系统
> **So that** 我可以使用自己的数据或第三方数据源进行回测

**导入方式：**

| 方式 | 描述 | 文件格式 |
|------|------|---------|
| CSV 上传 | Web界面上传 CSV 文件，支持拖拽 | CSV |
| REST API | POST /api/v1/kline/import，批量写入 | JSON |
| 交易所直采 | 前端触发，从 Binance 等交易所 API 拉取（pro-trader） | — |

**验收条件：**

```gherkin
Feature: K线数据导入

  Background:
    Given 用户已登录

  Scenario: CSV 上传成功
    Given 用户准备 CSV 文件，格式为: timestamp,open,high,low,close,volume
    When 用户上传 CSV 文件并指定 symbol="BTCUSDT" interval="1h"
    Then 系统解析文件并写入 kline_data 表
      And 返回导入结果：总条数、成功数、失败数、重复忽略数
      And 前端显示导入成功 toast

  Scenario: CSV 文件格式错误
    Given 用户上传的 CSV 文件缺少必需列 "close"
    When 系统解析文件
    Then 返回 400 错误
      And 提示 "缺少必需列: close"
      And 不写入任何数据

  Scenario: CSV 文件过大
    Given 用户上传超过 100MB 的 CSV 文件
    When 系统接收文件
    Then 返回 413 错误
      And 提示 "文件大小超过 100MB 限制，请分批上传"

  Scenario: API 批量导入
    Given 用户拥有有效的 JWT token
    When POST /api/v1/kline/import 发送 JSON 数据（1000条）
    Then 返回 200 状态码
      And 返回 {"imported": 1000, "duplicates": 0, "failed": 0}
      And 数据已写入数据库

  Scenario: 导入重复数据
    Given kline_data 已存在 symbol="BTCUSDT" interval="1h" open_time=1704067200000 的记录
    When 用户上传包含相同 open_time 的 CSV
    Then 系统跳过重复记录（不报错）
      And 返回 {"imported": 900, "duplicates": 100, "failed": 0}
      And 已存在数据未被覆盖

  Scenario: 交易所直采（pro-trader）
    Given 当前用户角色为 pro-trader
    When 用户选择 symbol="ETHUSDT" interval="15m" 范围 2024-01-01 至 2024-01-31
    Then 系统从 Binance API 获取数据
      And 数据写入 kline_data 表
      And 返回导入条数
```

---

### US-KM-02: K线数据查询 (P0)

> **As a** 交易者
> **I want to** 按时间范围、交易对、周期查询K线数据
> **So that** 我可以查看数据覆盖情况，确认回测所需数据是否完整

**验收条件：**

```gherkin
Feature: K线数据查询

  Background:
    Given 用户已登录
      And kline_data 已存在 symbol="BTCUSDT" interval="1h" 的记录

  Scenario: 按时间范围查询
    Given 用户指定 symbol="BTCUSDT" interval="1h"
      And start_time=1704067200000 (2024-01-01 00:00 UTC)
      And end_time=1706745599000 (2024-01-31 23:59 UTC)
    When 发起 GET /api/v1/kline/query
    Then 返回该时间范围内的 K线数据
      And 数据按 open_time ASC 排序
      And 默认每页 1000 条
      And 返回 total 字段表示总条数

  Scenario: 查询结果分页
    Given 用户指定 symbol="BTCUSDT" interval="1h"
      And 时间范围内共有 5000 条记录
    When 发起 GET /api/v1/kline/query?page=2&page_size=1000
    Then 返回第 1001-2000 条记录
      And 返回 meta: {total: 5000, page: 2, page_size: 1000}

  Scenario: 按交易对和周期查询最新数据
    When 发起 GET /api/v1/kline/latest?symbol=BTCUSDT&interval=1h
    Then 返回该交易对最新的1条 K线数据

  Scenario: 查询无数据
    Given 用户指定 symbol="INVALID99" interval="1h"
    When 发起查询
    Then 返回 200 状态码
      And 返回空数组 data:[]
      And total = 0

  Scenario: 跨周末查询（1h周期缺口检测）
    Given 用户查询 symbol="BTCUSDT" interval="1h" 范围跨周末
    When 发起查询
    Then 返回数据中标记周末缺口位置（如果有连续超过 8h 无数据）
      And 在返回 meta 中标记 gap_detected = true
```

---

### US-KM-03: 数据质量检测 (P1)

> **As a** 交易者
> **I want to** 系统自动检测K线数据中的缺尖、异常、重复
> **So that** 我可以及时发现数据问题，避免基于错误数据做回测

**质量检测规则：**

| 检测类型 | 规则 | 处理建议 |
|---------|------|---------|
| 缺尖检测 | 连续两根 K 线时间戳差值 > 预期 interval 的 2 倍 | 标记 gap，提供插值选项 |
| 异常值检测 | close 偏离 5 日均值 ±15% | 标记为 suspicious，不自动删除 |
| 重复检测 | 相同 (symbol, interval, open_time) 存在多条 | 自动去重，保留第一条 |
| 零成交量 | volume = 0 | 标记为异常 |
| 价格反向 | high < low | 标记为 corrupted，需要修正 |

**验收条件：**

```gherkin
Feature: 数据质量检测

  Background:
    Given 用户已登录
      And 用户拥有 symbol="BTCUSDT" interval="1h" 的 K线数据

  Scenario: 检测缺尖并标记
    Given 数据中存在连续 5 根 K线 缺少
    When 用户点击"数据质量检测"
    Then 系统返回检测报告：
      And 缺尖数量: 5
      And 缺尖位置: [open_time列表]
      And 建议: "自动插值填充"

  Scenario: 检测异常价格
    Given 数据中存在一根 K线 close=100000（异常偏离）
    When 触发数据质量检测
    Then 该 K线被标记为 suspicious
      And 显示: "检测到 1 条异常价格数据，建议复查"

  Scenario: 检测重复数据
    Given 数据中存在 3 条完全重复的 K线（symbol/interval/open_time 相同）
    When 触发数据质量检测
    Then 重复数据自动去重为 1 条
      And 报告显示: "去重 3 条 → 保留 1 条"

  Scenario: 数据覆盖率统计
    When 用户请求某 symbol+interval 的数据质量报告
    Then 系统返回:
      And 总条数: N
      And 有效条数: M
      And 缺尖条数: G
      And 异常标记条数: S
      And 覆盖率: M/N%
```

---

### US-KM-04: 数据清洗 (P1)

> **As a** 交易者
> **I want to** 对K线数据进行自动或手动清洗
> **So that** 我可以得到干净的数据用于回测

**清洗模式：**

| 模式 | 描述 |
|------|------|
| 自动清洗 | 系统根据质量检测结果，自动执行插值、去重、异常标记，无需人工确认 |
| 手动清洗 | 用户逐条/批量确认处理方式（删除/修正/保留） |

**验收条件：**

```gherkin
Feature: 数据清洗

  Background:
    Given 用户已登录
      And 用户拥有 K线数据存在质量问题

  Scenario: 自动清洗执行
    Given 数据中存在缺尖和重复
    When 用户选择"自动清洗"并点击执行
    Then 系统执行清洗操作：
      And 缺尖处使用线性插值填充
      And 重复数据去重
      And 异常数据标记为 suspicious（不自动删除）
      And 返回清洗报告

  Scenario: 手动删除异常数据
    Given 系统检测到 3 条异常数据
    When 用户勾选这 3 条并选择"删除"
    Then 这 3 条数据被物理删除
      And 返回删除条数: 3

  Scenario: 手动修正异常数据
    Given 系统检测到 1 条异常数据（high < low）
    When 用户手动修正该 K线价格为正确值
    Then 数据被更新
      And 该 K线取消 suspicious 标记

  Scenario: 清洗前数据备份
    Given 用户执行清洗操作
    When 系统开始清洗
    Then 自动创建数据快照（备份表 kline_data_backup_{timestamp}）
      And 用户可在清洗后 7 天内回滚
```

---

### US-KM-05: 数据存储格式 (P0)

> **As a** 系统
> **I want to** 使用 PostgreSQL TimescaleDB 存储K线数据并启用压缩
> **So that** 降低存储成本同时保持查询性能

**存储策略：**

| 周期 | 保留策略 | 压缩 |
|------|---------|------|
| 1m | 30 天 | 8:1 |
| 15m/1h | 1 年 | 4:1 |
| 4h/1d/1w | 永久 | 2:1 |

**验收条件：**

```gherkin
Feature: 数据存储格式

  Scenario: TimescaleDB 超表创建
    Given 系统初始化
    When 创建 kline_data 表
    Then 自动转换为 TimescaleDB hypertable
      And 按 open_time 自动分区

  Scenario: 数据压缩验证
    Given 用户已导入 100 万条 1h 数据
    When 15 天后（超过 1m 保留期）
    Then 系统自动将 1m 数据压缩
      And 存储空间节省 ≥ 60%

  Scenario: 压缩不影响查询
    Given 存在已压缩的历史数据
    When 用户查询压缩时间范围内的数据
    Then 查询结果正确返回
      And 查询延迟不受压缩影响（TimescaleDB 透明解压）
```

---

### US-KM-06: 数据导出 (P0)

> **As a** 交易者
> **I want to** 将K线数据导出为 CSV 或 JSON 格式
> **So that** 我可以将数据用于其他分析工具或备份

**验收条件：**

```gherkin
Feature: 数据导出

  Background:
    Given 用户已登录
      And 用户拥有 symbol="BTCUSDT" interval="1h" 数据

  Scenario: 导出 CSV
    Given 用户选择 symbol="BTCUSDT" interval="1h" 范围 2024-01
    When 用户点击"导出 CSV"
    Then 浏览器下载 CSV 文件
      And 文件名格式: kline_BTCUSDT_1h_20240101_20240131.csv
      And 包含表头: timestamp,open,high,low,close,volume

  Scenario: 导出 JSON
    Given 用户选择 symbol="BTCUSDT" interval="1h" 范围 2024-01
    When 用户点击"导出 JSON"
    Then 浏览器下载 JSON 文件
      And 格式: {"symbol":"BTCUSDT","interval":"1h","data":[...]}

  Scenario: 导出字段筛选
    Given 用户导出时取消勾选 "high" 和 "low"
    When 导出 CSV
    Then 文件仅包含: timestamp,open,close,volume
```

---

### US-KM-07: 权限控制 (P0)

> **As a** 交易者
> **I want to** 确保我的K线数据只能被我本人访问
> **So that** 我的交易数据不会被其他用户看到

> 注意：当前 `kline_data` 表设计无 `user_id` 列。需要扩展 schema 或引入 `kline_data_access` 关联表。

**验收条件：**

```gherkin
Feature: K线数据权限控制

  Background:
    Given 用户 A 导入了 symbol="BTCUSDT" interval="1h" 数据
      And 用户 B 已登录

  Scenario: 用户只能查询自己的数据
    When 用户 B 尝试查询 symbol="BTCUSDT" interval="1h"
    Then 返回用户 B 自己导入的数据（可能为空）
      And 用户 A 的数据对 B 完全不可见

  Scenario: admin 可跨用户审计
    Given 管理员已登录
    When 管理员查询任意用户的数据（带 user_id 参数）
    Then 返回该用户的数据
      And 管理员不可修改他人数据

  Scenario: 未授权用户无法通过 API 直接访问数据库
    Given 攻击者尝试构造 SQL 注入
    When 访问 /api/v1/kline/query
    Then 请求被权限中间件拦截
      And 返回 403 Forbidden
```

---

## 5. 数据模型

### 5.1 扩展 kline_data 表（新增 user_id）

```sql
-- 在现有 kline_data 基础上新增 user_id 列（支持多用户数据隔离）
ALTER TABLE kline_data ADD COLUMN user_id UUID REFERENCES users(id);
CREATE INDEX idx_kline_user ON kline_data(user_id, symbol, interval, open_time DESC);

-- 数据导入记录表
CREATE TABLE kline_import_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    symbol VARCHAR(50) NOT NULL,
    interval VARCHAR(5) NOT NULL,
    source VARCHAR(20) NOT NULL, -- 'csv', 'api', 'exchange'
    total_rows INTEGER NOT NULL,
    imported_rows INTEGER NOT NULL,
    duplicate_rows INTEGER DEFAULT 0,
    failed_rows INTEGER DEFAULT 0,
    file_name VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 数据质量检测报告表
CREATE TABLE kline_quality_report (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    symbol VARCHAR(50) NOT NULL,
    interval VARCHAR(5) NOT NULL,
    start_time BIGINT NOT NULL,
    end_time BIGINT NOT NULL,
    total_rows INTEGER NOT NULL,
    valid_rows INTEGER NOT NULL,
    gap_count INTEGER DEFAULT 0,
    anomaly_count INTEGER DEFAULT 0,
    duplicate_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 数据清洗快照表（7天自动过期）
CREATE TABLE kline_backup (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    symbol VARCHAR(50) NOT NULL,
    interval VARCHAR(5) NOT NULL,
    snapshot_time TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ DEFAULT NOW() + INTERVAL '7 days',
    data_json JSONB NOT NULL  -- 快照时刻的完整数据
);
```

---

## 6. API 端点设计

| 方法 | 路径 | 描述 | 优先级 |
|------|------|------|--------|
| POST | /api/v1/kline/import | 批量导入 K线数据 | P0 |
| GET | /api/v1/kline/query | 查询 K线数据（分页、过滤） | P0 |
| GET | /api/v1/kline/latest | 获取最新一根 K线 | P0 |
| GET | /api/v1/kline/export | 导出 K线数据（CSV/JSON） | P0 |
| GET | /api/v1/kline/quality | 数据质量检测报告 | P1 |
| POST | /api/v1/kline/clean | 执行数据清洗 | P1 |
| DELETE | /api/v1/kline/clean/rollback | 回滚至上一次清洗前 | P2 |
| GET | /api/v1/kline/import-history | 导入历史记录 | P1 |
| POST | /api/v1/kline/fetch | 交易所直采（pro-trader） | P2 |

### 6.1 API 详细设计

#### POST /api/v1/kline/import

**请求体：**
```json
{
  "symbol": "BTCUSDT",
  "interval": "1h",
  "source": "csv",
  "data": [
    {"open_time": 1704067200000, "open": "50000.0", "high": "50100.0", "low": "49900.0", "close": "50050.0", "volume": "100.5"},
    ...
  ]
}
```

**响应：**
```json
{
  "success": true,
  "imported": 1000,
  "duplicates": 5,
  "failed": 0,
  "errors": []
}
```

#### GET /api/v1/kline/query

**查询参数：**
- `symbol` (必填): 交易对
- `interval` (必填): 周期
- `start_time` (必填): 起始时间戳（毫秒）
- `end_time` (必填): 结束时间戳（毫秒）
- `page` (可选, 默认1): 页码
- `page_size` (可选, 默认1000, 最大5000): 每页条数

**响应：**
```json
{
  "success": true,
  "data": [
    {"open_time": 1704067200000, "open": "50000.0", "high": "50100.0", "low": "49900.0", "close": "50050.0", "volume": "100.5"}
  ],
  "meta": {
    "total": 5000,
    "page": 1,
    "page_size": 1000,
    "gap_detected": true
  }
}
```

---

## 7. 前端页面设计

### 7.1 页面路由

| 路径 | 组件 | 说明 |
|------|------|------|
| /kline | KLineManagementView.vue | K线数据管理首页 |
| /kline/import | KLineImportView.vue | 数据导入页 |
| /kline/query | KLineQueryView.vue | 数据查询页 |
| /kline/quality | KLineQualityView.vue | 数据质量检测页 |

### 7.2 页面布局

```
/kline 页面布局（Element Plus Tabs）
├── Tab1: 数据概览
│   ├── 统计卡片: 总条数、有效率、存储占用
│   └── 最近导入记录列表
├── Tab2: 数据导入
│   ├── 导入方式选择: CSV上传 | API导入 | 交易所直采
│   ├── CSV上传: 拖拽区域 + 文件预览
│   └── 导入进度条
├── Tab3: 数据查询
│   ├── 筛选表单: symbol / interval / 时间范围
│   ├── K线数据表格 (Element Plus Table)
│   └── 导出按钮
└── Tab4: 数据质量
    ├── 质量概览卡片
    ├── 问题数据列表
    └── 一键清洗 / 手动处理
```

### 7.3 组件清单

| 组件 | 说明 |
|------|------|
| `KLineManagementView.vue` | 主容器，Tab 导航 |
| `KLineStatsCards.vue` | 统计卡片（总条数、有效率、存储） |
| `KLineImportForm.vue` | 导入表单（symbol/interval/source） |
| `KLineFileUploader.vue` | CSV 拖拽上传组件 |
| `KLineImportProgress.vue` | 导入进度条 |
| `KLineQueryForm.vue` | 查询筛选表单 |
| `KLineTable.vue` | K线数据表格（分页、排序） |
| `KLineExportButton.vue` | 导出按钮组（CSV/JSON） |
| `KLineQualityReport.vue` | 质量检测报告 |
| `KLineGapMarker.vue` | 缺尖数据标记 |

---

## 8. 后端设计

### 8.1 模块结构

```
handlers/
  └── kline.rs        # API handler（导入/查询/导出/质量/清洗）

services/
  └── kline_service.rs # 业务逻辑
      - import_klines()
      - query_klines()
      - detect_quality()
      - auto_clean()
      - manual_clean()
      - export_klines()
      - fetch_from_exchange()

db/
  └── kline.rs        # SeaORM 模型 + 查询
      - load_klines()  # 供给回测引擎调用（P0 阻塞项）
```

### 8.2 回测引擎集成（关键路径）

`load_klines` 须实现以下签名（供 `services/backtest_engine.rs` 调用）：

```rust
// db/kline.rs
pub async fn load_klines(
    pool: &DatabaseConnection,
    symbol: &str,
    interval: &str,
    start_time: i64,
    end_time: i64,
) -> Result<Vec<Kline>, DbErr> {
    // 查询 kline_data，返回按 open_time ASC 排序的 Vec<Kline>
}
```

此接口为 PRD-backtest-engine.md 中回测引擎 P0 阻塞项的前置依赖。

---

## 9. 边界情况

| 场景 | 处理方式 |
|------|---------|
| CSV 文件超过 100MB | 返回 413，提示分批上传 |
| 导入时数据库连接断开 | 事务回滚，提示重试 |
| 查询时间范围跨 10 年 | 返回 400，提示范围过大（建议分段查询） |
| 清洗操作中用户关闭页面 | 后端使用事务，前端刷新后状态一致 |
| 回滚时快照已过期 | 返回 400，提示快照已过期 |
| 交易所 API 限流 | 指数退避重试，返回 partial 数据 + 警告 |
| TimescaleDB 扩展未安装 | 降级为普通 PostgreSQL 表，手动分区提示 |
| 并发导入同一 symbol+interval | 使用 UPSERT（ON CONFLICT DO NOTHING） |

---

## 10. 非功能性需求

| 需求 | 指标 |
|------|------|
| 查询性能 | P99 < 500ms（1000 条数据） |
| 导入性能 | 10 万条 CSV 数据 < 30s |
| 存储压缩率 | ≥ 60%（TimescaleDB 压缩） |
| 可用性 | 导入失败不污染已有数据（事务保证） |
| 可观测性 | 所有操作记录 audit_log |

---

## 11. 优先级矩阵

| User Story | 功能 | P0 | P1 | P2 |
|------------|------|----|----|-----|
| US-KM-01 | K线数据导入（CSV/API） | ✅ | | |
| US-KM-02 | K线数据查询 | ✅ | | |
| US-KM-03 | 数据质量检测 | | ✅ | |
| US-KM-04 | 数据清洗 | | ✅ | |
| US-KM-05 | 存储格式（TimescaleDB） | ✅ | | |
| US-KM-06 | 数据导出 | ✅ | | |
| US-KM-07 | 权限控制 | ✅ | | |
| US-KM-01（交易所直采） | | | | P2 |
| US-KM-04（回滚） | | | | P2 |

**P0 关键路径（影响回测引擎 P0）：**
`US-KM-01` (import) → `US-KM-02` (query/load_klines) → 回测引擎可调用

---

## 12. 实施建议（分阶段）

### 阶段一（当前 PRD 核心，P0 功能）：
1. 扩展 `kline_data` 表 schema，增加 `user_id`
2. 实现 `load_klines`（解除回测引擎阻塞）
3. 实现 CSV 导入 + REST API 导入
4. 实现分页查询 + 导出
5. 实现用户数据隔离

### 阶段二（P1 功能）：
1. 数据质量检测
2. 自动清洗

### 阶段三（P2 功能）：
1. 交易所直采
2. 手动清洗 + 回滚
