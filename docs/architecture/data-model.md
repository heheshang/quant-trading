# 量化交易系统 — 数据模型

> 版本: v1.0 | 技术栈: PostgreSQL 16 + SeaORM + Redis 7

---

## 1. 数据库总览

```
┌──────────────────────────────────────────────────────┐
│                  PostgreSQL 16                        │
├────────────┬──────────┬──────────┬───────────────────┤
│  用户域    │ 策略域    │ 交易域    │ 行情域            │
│  users     │ strategies│ orders   │ kline_data        │
│            │           │ trades   │ ticker_snapshots  │
│            │           │ positions│                   │
│            │           │          │                   │
│            │ backtest_ │          │                   │
│            │ results   │          │                   │
│            │           │          │                   │
│  系统域    │           │          │                   │
│  audit_log │           │          │                   │
│  risk_rules│           │          │                   │
└────────────┴───────────┴──────────┴───────────────────┘
```

---

## 2. 表结构定义

### 2.1 用户域

#### users（用户表）

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK, DEFAULT gen_random_uuid() | 用户 ID |
| email | VARCHAR(255) | UNIQUE, NOT NULL | 邮箱 |
| password_hash | VARCHAR(255) | NOT NULL | bcrypt 加密密码 |
| display_name | VARCHAR(100) | NOT NULL | 显示名称 |
| role | user_role | NOT NULL, DEFAULT 'trader' | 角色枚举 |
| status | user_status | NOT NULL, DEFAULT 'inactive' | 账户状态 |
| email_verified_at | TIMESTAMPTZ | | 邮箱验证时间 |
| login_attempts | SMALLINT | DEFAULT 0 | 连续登录失败次数 |
| locked_until | TIMESTAMPTZ | | 锁定截止时间 |
| preferences | JSONB | DEFAULT '{}' | 用户偏好（仪表盘布局等） |
| created_at | TIMESTAMPTZ | DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMPTZ | DEFAULT NOW() | 更新时间 |

```sql
CREATE TYPE user_role AS ENUM ('trader', 'pro_trader', 'admin');
CREATE TYPE user_status AS ENUM ('inactive', 'active', 'locked', 'disabled');
```

**索引：**
```sql
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_role ON users(role);
CREATE INDEX idx_users_status ON users(status);
```

---

### 2.2 策略域

#### strategies（策略表）

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 策略 ID |
| user_id | UUID | FK → users(id), NOT NULL | 所属用户 |
| name | VARCHAR(200) | NOT NULL | 策略名称 |
| description | TEXT | | 描述 |
| code | TEXT | | 策略代码（WASM/DSL） |
| params | JSONB | DEFAULT '{}' | 策略参数（如 {fast:5, slow:20}） |
| symbol | VARCHAR(50) | NOT NULL | 交易对（如 BTC/USDT） |
| interval | VARCHAR(10) | NOT NULL | K线周期（1m/5m/1h/1d） |
| status | strategy_status | DEFAULT 'stopped' | 运行状态 |
| mode | trade_mode | DEFAULT 'simulation' | 交易模式 |
| risk_config | JSONB | DEFAULT '{}' | 风控配置（如 {max_pos:1, stop_loss:0.05}） |
| engine_type | VARCHAR(20) | DEFAULT 'builtin' | 引擎类型（builtin/wasm/python） |
| last_run_at | TIMESTAMPTZ | | 最后运行时间 |
| created_at | TIMESTAMPTZ | DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMPTZ | DEFAULT NOW() | 更新时间 |

```sql
CREATE TYPE strategy_status AS ENUM ('stopped', 'running_simulation', 'running_live', 'paused', 'error');
CREATE TYPE trade_mode AS ENUM ('simulation', 'live');
```

**索引：**
```sql
CREATE INDEX idx_strategies_user_id ON strategies(user_id);
CREATE INDEX idx_strategies_status ON strategies(status);
CREATE INDEX idx_strategies_user_status ON strategies(user_id, status);
CREATE UNIQUE INDEX idx_strategies_user_name ON strategies(user_id, name);
```

#### strategy_logs（策略日志表）

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PK | 日志 ID |
| strategy_id | UUID | FK → strategies(id), NOT NULL | 策略 ID |
| level | log_level | NOT NULL | 日志级别 |
| message | TEXT | NOT NULL | 日志内容 |
| metadata | JSONB | | 额外信息 |
| created_at | TIMESTAMPTZ | DEFAULT NOW() | 时间戳 |

