# 套利管理 UI/UE 走查清单

> 对比设计稿 Design_Arbitrage.md v1.0 与前端实现
> 生成日期：2026-06-01
> 设计师：Designer

---

## 检查项统计

| 优先级 | 数量 |
|--------|------|
| P0 (阻塞) | 30 |
| P1 (重要) | 22 |
| P2 (优化) | 12 |
| **合计** | **64** |

---

## 1. 页面路由与导航

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 1 | 路由 `/arbitrage` 指向套利管理页 | §2 | ArbitrageView 组件 | P0 |
| 2 | 侧边栏菜单「套利管理」+ Zap 图标 | §2 | 侧边栏导航项 | P1 |
| 3 | 页面标题「套利管理」字号 18px/font-weight 600 | §2.2 | h1 或 span | P0 |
| 4 | 「新建套利对」按钮在标题右侧 | §2.1 | flex justify-between | P1 |

---

## 2. SpreadChart 价差监控图表

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 5 | ECharts 实时价差折线图 | §2.2 | ECharts line chart | P0 |
| 6 | X轴时间自适应格式 | §2.2 | formatter | P1 |
| 7 | Y轴价差百分比显示 | §2.2 | axisLabel formatter | P0 |
| 8 | 触发阈值虚线标记 | §6.2 | markLine | P0 |
| 9 | 面积渐变填充 | §6.2 | areaStyle gradient | P1 |
| 10 | 透明图表背景 | §6.2 | backgroundColor: transparent | P1 |
| 11 | Tooltip 显示时间 + 价差 + 百分比 | §2.2 | tooltip formatter | P1 |
| 12 | WebSocket 实时数据更新 | §5.1 | ws subscribe | P0 |
| 13 | 价差达到阈值时图表高亮 | §4.3 | visualMap | P0 |
| 14 | 空数据：「等待价差信号...」 | §4.2 | placeholder text | P1 |

---

## 3. ArbitrageStatsCards 套利统计卡片

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 15 | 四大卡片：总套利次数/成功次数/成功率/累计收益 | §2.2 | 4x Grid | P0 |
| 16 | 卡片主值 22px/font-weight 700 | §2.2 | card value | P0 |
| 17 | 数值千分位分隔 | §6.2 | formatMoney | P0 |
| 18 | 百分比保留2位小数 | §6.2 | formatPercent | P0 |
| 19 | 成功率颜色：>=60%绿, 40-60%黄, <40%红 | §4.3 | color logic | P0 |
| 20 | 卡片悬停 translate-y -2px | §6.2 | CSS transition | P1 |
| 21 | 数值变更闪烁动画 | §2.2 | flash animation | P1 |
| 22 | 等宽字体 JetBrains Mono | §6.2 | font-family | P1 |

---

## 4. ArbitragePairTable 套利对表格

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 23 | 列：套利对/交易所A/交易所B/当前价差/状态/操作 | §2.2 | el-table 6列 | P0 |
| 24 | 套利对显示交易对符号（如 BTC/USDT） | §2.2 | symbol | P0 |
| 25 | 交易所显示图标 + 名称 | §2.2 | exchange-icon + name | P0 |
| 26 | 当前价差显示百分比 + 颜色 | §4.3 | spread-cell | P0 |
| 27 | 价差接近阈值（70-99%）黄色 | §4.3 | warning color | P0 |
| 28 | 价差触发阈值（>=100%）红色闪烁 | §4.3 | danger + animation | P0 |
| 29 | 状态 Tag：运行中绿色/已暂停灰色/异常红色 | §4.4 | status-pill | P0 |
| 30 | 操作按钮：详情/执行/暂停/删除 | §3.2-3.4 | el-button text | P0 |
| 31 | 执行按钮绿色 | §3.3 | type=success | P0 |
| 32 | 删除按钮红色 | §3.2 | type=danger | P0 |
| 33 | 分页控件每页10条 | §2.2 | el-pagination | P0 |
| 34 | 空数据：空状态 + 「暂无套利对」 | §4.2 | el-empty | P1 |

