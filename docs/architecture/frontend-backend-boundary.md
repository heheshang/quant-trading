# 量化交易系统 — 前后端边界定义

> 版本: v1.0 | 约定: RESTful API + WebSocket Push

---

## 1. 通信协议

| 维度 | REST API | WebSocket |
|------|----------|-----------|
| 用途 | CRUD 操作、认证、查询 | 实时行情、策略状态推送、风控预警 |
| 基础 URL | `https://host/api/v1/` | `wss://host/api/v1/market/ws?token=<jwt>` |
| 数据格式 | JSON (snake_case) | JSON (snake_case) |
| 认证 | `Authorization: Bearer <access_token>` | 查询参数 `token=<jwt>` |
| 并发限制 | 100 req/min/user | 无（长连接） |
| 超时 | 10s 默认 | 30s 无声断开 |

---

## 2. API 契约

### 2.1 响应格式统一规范

**成功响应：**
```json
{
  "code": 0,
  "data": { ... },
  "message": "success"
}
```

**分页响应：**
```json
{
  "code": 0,
  "data": [ ... ],
  "meta": {
    "page": 1,
    "size": 20,
    "total": 100
  },
  "message": "success"
}
```

**错误响应：**
```json
{
  "code": 40001,
  "message": "参数错误",
  "details": [
    {"field": "email", "message": "邮箱格式不正确"}
  ]
}
```

**错误码范围：**

| 范围 | 模块 | 示例 |
|------|------|------|
| 100xx | 通用 | 10000=未知错误, 10001=参数验证失败 |
| 101xx | 认证 | 10101=未登录, 10102=Token过期, 10103=权限不足 |
| 102xx | 用户 | 10201=邮箱已存在, 10202=账户已锁定 |
| 103xx | 行情 | 10301=交易对不存在, 10302=数据源不可用 |
| 104xx | 策略 | 10401=策略名称重复, 10402=策略语法错误 |
| 105xx | 回测 | 10501=数据不足, 10502=回测正在运行 |
| 106xx | 交易 | 10601=余额不足, 10602=风控拦截, 10603=交易所维护 |
| 107xx | 持仓 | 10701=持仓不足 |

---

### 2.2 认证模块

| 端点 | 方法 | 协议 | 说明 |
|------|------|------|------|
| `/api/v1/auth/register` | POST | REST | 用户注册，返回 201 |
| `/api/v1/auth/login` | POST | REST | 登录，返回 access + refresh token |
| `/api/v1/auth/refresh` | POST | REST | 刷新 access_token |
| `/api/v1/auth/logout` | POST | REST | 登出，jti 加入黑名单 |

**POST /api/v1/auth/register**
```json
// Request
{
  "email": "trader@example.com",
  "password": "Abc@123456",
  "confirm_password": "Abc@123456"
}
// Response 201
{
  "code": 0,
  "data": { "user_id": "uuid" },
  "message": "注册成功，请查收验证邮件"
}
```

**POST /api/v1/auth/login**
```json
// Request
{
  "email": "trader@example.com",
  "password": "Abc@123456"
}
// Response 200
{
  "code": 0,
  "data": {
    "access_token": "eyJhbGciOi...",
    "refresh_token": "eyJhbGciOi...",
    "expires_in": 900,
    "user": {
      "id": "uuid",
      "email": "trader@example.com",
      "display_name": "trader",
      "role": "trader"
    }
  }
}
```

**POST /api/v1/auth/refresh**
```json
// Request (refresh_token in body, NOT in cookie)
{
  "refresh_token": "eyJhbGciOi..."
}
// Response
{
  "code": 0,
  "data": {
    "access_token": "eyJhbGciOi...",
    "expires_in": 900
  }
}
```

---

### 2.3 行情模块

