# Portfolio 组合权益模块 UI/UE 走查清单

> 对比设计稿 Design_Portfolio.md v1.0 与前端实现
> 生成日期：2026-05-14
> 设计师：Designer

---

## 检查项统计

| 优先级 | 数量 |
|--------|------|
| P0 (阻塞) | 42 |
| P1 (重要) | 39 |
| P2 (优化) | 10 |
| **合计** | **91** |

---

## 1. 页面路由与导航

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 1 | 路由 `/portfolio` 指向组合总览仪表板 | §6 路由设计 | `PortfolioDashboard` 组件 | P0 |
| 2 | 路由 `/portfolio/positions` 指向持仓汇总页 | §6 路由设计 | `PositionSummary` 组件 | P0 |
| 3 | 路由 `/portfolio/performance` 指向策略绩效页 | §6 路由设计 | `PerformanceComparison` 组件 | P0 |
| 4 | 侧边栏菜单项「组合权益」+ PieChart 图标 | §6 | 侧边栏导航项 | P1 |
| 5 | Tab 切换保持页面状态（非全量重渲染） | — | keep-alive 或状态缓存 | P2 |

## 2. PortfolioHeader

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 6 | 页面标题「组合权益」字号 18px/font-weight 600 | §2.2 | h1 或 span | P0 |
| 7 | DateRangePicker 默认最近 30 天 | §2.2 | el-date-picker daterange | P0 |
| 8 | GranularitySelector 选项：小时/日/周，默认「日」 | §2.2 | el-segmented | P0 |
| 9 | 刷新按钮 circle + 旋转动画 0.6s | §2.2 | el-button circle + CSS animation | P1 |
| 10 | Header 底部 border `--color-border` | §2.2 | border-bottom | P1 |

## 3. EquityOverviewCards 四大指标

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 11 | 总资产卡片：标签+主值(28px 700)+子值 | §2.3 | TotalEquityCard | P0 |
| 12 | 累计盈亏卡片：正数绿色+前缀+，负数红色 | §2.3 | CumulativePnlCard + 颜色逻辑 | P0 |
| 13 | 当日盈亏卡片：同上颜色规则 | §2.3 | DailyPnlCard | P0 |
| 14 | 收益率卡片：数值+迷你sparkline | §2.3 | ReturnRateCard + SVG sparkline | P0 |
| 15 | 金额千分位分隔，保留2位小数，前缀¥ | §2.3 数值格式化 | formatMoney 函数 | P0 |
| 16 | 百分比保留2位小数，正数前缀+ | §2.3 | formatPercent 函数 | P0 |
| 17 | 零值使用 `--color-neutral` | §2.3 | pnl-neutral class | P0 |
| 18 | 卡片悬停 translate-y -2px + box-shadow | §2.3 + §10.1 | CSS transition | P1 |
| 19 | 卡片右侧图标（Wallet/TrendingUp/Calendar/DataLine） | §2.3 | el-icon 24px 40% 透明 | P1 |
| 20 | 数值闪烁动画（WS更新时） | §2.3 + §10.2 | flash-positive/negative 2s | P1 |
| 21 | 等宽字体 JetBrains Mono 用于数值 | §1.1 | font-family var(--font-mono) | P1 |

## 4. EquityCurveChart

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 22 | ECharts 面积图，线色 #7170ff | §2.4 | area chart + lineStyle | P0 |
| 23 | 面积渐变 #7170ff 20%→0% | §2.4 | LinearGradient | P0 |
| 24 | X轴时间自适应格式 | §2.4 | 小时→HH:mm, 日→MM-DD | P1 |
| 25 | Y轴千分位 + ¥前缀 | §2.4 | axisLabel formatter | P1 |
| 26 | Tooltip 十字线 + 暗色背景 #212223 | §2.4 | trigger: 'axis' | P1 |
| 27 | 透明背景 | §2.4 | backgroundColor: 'transparent' | P1 |
| 28 | 空态：灰色虚线 + 文字提示 | §2.4 | 虚线标记 | P2 |
| 29 | 粒度/日期切换图表重绘动画 300ms | §2.4 | ECharts 动画配置 | P2 |

