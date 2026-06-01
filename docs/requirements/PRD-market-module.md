# PRD: 行情模块 — 实时K线、Ticker、深度与WebSocket推送

> 版本: v1.0
> 状态: Draft
> 作者: PM
> 关联: ADR-002 (WebSocket实时行情推送), ADR-005 (市场数据管道), data-model.md, PRD-kline-management.md, PRD.md US-04/US-05/US-06

---

## 1. 背景

行情模块是量化交易系统的"眼睛"——用户通过它观察市场、发现机会、验证策略。主 PRD (v1.0) 将行情列为 P0 核心功能，定义了 US-04 (实时K线)、US-05 (深度数据)、US-06 (Ticker行情) 三个用户故事。

### 当前状态 (2026-05-13)

- 前端 `MarketView.vue` 仅为占位组件，显示 "Market data coming soon"
- 前端 `api/market.ts` 已定义 `getKline`、`getTickers`、`getDepth` 三个 API 调用（但后端未实现对应端点）
- 前端 `types/market.ts` 已定义 `Kline`、`Ticker`、`Depth` 接口
- 后端 `handlers/ws.rs` 已实现 WebSocket 基础框架（echo 模式，JWT 鉴权已就绪）
- 后端 `services/market_data.rs` 不存在（PRD TECH_CHARTER 中有占位但未创建）
- 后端 `handlers/market.rs` 不存在（TECH_CHARTER 中有占位但未创建）
- Redis 缓存层未部署
- ADR-002 已批准 WebSocket (tokio-tungstenite + Axum) 方案
- ADR-005 已批准统一数据网关架构（Exchange→Collector→Redis→WS Hub→Client）
- PRD-kline-management.md 已覆盖 K线数据的导入/查询/清洗/存储/导出（历史数据管理），但不涉及实时行情展示和推送

### 本 PRD 与 PRD-kline-management.md 的关系

| 范围 | PRD-kline-management | 本 PRD (行情模块) |
|------|---------------------|-------------------|
| K线数据导入/导出/清洗 | ✅ | — |
| K线数据存储格式 | ✅ | — |
| K线数据权限隔离 | ✅ | — |
| 实时K线图表展示 | — | ✅ |
| 指标叠加 (MA/BOLL/MACD) | — | ✅ |
| Ticker 行情概览 | — | ✅ |
| 深度/盘口数据 | — | ✅ |
| WebSocket 实时推送 | — | ✅ |
| 市场数据采集管道 | — | ✅ |
| 自选列表 | — | ✅ |

---

## 2. 目标

| 目标 | 指标 | 当前状态 |
|------|------|---------|
| 实时K线展示 | 支持 7 个周期 (1m/5m/15m/30m/1h/4h/1d/1w)，首屏加载 < 1s | MarketView 为占位组件 |
| WebSocket 推送延迟 | < 500ms (交易所→Redis→WS→客户端) | WS 为 echo 模式 |
| Ticker 面板 | 支持全市场 Ticker 列表、搜索、排序、自选 | 不存在 |
| 深度数据 | 买/卖各 20 档，实时更新，深度图可视化 | 不存在 |
| 指标叠加 | MA / BOLL / MACD / RSI / KDJ，最多同时叠加 3 个 | 不存在 |
| 市场数据采集 | 至少 1 个交易所 (Binance) 的 WS 连接 + REST 降级 | Collector 不存在 |
| 并发连接 | 支持 ≥ 1000 同时在线 WebSocket 连接 | 未测试 |
| Redis 缓存命中率 | Ticker/Depth 查询命中率 ≥ 95% | Redis 未部署 |

### 非目标 (Non-Goals)

- ❌ 多交易所支持（Binance 优先，OKX/Bybit 为 v2.0）
- ❌ 交易所直采功能（已在 PRD-kline-management P2 中规划）
- ❌ 自定义指标编写（P2+）
- ❌ K线数据导出/导入/清洗（PRD-kline-management 范围）
- ❌ 交易功能（下单/撤单，P1 范围）
- ❌ 移动端适配（P2+）

---

## 3. 用户角色与权限

| 角色 | 行情权限 | 说明 |
|------|---------|------|
| **trader** (普通用户) | 查看所有行情、管理自选列表 | 默认角色，无需额外权限 |
| **pro-trader** (专业交易员) | 同 trader，无差异 | 行情数据对所有登录用户开放 |
| **admin** (管理员) | 同 trader + 可查看数据采集状态/监控 | 运维监控用途 |

> 注：行情数据为公共数据，所有已登录用户均可查看。区别仅在于 admin 可访问系统监控端点。

---

## 4. 用户故事 (User Stories)

### US-MK-01: 查看实时K线图 (P0)

> **As a** 交易者
> **I want to** 在行情页面查看实时K线图
> **So that** 我可以分析价格走势和识别交易机会

**验收条件：**

