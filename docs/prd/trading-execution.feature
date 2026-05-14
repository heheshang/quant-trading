# 交易执行模块 — Gherkin 验收条件
# 对应 PRD: PRD-trading-execution.md
# 覆盖: US-TE-01 ~ US-TE-10

Feature: 限价单下单 (US-TE-01)

  Background:
    Given 用户已登录
      And 用户角色为 "trader"
      And 模拟账户余额充足

  Scenario: 成功提交限价买单
    Given 交易页面已打开
      And 当前交易对为 "BTC/USDT"
      And 当前卖一价为 ¥50,000
    When 用户选择委托类型 "限价单"
      And 输入价格 ¥49,500
      And 输入数量 0.1 BTC
      And 选择方向 "买入"
      And 点击 "提交委托"
    Then 弹出确认对话框，显示: 交易对 BTC/USDT、方向 买入、类型 限价、价格 ¥49,500、数量 0.1 BTC、总金额 ¥4,950
      And 用户点击 "确认" 后
      And 委托提交成功
      And 系统返回 201
      And 委托出现在 "当前委托" 列表中
      And 委托状态为 "待成交"
      And 模拟账户冻结 ¥4,950 保证金

  Scenario: 成功提交限价卖单
    Given 用户持有 0.5 BTC
      And 当前买一价为 ¥50,000
    When 用户选择方向 "卖出"
      And 输入价格 ¥51,000
      And 输入数量 0.1 BTC
      And 点击 "提交委托" 并确认
    Then 委托提交成功
      And 委托状态为 "待成交"
      And 持仓中冻结 0.1 BTC

  Scenario: 限价单部分成交
    Given 存在状态为 "待成交" 的限价买单 (价格 ¥49,500, 数量 0.1 BTC)
      And 模拟撮合引擎检测到市场卖一价 ≤ ¥49,500
    When 撮合引擎匹配
    Then 委托状态变为 "部分成交"
      And 已成交数量更新
      And 通过 WebSocket 推送成交通知

  Scenario: 限价单完全成交
    Given 存在状态为 "部分成交" 的限价买单
      And 剩余数量已全部撮合
    Then 委托状态变为 "已成交"
      And 冻结保证金释放或结算
      And 持仓数量更新
      And 委托从 "当前委托" 移至 "历史委托"

  Scenario: 限价单价格等于市场价时立即撮合
    Given 当前卖一价为 ¥50,000
    When 用户提交限价买单，价格 ¥50,000
    Then 撮合引擎立即匹配
      And 委托直接进入 "已成交" 状态

  Scenario: 限价单价格偏离过大时提示
    Given 当前市场价为 ¥50,000
    When 用户输入限价买单价格 ¥100,000
    Then 弹出风险提示 "委托价格偏离当前市场价较大 (100%)，是否确认提交？"

  Scenario: 卖出数量超过持仓
    Given 用户持有 0.3 BTC
    When 用户提交限价卖单，数量 0.5 BTC
    Then 提示 "卖出数量超过可用持仓 (0.3 BTC)"
      And 提交按钮禁用

Feature: 市价单下单 (US-TE-02)

  Background:
    Given 用户已登录
      And 用户角色为 "trader"

  Scenario: 成功提交市价买单
    Given 交易页面已打开
      And 当前交易对为 "BTC/USDT"
      And 模拟账户可用余额 ¥10,000
      And 当前卖一价为 ¥50,000
    When 用户选择委托类型 "市价单"
      And 输入数量 0.1 BTC
      And 选择方向 "买入"
      And 点击 "提交委托" 并确认
    Then 委托提交至模拟撮合引擎
      And 按当前卖一价撮合成交
      And 委托状态为 "已成交"
      And 成交记录出现在 "历史委托" 列表
      And 持仓更新

  Scenario: 成功提交市价卖单
    Given 用户持有 0.5 BTC
      And 当前买一价为 ¥50,000
    When 用户选择方向 "卖出" 和委托类型 "市价单"
      And 输入数量 0.1 BTC 并确认
    Then 按当前买一价撮合成交
      And 委托状态为 "已成交"
      And 持仓减少 0.1 BTC

  Scenario: 市价单深度不足时部分成交
    Given 当前卖一至卖五总量为 0.05 BTC
    When 用户提交市价买单，数量 0.1 BTC
    Then 先按卖一至卖五价成交 0.05 BTC
      And 委托状态为 "部分成交"
      And 剩余 0.05 BTC 继续等待新卖单

  Scenario: 市价单无对手盘
    Given 当前深度数据为空 (无卖单)
    When 用户提交市价买单
    Then 提示 "当前市场无对手盘，市价单无法成交"
      And 委托不提交

  Scenario: 市价单不显示价格输入框
    Given 下单表单
    When 用户选择委托类型 "市价单"
    Then 价格输入框隐藏
      And 显示当前买一或卖一价参考