## 5. PositionSummaryTable

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 30 | 列：交易对/方向/数量/均价/现价/浮动盈亏 | §2.6 | el-table 6列 | P0 |
| 31 | SideTag：多头绿色「多」，空头红色「空」 | §2.6 | side-tag--long/short | P0 |
| 32 | UnrealizedPnlCell：金额+百分比，颜色跟随正负 | §2.6 | pnl-cell + 双行 | P0 |
| 33 | 多头行背景 `--color-long-bg`，空头行 `--color-short-bg` | §2.6 | row-class-name | P0 |
| 34 | 行悬停背景 `rgba(255,255,255,0.04)` | §2.6 | el-table row-hover | P1 |
| 35 | 默认按浮动盈亏绝对值降序 | §2.6 | sortable + sort-method | P1 |
| 36 | 分页：每页10条 + 总数显示 | §2.6 | el-pagination small | P1 |
| 37 | 数值使用等宽字体 | §2.6 | mono-cell class | P1 |
| 38 | 搜索交易对筛选 | §3.2 | el-input + Search icon | P1 |
| 39 | 持仓汇总标题 + Badge 数量 | §2.6 | surface-card__title + el-badge | P2 |
| 40 | 空数据：「暂无持仓」EmptyPortfolio 组件 | §11.1 | el-empty 或自定义 | P1 |

## 6. PerformanceMetricsCards

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 41 | 最大回撤卡片：值 `--color-negative` | §2.7 | MaxDrawdownCard | P0 |
| 42 | 夏普率卡片：≥2绿, 1-2默认, <1红 | §2.7 | sharpeClass 逻辑 | P0 |
| 43 | 胜率卡片：≥60%绿, 40-60%默认, <40%红 | §2.7 | winRateClass 逻辑 | P0 |
| 44 | 三卡片水平排列，间距 12px | §2.7 | grid 3列 | P1 |
| 45 | 值字号 22px/700 等宽字体 | §2.7 | font-size + font-weight | P1 |

## 7. StrategyComparisonTable

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 46 | 列：策略名/总盈亏/盈亏率/回撤/交易数/胜率 | §2.8 | el-table 6列 | P0 |
| 47 | 策略名称可点击（链接色 `--color-accent`） | §2.8 | strategy-link class | P1 |
| 48 | 盈亏列颜色跟随正负 | §2.8 | pnlClass | P0 |
| 49 | 回撤列统一 `--color-negative` | §2.8 | pnl-negative | P0 |
| 50 | 默认按总盈亏降序 | §2.8 | sortable | P1 |
| 51 | 行悬停 `rgba(255,255,255,0.04)` | §2.8 | el-table row-hover | P1 |

## 8. 持仓汇总独立页 (/portfolio/positions)

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 52 | 方向筛选 segmented：全部/多头/空头 | §3.2 | el-segmented | P0 |
| 53 | 交易对搜索输入框 | §3.2 | el-input + Search | P1 |
| 54 | 排序方式下拉：浮动盈亏↓/↑, 数量↓, 交易对A-Z | §3.2 | el-select | P1 |
| 55 | 占比列：数值+迷你进度条(4px高) | §3.3 | progress-mini | P1 |
| 56 | 占比进度条颜色跟随方向（多头绿/空头红） | §3.3 | progress-mini__fill | P1 |

## 9. 策略绩效对比页 (/portfolio/performance)

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 57 | 三大指标卡片（与仪表板相同，3列宽） | §4.1 | perf cards 3列 | P0 |
| 58 | 多策略权益曲线叠加图 | §4.2 | ECharts multi-line | P0 |
| 59 | 叠加图图例可点击切换显示/隐藏 | §4.2 | ECharts legend | P1 |
| 60 | 策略对比表（增加 Sharpe 列） | §4.1 | el-table 7列 | P1 |