| 端点 | 方法 | 协议 | 说明 |
|------|------|------|------|
| `/api/v1/market/kline` | GET | REST | 获取历史 K 线 |
| `/api/v1/market/ticker` | GET | REST | 获取所有 Ticker |
| `/api/v1/market/depth` | GET | REST | 获取深度数据 |
| `/api/v1/market/ws` | - | WS | WebSocket 行情推送 |
| `/api/v1/market/symbols` | GET | REST | 获取支持交易对列表 |

**GET /api/v1/market/kline?symbol=BTC/USDT&interval=1h&start=1704067200000&end=1735689599000&limit=1000**
```json
// Response
{
  "code": 0,
  "data": [
    {
      "t": 1704067200000,   // open_time (ms)
      "o": "42000.00",      // open
      "h": "42500.00",      // high
      "l": "41800.00",      // low
      "c": "42300.00",      // close
      "v": "1234.567"       // volume
    }
  ]
}
```

**GET /api/v1/market/ticker**
```json
// Response
{
  "code": 0,
  "data": [
    {
      "symbol": "BTC/USDT",
      "last": "42300.00",
      "change_24h": 2.35,
      "high_24h": "42500.00",
      "low_24h": "41000.00",
      "volume_24h": "123456.789",
      "bid": "42299.00",
      "ask": "42301.00"
    }
  ]
}
```

**GET /api/v1/market/depth?symbol=BTC/USDT&limit=20**
```json
{
  "code": 0,
  "data": {
    "symbol": "BTC/USDT",
    "bids": [["42299.00", "1.5"], ["42298.00", "2.3"]],
    "asks": [["42301.00", "1.2"], ["42302.00", "1.8"]],
    "timestamp": 1715500000000
  }
}
```

---

### 2.4 WebSocket 协议

**连接：** `wss://host/api/v1/market/ws?token=<jwt_access_token>`

**Client → Server 消息（订阅控制）：**

```json
// 订阅频道
{
  "action": "subscribe",
  "channels": [
    "kline:BTC/USDT:1m",
    "ticker:ETH/USDT",
    "depth:BTC/USDT"
  ]
}

// 取消订阅
{
  "action": "unsubscribe",
  "channels": ["kline:BTC/USDT:1m"]
}

// 心跳回复
{
  "action": "pong"
}
```

**Server → Client 消息（数据推送）：**

```json
// K线更新
{
  "type": "kline",
  "channel": "kline:BTC/USDT:1m",
  "data": {
    "t": 1715500000000,
    "o": "42000.00",
    "h": "42500.00",
    "l": "41800.00",
    "c": "42300.00",
    "v": "1234.567"
  },
  "ts": 1715500000000
}

// Ticker更新
{
  "type": "ticker",
  "channel": "ticker:ETH/USDT",
  "data": {
    "last": "2800.00",
    "bid": "2799.00",
    "ask": "2801.00",
    "change_24h": -1.23,
    "volume_24h": "56789.012"
  },
  "ts": 1715500000000
}

// 深度更新
{
  "type": "depth",
  "channel": "depth:BTC/USDT",
  "data": {
    "bids": [["42299.00", "1.5"]],
    "asks": [["42301.00", "1.2"]]
  },
  "ts": 1715500000000
}

// 策略状态更新
{
  "type": "strategy_status",
  "channel": "strategy_status",
  "data": {
    "strategy_id": "uuid",
    "status": "running_live",
    "message": "策略已启动 - 实盘模式"
  },
  "ts": 1715500000000
}

// 风控预警
{
  "type": "risk_alert",
  "channel": "risk_alert",
  "data": {
    "alert_type": "max_leverage",
    "severity": "warning",
    "message": "当前杠杆率超过阈值（3x）"
  },
  "ts": 1715500000000
}

// 心跳
{
  "type": "heartbeat",
  "ts": 1715500000000
}

// 回测进度
{
  "type": "backtest_progress",
  "channel": "backtest_progress",
  "data": {
    "backtest_id": "uuid",
    "progress": 65,
    "status": "running"
  },
  "ts": 1715500000000
}
```

---