```gherkin
Feature: 实时K线图展示

  Background:
    Given 用户已登录
      And 系统已连接到市场数据源

  Scenario: 加载指定交易对的K线数据
    Given 用户选择交易对 "BTC/USDT"
      And 选择时间周期 "1d"
    When 打开K线页面
    Then 显示最近 500 根 K 线
      And K线图使用 Lightweight-charts 渲染
      And 加载时间 < 1s

  Scenario: K线周期切换
    Given K线图已显示 "1d" 周期
    When 用户点击 "1h" 周期按钮
    Then K线图切换为 1h 周期数据
      And 图表重新渲染
      And 不刷新页面

  Scenario: 支持的K线周期
    Given K线周期选择器
    Then 显示以下周期选项: 1m, 5m, 15m, 30m, 1h, 4h, 1d, 1w

  Scenario: K线实时更新
    Given K线图已打开且 WebSocket 已连接
    When 新的 1m K线收盘
    Then 通过 WebSocket 推送新 K 线数据
      And 图表自动追加新 K 线（不刷新页面）
      And 当前未收盘 K 线实时更新（价格变动时最后一根K线动态变化）

  Scenario: K线图交互
    Given K线图已渲染
    Then 支持鼠标滚轮缩放（放大/缩小时间轴）
      And 支持鼠标拖拽平移（左/右查看历史数据）
      And 支持十字光标显示当前价格和时间
      And 十字光标旁显示 OHLCV 数据

  Scenario: 加载更多历史K线
    Given K线图已渲染
    When 用户向左拖拽到图表边界
    Then 自动加载更早的历史 K 线数据
      And 追加到图表左侧
      And 加载时显示 loading 指示器

  Scenario: 无数据时显示占位
    Given 选择的交易对无 K 线数据
    When 加载K线图
    Then 显示空状态 "暂无数据"
      And 提示用户可导入数据或等待采集
```

---

### US-MK-02: 指标叠加 (P0)

> **As a** 交易者
> **I want to** 在K线图上叠加技术指标
> **So that** 我可以更直观地分析趋势和判断买卖信号

**验收条件：**

```gherkin
Feature: K线指标叠加

  Background:
    Given 用户已登录
      And K线图已显示

  Scenario: 叠加均线指标
    Given K线图已渲染
    When 用户在指标面板选择 "MA(5,20)"
    Then K线上叠加显示 5 日和 20 日均线
      And 均线使用不同颜色区分
      And 图例显示 MA5 和 MA20 标签

  Scenario: 切换指标
    Given K线图已叠加 "MA(5,20)" 指标
    When 用户选择 "BOLL" 指标
    Then 均线隐藏，布林带显示
      And 布林带包含上轨、中轨、下轨三条线
      And 上下轨之间填充半透明色带

  Scenario: 同时叠加多个指标
    Given K线图已渲染
    When 用户选择 "MA(5)" 和 "MACD"
    Then K线主图叠加 MA5 均线
      And 副图区域显示 MACD 指标（DIF、DEA、柱状图）
      And 最多同时叠加 3 个指标

  Scenario: 超出指标数量限制
    Given K线图已叠加 3 个指标
    When 用户尝试添加第 4 个指标
    Then 提示 "最多同时显示 3 个指标，请先移除已有指标"
      And 不添加新指标

  Scenario: 移除指标
    Given K线图已叠加 "MA(5,20)" 指标
    When 用户点击指标标签的关闭按钮
    Then 该指标从图表移除
      And 图表重新渲染

  Scenario: 支持的指标列表
    Given 指标选择面板
    Then 显示以下指标: MA, BOLL, MACD, RSI, KDJ
      And 每个指标显示名称和简要说明
```

---

### US-MK-03: Ticker 行情概览 (P0)

> **As a** 交易者
> **I want to** 查看全市场 Ticker 概览
> **So that** 我可以快速掌握市场整体状况

**验收条件：**

```gherkin
Feature: Ticker 行情概览

  Background:
    Given 用户已登录

  Scenario: Ticker 列表显示
    Given 用户导航至行情页面
    When Ticker 面板加载
    Then 显示所有交易对 Ticker 列表
      And 每行包含: 交易对名称、最新价、24h涨跌幅、24h最高、24h最低、24h成交量
      And 默认按 24h 成交量降序排列
      And 数据通过 REST API 初始加载，后续通过 WebSocket 增量更新

  Scenario: Ticker 价格闪烁
    Given Ticker 面板已打开
    When 最新价发生变化
    Then 价格上涨时数字绿色闪烁
      And 价格下跌时数字红色闪烁
      And 0.5s 后恢复正常颜色
      And 闪烁效果使用 CSS transition 实现

  Scenario: Ticker 排序
    Given Ticker 列表已显示
    When 用户点击 "24h涨跌幅" 列头
    Then 列表按 24h 涨跌幅降序排列
      And 再次点击切换为升序
      And 排序图标指示当前排序方向

  Scenario: Ticker 搜索
    Given Ticker 列表已显示
    When 用户在搜索框输入 "BTC"
    Then 列表实时过滤，仅显示包含 "BTC" 的交易对
      And 搜索为前端本地过滤（不发起 API 请求）
      And 清空搜索框恢复完整列表

  Scenario: 自选列表
    Given Ticker 列表已显示
    When 用户点击 "BTC/USDT" 旁的星标图标
    Then "BTC/USDT" 加入自选列表
      And 星标变为实心
      And 自选列表在独立 Tab 展示
      And 自选列表持久化到后端用户偏好

  Scenario: 取消自选
    Given "BTC/USDT" 已在自选列表中
    When 用户再次点击 "BTC/USDT" 旁的星标
    Then "BTC/USDT" 从自选列表移除
      And 星标变为空心
      And 自选列表 Tab 中不再显示该交易对

  Scenario: Ticker 面板与K线联动
    Given Ticker 列表已显示
    When 用户点击某个交易对行
    Then K线图切换至该交易对
      And Ticker 行高亮显示当前选中
```

