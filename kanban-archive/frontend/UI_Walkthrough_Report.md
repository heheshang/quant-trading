# UI/UE 走查报告：前端实现 vs 设计稿

> 项目：量化交易系统 (quant-trading)
> 设计稿：docs/design/Design_QuantTrading_UI.md (v1.2)
> 走查日期：2026-05-13
> 走查人：Designer

---

## 一、总览

| 维度 | 评分 | 说明 |
|------|------|------|
| 设计系统 Token | ✅ 95% | CSS 变量与设计稿完全一致 |
| 页面结构布局 | ⚠️ 70% | 回测页布局偏离设计稿最大 |
| 组件状态覆盖 | ⚠️ 75% | 核心4态覆盖，缺部分边界态 |
| 交互反馈 | ⚠️ 70% | 部分缺少 hover/focus 状态 |
| 表单验证 | ✅ 85% | 验证规则完整，部分缺失 |
| 响应式适配 | ⚠️ 65% | 移动端适配不充分 |
| 视觉细节一致性 | ⚠️ 75% | 部分硬编码色值偏离设计系统 |
| ECharts 图表 | ⚠️ 75% | 颜色/tooltip 有偏差 |

---

## 二、逐项走查

### 1. 页面结构：布局、间距、层级关系

#### 1.1 整体布局 (MainLayout)

| # | 位置 | 预期 | 实际 | 严重度 | 修复建议 |
|---|------|------|------|--------|----------|
| 1 | 侧栏宽度 | 240px 固定 | `var(--sidebar-width)` = 240px ✅ | — | 一致 |
| 2 | Header 高度 | 48px | `var(--header-height)` = 48px ✅ | — | 一致 |
| 3 | 内容区 padding | 24px 桌面/16px 移动 | `$layout-padding-desktop: 24px` ✅ | — | 一致 |
| 4 | 内容区 max-width | 1344px | `$content-max-width: 1344px` ✅ | — | 一致 |

**结论：整体布局完全符合设计稿。**

#### 1.2 回测页布局 (BacktestView)

| # | 位置 | 预期 | 实际 | 严重度 | 修复建议 |
|---|------|------|------|--------|----------|
| 2.1 | 回测页布局 | 左右分栏：参数表单(340px) + 绩效报告(flex:1) | 上下单列：配置表单 → 空态/运行态/结果态 顺序展示 | **P1** | 改为左右分栏布局，参数面板固定左侧 340px，右侧展示报告区域 |
| 2.2 | 绩效指标位置 | 参数面板右侧 | 独立 metrics-grid 在结果区上方 | **P2** | 移入右侧面板，3x2 网格 |
| 2.3 | 权益曲线位置 | 参数面板右侧(绩效报告内) | 独立 tab 面板 | **P2** | 应在右侧面板绩效指标下方 |
| 2.4 | 交易明细位置 | 全宽表格在底部 | tab 面板切换 | P3 | 保持 tab 可接受，但建议改为全宽 |
| 2.5 | 保存配置按钮 | 有"保存配置" text button | 不存在 | P2 | 添加"保存配置"按钮 |

**结论：回测页布局与设计稿差异最大，从左右分栏改为了单列流式，核心结构偏离。**

#### 1.3 登录页布局

| # | 位置 | 预期 | 实际 | 严重度 | 修复建议 |
|---|------|------|------|--------|----------|
| 3.1 | 登录卡片宽度 | 400px | max-width: 384px | P3 | 差 16px，可接受但建议对齐 |
| 3.2 | 登录/注册 Tab 切换 | Tab 切换登录/注册 | 独立路由 /login 和 /register | P3 | 功能等效，但设计稿是 Tab 内切换 |
| 3.3 | 品牌色 | accent #7170ff | 紫色 #7C3AED | **P1** | 登录页品牌色与设计系统不一致，应使用 --color-accent |
| 3.4 | 页面标题 | "登录" / "注册" | "Welcome back" / "Sign in" | P3 | 语言不一致（设计稿中文，实现英文） |

#### 1.4 仪表盘布局

| # | 位置 | 预期 | 实际 | 严重度 | 修复建议 |
|---|------|------|------|--------|----------|
| 4.1 | StatCards 数量 | 4 个(总资产/今日盈亏/累计收益/最大回撤) | 4 个(Total PnL/Win Rate/Sharpe/Positions) | P3 | 内容不同但数量一致 |
| 4.2 | PnLChart 时间切换 | pill 按钮组 1周/1月/3月/6月/1年/全部 | 完全实现 ✅ | — | 一致 |
| 4.3 | 页面标题 | 中文"仪表盘" | 英文 "Dashboard" | P3 | i18n 待统一 |