### 2.5 策略模块

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/strategies` | GET | 策略列表（分页） |
| `/api/v1/strategies` | POST | 创建策略 |
| `/api/v1/strategies/{id}` | GET | 策略详情 |
| `/api/v1/strategies/{id}` | PUT | 更新策略 |
| `/api/v1/strategies/{id}` | DELETE | 删除策略 |
| `/api/v1/strategies/{id}/start` | POST | 启动策略 |
| `/api/v1/strategies/{id}/stop` | POST | 停止策略 |
| `/api/v1/strategies/templates` | GET | 策略模板列表 |

**POST /api/v1/strategies (创建策略)**
```json
// Request
{
  "name": "双均线策略",
  "symbol": "BTC/USDT",
  "interval": "1h",
  "params": {
    "fast_period": 5,
    "slow_period": 20
  },
  "risk_config": {
    "max_position": 1,
    "stop_loss_pct": 5.0,
    "take_profit_pct": 10.0
  }
}

// Response 201
{
  "code": 0,
  "data": {
    "id": "uuid",
    "name": "双均线策略",
    "status": "stopped",
    "mode": "simulation"
  }
}
```

**POST /api/v1/strategies/{id}/start (启动策略)**
```json
// Request
{
  "mode": "simulation"  // 或 "live"
}

// Response (实盘模式下触发风控确认)
{
  "code": 0,
  "data": {
    "status": "running_simulation",
    "risk_warnings": null
  }
}

// 实盘风控确认需要：
{
  "mode": "live",
  "risk_confirm": true,
  "risk_config": {
    "max_position": 1,
    "daily_loss_limit": 10000
  }
}
```

---

### 2.6 回测模块

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/backtest/run` | POST | 运行回测 |
| `/api/v1/backtest/{id}` | GET | 回测结果 |
| `/api/v1/backtest/{id}` | DELETE | 删除回测记录 |
| `/api/v1/backtest` | GET | 历史回测列表 |
| `/api/v1/backtest/{id}/compare` | GET | 多策略对比 |

**POST /api/v1/backtest/run**
```json
// Request
{
  "strategy_id": "uuid",
  "config": {
    "date_range": {
      "start": "2024-01-01",
      "end": "2024-12-31"
    },
    "initial_capital": 100000,
    "fee_rate": 0.001,
    "slippage_rate": 0.0005
  }
}

// Response 202 (Accepted)
{
  "code": 0,
  "data": {
    "backtest_id": "uuid",
    "status": "pending"
  }
}
```

**GET /api/v1/backtest/{id}**
```json
// Response
{
  "code": 0,
  "data": {
    "id": "uuid",
    "strategy_name": "双均线策略",
    "status": "completed",
    "config": { ... },
    "metrics": {
      "total_return_pct": 25.3,
      "annualized_return_pct": 22.1,
      "max_drawdown_pct": -12.5,
      "sharpe_ratio": 1.85,
      "win_rate": 0.62,
      "total_trades": 85,
      "profit_factor": 2.3
    },
    "equity_curve": [
      {"t": "2024-01-01", "equity": 100000},
      {"t": "2024-06-30", "equity": 115000},
      {"t": "2024-12-31", "equity": 125300}
    ],
    "summary_trades": [
      {
        "open_time": "2024-01-15T10:00:00Z",
        "close_time": "2024-01-20T14:00:00Z",
        "direction": "long",
        "open_price": "42000",
        "close_price": "43500",
        "pnl": 1500,
        "roi_pct": 3.57
      }
    ],
    "created_at": "2024-01-01T10:00:00Z"
  }
}
```

---