---

### US-MK-04: 深度数据与盘口 (P1)

> **As a** 交易者
> **I want to** 查看市场深度和盘口数据
> **So that** 我可以判断买卖压力和市场流动性

**验收条件：**

```gherkin
Feature: 深度数据与盘口

  Background:
    Given 用户已登录
      And 已选择交易对 "BTC/USDT"

  Scenario: 加载深度数据
    Given 用户点击深度标签
    When 深度面板加载
    Then 显示买一至买二十档位（价格 + 数量 + 累计量）
      And 显示卖一至卖二十档位（价格 + 数量 + 累计量）
      And 买单价格降序排列（买一最高价在顶部）
      And 卖单价格升序排列（卖一最低价在顶部）
      And 买卖盘之间显示当前最新价和价差

  Scenario: 深度图可视化
    Given 深度面板已加载
    When 用户查看深度图区域
    Then 显示深度图（横轴价格、纵轴累计量）
      And 买单累计线为绿色
      And 卖单累计线为红色
      And 两线交叉处标注当前价格

  Scenario: 深度实时更新
    Given 深度面板已打开
    When 市场盘口发生变化
    Then 深度数据通过 WebSocket 实时更新
      And 深度图平滑过渡动画（过渡时间 200ms）
      And 盘口变化行闪烁提示

  Scenario: 深度档位调整
    Given 深度面板已打开
    When 用户选择档位为 "10档"
    Then 买卖盘各显示 10 档
      And 深度图相应调整

  Scenario: 深度数据为空
    Given 交易对 "NEWCOIN/USDT" 刚上线
      And 暂无深度数据
    When 加载深度面板
    Then 显示空状态 "暂无深度数据"
      And 深度图显示空占位
```

---

### US-MK-05: WebSocket 实时推送 (P0)

> **As a** 系统
> **I want to** 通过 WebSocket 实时推送行情数据
> **So that** 用户无需手动刷新即可获取最新行情

**验收条件：**

```gherkin
Feature: WebSocket 实时推送

  Background:
    Given 用户已登录

  Scenario: 建立 WebSocket 连接
    Given 用户打开行情页面
    When 客户端发起 WebSocket 连接请求
      And 连接 URL 为 wss://host/api/v1/market/ws?token={jwt}
    Then 连接建立成功
      And 服务端验证 JWT token
      And 连接状态显示为 "已连接"

  Scenario: 订阅行情频道
    Given WebSocket 连接已建立
    When 客户端发送订阅消息:
      | action | channels |
      | subscribe | ["kline:BTC/USDT:1m", "ticker:BTC/USDT", "depth:BTC/USDT"] |
    Then 服务端确认订阅
      And 客户端开始接收对应频道的推送数据

  Scenario: 取消订阅
    Given 已订阅 "kline:BTC/USDT:1m" 频道
    When 客户端发送取消订阅消息:
      | action | channels |
      | unsubscribe | ["kline:BTC/USDT:1m"] |
    Then 服务端确认取消
      And 客户端不再接收该频道数据

  Scenario: 心跳保活
    Given WebSocket 连接已建立
    When 每 15 秒
    Then 服务端发送 heartbeat 消息
      And 客户端 30s 无响应则服务端断开连接

  Scenario: 断线重连
    Given WebSocket 连接已建立
    When 网络中断
    Then 客户端检测到连接断开
      And 自动重连（指数退避: 1s → 2s → 4s → 8s, 最大 30s）
      And 重连成功后自动恢复之前的订阅
      And 补发断线期间的增量 K 线数据
      And 页面显示 "已重连" 提示（3s 后消失）

  Scenario: JWT 过期处理
    Given WebSocket 连接已建立
    When JWT access_token 过期
    Then 客户端使用 refresh_token 获取新 access_token
      And 使用新 token 重新建立 WebSocket 连接
      And 自动恢复订阅

  Scenario: 推送消息格式
    Given 已订阅 "ticker:BTC/USDT" 频道
    When 服务端推送数据
    Then 消息格式为:
      | 字段 | 值 |
      | type | "ticker" |
      | symbol | "BTC/USDT" |
      | data | {last, bid, ask, volume_24h, change_24h, high_24h, low_24h, timestamp} |
      | ts | 1715500000000 |
```

---

### US-MK-06: 市场数据采集管道 (P0)