---

### 2. 组件状态：空态/加载态/错误态/成功态

| # | 组件 | 空态 | 加载态 | 错误态 | 成功态 | 严重度 |
|---|------|------|--------|--------|--------|--------|
| 5 | BacktestView | ✅ idle 空态(Odometer 图标+提示) | ✅ running 进度条+取消 | ✅ failed Alert+返回按钮 | ✅ completed 结果展示 | — |
| 6 | BacktestConfigForm | ❌ 策略列表为空无处理 | ✅ loadingStrategies | ❌ 加载失败无错误提示 | ✅ 表单正常 | P2 |
| 7 | BacktestMetricsCards | ✅ metrics-empty | ✅ skeleton | ❌ 无错误态 | ✅ 数据展示 | P2 |
| 8 | BacktestEquityChart | ✅ chart-empty | ✅ skeleton | ❌ 无错误态(数据加载失败) | ✅ 图表渲染 | P2 |
| 9 | BacktestTradesTable | ✅ table-empty | ✅ skeleton | ❌ 无错误态 | ✅ 表格+分页 | P2 |
| 10 | DashboardView | ❌ statCards 无独立空态 | ✅ skeleton | ✅ el-result error | ✅ | P3 |
| 11 | LoginView | N/A | ✅ 按钮loading | ✅ Toast 错误 | ✅ 跳转 | — |

**结论：回测子组件均缺少 error 重试态。设计稿明确要求"加载失败 → 居中'数据加载失败，点击重试'"，实际未实现。**

---

### 3. 交互反馈：点击、悬停、输入校验、Loading 过渡

| # | 位置 | 预期 | 实际 | 严重度 | 修复建议 |
|---|------|------|------|--------|----------|
| 12 | 配置表单-运行中 | 表单全部 disabled（只读） | 表单未 disabled，仍可修改 | **P1** | 运行回测时表单字段应全部 disabled |
| 13 | 侧栏导航-禁用态 | 禁用项文字色 #62666d | 未实现禁用态 | P3 | 部分页面可设置禁用态 |
| 14 | metric-card hover | — | border-color + box-shadow 过渡 ✅ | — | 实现了设计稿未明确要求但合理的 hover |
| 15 | 删除按钮确认 | 设计稿要求模态框确认 | 直接删除无确认 | **P1** | 添加 ElMessageBox.confirm 确认弹窗 |
| 16 | 导出按钮反馈 | 设计稿未明确 | ✅ ElMessage.success 提示 | — | 合理补充 |
| 17 | 回测取消 | 设计稿未明确 | ✅ 有取消按钮+API调用 | — | 合理补充 |

---

### 4. 表单验证：必填项、格式校验、错误提示样式

| # | 位置 | 预期 | 实际 | 严重度 | 修复建议 |
|---|------|------|------|--------|----------|
| 18 | 策略必选 | 必选，未选时 disabled | ✅ required + isFormValid computed | — | 一致 |
| 19 | 日期合法性 | 结束日不能早于起始日 | dateRange 单字段 required，无跨字段校验 | **P2** | 添加自定义 validator 校验 start<end |
| 20 | 初始资金验证 | >0 | min=1000 ✅ (比设计稿更严格) | — | 一致 |
| 21 | 手续费率范围 | 0-1% | min=0, max=1 → 实际允许 0-100% | **P1** | fee_rate max 应为 1(代表1%)，但数值含义需确认；设计稿要求 0-1% |
| 22 | 滑点范围 | 0-0.5% | min=0, max=1 → 允许0-100% | **P1** | slippage max 应为 0.5，当前允许过大值 |
| 23 | 策略参数动态校验 | 设计稿要求 min/max 范围校验 | ✅ getParamRule 动态生成 | — | 一致 |
| 24 | 错误提示样式 | 红色边框 + 红色文字 | Element Plus 默认 ✅ | — | 一致 |

---

### 5. 响应式：移动端(375px)和桌面端(1440px)