### 2.7 交易模块

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/orders` | GET | 委托列表（支持筛选） |
| `/api/v1/orders` | POST | 创建委托 |
| `/api/v1/orders/{id}` | DELETE | 撤单 |

**GET /api/v1/orders?status=pending&symbol=BTC/USDT&page=1&size=20**
```json
{
  "code": 0,
  "data": [
    {
      "id": "uuid",
      "symbol": "BTC/USDT",
      "side": "buy",
      "type": "limit",
      "price": "42000.00",
      "amount": "0.1",
      "filled": "0.05",
      "status": "partial_filled",
      "mode": "simulation",
      "created_at": "2024-01-15T10:00:00Z"
    }
  ],
  "meta": { "page": 1, "size": 20, "total": 5 }
}
```

**POST /api/v1/orders**
```json
// Request - 限价单
{
  "symbol": "BTC/USDT",
  "side": "buy",
  "type": "limit",
  "price": "42000",
  "amount": "0.1"
}

// Request - 市价单
{
  "symbol": "BTC/USDT",
  "side": "sell",
  "type": "market",
  "amount": "0.1"
}

// Response 201
{
  "code": 0,
  "data": {
    "id": "uuid",
    "status": "pending"
  }
}
```

---

### 2.8 持仓模块

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/portfolio` | GET | 持仓列表 |
| `/api/v1/portfolio/pnl` | GET | 盈亏分析 |
| `/api/v1/portfolio/risk` | GET | 风控指标 |
| `/api/v1/portfolio/{symbol}` | POST | 平仓 |

**GET /api/v1/portfolio**
```json
{
  "code": 0,
  "data": [
    {
      "symbol": "BTC/USDT",
      "quantity": "0.5",
      "avg_price": "42000.00",
      "current_price": "42300.00",
      "realized_pnl": "150.00",
      "unrealized_pnl": "150.00",
      "unrealized_pnl_pct": 0.71,
      "margin": "21000.00"
    }
  ],
  "meta": {
    "total_equity": 125000,
    "cash_balance": 83000,
    "used_margin": 42000
  }
}
```

**GET /api/v1/portfolio/pnl?period=30d**
```json
{
  "code": 0,
  "data": {
    "today_pnl": 250,
    "this_week_pnl": 1200,
    "this_month_pnl": 3500,
    "cumulative_pnl": 25000,
    "daily_pnl": [
      {"date": "2024-12-01", "pnl": 100},
      {"date": "2024-12-02", "pnl": -50}
    ],
    "by_symbol": [
      {"symbol": "BTC/USDT", "pnl": 18000},
      {"symbol": "ETH/USDT", "pnl": 7000}
    ]
  }
}
```

**GET /api/v1/portfolio/risk**
```json
{
  "code": 0,
  "data": {
    "var_95_1d": 3500,
    "leverage": 1.5,
    "concentration_max_pct": 0.4,
    "max_drawdown_pct": -12.5,
    "current_drawdown_pct": -3.2
  }
}
```

---