> **As a** 系统
> **I want to** 从交易所采集实时行情数据
> **So that** 系统有可靠的数据源推送给用户

**验收条件：**

```gherkin
Feature: 市场数据采集管道

  Background:
    Given 系统已启动
      And Binance API 凭证已配置

  Scenario: WebSocket 数据采集启动
    Given Market Data Collector 启动
    When 连接 Binance WebSocket API
    Then 成功订阅 BTC/USDT 的 kline、ticker、depth 频道
      And 数据规范化为内部统一格式
      And 数据写入 Redis 缓存
      And 数据通过 Redis PubSub 广播至 WS Hub

  Scenario: REST 降级采集
    Given Binance WebSocket 连接断开
    When 重连退避期间
    Then 自动切换为 REST API 轮询
      And 轮询间隔: kline 5s, ticker 2s, depth 2s
      And 数据写入 Redis 缓存（与 WS 模式相同路径）

  Scenario: 交易所 API 限流
    Given Binance API 返回 429 (Too Many Requests)
    When Collector 检测到限流
    Then 自动退避重试（指数退避，最长 60s）
      And 降级使用 Redis 缓存数据
      And 记录告警日志
      And admin 可在监控面板看到限流事件

  Scenario: 数据规范化
    Given 从 Binance 接收到原始 K 线数据
    When Collector 处理数据
    Then 转换为内部统一格式:
      | 字段 | 说明 |
      | exchange | "binance" |
      | symbol | "BTC/USDT" (统一格式) |
      | type | "kline" |
      | data | {open, high, low, close, volume, timestamp, interval} |

  Scenario: 历史数据持久化
    Given 实时 K 线数据已规范化
    When 数据写入 Redis 缓存
    Then 同时批量写入 PostgreSQL klines 表
      And 批量写入间隔: 每 5 秒 flush 一次
      And 写入失败不影响实时推送

  Scenario: Collector 健康检查
    Given Market Data Collector 运行中
    When GET /api/v1/market/collector/status
    Then 返回:
      | 字段 | 说明 |
      | status | "running" / "degraded" / "stopped" |
      | exchange | "binance" |
      | ws_connected | true/false |
      | last_heartbeat | timestamp |
      | symbols_count | 已订阅交易对数 |
      | messages_per_second | 当前消息速率 |
```

---

### US-MK-07: 行情页面布局 (P0)

> **As a** 交易者
> **I want to** 在一个页面内同时查看K线、Ticker、深度
> **So that** 我可以高效获取全面的市场信息

**验收条件：**

```gherkin
Feature: 行情页面布局

  Background:
    Given 用户已登录

  Scenario: 行情页面三栏布局
    Given 用户导航至 /market 页面
    When 页面加载完成
    Then 显示三栏布局:
      | 区域 | 位置 | 内容 |
      | Ticker 列表 | 左侧 (240px) | 交易对列表 + 自选 + 搜索 |
      | K线主图 | 中间 (flex:1) | K线图 + 指标 + 周期选择 |
      | 深度/信息面板 | 右侧 (320px) | 深度数据 + 最新成交 + 交易对信息 |

  Scenario: 左侧面板折叠
    Given 行情页面已加载
    When 用户点击左侧面板折叠按钮
    Then Ticker 列表收起为图标列（60px）
      And K线主图自动扩展占用空间

  Scenario: 右侧面板 Tab 切换
    Given 右侧面板已显示
    Then 包含以下 Tab:
      | Tab | 内容 |
      | 深度 | 盘口数据 + 深度图 |
      | 信息 | 交易对详情 + 24h 统计 |

  Scenario: 交易对选择联动
    Given 用户在左侧 Ticker 列表点击 "ETH/USDT"
    Then 中间 K线图切换至 ETH/USDT
      And 右侧深度面板切换至 ETH/USDT
      And URL 更新为 /market/ETH-USDT
      And 左侧列表高亮当前选中项

  Scenario: 响应式布局
    Given 浏览器窗口宽度 < 1024px
    When 行情页面加载
    Then 布局切换为单栏模式
      And 左侧 Ticker 列表收起为下拉选择器
      And 右侧深度面板移至 K线图下方
```

---

### US-MK-08: 自选列表管理 (P1)

> **As a** 交易者
> **I want to** 管理我的自选交易对列表
> **So that** 我可以快速访问我关注的交易对

**验收条件：**

```gherkin
Feature: 自选列表管理

  Background:
    Given 用户已登录

  Scenario: 添加自选
    Given 用户在 Ticker 列表中
    When 点击 "BTC/USDT" 旁的星标
    Then "BTC/USDT" 添加到自选列表
      And 操作同步到后端 (PUT /api/v1/market/watchlist)
      And 自选列表上限 50 个交易对

  Scenario: 自选列表已满
    Given 用户自选列表已有 50 个交易对
    When 尝试添加第 51 个
    Then 提示 "自选列表已满（最多 50 个），请先移除部分交易对"
      And 不添加新交易对

  Scenario: 自选列表排序
    Given 自选列表有 5 个交易对
    When 用户拖拽排序
    Then 自选列表顺序更新
      And 排序保存到后端

  Scenario: 自选列表持久化
    Given 用户添加了自选交易对
    When 用户刷新页面或重新登录
    Then 自选列表从后端恢复
      And 顺序与之前一致
```