Feature: 下单风控校验 (US-TE-03)

  Background:
    Given 用户已登录

  Scenario: 余额不足拦截
    Given 用户模拟账户可用余额 ¥1,000
    When 用户提交限价买单，总金额 ¥10,000
    Then 系统返回 400
      And 错误码为 40001
      And 提示 "账户余额不足"

  Scenario: 卖出数量超过持仓拦截
    Given 用户持有 0.1 BTC
    When 用户提交卖单，数量 0.5 BTC
    Then 系统返回 400
      And 错误码为 40002
      And 提示 "可用持仓不足"

  Scenario: 交易对不支持
    Given 系统支持的交易对为 ["BTC/USDT", "ETH/USDT", "SOL/USDT"]
    When 用户提交委托，交易对为 "DOGE/USDT"
    Then 系统返回 400
      And 错误码为 40003

  Scenario: 数量为零或负数
    Given 下单表单
    When 用户输入数量 0 或 -1
    Then 前端实时校验，提示 "数量必须大于0"
      And 提交按钮禁用

  Scenario: 价格为零或负数
    Given 下单表单，委托类型为 "限价单"
    When 用户输入价格 0 或 -100
    Then 前端实时校验，提示 "价格必须大于0"
      And 提交按钮禁用

  Scenario: 未成交委托数量上限
    Given 用户当前有 50 个未成交委托
    When 用户提交新委托
    Then 系统返回 429
      And 错误码为 42901

  Scenario: trader 角色无法提交实盘委托
    Given 用户角色为 "trader"
    When 委托 mode 为 "live"
    Then 系统返回 403
      And 错误码为 40301

  Scenario: 最小下单数量校验
    Given 交易对 "BTC/USDT" 最小下单量为 0.001 BTC
    When 用户输入数量 0.0001 BTC
    Then 前端实时校验，提示 "最小下单数量为 0.001 BTC"

  Scenario: 价格精度校验
    Given 交易对 "BTC/USDT" 价格精度为 2 位小数
    When 用户输入价格 ¥50,000.123
    Then 前端实时校验，提示 "价格精度不超过 2 位小数"

  Scenario: 数量精度校验
    Given 交易对 "BTC/USDT" 数量精度为 4 位小数
    When 用户输入数量 0.12345
    Then 前端实时校验，提示 "数量精度不超过 4 位小数"

Feature: 查看当前委托 (US-TE-04)

  Background:
    Given 用户已登录

  Scenario: 当前委托列表显示
    Given 用户有 3 个未成交委托
    When 打开 "当前委托" 页面
    Then 显示所有未完全成交的委托
      And 每行包含: 时间、交易对、方向、类型、价格、数量、已成交、状态、操作
      And 默认按时间降序排列

  Scenario: 当前委托实时更新
    Given 当前委托页面已打开
    When 某委托状态发生变化
    Then 通过 WebSocket 推送状态变更
      And 列表实时更新

  Scenario: 按交易对筛选
    Given 当前委托列表已显示
    When 用户在筛选框选择 "BTC/USDT"
    Then 列表仅显示 BTC/USDT 的委托

  Scenario: 无未成交委托
    Given 用户无未成交委托
    When 打开 "当前委托" 页面
    Then 显示空状态 "暂无未成交委托"