---

## 5. ArbitragePairDialog 新建/编辑对话框

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 35 | 对话框宽度 520px | §6.3 | width: 520px | P0 |
| 36 | 交易对下拉选择 | §3.1 | el-select | P0 |
| 37 | 交易所A下拉（不能与B相同） | §3.1 | el-select + validator | P0 |
| 38 | 交易所B下拉（不能与A相同） | §3.1 | el-select + validator | P0 |
| 39 | 价差阈值输入（百分比） | §3.1 | el-input-number | P0 |
| 40 | 每笔交易数量输入 | §3.1 | el-input-number | P0 |
| 41 | 执行模式下拉：自动/手动 | §3.1 | el-select | P0 |
| 42 | 表单必填校验 | §3.1 | el-form rules | P0 |
| 43 | 保存按钮调用 `POST /api/arbitrage/pairs` | §5.1 | POST 请求 | P0 |
| 44 | 创建成功关闭对话框并刷新 | §3.1 | refresh | P0 |

---

## 6. ArbitrageDetailDrawer 套利详情抽屉

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 45 | 点击「详情」打开右侧抽屉 | §3.2 | el-drawer | P1 |
| 46 | 显示历史套利记录表格 | §3.2 | history table | P0 |
| 47 | 显示收益曲线图表 | §3.2 | ECharts line | P1 |
| 48 | 显示成功率趋势 | §3.2 | success rate | P1 |
| 49 | 抽屉宽度 600px | §3.2 | width: 600px | P1 |

---

## 7. 手动执行套利

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 50 | 点击「执行」按钮 | §3.3 | el-button | P0 |
| 51 | 弹出确认对话框（当前价差信息） | §3.3 | ElMessageBox.confirm | P0 |
| 52 | 确认后调用 `POST /api/arbitrage/pairs/:id/execute` | §5.1 | execute request | P0 |
| 53 | 执行中按钮 loading | §3.3 | :loading | P0 |
| 54 | 执行成功 Toast + 刷新统计 | §3.3 | ElMessage.success | P0 |
| 55 | 执行失败 Toast + 错误详情 | §3.3 | ElMessage.error | P0 |

---

## 8. 启用/暂停套利对

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 56 | 点击「暂停」按钮 | §3.4 | el-button | P0 |
| 57 | 调用 `PUT /api/arbitrage/pairs/:id/status` | §5.1 | PUT status=paused | P0 |
| 58 | 点击「启用」按钮 | §3.4 | el-button | P0 |
| 59 | 调用 `PUT /api/arbitrage/pairs/:id/status` | §5.1 | PUT status=active | P0 |
| 60 | 状态变更刷新表格 | §3.4 | refresh table | P0 |

---

## 9. 状态与交互细节

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 61 | 页面初始骨架屏 + 图表占位 | §4.1 | el-skeleton | P0 |
| 62 | 实时数据 loading spinner | §4.1 | v-loading | P1 |
| 63 | 网络断开提示 | §4.3 | ElAlert banner | P0 |
| 64 | WebSocket 重连机制 | §3.1 | reconnect | P1 |

---

## 10. 响应式适配

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 65 | Desktop (>=1280px) 图表 + 表格双列 | §7.1 | grid 双列 | P0 |
| 66 | Laptop (1024-1279px) 图表上方 | §7.1 | grid 单列 | P0 |
| 67 | Tablet (768-1023px) 图表可折叠 | §7.1 | collapsible | P1 |
| 68 | Mobile (<768px) 卡片式展示 | §7.1 | 卡片列表 | P1 |

---

## 优先级标记说明

- **P0 (红色)**: 必须实现，阻塞性问题
- **P1 (黄色)**: 重要功能，建议实现
- **P2 (绿色)**: 优化项，可后续迭代