---

## 5. 数据模型

### 5.1 新增表

#### user_watchlist（用户自选列表）

```sql
CREATE TABLE user_watchlist (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    symbol VARCHAR(50) NOT NULL,
    sort_order SMALLINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, symbol)
);

CREATE INDEX idx_watchlist_user_id ON user_watchlist(user_id, sort_order);
```

### 5.2 Redis 缓存 Key（新增）

| Key | 类型 | TTL | 说明 |
|-----|------|-----|------|
| `ticker:{symbol}` | HASH | 5s | 最新 Ticker 快照 |
| `depth:{symbol}` | STRING(JSON) | 1s | 最新深度快照 |
| `kline:{symbol}:{interval}` | LIST | 存 500 根 | 最新 K 线缓存 |
| `tickers:all` | STRING(JSON) | 5s | 全市场 Ticker 汇总 |

### 5.3 Redis Pub/Sub 频道（新增）

| 频道 | 用途 | 发布者 | 订阅者 |
|------|------|--------|--------|
| `market:kline:{symbol}:{interval}` | K线更新 | Collector | WS Hub |
| `market:ticker:{symbol}` | Ticker更新 | Collector | WS Hub |
| `market:depth:{symbol}` | 深度更新 | Collector | WS Hub |

---

## 6. API 端点设计

遵循 TECH_CHARTER 规范：`/api/v1/{resource}` + snake_case JSON。

### 6.1 REST API

| 方法 | 路径 | 说明 | 优先级 |
|------|------|------|--------|
| GET | `/api/v1/market/kline` | 获取历史 K 线数据 | P0 |
| GET | `/api/v1/market/ticker` | 获取单个交易对 Ticker | P0 |
| GET | `/api/v1/market/tickers` | 获取全市场 Ticker 列表 | P0 |
| GET | `/api/v1/market/depth` | 获取深度数据 | P1 |
| WS | `/api/v1/market/ws` | 行情 WebSocket 推送 | P0 |
| GET | `/api/v1/market/watchlist` | 获取用户自选列表 | P1 |
| PUT | `/api/v1/market/watchlist` | 更新用户自选列表 | P1 |
| GET | `/api/v1/market/collector/status` | 采集器状态 (admin) | P2 |

### 6.2 API 详细设计

#### GET /api/v1/market/kline

**查询参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| symbol | string | 是 | 交易对，如 "BTC/USDT" |
| interval | string | 是 | 周期: 1m/5m/15m/30m/1h/4h/1d/1w |
| start_time | integer | 否 | 起始时间戳(ms) |
| end_time | integer | 否 | 结束时间戳(ms) |
| limit | integer | 否 | 返回条数，默认 500，最大 1500 |

**响应示例：**
```json
{
  "code": 0,
  "data": [
    {
      "open_time": 1715500800000,
      "open": "50000.00",
      "high": "51000.00",
      "low": "49000.00",
      "close": "50500.00",
      "volume": "1234.56",
      "close_time": 1715504399999
    }
  ],
  "message": "success"
}
```

#### GET /api/v1/market/tickers

**响应示例：**
```json
{
  "code": 0,
  "data": [
    {
      "symbol": "BTC/USDT",
      "last": "50500.00",
      "bid": "50499.00",
      "ask": "50501.00",
      "change_24h": 2.35,
      "high_24h": "51000.00",
      "low_24h": "49000.00",
      "volume_24h": "123456.78",
      "timestamp": 1715500000000
    }
  ],
  "message": "success"
}
```

#### GET /api/v1/market/depth

**查询参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| symbol | string | 是 | 交易对 |
| levels | integer | 否 | 档位数，默认 20，最大 50 |

**响应示例：**
```json
{
  "code": 0,
  "data": {
    "symbol": "BTC/USDT",
    "bids": [["50499.00", "1.500"], ["50498.00", "2.300"]],
    "asks": [["50501.00", "1.200"], ["50502.00", "1.800"]],
    "timestamp": 1715500000000
  },
  "message": "success"
}
```

#### PUT /api/v1/market/watchlist

**请求体：**
```json
{
  "symbols": ["BTC/USDT", "ETH/USDT", "SOL/USDT"]
}
```

**响应：**
```json
{
  "code": 0,
  "data": {
    "symbols": ["BTC/USDT", "ETH/USDT", "SOL/USDT"],
    "count": 3
  },
  "message": "success"
}
```

### 6.3 WebSocket 消息协议

**连接：** `wss://host/api/v1/market/ws?token={jwt_access_token}`

**客户端→服务端消息：**

```json
// 订阅
{
  "action": "subscribe",
  "channels": ["kline:BTC/USDT:1m", "ticker:BTC/USDT", "depth:BTC/USDT"]
}

// 取消订阅
{
  "action": "unsubscribe",
  "channels": ["kline:BTC/USDT:1m"]
}

// 心跳响应
{
  "action": "pong"
}
```

