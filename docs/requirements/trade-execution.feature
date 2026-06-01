# 交易执行模块 — Gherkin 验收条件
# PRD-trade-execution.md 对应 Feature 文件
# 用户故事: US-TE-01 ~ US-TE-10

# ============================================================
# US-TE-01: 下单面板 (P0)
# ============================================================

Feature: 下单面板
  As a 交易者
  I want to 在行情页面右侧快速下单
  So that 我可以边看行情边执行交易

  Background:
    Given 用户已登录
    And 当前处于行情页面或交易页面

  Scenario: 下单面板展示
    Given 用户在行情页面
    Then 右侧面板显示"下单"区域
    And 包含: 交易对选择器、方向选择、委托类型选择、价格输入、数量输入、金额显示、提交按钮
    And 买入按钮为绿色，卖出按钮为红色
    And 默认交易对与当前行情页选中的交易对一致

  Scenario: 限价单下单
    Given 下单面板已打开
    When 选择委托类型 "限价单"
    And 输入价格 50000.00
    And 输入数量 0.1
    And 金额显示为 5000.00 USDT
    And 选择方向 "买入"
    And 点击"买入限价"按钮
    Then 弹出确认对话框显示交易对、方向、类型、价格、数量、总金额、模式
    When 确认提交
    Then 委托提交成功
    And Toast 提示 "委托已提交"
    And 委托出现在"当前委托"列表
    And 状态为 "pending"

  Scenario: 市价单下单
    Given 下单面板已打开
    When 选择委托类型 "市价单"
    Then 价格输入框隐藏
    And 显示 "以市场最优价成交" 提示
    When 输入数量 0.1
    And 选择方向 "卖出"
    And 点击"卖出市价"按钮
    And 确认提交
    Then 委托提交成功
    And 委托立即进入撮合引擎
    And 成交后状态更新为 "filled"

  Scenario: 止损单下单
    Given 下单面板已打开
    When 选择委托类型 "止损单"
    Then 显示触发价输入框和委托价输入框
    When 输入触发价 48000.00
    And 输入委托价 47900.00
    And 输入数量 0.1
    And 选择方向 "卖出"
    Then 金额显示区显示触发说明

  Scenario: 价格联动行情
    Given 下单面板已打开
    And 当前交易对为 BTC/USDT
    And 最新价为 50500.00
    When 点击价格输入框旁的"市价"按钮
    Then 价格自动填充为 50500.00
    When 行情推送新价格 50600.00
    Then 已填入的价格不自动更新
    And 价格输入框旁显示最新价参考

  Scenario: 数量快捷选择
    Given 下单面板已打开
    And 用户可用余额为 10000 USDT
    When 点击 "25%" 按钮
    Then 数量自动计算为 (10000 * 0.25 / 当前价) 并填入

  Scenario: 输入校验 - 必填项
    Given 下单面板已打开
    When 价格为空且数量已填写
    Then "提交"按钮禁用
    And 价格输入框显示红色边框
    When 价格填写为 0 或负数
    Then 提示 "价格必须大于0"

  Scenario: 输入校验 - 数量精度
    Given 当前交易对最小下单量为 0.001
    When 输入数量 0.0001
    Then 提示 "最小下单量: 0.001 BTC"
    And 提交按钮禁用

  Scenario: 余额不足提示
    Given 用户可用余额为 1000 USDT
    When 输入限价买单 价格 50000 数量 0.1
    Then 金额显示红色
    And 提示 "余额不足"
    And 提交按钮禁用

  Scenario: 模拟模式标识
    Given 当前为模拟交易模式
    Then 下单面板顶部显示 "模拟交易" 标签
    And 确认对话框包含 "模拟交易 — 不涉及真实资金" 提示

# ============================================================
# US-TE-02: 模拟撮合引擎 (P0)
# ============================================================