| # | 位置 | 预期 | 实际 | 严重度 | 修复建议 |
|---|------|------|------|--------|----------|
| 25 | 侧栏移动端 | <768px 折叠为汉堡菜单+抽屉 | ✅ transform: translateX(-100%) + overlay | — | 一致 |
| 26 | Header 移动端 | 汉堡菜单按钮 | ✅ mobile-menu-btn @media 显示 | — | 一致 |
| 27 | 配置表单-移动端 | 单列 | ✅ @media grid 1fr | — | 一致 |
| 28 | 绩效指标-移动端 | 单列 | ✅ @media 640px 1fr | — | 一致 |
| 29 | 回测结果 action-bar | 移动端堆叠 | ❌ 无响应式处理，按钮溢出 | **P2** | 添加 flex-wrap 或堆叠布局 |
| 30 | 交易明细表格 | 移动端横向滚动 | ✅ overflow-x: auto | — | 一致 |
| 31 | Dashboard stats-row | 移动端 2列或1列 | ❌ 固定 4 列，小屏幕挤压 | **P1** | 添加 @media grid 2fr 或 1fr |
| 32 | Dashboard bottom-row | 移动端单列 | ❌ 固定 2 列 | **P2** | 添加 @media grid 1fr |
| 33 | 仪表盘 StatCard 高度 | 112px | 无固定高度，内容自适应 | P3 | 差异小可接受 |
| 34 | 登录卡片移动端 | 适配 | max-width: 384px + padding 缩减 | — | 基本适配 |

**结论：Dashboard 的响应式适配缺失是 P1 问题。回测页基本有响应式但 action-bar 缺失。**

---

### 6. 视觉细节：字体、颜色、圆角、阴影