**服务端→客户端消息：**

```json
// K线推送
{
  "type": "kline",
  "symbol": "BTC/USDT",
  "data": {
    "interval": "1m",
    "open_time": 1715500800000,
    "open": "50000.00",
    "high": "51000.00",
    "low": "49000.00",
    "close": "50500.00",
    "volume": "1234.56",
    "is_closed": false
  },
  "ts": 1715500900000
}

// Ticker 推送
{
  "type": "ticker",
  "symbol": "BTC/USDT",
  "data": {
    "last": "50500.00",
    "bid": "50499.00",
    "ask": "50501.00",
    "volume_24h": "123456.78",
    "change_24h": 2.35,
    "high_24h": "51000.00",
    "low_24h": "49000.00"
  },
  "ts": 1715500900000
}

// 深度推送
{
  "type": "depth",
  "symbol": "BTC/USDT",
  "data": {
    "bids": [["50499.00", "1.500"], ["50498.00", "2.300"]],
    "asks": [["50501.00", "1.200"], ["50502.00", "1.800"]]
  },
  "ts": 1715500900000
}

// 心跳
{
  "type": "heartbeat",
  "ts": 1715500900000
}

// 订阅确认
{
  "type": "subscribed",
  "channels": ["kline:BTC/USDT:1m", "ticker:BTC/USDT"]
}

// 错误
{
  "type": "error",
  "code": 40001,
  "message": "Invalid channel format"
}
```

### 6.4 额外错误码

```rust
pub const ERR_MARKET_SYMBOL_NOT_FOUND: i32 = 40401;     // 交易对不存在
pub const ERR_MARKET_INTERVAL_INVALID: i32 = 40001;     // 无效周期
pub const ERR_MARKET_WS_AUTH_FAILED: i32 = 40101;       // WebSocket 鉴权失败
pub const ERR_MARKET_WS_CHANNEL_INVALID: i32 = 40002;   // 无效频道格式
pub const ERR_MARKET_WATCHLIST_FULL: i32 = 42901;       // 自选列表已满
pub const ERR_MARKET_COLLECTOR_DOWN: i32 = 50301;       // 采集器离线
```

---

## 7. 前端路由与页面设计

### 7.1 路由

| 路径 | 组件 | 说明 |
|------|------|------|
| `/market` | `MarketView.vue` | 行情主页面（三栏布局） |
| `/market/:symbol` | `MarketView.vue` | 指定交易对的行情页面 |

### 7.2 页面布局

```
┌──────────────────────────────────────────────────────────────────┐
│  [左侧面板 240px]  │  [中间K线主图 flex:1]  │  [右侧面板 320px] │
│                    │                        │                   │
│  ┌──────────────┐  │  ┌──────────────────┐  │  ┌─────────────┐  │
│  │ 🔍 搜索框     │  │  │ 周期选择: 1m..1w  │  │  │ [深度][信息] │  │
│  ├──────────────┤  │  ├──────────────────┤  │  ├─────────────┤  │
│  │ [全部][自选]  │  │  │                  │  │  │  卖20 105000 │  │
│  ├──────────────┤  │  │   K线图           │  │  │  卖19 104999 │  │
│  │ ★ BTC/USDT   │  │  │   Lightweight-   │  │  │  ...        │  │
│  │   50500 +2.3%│  │  │   charts         │  │  │  买1  50499  │  │
│  │   ETH/USDT   │  │  │                  │  │  │  买2  50498  │  │
│  │   3200  -1.2%│  │  ├──────────────────┤  │  │  ...        │  │
│  │   SOL/USDT   │  │  │ 指标: MA BOLL    │  │  ├─────────────┤  │
│  │   150  +5.1% │  │  │ + MACD (副图)    │  │  │  深度图      │  │
│  │   ...        │  │  └──────────────────┘  │  │             │  │
│  └──────────────┘  │                        │  └─────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

### 7.3 组件树

```
MarketView.vue
├── MarketSidebar.vue              # 左侧面板
│   ├── SymbolSearch.vue           # 交易对搜索
│   ├── WatchlistTab.vue           # 自选列表
│   └── AllTickersTab.vue          # 全部 Ticker
│       └── TickerRow.vue          # 单行 Ticker
├── MarketChart.vue                # 中间K线主图
│   ├── PeriodSelector.vue         # 周期选择器
│   ├── CandlestickChart.vue       # K线图 (Lightweight-charts)
│   ├── IndicatorPanel.vue         # 指标选择面板
│   └── SubChart.vue               # 副图 (MACD/RSI/KDJ)
└── MarketDetail.vue               # 右侧面板
    ├── DepthPanel.vue             # 深度盘口
    │   ├── OrderBook.vue          # 盘口列表
    │   └── DepthChart.vue         # 深度图
    └── SymbolInfo.vue             # 交易对信息