```sql
CREATE TYPE log_level AS ENUM ('info', 'warn', 'error', 'signal');
CREATE INDEX idx_strategy_logs_strategy_id ON strategy_logs(strategy_id);
CREATE INDEX idx_strategy_logs_created_at ON strategy_logs(created_at);
```

---

### 2.3 回测域

#### backtest_results（回测结果表）

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 结果 ID |
| strategy_id | UUID | FK → strategies(id), NOT NULL | 策略 ID |
| user_id | UUID | FK → users(id), NOT NULL | 用户 ID |
| config | JSONB | NOT NULL | 回测配置 |
| status | backtest_status | DEFAULT 'pending' | 状态 |
| progress | SMALLINT | DEFAULT 0 | 进度 0-100 |
| start_time | TIMESTAMPTZ | | 实际开始时间 |
| end_time | TIMESTAMPTZ | | 实际结束时间 |
| metrics | JSONB | | 性能指标 |
| trades | JSONB | | 交易明细数组 |
| equity_curve | JSONB | | 资金曲线数组 |
| error | TEXT | | 错误信息 |
| created_at | TIMESTAMPTZ | DEFAULT NOW() | 创建时间 |

**config 结构：**
```json
{
  "date_range": {"start": "2024-01-01", "end": "2024-12-31"},
  "initial_capital": 100000.0,
  "fee_rate": 0.001,
  "slippage_rate": 0.0005,
  "symbol": "BTC/USDT",
  "interval": "1h"
}
```

**metrics 结构：**
```json
{
  "total_return_pct": 25.3,
  "annualized_return_pct": 22.1,
  "max_drawdown_pct": -12.5,
  "sharpe_ratio": 1.85,
  "win_rate": 0.62,
  "total_trades": 85,
  "profit_factor": 2.3,
  "avg_win_pct": 3.2,
  "avg_loss_pct": -1.8
}
```

```sql
CREATE TYPE backtest_status AS ENUM ('pending', 'running', 'completed', 'failed', 'cancelled');
```

**索引：**
```sql
CREATE INDEX idx_backtest_strategy_id ON backtest_results(strategy_id);
CREATE INDEX idx_backtest_user_id ON backtest_results(user_id);
CREATE INDEX idx_backtest_created_at ON backtest_results(created_at DESC);
CREATE INDEX idx_backtest_status ON backtest_results(status);
```

---

### 2.4 交易域

#### orders（委托表）

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 委托 ID |
| user_id | UUID | FK → users(id), NOT NULL | 用户 ID |
| strategy_id | UUID | FK → strategies(id), NULLABLE | 策略 ID（手动单为 null） |
| symbol | VARCHAR(50) | NOT NULL | 交易对 |
| side | order_side | NOT NULL | 买卖方向 |
| type | order_type | NOT NULL | 委托类型 |
| price | DECIMAL(20,8) | | 限价（市价单为 null） |
| amount | DECIMAL(20,8) | NOT NULL | 委托数量 |
| filled | DECIMAL(20,8) | DEFAULT 0 | 已成交数量 |
| status | order_status | NOT NULL, DEFAULT 'pending' | 委托状态 |
| mode | trade_mode | NOT NULL, DEFAULT 'simulation' | 交易模式 |
| fee | DECIMAL(20,8) | DEFAULT 0 | 手续费 |
| expired_at | TIMESTAMPTZ | | 过期时间 |
| created_at | TIMESTAMPTZ | DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMPTZ | DEFAULT NOW() | 更新时间 |

```sql
CREATE TYPE order_side AS ENUM ('buy', 'sell');
CREATE TYPE order_type AS ENUM ('limit', 'market', 'stop', 'stop_limit');
CREATE TYPE order_status AS ENUM ('pending', 'partial_filled', 'filled', 'cancelled', 'expired', 'rejected');
```

**索引：**
```sql
CREATE INDEX idx_orders_user_id ON orders(user_id);
CREATE INDEX idx_orders_strategy_id ON orders(strategy_id);
CREATE INDEX idx_orders_status ON orders(status);
CREATE INDEX idx_orders_created_at ON orders(created_at DESC);
CREATE INDEX idx_orders_user_status ON orders(user_id, status);
CREATE INDEX idx_orders_symbol ON orders(symbol);
```