## 10. API 数据流与 composable

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 61 | 页面加载并行请求 4 个 API | §7.1 | Promise.all | P0 |
| 62 | usePortfolioSummary composable | §7.3 | composable + ref/loading/error | P0 |
| 63 | usePortfolioPositions composable | §7.3 | composable + filters/pagination | P0 |
| 64 | useEquityCurve composable | §7.3 | composable + granularity/dateRange | P0 |
| 65 | usePortfolioPerformance composable | §7.3 | composable + ref/loading | P0 |
| 66 | 筛选/分页操作触发对应 API 请求 | §7.2 | watch + fetch | P1 |
| 67 | 金额使用 string 类型传输（ADR B4） | §7 + ADR | API 契约 | P0 |

## 11. WebSocket 实时更新

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 68 | subscribe 三个频道 on mount | §8.3 | onMounted subscribe | P0 |
| 69 | unsubscribe 三个频道 on unmount | §8.3 | onUnmounted unsubscribe | P0 |
| 70 | Summary 更新：合并字段 + 闪烁动画 | §8.2 | applyWsUpdate + triggerFlash | P0 |
| 71 | Position 更新：按 symbol+side 查找行更新 | §8.2 | 行级更新 | P1 |
| 72 | Equity 追加：ECharts appendData | §8.2 | 无全量重绘 | P1 |
| 73 | 网络断开 → 重连 + 全量刷新 | §8.3 | 重连逻辑 | P1 |
| 74 | Tab 切换保持连接，暂停渲染 | §8.3 | visibility change | P2 |

## 12. 响应式适配

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 75 | ≥1200px：四卡片4列 + 60/40分栏 | §9.1 | grid-template-columns | P0 |
| 76 | 768-1199px：四卡片2×2 + 单栏 | §9.2 | media query | P1 |
| 77 | <768px：四卡片单列 + 卡片列表 | §9.3 | media query | P1 |
| 78 | 平板端隐藏「均价」列 | §9.2 | v-if 或 responsive column | P2 |
| 79 | 移动端表格转卡片列表 | §9.3 | 条件渲染 | P2 |
| 80 | 移动端筛选栏折叠为 drawer | §9.3 | el-drawer | P2 |

## 13. 空状态与错误处理

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 81 | 新用户空状态：图标+文案+「前往交易」按钮 | §11.1 | EmptyPortfolio | P0 |
| 82 | 无持仓空状态 | §11.1 | EmptyPosition | P1 |
| 83 | 无策略空状态 | §11.1 | EmptyStrategy | P1 |
| 84 | API 401 跳转登录 | §11.2 | router.push /login | P0 |
| 85 | API 403 错误页 | §11.2 | el-result 403 | P0 |
| 86 | API 500 错误页 + 重试 | §11.2 | el-result 500 | P0 |
| 87 | WS 断开顶部横幅 | §11.2 | 通知横幅 | P1 |
| 88 | 加载超时(>10s)提示 | §11.2 | 超时检测 | P2 |

## 14. 设计 Token 继承

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 89 | 所有 CSS 变量与全局设计系统一致 | §1.1 | :root 变量 | P0 |
| 90 | 买入色 #67C23A / 卖出色 #F56C6C (Element Plus) | §1.1 | --color-buy/sell | P0 |
| 91 | 字体 Inter + JetBrains Mono | §1.1 | font-family | P0 |
| 92 | 间距基础单元 4px | §1.1 | padding/margin | P1 |
| 93 | 面板 border-radius 12px | §1.1 | border-radius | P1 |
| 94 | 骨架屏 shimmer 动画 | §10.5 | skeleton-line + animation | P2 |

---

## 检查方法

1. 逐项打开对应页面，对照设计稿截图/原型
2. 检查 CSS 变量是否正确继承
3. 检查颜色逻辑（正/负/零）是否正确
4. 检查 WS 推送时闪烁动画
5. 检查空状态/错误状态展示
6. 检查响应式断点（375px / 768px / 1200px / 1440px）
7. 检查数值格式化（千分位、小数位、正负前缀）