```

### 7.4 前端状态管理 (Pinia Store)

```typescript
// stores/market.ts
interface MarketState {
  currentSymbol: string           // 当前选中交易对
  currentInterval: string         // 当前K线周期
  klineData: Kline[]              // K线数据
  tickers: Ticker[]               // 全市场 Ticker
  watchlist: string[]             // 自选列表
  depth: Depth | null             // 当前深度
  wsConnected: boolean            // WebSocket 连接状态
  wsSubscriptions: string[]       // 当前订阅频道
  indicators: IndicatorConfig[]   // 已叠加指标
}
```

### 7.5 前端 WebSocket 客户端设计

```typescript
// composables/useMarketWs.ts
interface UseMarketWs {
  connect: () => void
  disconnect: () => void
  subscribe: (channels: string[]) => void
  unsubscribe: (channels: string[]) => void
  onKline: (callback: (data: KlineWsMessage) => void) => void
  onTicker: (callback: (data: TickerWsMessage) => void) => void
  onDepth: (callback: (data: DepthWsMessage) => void) => void
  connectionStatus: Ref<'connecting' | 'connected' | 'disconnected' | 'reconnecting'>
}
```

**重连策略：**
- 检测到断开后启动重连
- 指数退避: 1s → 2s → 4s → 8s → 16s → 30s (最大)
- 重连成功后自动恢复之前的订阅
- 重连后请求断线期间的增量 K 线数据

---

## 8. 后端设计

### 8.1 新增模块

```
backend/src/
├── services/
│   └── market_collector.rs    # 市场
│       └── mod.rs             # 数据采集器
├── handlers/
│   └── market.rs              # 行情 REST API handlers
└── models/
    └── schemas.rs             # 新增行情相关 schemas
```

### 8.2 Market Data Collector

```rust
// services/market_collector.rs

/// 市场
pub struct MarketCollector {
    exchange: String,                              // "binance"
    ws_url: String,                                // Binance WS URL
    rest_url: String,                              // Binance REST URL
    redis: redis::Client,                          // Redis 连接
    db: Arc<DatabaseConnection>,                   // PostgreSQL 连接
    ws_connected: Arc<AtomicBool>,                 // WS 连接状态
    symbols: Vec<String>,                          // 订阅的交易对
    kline_buffer: Arc<Mutex<Vec<KlineData>>>,      // K线批量写入缓冲
}

impl MarketCollector {
    pub async fn start(&self) -> Result<()>;       // 启动采集
    async fn connect_ws(&self) -> Result<()>;      // 连接交易所 WS
    async fn fallback_rest(&self) -> Result<()>;   // REST 降级采集
    async fn normalize(&self, raw: &RawData) -> NormalizedData; // 数据规范化
    async fn write_redis(&self, data: &NormalizedData);         // 写入 Redis
    async fn flush_klines(&self);                               // 批量写入 PG
    pub fn status(&self) -> CollectorStatus;                    // 健康状态
}
```

### 8.3 WebSocket Hub

```rust
// 在现有 ws.rs 基础上扩展

pub struct MarketWsHub {
    /// 频道 → 订阅者集合
    channels: DashMap<String, HashSet<String>>,
    /// 用户会话 → 发送端
    sessions: DashMap<String, mpsc::Sender<String>>,
    /// Redis 订阅端
    redis_sub: redis::aio::PubSub,
}

impl MarketWsHub {
    pub async fn handle_subscribe(&self, session_id: &str, channels: &[String]);
    pub async fn handle_unsubscribe(&self, session_id: &str, channels: &[String]);
    pub async fn broadcast_to_channel(&self, channel: &str, message: &str);
    pub async fn on_redis_message(&self, channel: &str, data: &str);
}
```

### 8.4 数据流

```
[Binance WebSocket API]
    ↓ 原始数据
[MarketCollector: 解析 + 规范化 + 去重]
    ↓ 统一格式
[Redis: 写入缓存 + PubSub 广播]
    ├─→ Redis Hash (ticker:BTC/USDT)     ← REST API 读取
    ├─→ Redis String (depth:BTC/USDT)    ← REST API 读取
    ├─→ Redis List (kline:BTC/USDT:1m)   ← REST API 读取
    └─→ Redis PubSub (market:*)          ← WS Hub 订阅
          ↓
    [MarketWsHub: 接收 PubSub 消息]
          ↓
    [分发至订阅了对应频道的客户端]