### 2.9 仪表盘

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/dashboard` | GET | 仪表盘总览 |
| `/api/v1/dashboard/layout` | PUT | 保存仪表盘布局 |

**GET /api/v1/dashboard**
```json
{
  "code": 0,
  "data": {
    "total_asset_history": [
      {"date": "2024-12-01", "value": 100000},
      {"date": "2024-12-31", "value": 125000}
    ],
    "today_pnl": {"amount": 250, "pct": 0.2},
    "running_strategies": 3,
    "pending_orders": 2,
    "recent_trades": [
      {"symbol": "BTC/USDT", "side": "buy", "price": "42000", "amount": "0.1", "time": "2024-12-31T10:00:00Z"}
    ],
    "favorite_tickers": [
      {"symbol": "BTC/USDT", "last": "42300", "change_24h": 2.35}
    ]
  }
}
```

---

### 2.10 后台管理

| 端点 | 方法 | 权限 | 说明 |
|------|------|------|------|
| `/api/v1/admin/users` | GET | admin | 用户列表 |
| `/api/v1/admin/users/{id}` | PUT | admin | 修改用户角色/状态 |
| `/api/v1/admin/users/{id}` | DELETE | admin | 禁用用户 |
| `/api/v1/admin/risk-rules` | CRUD | admin | 风控规则配置 |

---

## 3. 前端状态边界

Pinia Store 划分（前端侧，不持久化到后端）：

| Store | 数据来源 | 更新方式 | 说明 |
|-------|----------|----------|------|
| `useAuthStore` | API + localStorage | 登录/刷新/登出 | 管理 token + 用户信息 |
| `useMarketStore` | REST + WebSocket | WS 实时更新 | 行情数据、自选列表 |
| `useStrategyStore` | REST | 用户操作 | 策略 CRUD 列表 |
| `useTradeStore` | REST | 用户操作 | 委托、成交记录 |
| `usePortfolioStore` | REST | 定时轮询 | 持仓、盈亏、风控 |
| `useDashboardStore` | REST | 定时轮询 | 仪表盘数据 |

**数据缓存策略：**

| 数据 | 缓存位置 | TTL | 刷新策略 |
|------|----------|-----|----------|
| access_token | localStorage | 15min | 自动 refresh |
| user_info | Pinia + localStorage | 页面关闭 | 登录时写入 |
| market data | Pinia + indexedDB (K线) | 页面关闭 | WS 增量更新 |
| strategy list | Pinia | 页面关闭 | 用户操作后刷新 |
| ticker list | Pinia | 5s | WS 推送 |

---

## 4. 前端组件 → API 映射

| 页面 | 组件 | 调用的 API / WS 频道 |
|------|------|----------------------|
| 登录页 | LoginForm | POST /auth/login |
| 注册页 | RegisterForm | POST /auth/register |
| 仪表盘 | DashboardView | GET /dashboard + WS strategy_status |
| 行情页 | MarketView, KLineChart | GET /market/kline, /ticker, /depth + WS kline/ticker/depth |
| 策略列表 | StrategyListView | GET /strategies, POST /strategies/{id}/start/stop + WS strategy_status |
| 策略创建 | StrategyEditor | POST /strategies, GET /strategies/templates |
| 回测配置 | BacktestConfig | POST /backtest/run + WS backtest_progress |
| 回测结果 | BacktestResult | GET /backtest/{id} |
| 交易页 | TradePanel | GET/POST /orders, DELETE /orders/{id} |
| 委托页 | OrderBook | GET /orders |
| 持仓页 | PortfolioView | GET /portfolio, POST /portfolio/{symbol} |
| 盈亏分析 | PnLView | GET /portfolio/pnl |
| 风控页 | RiskView | GET /portfolio/risk, WS risk_alert |
| 用户管理 | UserManagement | GET/PUT /admin/users |
| 个人中心 | ProfileView | PUT /auth/profile |

---

## 5. 请求流程示例

### 用户登录 → 查看 K 线 → 创建策略 → 运行回测

```
┌───────────┐     REST API                   ┌───────────────┐
│           │──────────────── POST /login ───→│               │
│           │←────── {access, refresh} ───────│               │
│           │                                 │               │
│           │──── GET /market/kline ─────────→│               │
│           │←──────── {kline[]} ─────────────│               │
│           │                                 │               │
│           │──── WS connect ?token=xxx ─────→│               │
│           │←──── WS: kline/ticker/depth ────│  Rust Axum    │
│           │                                 │  Backend      │
│  Vue 3    │──── POST /strategies ──────────→│               │
│  Client   │←────── {strategy_id} ───────────│               │
│           │                                 │               │
│           │──── POST /strategies/{id}/start─→│               │
│           │←──── {status: "running_sim"} ───│               │
│           │                                 │               │
│           │──── POST /backtest/run ────────→│               │
│           │←──── {backtest_id, pending} ────│               │
│           │←──── WS: backtest_progress 65% ─│               │
│           │←──── WS: backtest_progress 100% │               │
│           │──── GET /backtest/{id} ────────→│               │
│           │←──── {metrics, equity_curve} ───│               │
└───────────┘                                 └───────────────┘
```
