# Language: zh-CN
# 行情模块 — 深度数据与实时Ticker
# 关联 PRD: PRD-market-depth-ticker.md
# User Stories: US-MD-01 ~ US-MD-08

Feature: 行情模块 — 深度数据与实时Ticker

  # ==========================================
  # US-MD-01: Ticker 列表 REST 查询 (P0)
  # ==========================================

  Feature: Ticker 列表 REST 查询

    Background:
      Given 用户已登录
        And Redis 中存在多个交易对的 Ticker 缓存

    Scenario: 获取所有交易对 Ticker
      When GET /api/v1/market/tickers
      Then 返回 200 状态码
        And 返回所有交易对的 Ticker 数组
        And 每条 Ticker 包含: symbol, price, change, change_percent, volume, high, low, bid, ask, timestamp
        And 响应时间 P99 < 200ms

    Scenario: 获取单个交易对 Ticker
      When GET /api/v1/market/ticker?symbol=BTCUSDT
      Then 返回 200 状态码
        And 返回 BTCUSDT 的 Ticker 数据
        And 包含 bid 和 ask 字段

    Scenario: 交易对不存在
      When GET /api/v1/market/ticker?symbol=INVALID99
      Then 返回 404 状态码
        And 错误提示 "交易对不存在"

    Scenario: Redis 缓存命中时直接返回
      Given Redis 中存在 symbol=BTCUSDT 的 Ticker 缓存
      When GET /api/v1/market/ticker?symbol=BTCUSDT
      Then 直接从 Redis 返回数据
        And 不查询数据库

    Scenario: Redis 缓存未命中时降级获取
      Given Redis 中不存在 symbol=NEWUSDT 的 Ticker 缓存
      When GET /api/v1/market/ticker?symbol=NEWUSDT
      Then 从 Collector 获取最新数据
        And 写入 Redis 缓存 TTL=5s
        And 返回数据

  # ==========================================
  # US-MD-02: 深度数据 REST 查询 (P0)
  # ==========================================

  Feature: 深度数据 REST 查询

    Background:
      Given 用户已登录
        And Redis 中存在 symbol=BTCUSDT 的深度缓存

    Scenario: 获取默认档位深度
      When GET /api/v1/market/depth?symbol=BTCUSDT
      Then 返回 200 状态码
        And 返回 10 档买盘和 10 档卖盘
        And 每档包含: price, quantity, total
        And 买盘按价格降序排列
        And 卖盘按价格升序排列
        And 包含 timestamp 字段
        And 响应时间 P99 < 100ms

    Scenario: 指定档位数量
      When GET /api/v1/market/depth?symbol=BTCUSDT&levels=20
      Then 返回 20 档买盘和 20 档卖盘

    Scenario: pro-trader 请求 50 档
      Given 当前用户角色为 pro-trader
      When GET /api/v1/market/depth?symbol=BTCUSDT&levels=50
      Then 返回 50 档买盘和 50 档卖盘

    Scenario: trader 请求超过 20 档被拒绝
      Given 当前用户角色为 trader
      When GET /api/v1/market/depth?symbol=BTCUSDT&levels=50
      Then 返回 403 状态码
        And 提示 "当前角色仅支持 20 档深度，升级至 pro-trader 可查看 50 档"

    Scenario: 无效档位参数
      When GET /api/v1/market/depth?symbol=BTCUSDT&levels=0
      Then 返回 400 状态码
        And 提示 "levels 参数必须在 5/10/20/50 中选择"

    Scenario: Redis 缓存命中时直接返回
      Given Redis 中存在 depth:BTCUSDT 缓存
      When GET /api/v1/market/depth?symbol=BTCUSDT
      Then 直接从 Redis 返回数据

  # ==========================================
  # US-MD-03: WebSocket Ticker 实时推送 (P0)
  # ==========================================

  Feature: WebSocket Ticker 实时推送

    Background:
      Given 用户已通过 JWT 鉴权建立 WebSocket 连接
        And 连接地址为 wss://host/api/v1/ws?token=***

    Scenario: 订阅 Ticker 频道
      When 客户端发送 {"action":"subscribe","channels":["ticker:BTCUSDT"]}
      Then 服务端返回 {"type":"subscribed","channel":"ticker:BTCUSDT"}
        And 后续 BTCUSDT 的 Ticker 更新通过 WS 推送

    Scenario: Ticker 推送格式
      Given 已订阅 ticker:BTCUSDT
      When BTCUSDT 价格发生变化
      Then 服务端推送包含 type="ticker" symbol="BTCUSDT" 的消息
        And data 中包含 price, change, change_percent, volume, high, low, bid, ask, timestamp
        And 推送延迟 < 500ms

    Scenario: 取消订阅 Ticker
      Given 已订阅 ticker:BTCUSDT
      When 客户端发送 {"action":"unsubscribe","channels":["ticker:BTCUSDT"]}
      Then 服务端返回 {"type":"unsubscribed","channel":"ticker:BTCUSDT"}
        And 不再推送 BTCUSDT Ticker 数据

    Scenario: 订阅多个交易对 Ticker
      When 客户端发送 {"action":"subscribe","channels":["ticker:BTCUSDT","ticker:ETHUSDT"]}
      Then 服务端返回两个频道的订阅确认
        And 两个交易对的 Ticker 更新均推送

    Scenario: Ticker 推送频率限制
      Given 已订阅 ticker:BTCUSDT
      When 1 秒内 BTCUSDT Ticker 更新 10 次
      Then 服务端最多推送 4 次
        And 每次推送包含最新状态

    Scenario: 未鉴权连接被拒绝
      When WebSocket 连接不携带 token 参数
      Then 连接被拒绝
        And 返回 401 错误

  # ==========================================
  # US-MD-04: WebSocket 深度数据实时推送 (P0)
  # ==========================================

  Feature: WebSocket 深度数据实时推送

    Background:
      Given 用户已通过 JWT 鉴权建立 WebSocket 连接

    Scenario: 订阅深度频道
      When 客户端发送 {"action":"subscribe","channels":["depth:BTCUSDT"]}
      Then 服务端返回 {"type":"subscribed","channel":"depth:BTCUSDT"}
        And 立即推送当前完整深度快照

    Scenario: 首次推送完整快照
      Given 已订阅 depth:BTCUSDT
      When 订阅成功后
      Then 服务端推送完整深度数据
        And type 为 "depth"
        And data 中包含 bids 和 asks 完整数组

    Scenario: 增量深度更新
      Given 已订阅 depth:BTCUSDT 且已收到完整快照
      When 盘口发生变化
      Then 服务端推送 type="depth_update" 的增量消息
        And 客户端本地合并增量到快照

    Scenario: 深度推送频率限制
      Given 已订阅 depth:BTCUSDT
      When 100ms 内盘口变化 20 次
      Then 服务端最多推送 2 次
        And 每次推送包含合并后的最新增量

    Scenario: 取消订阅深度
      Given 已订阅 depth:BTCUSDT
      When 客户端发送 {"action":"unsubscribe","channels":["depth:BTCUSDT"]}
      Then 服务端返回取消确认
        And 停止推送

  # ==========================================
  # US-MD-05: Ticker 面板前端展示 (P0)
  # ==========================================

  Feature: Ticker 面板前端展示

    Background:
      Given 用户已登录并进入 /market 页面

    Scenario: Ticker 列表加载
      When 页面首次加载
      Then 调用 GET /api/v1/market/tickers 获取初始数据
        And 显示 Ticker 列表表格
        And 每行包含: 交易对、最新价、24h涨跌幅、24h最高/最低、24h成交量
        And 涨跌幅为正显示绿色，为负显示红色

    Scenario: Ticker 实时更新
      Given 页面已加载
      When 建立 WebSocket 连接并订阅所有已展示的 Ticker 频道
      Then 价格变化时实时更新表格数据
        And 无需刷新页面

    Scenario: 价格上涨时绿色闪烁
      Given Ticker 面板已打开
      When 最新价上涨
      Then 价格数字绿色闪烁
        And 0.5s 后恢复正常颜色

    Scenario: 价格下跌时红色闪烁
      Given Ticker 面板已打开
      When 最新价下跌
      Then 价格数字红色闪烁
        And 0.5s 后恢复正常颜色

    Scenario: 按涨跌幅排序
      When 用户点击"24h涨跌幅"列头
      Then 列表按涨跌幅降序排列
      When 再次点击
      Then 列表按涨跌幅升序排列

    Scenario: 搜索交易对
      When 用户在搜索框输入 "BTC"
      Then 列表仅显示包含 "BTC" 的交易对
        And 实时过滤 debounce 300ms

    Scenario: WebSocket 断线重连
      When WebSocket 连接断开
      Then 前端自动重连 指数退避 1s到30s
        And 重连期间显示"连接中断"提示
        And 重连成功后重新订阅所有频道
        And Ticker 数据恢复实时更新

  # ==========================================
  # US-MD-06: 深度盘口前端展示 (P0)
  # ==========================================

  Feature: 深度盘口前端展示

    Background:
      Given 用户已登录并进入 /market 页面
        And 已选择交易对 "BTCUSDT"

    Scenario: 加载深度数据
      When 点击"深度"标签
      Then 调用 GET /api/v1/market/depth?symbol=BTCUSDT 获取初始数据
        And 显示买盘列表 右侧绿色 买一至买十
        And 显示卖盘列表 左侧红色 卖一至卖十
        And 每行显示 价格、数量、累计量
        And 累计量从最优价开始累加

    Scenario: 深度实时更新
      Given 深度面板已打开
      When 建立 WebSocket 连接并订阅 depth:BTCUSDT
      Then 盘口变化时实时更新
        And 深度数据平滑过渡 无闪烁

    Scenario: 深度图可视化
      Given 深度数据已加载
      Then 显示深度面积图 ECharts
        And 横轴为价格
        And 纵轴为累计量
        And 买盘为绿色面积 卖盘为红色面积
        And 中间为最新成交价分隔线

    Scenario: 档位切换
      When 用户选择档位下拉框切换为 "20档"
      Then 重新请求 GET /api/v1/market/depth?symbol=BTCUSDT&levels=20
        And 更新盘口和深度图

    Scenario: 深度图悬浮交互
      When 鼠标悬浮在深度图上
      Then 显示 tooltip 包含价格、累计量、买卖方向
        And 对应价格行高亮

    Scenario: 盘口数据精度
      Given BTCUSDT 最小价格变动为 0.01
      Then 盘口价格显示精度为 2 位小数
        And 数量显示精度为 3 位小数
        And 累计量显示精度为 3 位小数

  # ==========================================
  # US-MD-07: 心跳与连接管理 (P1)
  # ==========================================

  Feature: WebSocket 心跳与连接管理

    Scenario: 服务端心跳
      Given WebSocket 连接已建立
      When 每 15 秒
      Then 服务端发送 {"type":"heartbeat","ts":1715500000000}

    Scenario: 客户端超时断开
      Given WebSocket 连接已建立
      When 服务端 30 秒未收到客户端任何消息
      Then 服务端主动断开连接
        And 释放资源

    Scenario: 客户端心跳响应
      Given WebSocket 连接已建立
      When 客户端收到 heartbeat 消息
      Then 客户端发送 {"type":"pong"} 响应

    Scenario: 单用户连接数限制
      Given 单个用户
      When 同一用户建立超过 5 个 WebSocket 连接
      Then 最早建立的连接被服务端断开
        And 返回 {"type":"kick","reason":"max_connections_exceeded"}

  # ==========================================
  # US-MD-08: Ticker 快照持久化 (P1)
  # ==========================================

  Feature: Ticker 快照持久化

    Scenario: 定时快照写入
      Given 系统 Collector 正在运行
      When 每 1 分钟
      Then 系统将所有交易对的 Ticker 快照写入 ticker_snapshots 表
        And 包含 symbol, price, change, change_percent, volume, high, low, bid, ask, timestamp

    Scenario: 快照数据自动过期
      Given ticker_snapshots 表中存在超过 90 天的数据
      When 系统执行数据清理
      Then 自动删除过期数据

    Scenario: 历史快照查询
      When GET /api/v1/market/ticker/history?symbol=BTCUSDT&start=1715500000000&end=1718179200000
      Then 返回该时间范围内的 Ticker 快照
        And 支持分页