```

### 8.5 降级策略

| 故障场景 | 降级行为 | 恢复条件 |
|----------|---------|---------|
| Binance WS 断开 | 退避重连 + 切换 REST 轮询 | WS 重连成功 |
| Binance API 429 限流 | 退避重试 + 使用 Redis 缓存 | 限流解除 |
| Redis 不可用 | 缓存降级为内存 HashMap | Redis 恢复 |
| PostgreSQL 不可用 | 实时行情正常推送，K线持久化暂停 | PG 恢复后补写 |
| MarketCollector 崩溃 | 自动重启，从 Redis 缓存恢复状态 | 进程重启 |

---

## 9. 边界情况

| 边界情况 | 处理方式 |
|----------|---------|
| 交易所维护/停机 | Collector 检测到连接失败，切换 REST 降级，admin 看到降级状态 |
| 交易对下架 | Ticker/Depth 返回空数据，前端显示 "该交易对已下架" |
| K线数据量巨大 (百万级) | REST API 强制分页 (limit ≤ 1500)，前端按需加载 |
| WebSocket 连接数超过 1000 | 拒绝新连接，返回 503 + "系统繁忙，请稍后重试" |
| 客户端恶意订阅大量频道 | 单连接订阅上限 20 个频道，超出拒绝 |
| 客户端发送非法消息 | 返回 error 消息，连续 5 次非法消息断开连接 |
| 交易所数据延迟 (> 5s) | Collector 标记数据为 "stale"，前端显示延迟警告 |
| 交易所返回异常价格 (如闪崩) | 数据照常推送，前端K线图自动缩放适应 |
| 多标签页同时打开 | 每个标签页独立 WebSocket 连接，独立订阅 |
| 用户切换交易对 | 取消旧频道订阅，订阅新频道，K线数据从缓存/REST 加载 |
| 深度数据量极大 (> 50 档) | 默认 20 档，用户可选 10/20/50 档 |
| 自选列表跨设备同步 | 通过后端 API 持久化，多设备自动同步 |

---

## 10. 非功能性需求

### 10.1 性能

| 指标 | 目标值 |
|------|--------|
| K线首屏加载 | < 1s (500 根) |
| Ticker 列表加载 | < 500ms |
| 深度数据加载 | < 300ms |
| WebSocket 推送延迟 | < 500ms (端到端) |
| WebSocket 并发连接 | ≥ 1000 |
| Redis 缓存 Ticker 命中率 | ≥ 95% |
| API 响应 P99 | < 200ms |

### 10.2 可靠性

| 要求 | 说明 |
|------|------|
| 断线重连 | 指数退避，最大 30s，自动恢复订阅 |
| 数据补发 | 重连后补发断线期间 K 线增量 |
| 降级 | 交易所 WS 断开自动切换 REST |
| 心跳 | 15s 服务端心跳，30s 无响应断开 |
| 幂等 | 订阅/取消订阅操作幂等 |

### 10.3 安全

| 要求 | 说明 |
|------|------|
| WS 鉴权 | JWT token 通过 query param 传递 |
| 限流 | 每用户 100 req/min (REST)，20 频道/连接 (WS) |
| CORS | 仅允许前端域名 |
| 数据脱敏 | 行情数据为公共数据，无需脱敏 |

---

## 11. 优先级矩阵

| 用户故事 | 优先级 | 说明 |
|----------|--------|------|
| US-MK-01: 实时K线图 | P0 | 核心功能，行情页面基石 |
| US-MK-02: 指标叠加 | P0 | 分析必备，无指标则K线价值减半 |
| US-MK-03: Ticker 行情概览 | P0 | 市场全貌，用户入口 |
| US-MK-05: WebSocket 实时推送 | P0 | 实时性基础，支撑所有实时功能 |
| US-MK-06: 市场数据采集管道 | P0 | 数据源头，无采集则无数据 |
| US-MK-07: 行情页面布局 | P0 | 用户体验，整合三大功能 |
| US-MK-04: 深度数据与盘口 | P1 | 专业功能，非 MVP 必须 |
| US-MK-08: 自选列表管理 | P1 | 个性化功能，提升效率 |

**总计: P0 x 6 / P1 x 2**

---

## 12. 实施分阶段建议

### Phase 1: 基础架构 (1-2 周)

1. 部署 Redis，配置缓存 Key 和 PubSub 频道
2. 实现 `MarketCollector` (Binance WS 连接 + REST 降级)
3. 实现数据规范化 + Redis 写入 + PostgreSQL 批量持久化
4. 扩展 `handlers/ws.rs` 为 `MarketWsHub` (订阅/取消/广播)
5. 实现 `handlers/market.rs` (GET /kline, GET /tickers)

### Phase 2: 前端核心 (1-2 周)

1. 实现 `MarketView.vue` 三栏布局
2. 实现 `CandlestickChart.vue` (Lightweight-charts)
3. 实现 `PeriodSelector.vue` 和 `IndicatorPanel.vue`
4. 实现 `useMarketWs.ts` composable (连接/订阅/重连)
5. 实现 `MarketSidebar.vue` (Ticker 列表 + 搜索)

### Phase 3: 完善功能 (1 周)

1. 实现 `DepthPanel.vue` (盘口 + 深度图)
2. 实现自选列表 API + 前端
3. 实现指标计算 (MA/BOLL/MACD/RSI/KDJ)
4. 实现降级和边界情况处理
5. 性能测试和优化

---

## 13. 关联文档

| 文档 | 关系 |
|------|------|
| ADR-002 | WebSocket 技术选型决策 |
| ADR-005 | 市场数据管道架构决策 |
| PRD.md US-04/05/06 | 原始用户故事定义 |
| PRD-kline-management.md | K线数据管理 (互补，不重叠) |
| data-model.md | 数据库 schema 定义 |
| TECH_CHARTER.md | 技术栈和开发规范 |