| # | 位置 | 预期 | 实际 | 严重度 | 修复建议 |
|---|------|------|------|--------|----------|
| 35 | 等宽字体 | JetBrains Mono 用于数字 | `--font-mono` 定义了 ✅ | — | 一致 |
| 36 | EquityChart 线条色 | --color-accent (#7170ff) + 渐变填充 | #22c55e(涨)/#ef4444(跌) 硬编码 | **P1** | 应使用设计稿的 accent 色(#7170ff)，涨跌色用于绩效指标 |
| 37 | EquityChart tooltip | 设计稿 accent 风格 | rgba(30,41,59,0.95) 硬编码 Tailwind 色 | **P2** | 应使用 --color-surface-elevated + --color-border |
| 38 | EquityChart 网格/轴 | --color-chart-grid / --color-text-tertiary | #334155, #94a3b8, #1e293b 硬编码 Tailwind 色 | **P2** | 应使用 CSS 变量 |
| 39 | metric-card 圆角 | 8px (设计稿) | 10px | P3 | 2px 差异，建议统一为 8px |
| 40 | 登录页背景 | --color-bg (#08090a) | #121212 硬编码 | **P2** | 应使用 --color-bg |
| 41 | 登录页边框 | --color-border | #333344 硬编码 | **P2** | 应使用 --color-border |
| 42 | 登录页按钮色 | --color-accent (#7170ff) | #7C3AED 硬编码紫色 | **P1** | 应使用 --color-accent |
| 43 | 交易方向 tag | 做多(绿) / 做空(红) | long→danger(红) / short→success(绿) ⚠️ | **P1** | 方向颜色映射反了：long(多头)应为绿色success，short(空头)应为红色danger |
| 44 | PnL 正负色 | --color-buy / --color-sell | BacktestTradesTable 使用 --color-buy/--color-sell fallback ✅ | — | 一致 |
| 45 | 侧栏 Logo | 32x32 图标 + "量化交易系统" | 22x22 SVG + "Quant Trading" | P3 | 尺寸偏小+语言差异 |

---

### 7. 图表渲染：ECharts 曲线颜色、轴标签、tooltip

| # | 位置 | 预期 | 实际 | 严重度 | 修复建议 |
|---|------|------|------|--------|----------|
| 46 | 权益曲线主色 | accent (#7170ff) 渐变面积图 | 涨绿(#22c55e)跌红(#ef4444) | **P1** | 权益曲线应使用 accent 色，涨跌色仅用于绩效数字 |
| 47 | 权益曲线基准线 | 虚线(起始资金) | 无基准线 | **P2** | 添加 markLine 显示初始资金 |
| 48 | 权益曲线高度 | 240px (设计稿) | 320px | P3 | 比设计稿高，信息密度可接受 |
| 49 | X轴标签色 | --color-text-tertiary | #94a3b8 硬编码 | P2 | 用 CSS 变量 |
| 50 | Y轴标签格式 | 大数缩写(万/亿) | 实现 ✅ (10000→w) | — | 一致但格式符应用"万"而非"w" |
| 51 | Dashboard PnLChart | accent 渐变面积图 | 未查看实现(PnLChart.vue) | — | 待验证 |

---

## 三、问题汇总

### P0/P1 阻塞性问题 (必须修复)

| # | 位置 | 问题 | 修复建议 |
|---|------|------|----------|
| 2.1 | BacktestView | 回测页布局从左右分栏改为单列流式，核心结构偏离设计稿 | 改为左右分栏：左侧 340px 参数面板 + 右侧绩效报告 |
| 12 | BacktestConfigForm | 运行回测时表单未 disabled，仍可修改参数 | 运行中设置表单所有字段 disabled |
| 15 | BacktestView | 删除结果无确认弹窗，直接删除 | 添加 ElMessageBox.confirm |
| 21 | BacktestConfigForm | fee_rate max=1 允许100%，设计稿要求0-1% | 确认单位：如百分比则 max=1 且 step=0.01 对应0-1% |
| 22 | BacktestConfigForm | slippage max=1 允许100%，设计稿要求0-0.5% | 修改 max=0.5 |
| 31 | DashboardView | stats-row 固定4列无响应式，移动端挤压变形 | 添加 @media 断点：768px→2列，480px→1列 |
| 36 | BacktestEquityChart | 权益曲线使用涨绿跌红而非 accent 色 | 线条色改为 --color-accent，面积渐变改为 accent 渐变 |
| 42 | LoginView | 品牌色 #7C3AED 与设计系统 #7170ff 不一致 | 统一使用 --color-accent |
| 43 | BacktestTradesTable | 方向 tag 颜色反了：long→danger(红)，short→success(绿) | long→success(绿)，short→danger(红) |

### P2 重要问题 (后续优化)

| # | 位置 | 问题 |
|---|------|------|
| 6 | BacktestConfigForm | 策略列表为空/加载失败无错误处理 |
| 7 | BacktestMetricsCards | 缺少错误态（API失败无重试） |
| 8 | BacktestEquityChart | 缺少错误态（数据加载失败无重试） |
| 9 | BacktestTradesTable | 缺少错误态 |
| 19 | BacktestConfigForm | 日期范围无跨字段校验（结束日不能早于起始日） |
| 29 | BacktestView | 结果 action-bar 移动端未响应式 |
| 32 | DashboardView | bottom-row 固定2列无响应式 |
| 37 | BacktestEquityChart | tooltip 背景色硬编码 Tailwind 色 |
| 38 | BacktestEquityChart | 网格/轴标签色硬编码 Tailwind 色 |
| 40 | LoginView | 背景色 #121212 偏离 --color-bg #08090a |
| 41 | LoginView | 边框色 #333344 偏离 --color-border |
| 47 | BacktestEquityChart | 缺少初始资金基准线 |
| 2.2 | BacktestView | 绩效指标位置应在右侧面板而非独立区域 |
| 2.5 | BacktestView | 缺少"保存配置"按钮 |

### P3 可放行问题 (记录即可)

| # | 位置 | 问题 |
|---|------|------|
| 3.1 | LoginView | 卡片宽度 384px vs 设计稿 400px |
| 4.3 | DashboardView | 页面标题英文 vs 设计稿中文 |
| 39 | BacktestMetricsCards | metric-card 圆角 10px vs 设计稿 8px |
| 45 | AppSidebar | Logo 22px vs 设计稿 32px |
| 48 | BacktestEquityChart | 图表高度 320px vs 设计稿 240px |

---

## 四、通过项

以下方面实现与设计稿完全一致或超出预期：

1. **设计系统 Token** — variables.scss 中所有 CSS 变量与设计稿一一对应
2. **侧栏导航** — 宽度、高度、圆角、hover/active 状态、图标、字重完全符合
3. **Header** — 高度 48px、面包屑、主题切换、用户头像下拉全部符合
4. **移动端侧栏** — 抽屉+遮罩层实现正确
5. **表单验证框架** — Element Plus rules + computed isFormValid 双重保障
6. **回测状态机** — idle/running/completed/failed 四态切换逻辑正确
7. **Skeleton 加载态** — shimmer 动画效果良好
8. **数字格式化** — useFormat composable 覆盖 currency/percent/price/date/dateTime
9. **CSV 导出** — BOM + UTF-8 兼容 + 正确转义
10. **Element Plus 暗色主题覆盖** — 全面的 CSS 变量覆盖

---

## 五、修复优先级建议

**立即修复 (P0/P1)**：9 项
- 回测页布局重构 (最大工作量)
- 表单运行中禁用
- 删除确认弹窗
- 表单数值范围修正
- Dashboard 响应式
- 权益曲线配色
- 登录页品牌色统一
- 交易方向 tag 颜色修正

**下一迭代 (P2)**：14 项
- 组件错误态补充
- 硬编码色值替换为 CSS 变量
- 日期跨字段校验
- 保存配置按钮

**记录 (P3)**：5 项
- 细微尺寸差异
- 中英文统一