#### trades（成交记录表）

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PK | 成交 ID |
| order_id | UUID | FK → orders(id), NOT NULL | 关联委托 |
| symbol | VARCHAR(50) | NOT NULL | 交易对 |
| side | order_side | NOT NULL | 方向 |
| price | DECIMAL(20,8) | NOT NULL | 成交价 |
| amount | DECIMAL(20,8) | NOT NULL | 成交数量 |
| total | DECIMAL(20,8) | NOT NULL | 成交金额 |
| fee | DECIMAL(20,8) | DEFAULT 0 | 手续费 |
| pnl | DECIMAL(20,8) | | 盈亏（平仓单） |
| trade_time | TIMESTAMPTZ | NOT NULL | 成交时间 |

**索引：**
```sql
CREATE INDEX idx_trades_order_id ON trades(order_id);
CREATE INDEX idx_trades_symbol ON trades(symbol);
CREATE INDEX idx_trades_trade_time ON trades(trade_time DESC);
```

#### positions（持仓表）

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 持仓 ID |
| user_id | UUID | FK → users(id), NOT NULL | 用户 ID |
| strategy_id | UUID | FK → strategies(id), NULLABLE | 策略 ID |
| symbol | VARCHAR(50) | NOT NULL | 交易对 |
| quantity | DECIMAL(20,8) | NOT NULL | 持仓数量 |
| avg_price | DECIMAL(20,8) | NOT NULL | 开仓均价 |
| mode | trade_mode | DEFAULT 'simulation' | 交易模式 |
| unrealized_pnl | DECIMAL(20,8) | DEFAULT 0 | 浮动盈亏 |
| margin | DECIMAL(20,8) | DEFAULT 0 | 占用保证金 |
| opened_at | TIMESTAMPTZ | DEFAULT NOW() | 开仓时间 |
| updated_at | TIMESTAMPTZ | DEFAULT NOW() | 更新时间 |

```sql
CREATE UNIQUE INDEX idx_positions_user_symbol ON positions(user_id, symbol, mode);
CREATE INDEX idx_positions_user_id ON positions(user_id);
CREATE INDEX idx_positions_strategy_id ON positions(strategy_id);
```

---

### 2.5 行情域

#### kline_data（K线数据表）

> **注意：** K线数据量大，建议按 symbol + interval 分表（或使用 TimescaleDB 扩展）
> 当前设计为单表 + 复合主键 + 分区（按时间）

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| symbol | VARCHAR(50) | NOT NULL | 交易对 |
| interval | VARCHAR(5) | NOT NULL | 周期（1m,5m,15m,1h,4h,1d,1w） |
| open_time | BIGINT | NOT NULL | 开盘时间戳（毫秒） |
| open | DECIMAL(20,8) | NOT NULL | 开盘价 |
| high | DECIMAL(20,8) | NOT NULL | 最高价 |
| low | DECIMAL(20,8) | NOT NULL | 最低价 |
| close | DECIMAL(20,8) | NOT NULL | 收盘价 |
| volume | DECIMAL(20,8) | NOT NULL | 成交量 |
| exchange | VARCHAR(20) | DEFAULT 'binance' | 数据来源 |

```sql
-- 复合主键（避免重复插入，且支持高效的范围查询）
CREATE TABLE kline_data (
    symbol VARCHAR(50) NOT NULL,
    interval VARCHAR(5) NOT NULL,
    open_time BIGINT NOT NULL,
    open DECIMAL(20,8) NOT NULL,
    high DECIMAL(20,8) NOT NULL,
    low DECIMAL(20,8) NOT NULL,
    close DECIMAL(20,8) NOT NULL,
    volume DECIMAL(20,8) NOT NULL,
    exchange VARCHAR(20) DEFAULT 'binance',
    PRIMARY KEY (symbol, interval, open_time)
);

-- 按时间范围分区示例（月度分区）
-- CREATE TABLE kline_data_2024_01 PARTITION OF kline_data
--   FOR VALUES FROM (1704067200000) TO (1706745599000);
```

**索引：**
```sql
-- 回测查询：按 symbol + interval + 时间范围
CREATE INDEX idx_kline_lookup ON kline_data(symbol, interval, open_time DESC);
-- 最新数据查询
CREATE INDEX idx_kline_latest ON kline_data(symbol, interval, open_time DESC);
```

