# API Key 管理 UI/UE 走查清单

> 对比设计稿 Design_ApiKeyManagement.md v1.0 与前端实现
> 生成日期：2026-06-01
> 设计师：Designer

---

## 检查项统计

| 优先级 | 数量 |
|--------|------|
| P0 (阻塞) | 26 |
| P1 (重要) | 20 |
| P2 (优化) | 10 |
| **合计** | **56** |

---

## 1. 页面路由与导航

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 1 | 路由 `/api-keys` 指向 API Key 管理页 | §2 | ApiKeyManagementView 组件 | P0 |
| 2 | 侧边栏菜单「API Key 管理」+ Key 图标 | §2 | 侧边栏导航项 | P1 |
| 3 | 页面标题「API Key 管理」字号 18px/font-weight 600 | §2.2 | h1 或 span | P0 |
| 4 | 「添加 Key」按钮在标题右侧 | §2.1 | flex justify-between | P1 |

---

## 2. ExchangeFilterTabs 交易所筛选

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 5 | 筛选标签：全部/Binance/OKX/Gate.io/Bybit | §2.1 | el-radio-group | P0 |
| 6 | 当前选中标签高亮 `--color-accent` | §2.1 | el-radio-button checked | P0 |
| 7 | 点击调用 `GET /api/apikeys?exchange=xxx` | §3.5 | fetch with param | P0 |
| 8 | 移动端筛选改为下拉选择 | §7.2 | el-select | P1 |

---

## 3. ApiKeyTable API Key 表格

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 9 | 列：标签/交易所/Key前缀/状态/添加时间/操作 | §2.1 | el-table 6列 | P0 |
| 10 | Key 前缀显示格式：BN***8F（脱敏） | §1.3 | 脱敏函数 | P0 |
| 11 | 交易所显示对应图标 + 名称 | §6.2 | exchange-icon + name | P0 |
| 12 | 状态 Pill：正常绿色/异常红色/未测试灰色 | §4.4 | status-pill | P0 |
| 13 | 操作按钮：测试/编辑/删除 | §3.1-3.3 | el-button text | P0 |
| 14 | 删除按钮红色 | §6.1 | type=danger | P0 |
| 15 | 分页控件每页10条 | §2.1 | el-pagination | P0 |
| 16 | 空数据：空状态 + 「暂无 API Key」 | §4.2 | el-empty | P1 |

---

## 4. ApiKeyDialog 添加/编辑对话框

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 17 | 对话框宽度 480px | §6.3 | width: 480px | P0 |
| 18 | 交易所下拉选择（Binance/OKX/Gate.io/Bybit） | §3.1 | el-select | P0 |
| 19 | 标签名称输入框（必填） | §3.1 | el-input | P0 |
| 20 | API Key 输入框（必填） | §3.1 | el-input | P0 |
| 21 | Secret Key 输入框（必填，加密） | §3.1 | el-input type=password | P0 |
| 22 | Secret Key 右侧显示/隐藏切换图标 | §6.3 | suffix icon | P0 |
| 23 | Passphrase 输入框（OKX 等可选） | §3.1 | el-input (conditional) | P1 |
| 24 | Passphrase 字段根据交易所动态显示 | §3.1 | v-if + exchange | P1 |
| 25 | 「测试连接」按钮 | §3.1 | el-button | P0 |
| 26 | 测试连接中按钮 loading | §3.1 | :loading | P1 |
| 27 | 测试结果成功绿色提示 | §3.1 | ElMessage.success | P0 |
| 28 | 测试结果失败红色 + 错误信息 | §3.1 | ElMessage.error + detail | P0 |
| 29 | 保存按钮调用 `POST /api/apikeys` | §5.1 | POST 请求 | P0 |
| 30 | 编辑时 Secret 不预填 | §1.3 | 空或 placeholder | P0 |
| 31 | 创建成功关闭对话框并刷新列表 | §3.1 | ElMessage + refresh | P0 |

---

## 5. 删除确认流程

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 32 | 点击删除弹出确认对话框 | §3.3 | ElMessageBox.confirm | P0 |
| 33 | 对话框显示该 Key 的前缀信息 | §3.3 | key prefix | P0 |
| 34 | 输入框要求填写 Key 前4位确认 | §3.3 | el-input + validator | P0 |
| 35 | 警告文字：「此操作不可撤销」 | §3.3 | Warning text | P0 |
| 36 | 确认按钮红色「确认删除」 | §3.3 | type=danger | P0 |
| 37 | 调用 `DELETE /api/apikeys/:id` | §5.1 | DELETE 请求 | P0 |
| 38 | 删除成功刷新列表 | §3.3 | refresh | P0 |

---

## 6. 连接测试功能

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 39 | 点击「测试」按钮调用 `POST /api/apikeys/:id/test` | §3.4 | test request | P0 |
| 40 | 测试中按钮显示 loading | §3.4 | :loading | P0 |
| 41 | 测试成功：绿色对勾 + 「连接正常」 | §3.4 | ElMessage.success | P0 |
| 42 | 测试失败：红色叉 + 错误详情（权限不足/Key无效） | §3.4 | ElMessage.error | P0 |
| 43 | 测试时间超过5秒显示超时 | §3.4 | timeout handler | P1 |

---

## 7. 状态与交互细节

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 44 | 页面初始骨架屏 | §4.1 | el-skeleton | P0 |
| 45 | 测试连接时禁用其他按钮 | §4.1 | :disabled | P1 |
| 46 | 测试失败 Toast + 具体错误 | §4.3 | ElMessage.error | P0 |
| 47 | 保存失败表单内错误提示 | §4.3 | el-form-item error | P0 |
| 48 | 网络错误全局 Toast | §4.3 | ElMessage.error | P1 |
| 49 | 添加成功 Toast | §3.1 | ElMessage.success | P1 |
| 50 | 编辑成功 Toast | §3.2 | ElMessage.success | P1 |
| 51 | 删除成功 Toast | §3.3 | ElMessage.success | P1 |

---

## 8. 响应式适配

| # | 检查项 | 设计稿 | 预期实现 | 优先级 |
|---|--------|--------|---------|--------|
| 52 | Desktop (>=1280px) 完整表格 | §7.1 | 完整布局 | P0 |
| 53 | Laptop (1024-1279px) 表格滚动 | §7.1 | overflow-x auto | P0 |
| 54 | Tablet (768-1023px) 表格简化为卡片 | §7.1 | 卡片布局 | P1 |
| 55 | Mobile (<768px) 纯卡片列表 | §7.1 | 卡片列表 | P1 |
| 56 | 移动端添加按钮固定底部 | §7.2 | fixed bottom | P1 |

---

## 优先级标记说明

- **P0 (红色)**: 必须实现，阻塞性问题
- **P1 (黄色)**: 重要功能，建议实现
- **P2 (绿色)**: 优化项，可后续迭代