Feature: 撤单 (US-TE-05)

  Background:
    Given 用户已登录

  Scenario: 撤销待成交委托
    Given 存在状态为 "待成交" 的委托
    When 用户点击该委托的 "撤单" 按钮并确认
    Then 委托状态变为 "已撤销"
      And 冻结保证金释放回可用余额
      And WebSocket 推送委托状态变更

  Scenario: 撤销部分成交委托
    Given 存在状态为 "部分成交" 的委托 (已成交 0.05 BTC，剩余 0.05 BTC)
    When 用户点击 "撤单"
    Then 委托状态变为 "已撤销"
      And 已成交的 0.05 BTC 保留
      And 未成交部分的冻结保证金释放

  Scenario: 已成交委托无法撤单
    Given 存在状态为 "已成交" 的委托
    When 用户查看该委托
    Then "撤单" 按钮不显示或禁用

  Scenario: 撤单并发竞争
    Given 委托状态为 "待成交" 且撮合引擎正在处理
    When 用户同时点击撤单
    Then 如果委托已成交，撤单失败并提示 "委托已成交，无法撤销"
      And 如果委托尚未成交，撤单成功

  Scenario: 批量撤单
    Given 用户有 5 个未成交委托
    When 用户点击 "全部撤单" 按钮并确认
    Then 所有未成交委托依次撤销
      And 显示撤单结果统计

Feature: 查看历史委托 (US-TE-06)

  Background:
    Given 用户已登录

  Scenario: 历史委托列表显示
    Given 用户有 20 条历史委托记录
    When 打开 "历史委托" 页面
    Then 显示所有已成交或已撤销或已过期的委托
      And 分页显示 (每页 20 条)

  Scenario: 按日期范围筛选
    Given 历史委托列表已显示
    When 用户选择日期范围 "2026-05-01" 至 "2026-05-14"
    Then 列表仅显示该时间范围内的委托

  Scenario: 按状态筛选
    Given 历史委托列表已显示
    When 用户选择状态 "已成交"
    Then 列表仅显示已成交的委托

  Scenario: 委托详情查看
    Given 历史委托列表已显示
    When 用户点击某条委托
    Then 展开详情面板，显示委托ID、成交时间、手续费、成交明细

Feature: 模拟撮合引擎 (US-TE-07)

  Background:
    Given 系统已连接到市场数据源
      And 深度数据通过 WebSocket 实时更新

  Scenario: 限价买单撮合
    Given 存在待成交限价买单 (价格 ¥49,500，数量 0.1 BTC)
      And 市场卖一价为 ¥49,500
    When 深度数据更新，卖一价降至 ¥49,500 或更低
    Then 撮合引擎匹配该委托
      And 委托状态更新为 "已成交"

  Scenario: 市价买单多档撮合
    Given 用户提交市价买单 (数量 0.1 BTC)
      And 当前深度数据:
        | 档位 | 价格  | 数量    |
        | 卖一 | 50000 | 0.03 BTC |
        | 卖二 | 50010 | 0.05 BTC |
        | 卖三 | 50020 | 0.10 BTC |
    When 撮合引擎处理
    Then 按卖一价成交 0.03 BTC
      And 按卖二价成交 0.05 BTC
      And 按卖三价成交 0.02 BTC
      And 成交均价为 ¥50,006.4

  Scenario: 深度不足时部分撮合
    Given 存在待成交限价买单 (数量 0.5 BTC)
      And 累计可撮合数量为 0.3 BTC
    When 撮合引擎处理
    Then 成交 0.3 BTC
      And 委托状态为 "部分成交"

  Scenario: 委托过期
    Given 存在待成交限价委托，创建时间为 24h 前
      And 委托设置了 GTC 为 24h
    When 系统定时检查过期委托
    Then 委托状态变为 "已过期"
      And 冻结保证金释放

  Scenario: 行情数据断开时限价单正常挂单
    Given 行情数据源断开
      And 深度数据不可用
    When 用户提交限价单
    Then 限价单正常提交并进入 "待成交" 状态
      And 待深度数据恢复后撮合

  Scenario: 行情数据断开时市价单拒绝
    Given 行情数据源断开
    When 用户提交市价单
    Then 提示 "行情数据暂不可用，无法提交市价单"