---

### 2.6 系统域

#### audit_log（审计日志表）

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PK | 日志 ID |
| user_id | UUID | FK → users(id) | 操作者 |
| action | VARCHAR(50) | NOT NULL | 操作类型 |
| resource_type | VARCHAR(50) | | 资源类型 |
| resource_id | VARCHAR(100) | | 资源 ID |
| detail | JSONB | | 操作详情 |
| ip_address | INET | | 来源 IP |
| created_at | TIMESTAMPTZ | DEFAULT NOW() | 时间 |

```sql
CREATE INDEX idx_audit_log_user_id ON audit_log(user_id);
CREATE INDEX idx_audit_log_action ON audit_log(action);
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at DESC);
```

#### risk_rules（风控规则表）

| 列名 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 规则 ID |
| user_id | UUID | FK → users(id), NULLABLE | 用户级规则（null=全局） |
| name | VARCHAR(100) | NOT NULL | 规则名称 |
| rule_type | VARCHAR(50) | NOT NULL | 规则类型 |
| parameters | JSONB | NOT NULL | 规则参数 |
| enabled | BOOLEAN | DEFAULT true | 启用状态 |
| created_at | TIMESTAMPTZ | DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMPTZ | DEFAULT NOW() | 更新时间 |

**rule_type 枚举值：**
- `max_position` — 最大持仓数量
- `max_leverage` — 最大杠杆倍数
- `max_daily_loss` — 每日最大亏损
- `max_single_loss` — 单笔最大亏损
- `max_position_concentration` — 集中度限制
- `min_margin_ratio` — 最低保证金率

---

## 3. 实体关系图（ER）

```
users 1──N strategies           （用户拥有多个策略）
users 1──N orders               （用户下多个委托）
users 1──N positions            （用户有多个持仓）
users 1──N backtest_results     （用户运行多个回测）

strategies 1──N orders          （策略生成多个委托）
strategies 1──N positions       （策略有多个持仓）
strategies 1──N backtest_results（策略有多个回测）
strategies 1──N strategy_logs   （策略有多个日志）

orders 1──N trades              （一个委托可分多次成交）
orders 1──1 positions           （委托可能开平持仓）

admin ─N audit_log              （审计日志）
```

---

## 4. Redis 数据结构

### 4.1 缓存 Key 设计

| Key | 类型 | TTL | 说明 |
|-----|------|-----|------|
| `ticker:{symbol}` | HASH | 5s | 最新 Ticker |
| `depth:{symbol}` | STRING(JSON) | 1s | 最新深度快照 |
| `kline:{symbol}:{interval}` | LIST | 存 500 根 | 最新 K 线缓存 |
| `token:blacklist:{jti}` | STRING | 剩余有效期 | JWT 黑名单 |
| `rate_limit:{user_id}` | SortedSet | 1min | API 限流计数器 |
| `ws:sessions:{user_id}` | SET | - | 用户 WebSocket 连接 ID |
| `position_cache:{user_id}:{symbol}` | HASH | 5s | 持仓缓存 |

### 4.2 Pub/Sub 频道

| 频道 | 用途 | 发布者 | 订阅者 |
|------|------|--------|--------|
| `market:kline:{symbol}:{interval}` | K线更新 | Collector | WS Hub |
| `market:ticker:{symbol}` | Ticker更新 | Collector | WS Hub |
| `market:depth:{symbol}` | 深度更新 | Collector | WS Hub |
| `strategy:signal:{strategy_id}` | 策略信号 | Strategy Engine | Trade Service |
| `strategy:status:{strategy_id}` | 策略状态变更 | Strategy Engine | WS Hub |
| `risk:alert:{user_id}` | 风控预警 | Risk Manager | WS Hub |

---

## 5. 数据保留策略

| 数据表 | 保留时间 | 归档方式 |
|--------|----------|----------|
| kline_data (1m) | 30 天 | 自动删除 |
| kline_data (15m/1h) | 1 年 | 自动删除 |
| kline_data (4h/1d/1w) | 永久 | - |
| strategy_logs | 90 天 | 自动删除 |
| audit_log | 1 年 | 归档到冷存储 |
| orders (filled/cancelled) | 永久 | 保留 |
| trades | 永久 | 保留 |
| backtest_results | 永久 | 保留 |