Feature: 模拟撮合引擎
  As a 系统
  I want to 对模拟模式委托进行本地撮合
  So that 用户可以在无真实资金的情况下体验完整的交易流程

  Background:
    Given 系统已启动
    And 撮合引擎为 SIMULATE 模式
    And Market Data Collector 已提供实时行情

  Scenario: 市价单即时成交
    Given 用户提交 BTC/USDT 市价买入单 数量 0.1
    And 最新卖一价为 50500.00
    When 委托进入撮合引擎
    Then 以 50500.00 价格成交 0.1 BTC
    And 生成成交记录
    And 委托状态更新为 "filled"
    And 用户的 BTC 持仓增加 0.1
    And 用户的 USDT 余额减少 5050.00
    And WebSocket 推送成交通知

  Scenario: 限价单挂单
    Given 用户提交 BTC/USDT 限价买入单 价格 50000.00 数量 0.1
    And 当前卖一价为 50500.00
    When 委托进入撮合引擎
    Then 委托进入 Order Book 买盘
    And 委托状态为 "pending"
    And 不产生成交记录

  Scenario: 限价单部分成交
    Given 用户提交 BTC/USDT 限价买入单 价格 50500.00 数量 0.5
    And Order Book 卖一价 50500.00 数量 0.3
    When 委托进入撮合引擎
    Then 成交 0.3 BTC @ 50500.00
    And 委托状态为 "partial_filled"
    And filled 更新为 0.3
    And 剩余 0.2 继续挂在 Order Book

  Scenario: 限价单完全成交
    Given Order Book 中有卖一 50500.00 x 0.3 和卖二 50501.00 x 0.3
    And 用户提交限价买入 价格 50501.00 数量 0.5
    When 委托进入撮合引擎
    Then 成交 0.3 @ 50500.00
    And 成交 0.2 @ 50501.00
    And 委托状态为 "filled"
    And 生成 2 条成交记录

  Scenario: 止损单触发
    Given 用户提交止损卖出单 触发价 48000.00 委托价 47900.00 数量 0.1
    And 当前价格 49000.00
    When 行情推送最新价 48000.00
    Then 止损单被触发
    And 生成限价卖出委托 价格 47900.00 数量 0.1

  Scenario: 撮合价格优先时间优先
    Given Order Book 买盘有 价格50100x0.2@T1, 价格50000x0.3@T2, 价格50000x0.1@T3
    When 新卖出委托 价格 50000 数量 0.2 进入
    Then 优先成交 50100 x 0.2

  Scenario: 模拟手续费计算
    Given 系统配置模拟手续费率 0.001
    When 成交 0.1 BTC @ 50500.00
    Then 手续费为 5.05 USDT
    And 成交记录中 fee 字段为 5.05

  Scenario: 撮合引擎初始化
    Given 系统启动
    When 撮合引擎初始化
    Then 从 Redis 加载最新深度数据初始化 Order Book
    And 从数据库加载 pending 状态的委托恢复 Order Book

# ============================================================
# US-TE-03: 委托管理 (P0)
# ============================================================

Feature: 委托管理
  As a 交易者
  I want to 查看和管理我的所有委托
  So that 我可以跟踪订单状态和及时撤单

  Background:
    Given 用户已登录

  Scenario: 当前委托列表
    Given 用户有 3 个未完全成交的委托
    When 导航至 /trade 页面并切换至"当前委托" Tab
    Then 显示未完全成交的委托列表
    And 每行包含: 时间、交易对、方向、类型、价格、数量、已成交、状态、操作
    And 买入方向显示绿色标签
    And 卖出方向显示红色标签
    And pending 状态的委托显示"撤单"按钮
    And partial_filled 状态的委托显示"撤单"按钮

  Scenario: 撤单操作
    Given 存在状态为 "pending" 的委托
    When 点击"撤单"按钮
    Then 弹出确认对话框
    When 确认撤销
    Then 委托状态更新为 "cancelled"
    And 已成交部分保留
    And Toast 提示 "委托已撤销"
    And 委托从"当前委托"列表移至"历史委托"

  Scenario: 撤单 - 部分成交
    Given 存在状态为 "partial_filled" 的委托 已成交 0.3/0.5
    When 撤销该委托
    Then 未成交部分 0.2 被撤销
    And 已成交部分 0.3 保留
    And 生成持仓记录 0.3 BTC
    And 委托状态变为 "cancelled"

  Scenario: 历史委托列表
    Given 用户有 50 条历史委托
    When 切换至"历史委托" Tab
    Then 显示已成交/已撤销/已过期的委托
    And 每页 20 条
    And 支持按日期范围、交易对、状态筛选

  Scenario: 委托状态实时更新
    Given 用户在"当前委托"页面
    And 有一个 pending 状态的限价买入单
    When 撮合引擎成交了该委托
    Then 委托状态实时更新为 "filled"
    And 委托行从"当前委托"消失
    And 自动出现在"历史委托"列表顶部

  Scenario: 委托详情
    Given 委托列表已显示
    When 点击某委托行
    Then 展开委托详情面板包含: 委托ID、关联策略、创建时间、更新时间、成交明细、手续费

# ============================================================
# US-TE-04: 成交记录 (P0)
# ============================================================

