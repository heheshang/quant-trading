@strategy-management
Feature: 策略管理模块

  Background:
    Given 用户已登录

  # ============================================================
  # US-SM-01: 查看策略列表
  # ============================================================
  Scenario: 成功加载策略列表
    Given 用户拥有 3 个策略实例
    When 导航至 /strategies 页面
    Then 显示策略列表
      And 每行包含: 策略名称、模板类型、参数摘要、状态、创建时间
      And 页面标题显示"策略管理"

  Scenario: 策略列表为空
    Given 用户没有任何策略
    When 导航至 /strategies 页面
    Then 显示空状态组件
      And 提示"还没有策略，点击创建你的第一个策略"
      And 显示"创建策略"按钮

  Scenario: 策略列表分页
    Given 用户拥有 25 个策略
    When 加载策略列表
    Then 每页显示 20 条
      And 底部显示分页组件
      And 翻页后加载下一页数据

  Scenario: 按状态筛选策略列表
    Given 用户拥有多个策略，状态分别为 draft/active/paused/stopped
    When 选择状态筛选器 "active"
    Then 仅显示状态为 active 的策略
      And 其他状态策略被隐藏

  Scenario: 搜索策略名称
    Given 用户拥有策略 "BTC双均线" 和 "ETH布林带"
    When 输入搜索词 "BTC"
    Then 仅显示名称包含 "BTC" 的策略

  # ============================================================
  # US-SM-02: 创建策略
  # ============================================================
  Scenario: 成功创建策略
    Given 用户点击"创建策略"按钮
    When 系统显示模板选择面板
      And 用户选择"双均线交叉"模板
      And 输入策略名称 "我的双均线策略"
      And 配置 fast_period = 10, slow_period = 30
      And 选择交易对 "BTC/USDT"
      And 选择时间周期 "1h"
      And 点击"保存"
    Then 策略创建成功
      And 返回策略列表
      And 列表中出现新策略
      And 状态显示为 "draft"

  Scenario: 模板选择后自动填充默认参数
    Given 用户选择"布林带反转"模板
    Then 参数表单自动填充默认值: period=20, std_dev=2.0
      And 用户可手动修改任意参数

  Scenario: 必填参数未填时阻止保存
    Given 创建策略弹窗打开
    When 策略名称为空
      And 其他参数已填写
    Then "保存"按钮置灰/禁用
      And 策略名称输入框显示错误提示"策略名称不能为空"

  Scenario: 参数值超出范围拒绝保存
    Given 选择"双均线交叉"模板
    When fast_period 设置为 100
    Then 输入框显示错误提示"fast_period 有效范围: 5-50"
      And "保存"按钮禁用

  Scenario: 取消创建不保存
    Given 创建策略弹窗打开
    When 填写了部分参数
      And 点击"取消"或关闭弹窗
    Then 不保存任何数据
      And 返回策略列表页

  Scenario: 策略名称重复时拒绝创建
    Given 用户已有名为 "我的双均线" 的策略
    When 创建新策略并输入名称 "我的双均线"
    Then 保存失败
      And 提示"策略名称已存在，请使用其他名称"

  # ============================================================
  # US-SM-03: 策略状态机
  # ============================================================
  Scenario: 启用草稿策略
    Given 策略状态为 "draft"
    When 点击"启用"按钮
    Then 状态变为 "active"
      And 按钮变为"暂停"和"停止"

  Scenario: 暂停运行中的策略
    Given 策略状态为 "active"
    When 点击"暂停"按钮
    Then 弹出确认对话框"确定要暂停该策略吗？"
      And 用户确认后
    Then 状态变为 "paused"
      And 按钮变为"启用"和"停止"

  Scenario: 停止策略（终止运行）
    Given 策略状态为 "active" 或 "paused"
    When 点击"停止"按钮
    Then 弹出确认对话框"策略停止后不可恢复，确定吗？"
      And 用户确认后
    Then 状态变为 "stopped"
      And 仅可删除或导出

  Scenario: 恢复暂停的策略
    Given 策略状态为 "paused"
    When 点击"启用"按钮
    Then 状态恢复为 "active"

  Scenario: 已停止的策略无法重新启用
    Given 策略状态为 "stopped"
    When 点击"启用"
    Then "启用"按钮不可点击
      And 工具提示"已停止的策略无法重新启用"

  Scenario: 非法状态转换返回错误
    Given 策略状态为 "draft"
    When 前端直接发送 PATCH /api/v1/strategies/{id}/status { "status": "paused" }
    Then 返回 400 错误
      And 提示"草稿状态不能直接转为暂停，请先启用"

  # ============================================================
  # US-SM-04: 编辑策略
  # ============================================================
  Scenario: 编辑 draft 策略
    Given 策略状态为 "draft"
    When 点击"编辑"按钮
    Then 打开参数配置弹窗（同创建弹窗，预填当前值）
    When 修改 fast_period 从 10 改为 5
      And 点击"保存"
    Then 策略参数更新成功
      And 列表参数摘要更新为新值

  Scenario: 编辑 active 策略被拒绝
    Given 策略状态为 "active"
    When 点击"编辑"按钮
    Then 提示"策略运行中，请先暂停后再编辑"
      And 不允许编辑

  Scenario: 编辑 paused 策略
    Given 策略状态为 "paused"
    When 点击"编辑"按钮
    Then 可修改参数
      And 保存后状态保持 paused

  Scenario: 编辑后取消
    Given 编辑弹窗已打开并修改了参数
    When 点击"取消"
    Then 参数恢复为修改前的值
      And 不触发后端更新

  Scenario: 修改策略名称
    Given 策略名称为 "旧名称"
    When 编辑并修改名称为 "新名称"
    Then 保存成功
      And 列表中显示新名称

  # ============================================================
  # US-SM-05: 删除策略
  # ============================================================
  Scenario: 删除 stopped 状态的策略
    Given 策略状态为 "stopped"
    When 点击"删除"按钮
    Then 弹出确认对话框"确定要删除策略「xxx」吗？此操作不可恢复"
      And 用户确认后
    Then 策略被删除
      And 策略从列表中消失
      And 提示"策略已删除"

  Scenario: 删除 active 状态策略被阻止
    Given 策略状态为 "active"
    When 点击"删除"按钮
    Then 提示"请先停止策略后再删除"
      And 阻止删除操作

  Scenario: 删除 paused 状态策略被阻止
    Given 策略状态为 "paused"
    When 点击"删除"按钮
    Then 提示"请先停止策略后再删除"
      And 阻止删除操作

  Scenario: 删除 draft 状态策略
    Given 策略状态为 "draft"
    When 点击"删除"按钮
    Then 直接删除（无需确认）
      And 策略从列表中消失

  # ============================================================
  # US-SM-06: 导入/导出
  # ============================================================
  Scenario: 导出单条策略为 JSON
    Given 用户点击某策略的"导出"按钮
    Then 浏览器下载 strategy_export_<id>.json 文件
      And JSON 包含: name, template_id, params, symbol, timeframe, status, created_at

  Scenario: 导出 JSON 可完整导入恢复
    Given 导出了策略 JSON
    When 在策略列表页点击"导入策略"
      And 上传该 JSON 文件
    Then 创建成功
      And 新策略名称后缀" (导入)"
      And 其他参数与原策略一致

  Scenario: 导入损坏的 JSON 文件
    Given 用户上传一个格式错误的 JSON
    When 点击"导入"
    Then 提示"文件格式错误，请上传有效的策略 JSON"
      And 不创建任何策略

  Scenario: 导入时模板 ID 不存在
    Given 导出的 JSON 中 template_id 为一个不存在的模板
    When 导入该文件
    Then 提示"策略模板不存在，导入失败"

  Scenario: 导入时模板版本不匹配
    Given 导出的 JSON 中模板版本低于当前版本
    When 导入该文件
    Then 提示"模板版本已更新，部分参数可能不兼容"
      And 允许用户手动调整后继续导入

  # ============================================================
  # US-SM-07: 策略模板管理
  # ============================================================
  Scenario: 查看内置模板列表
    Given 用户点击"创建策略"
    When 模板选择面板显示
    Then 显示所有内置模板
      And 每个模板显示: 名称、简要描述、分类标签
      And 模板按分类分组展示
      And 支持搜索模板名称

  Scenario: 查看模板详情
    Given 模板选择面板
    When 点击某个模板卡片
    Then 展开显示模板的完整描述
      And 显示该模板的所有可配置参数及默认值
      And 显示"使用此模板"按钮

  Scenario: 查看自定义模板列表
    Given 用户拥有自定义模板
    When 在创建策略面板切换到"我的模板"标签
    Then 显示用户创建的所有自定义模板
      And 每个显示: 名称、描述、参数数量

  Scenario: 创建自定义模板
    Given 用户点击"新建模板"
    When 输入模板名称、描述
      And 定义参数列表（名称/类型/默认值/范围）
      And 点击"保存"
    Then 自定义模板创建成功
      And 在"我的模板"标签页可见

  Scenario: 编辑自定义模板
    Given 用户拥有 1 个自定义模板
    When 点击"编辑"该自定义模板
    Then 可修改名称、描述、参数定义
      And 保存后不影响已有策略实例

  Scenario: 删除自定义模板
    Given 用户拥有 1 个自定义模板
      And 该模板没有被任何策略使用
    When 点击"删除"该自定义模板
    Then 删除成功
      And 模板从列表消失

  Scenario: 删除已被策略使用的自定义模板
    Given 自定义模板正被 1 个策略实例引用
    When 点击"删除"该自定义模板
    Then 提示"该模板正在被 N 个策略使用，无法删除"
      And 删除操作被阻止

  # ============================================================
  # US-SM-08: 权限控制
  # ============================================================
  Scenario: 用户无法查看他人策略列表
    Given 用户 A 尝试访问 GET /api/v1/strategies
    Then 仅返回用户 A 自己的策略
      And 不包含用户 B 的任何策略

  Scenario: 用户无法查看他人策略详情
    Given 用户 A 尝试访问 GET /api/v1/strategies/{b_strategy_id}
    Then 返回 403 Forbidden
      And 提示"无权访问该策略"

  Scenario: 用户无法修改他人策略
    Given 用户 A 尝试 PUT /api/v1/strategies/{b_strategy_id}
    Then 返回 403 Forbidden
      And 提示"无权操作该策略"

  Scenario: 用户无法删除他人策略
    Given 用户 A 尝试 DELETE /api/v1/strategies/{b_strategy_id}
    Then 返回 403 Forbidden
      And 提示"无权操作该策略"

  Scenario: 用户无法切换他人策略状态
    Given 用户 A 尝试 PATCH /api/v1/strategies/{b_strategy_id}/status
    Then 返回 403 Forbidden

  Scenario: 管理员可查看所有策略
    Given 管理员尝试访问 GET /api/v1/strategies
    Then 返回所有用户的策略（分页）
      And 包含 user_id 字段用于区分拥有者
