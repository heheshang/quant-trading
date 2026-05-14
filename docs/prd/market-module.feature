# 语言: zh-CN
# 行情模块 Gherkin 验收条件
# 关联: PRD-market-module.md

Feature: 实时K线图展示 (US-MK-01)
  用户查看实时K线图，分析价格走势

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
    And 图表自动追加新 K 线
    And 当前未收盘 K 线实时更新

  Scenario: K线图交互
    Given K线图已渲染
    Then 支持鼠标滚轮缩放
    And 支持鼠标拖拽平移
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

---

Feature: K线指标叠加 (US-MK-02)
  用户在K线图上叠加技术指标进行分析

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
    And 副图区域显示 MACD 指标
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

---

Feature: Ticker 行情概览 (US-MK-03)
  用户查看全市场 Ticker 概览

  Background:
    Given 用户已登录

  Scenario: Ticker 列表显示
    Given 用户导航至行情页面
    When Ticker 面板加载
    Then 显示所有交易对 Ticker 列表
    And 每行包含: 交易对名称、最新价、24h涨跌幅、24h最高、24h最低、24h成交量
    And 默认按 24h 成交量降序排列
    And 数据通过 REST API 初始加载
    And 后续通过 WebSocket 增量更新

  Scenario: Ticker 价格闪烁
    Given Ticker 面板已打开
    When 最新价发生变化
    Then 价格上涨时数字绿色闪烁
    And 价格下跌时数字红色闪烁
    And 0.5s 后恢复正常颜色

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
    And 搜索为前端本地过滤
    And 清空搜索框恢复完整列表

  Scenario: 自选列表添加
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

  Scenario: Ticker 面板与K线联动
    Given Ticker 列表已显示
    When 用户点击某个交易对行
    Then K线图切换至该交易对
    And Ticker 行高亮显示当前选中

---

Feature: 深度数据与盘口 (US-MK-04)
  用户查看市场深度和盘口数据

  Background:
    Given 用户已登录
    And 已选择交易对 "BTC/USDT"

  Scenario: 加载深度数据
    Given 用户点击深度标签
    When 深度面板加载
    Then 显示买一至买二十档位
    And 显示卖一至卖二十档位
    And 买单价格降序排列
    And 卖单价格升序排列
    And 买卖盘之间显示当前最新价和价差

  Scenario: 深度图可视化
    Given 深度面板已加载
    When 用户查看深度图区域
    Then 显示深度图
    And 买单累计线为绿色
    And 卖单累计线为红色
    And 两线交叉处标注当前价格

  Scenario: 深度实时更新
    Given 深度面板已打开
    When 市场盘口发生变化
    Then 深度数据通过 WebSocket 实时更新
    And 深度图平滑过渡动画
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

---

Feature: WebSocket 实时推送 (US-MK-05)
  系统通过 WebSocket 实时推送行情数据

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
      | action     | channels                                              |
      | subscribe  | ["kline:BTC/USDT:1m", "ticker:BTC/USDT", "depth:BTC/USDT"] |
    Then 服务端确认订阅
    And 客户端开始接收对应频道的推送数据

  Scenario: 取消订阅
    Given 已订阅 "kline:BTC/USDT:1m" 频道
    When 客户端发送取消订阅消息:
      | action      | channels                |
      | unsubscribe | ["kline:BTC/USDT:1m"]   |
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
    And 自动重连，指数退避 1s 到 30s
    And 重连成功后自动恢复之前的订阅
    And 补发断线期间的增量 K 线数据
    And 页面显示 "已重连" 提示，3s 后消失

  Scenario: JWT 过期处理
    Given WebSocket 连接已建立
    When JWT access_token 过期
    Then 客户端使用 refresh_token 获取新 access_token
    And 使用新 token 重新建立 WebSocket 连接
    And 自动恢复订阅

  Scenario: 推送消息格式
    Given 已订阅 "ticker:BTC/USDT" 频道
    When 服务端推送数据
    Then 消息包含 type、symbol、data、ts 字段
    And type 为 "ticker"
    And symbol 为 "BTC/USDT"
    And data 包含 last, bid, ask, volume_24h, change_24h, high_24h, low_24h, timestamp

---

Feature: 市场数据采集管道 (US-MK-06)
  系统从交易所采集实时行情数据

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
    And 轮询间隔 kline 5s, ticker 2s, depth 2s
    And 数据写入 Redis 缓存

  Scenario: 交易所 API 限流
    Given Binance API 返回 429
    When Collector 检测到限流
    Then 自动退避重试
    And 降级使用 Redis 缓存数据
    And 记录告警日志

  Scenario: 数据规范化
    Given 从 Binance 接收到原始 K 线数据
    When Collector 处理数据
    Then 转换为内部统一格式
    And 包含 exchange, symbol, type, data 字段

  Scenario: 历史数据持久化
    Given 实时 K 线数据已规范化
    When 数据写入 Redis 缓存
    Then 同时批量写入 PostgreSQL klines 表
    And 批量写入间隔每 5 秒 flush 一次
    And 写入失败不影响实时推送

  Scenario: Collector 健康检查
    Given Market Data Collector 运行中
    When GET /api/v1/market/collector/status
    Then 返回 status, exchange, ws_connected, last_heartbeat, symbols_count, messages_per_second

---

Feature: 行情页面布局 (US-MK-07)
  用户在一个页面内同时查看K线、Ticker、深度

  Background:
    Given 用户已登录

  Scenario: 行情页面三栏布局
    Given 用户导航至 /market 页面
    When 页面加载完成
    Then 显示三栏布局: 左侧 Ticker 列表、中间 K线主图、右侧深度/信息面板
    And 左侧面板宽度 240px
    And 右侧面板宽度 320px
    And 中间区域自适应填充

  Scenario: 左侧面板折叠
    Given 行情页面已加载
    When 用户点击左侧面板折叠按钮
    Then Ticker 列表收起为图标列
    And K线主图自动扩展占用空间

  Scenario: 右侧面板 Tab 切换
    Given 右侧面板已显示
    Then 包含深度和信息两个 Tab
    And 点击 Tab 切换显示内容

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

---

Feature: 自选列表管理 (US-MK-08)
  用户管理自选交易对列表

  Background:
    Given 用户已登录

  Scenario: 添加自选
    Given 用户在 Ticker 列表中
    When 点击 "BTC/USDT" 旁的星标
    Then "BTC/USDT" 添加到自选列表
    And 操作同步到后端
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