Feature: 成交记录
  As a 交易者
  I want to 查看我的所有成交记录
  So that 我可以分析交易执行情况和复盘交易

  Background:
    Given 用户已登录

  Scenario: 成交记录列表
    Given 用户有成交记录
    When 导航至 /trade 页面并切换至"成交记录" Tab
    Then 显示成交记录列表
    And 每行包含: 成交时间、交易对、方向、价格、数量、金额、手续费、关联委托ID
    And 列表按时间降序排列

  Scenario: 成交记录筛选
    Given 成交记录列表已显示
    When 选择筛选条件: 交易对、方向、日期范围
    Then 列表按条件过滤
    And 显示匹配的记录数

  Scenario: 成交记录分页
    Given 用户有 100 条成交记录
    When 查看成交记录列表
    Then 每页 20 条
    And 底部显示分页组件

  Scenario: 成交记录实时推送
    Given 用户在成交记录页面
    When 撮合引擎产生新成交
    Then 成交记录列表顶部插入新记录
    And 新记录高亮显示 3 秒

# ============================================================
# US-TE-05: 持仓管理 (P0)
# ============================================================

Feature: 持仓管理
  As a 交易者
  I want to 查看和管理我的持仓
  So that 我可以了解当前资产配置和浮动盈亏

  Background:
    Given 用户已登录

  Scenario: 持仓列表
    Given 用户有 3 个持仓
    When 导航至 /trade 页面并切换至"持仓" Tab
    Then 显示持仓列表
    And 每行包含: 交易对、方向、持仓数量、开仓均价、当前价、浮动盈亏、盈亏率、操作
    And 浮动盈亏实时更新
    And 盈利显示绿色，亏损显示红色

  Scenario: 持仓盈亏实时更新
    Given 持仓列表已显示
    And 用户持有 0.5 BTC 均价 50000
    And 当前 BTC 价格 50500
    Then 浮动盈亏显示 +250.00 USDT
    And 盈亏率显示 +0.50%
    When 行情推送价格变为 49000
    Then 浮动盈亏更新为 -500.00 USDT
    And 颜色变为红色

  Scenario: 平仓操作
    Given 用户持有 0.5 BTC
    When 点击"平仓"按钮
    Then 弹出平仓确认对话框
    When 选择 "全部" 并确认
    Then 生成市价卖出委托 0.5 BTC
    And 成交后持仓清零
    And 盈亏计入已实现盈亏

  Scenario: 部分平仓
    Given 用户持有 0.5 BTC
    When 选择部分平仓 0.2 BTC 并确认
    Then 生成市价卖出委托 0.2 BTC
    And 成交后持仓数量变为 0.3 BTC
    And 开仓均价不变

  Scenario: 持仓为空
    Given 用户没有任何持仓
    When 查看持仓 Tab
    Then 显示空状态 "暂无持仓"

  Scenario: 持仓汇总
    Given 持仓列表已显示
    Then 底部显示汇总行: 总持仓市值、总浮动盈亏、总盈亏率

# ============================================================
# US-TE-06: 风控拦截 (P1)
# ============================================================

Feature: 风控拦截
  As a 交易者
  I want to 在下单前获得风控校验
  So that 我不会因为误操作或过度交易导致不可控的风险

  Background:
    Given 用户已登录
    And 风控规则已配置

  Scenario: 余额不足拦截
    Given 用户可用余额为 1000 USDT
    When 提交限价买入 总金额 5000 USDT
    Then 下单被拒绝
    And 返回错误 "余额不足"

  Scenario: 最大持仓限制
    Given 风控规则: 单交易对最大持仓 5 BTC
    And 用户已持有 4.8 BTC
    When 尝试买入 0.5 BTC
    Then 下单被拒绝
    And 返回错误 "超过最大持仓限制"

  Scenario: 每日最大亏损预警
    Given 风控规则: 每日最大亏损 500 USDT
    And 今日已实现亏损 480 USDT
    When 尝试开新仓
    Then 弹出风控警告
    And 用户可选择 "继续" 或 "取消"

  Scenario: 每日最大亏损触发
    Given 风控规则: 每日最大亏损 500 USDT
    And 今日已实现亏损 500 USDT
    When 尝试开新仓
    Then 下单被拒绝
    And 返回错误 "已达到每日最大亏损限制"

  Scenario: 单笔最大亏损预警
    Given 风控规则: 单笔最大亏损 200 USDT
    And 用户提交的委托预估亏损可能超过 200 USDT
    When 下单
    Then 弹出确认对话框要求二次确认

  Scenario: 风控规则可配置
    Given 用户为 admin
    When 导航至风控配置页面
    Then 可查看和修改风控规则
    When 修改 max_daily_loss 为 1000 USDT
    Then 规则立即生效

# ============================================================
# US-TE-07: 策略信号自动下单 (P1)
# ============================================================