Feature: 交易页面布局与交互 (US-TE-08)

  Background:
    Given 用户已登录

  Scenario: 交易页面三栏布局
    When 用户导航至 /trade 页面
    Then 页面显示三栏布局: 左侧图表区、中间下单区、右侧委托区
      And 三栏可拖拽调整宽度 (最小宽度 300px)

  Scenario: 下单表单与行情联动
    Given 交易页面已打开
      And 当前卖一价为 ¥50,100
    When 用户切换方向为 "买入"
    Then 价格输入框默认填入卖一价 ¥50,100

  Scenario: 交易对切换
    Given 交易页面已打开，当前为 "BTC/USDT"
    When 用户切换至 "ETH/USDT"
    Then K线图和深度数据切换至 ETH/USDT
      And 下单表单重置

  Scenario: 数量滑块
    Given 用户可用余额 ¥10,000 且当前价格为 ¥50,000
    When 用户点击 "25%" 滑块
    Then 数量输入框填入 0.05 BTC

  Scenario: 交易模式标识
    Given 当前为模拟交易模式
    Then 交易页面顶部显示 "模拟交易" 标签 (蓝色)

Feature: 委托 WebSocket 实时推送 (US-TE-09)

  Background:
    Given 用户已登录
      And WebSocket 已连接

  Scenario: 订阅委托频道
    Given 用户已连接 WebSocket
    When 用户发送订阅消息 {"action": "subscribe", "channels": ["order:all"]}
    Then 服务端返回订阅确认
      And 后续该用户的所有委托状态变更通过 WebSocket 推送

  Scenario: 委托状态变更推送
    Given 用户已订阅 "order:all" 频道
    When 委托 #123 状态从 "待成交" 变为 "部分成交"
    Then WebSocket 推送 type 为 "order_update" 的消息
      And 包含 order_id、status、filled_quantity 等字段

  Scenario: 成交通知推送
    Given 用户已订阅 "order:all" 频道
    When 委托完全成交
    Then WebSocket 推送 type 为 "trade" 的消息
      And 包含 trade_id、price、quantity、fee 等字段

  Scenario: 断线重连后补发
    Given WebSocket 断线后重连成功
    When 用户重新订阅 "order:all"
    Then 服务端补发断线期间的委托状态变更
      And 按时间顺序推送
      And 客户端去重

  Scenario: 未订阅时委托状态可通过 REST 查询
    Given 用户未连接 WebSocket
    When 用户打开当前委托页面
    Then 通过 REST API 轮询获取最新状态

Feature: 持仓管理 (US-TE-10)

  Background:
    Given 用户已登录
      And 用户有模拟交易持仓

  Scenario: 持仓列表显示
    Given 用户有 3 个持仓
    When 打开 "持仓" 页面
    Then 显示所有持仓汇总
      And 每行包含: 交易对、方向、持仓数量、开仓均价、当前价、浮动盈亏、盈亏率
      And 盈利显示绿色、亏损显示红色

  Scenario: 一键平仓
    Given 用户持有 "BTC/USDT" 多头 0.3 BTC
    When 用户点击 "平仓" 按钮并确认
    Then 生成市价卖单 0.3 BTC
      And 成交后持仓清零
      And 盈亏计入已实现盈亏

  Scenario: 部分平仓
    Given 用户持有 "BTC/USDT" 多头 0.5 BTC
    When 用户点击 "部分平仓" 并输入 0.2 BTC
    Then 生成市价卖单 0.2 BTC
      And 成交后持仓数量变为 0.3 BTC

  Scenario: 持仓盈亏实时更新
    Given 持仓页面已打开
      And 用户持有 "BTC/USDT" 多头，开仓均价 ¥49,000
    When BTC/USDT 当前价从 ¥50,000 变为 ¥51,000
    Then 浮动盈亏和盈亏率实时更新