Feature: 策略信号自动下单
  As a 交易者
  I want to 策略运行时自动生成委托
  So that 我不需要手动操作就能执行策略信号

  Background:
    Given 用户已登录
    And 用户有一个 active 状态的策略

  Scenario: 策略信号生成委托
    Given 策略 "双均线交叉" 运行中
    When 策略引擎产生买入信号
    Then 交易服务接收信号
    And 自动创建限价买入委托
    And 委托的 strategy_id 关联到该策略
    And WebSocket 推送通知用户

  Scenario: 策略信号触发风控
    Given 策略生成买入信号
    And 风控规则: 单交易对最大持仓 5 BTC
    And 用户已持有 4.8 BTC
    And 信号数量为 0.5 BTC
    When 交易服务接收信号
    Then 委托被风控拒绝
    And 策略日志记录风控拒绝信息
    And WebSocket 推送通知用户

  Scenario: 策略停止后不再生成委托
    Given 策略 "双均线交叉" 被用户停止
    When 策略引擎不再产生信号
    Then 交易服务不再接收该策略的信号
    And 已有的 pending 委托不受影响

  Scenario: 策略信号与手动委托区分
    Given 委托列表已显示
    Then 策略生成的委托显示关联策略名称
    And 手动下单的委托显示 "手动" 标签
    And 支持按来源筛选

# ============================================================
# US-TE-08: 交易页面布局 (P0)
# ============================================================

Feature: 交易页面布局
  As a 交易者
  I want to 在一个页面内同时查看行情、下单、委托和持仓
  So that 我可以高效执行交易而不需要频繁切换页面

  Background:
    Given 用户已登录

  Scenario: 交易页面三栏布局
    Given 用户导航至 /trade 页面
    When 页面加载完成
    Then 显示三栏布局: 左侧行情面板、右上侧下单面板、右下侧委托/成交/持仓

  Scenario: Tab 切换
    Given 交易页面已加载
    Then 右下区域包含 Tab: 当前委托、历史委托、成交记录、持仓

  Scenario: 行情与下单联动
    Given 交易页面已加载
    When 用户在行情面板切换交易对为 ETH/USDT
    Then 下单面板的交易对自动切换为 ETH/USDT
    And 委托/成交/持仓列表按 ETH/USDT 筛选

  Scenario: 快捷键支持
    Given 交易页面已加载
    When 按下快捷键 "B"
    Then 下单面板方向切换为 "买入"
    When 按下快捷键 "S"
    Then 下单面板方向切换为 "卖出"
    When 按下快捷键 "Ctrl+Enter"
    Then 提交当前委托

  Scenario: 交易对从行情页跳转
    Given 用户在 /market 页面
    When 点击某交易对的"交易"按钮
    Then 跳转至 /trade/{symbol} 页面
    And 下单面板自动选择该交易对

# ============================================================
# US-TE-09: 账户余额与资产概览 (P1)
# ============================================================

Feature: 账户余额与资产概览
  As a 交易者
  I want to 在交易页面查看我的账户余额
  So that 我可以了解可用资金和资产分布

  Background:
    Given 用户已登录

  Scenario: 余额显示
    Given 交易页面已加载
    Then 下单面板上方显示: 可用余额、冻结金额、总资产

  Scenario: 余额实时更新
    Given 可用余额为 10000 USDT
    When 用户提交限价买入 5000 USDT
    Then 可用余额更新为 5000 USDT
    And 冻结金额增加 5000 USDT
    When 委托成交
    Then 冻结金额减少
    And 持仓市值增加
    When 委托撤销
    Then 冻结金额减少
    And 可用余额恢复

  Scenario: 模拟账户初始化
    Given 新用户首次进入交易页面
    Then 系统自动创建模拟账户
    And 初始余额为 100000 USDT

# ============================================================
# US-TE-10: 交易通知 (P1)
# ============================================================

Feature: 交易通知
  As a 交易者
  I want to 收到交易相关的实时通知
  So that 我不会错过重要的成交和风控事件

  Background:
    Given 用户已登录

  Scenario: 成交通知
    Given 用户有一个 pending 状态的委托
    When 该委托被撮合引擎成交
    Then 前端显示 Toast 通知包含成交信息
    And 通知自动消失 5 秒

  Scenario: 部分成交通知
    Given 用户有一个数量为 0.5 的委托
    When 部分成交 0.3
    Then 前端显示部分成交 Toast 通知

  Scenario: 止损触发通知
    Given 用户有一个止损卖出单
    When 价格触及触发价
    Then 前端显示止损触发通知

  Scenario: 风控预警通知
    Given 风控规则: 每日最大亏损 500 USDT
    And 今日已亏损 400 USDT
    When 风控检测到接近阈值
    Then 前端显示警告通知

  Scenario: 通知中心
    Given 用户收到多条通知
    When 点击导航栏通知图标
    Then 显示通知中心列表
    And 按时间降序排列
    And 未读通知加粗显示
    And 支持标记已读/全部已读
