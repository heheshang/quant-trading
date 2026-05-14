# Portfolio 组合权益使用指南

> 本文档介绍 Portfolio 组合权益模块的完整操作流程，包括组合总览查看、持仓查询、策略绩效对比、权益曲线分析以及前端集成方法。

---

## 目录

- [概述](#概述)
- [核心概念](#核心概念)
- [查看组合总览](#查看组合总览)
- [查询持仓列表](#查询持仓列表)
- [策略绩效对比](#策略绩效对比)
- [权益曲线分析](#权益曲线分析)
- [管理员操作](#管理员操作)
- [前端集成指南](#前端集成指南)
- [已知限制](#已知限制)
- [常见问题](#常见问题)

---

## 概述

Portfolio 组合权益模块提供量化交易系统的组合级数据聚合能力，帮助用户：

- 实时查看组合总资产、当日盈亏和累计盈亏
- 查询当前持仓明细及浮动盈亏
- 横向对比多策略绩效表现
- 分析权益曲线走势

本模块数据来源于模拟账户（paper_accounts）、持仓记录（positions）、订单记录（orders）和回测结果（backtest_results），通过服务层聚合计算后对外提供 REST API。

---

## 核心概念

### 组合权益

组合权益 = 所有模拟账户余额之和，包括现金和持仓市值。系统通过 `paper_accounts` 表聚合计算。

### 当日盈亏

当日盈亏 = 当日所有已成交订单的盈亏合计（含手续费），计算公式为：

```
daily_pnl = Σ (sign × (avg_fill_price - order_price) × filled_quantity - fee)
```

- 做多（Buy）：sign = +1
- 做空（Sell）：sign = -1

### 累计盈亏

累计盈亏 = 所有模拟账户的 `total_pnl` 之和，反映自开户以来的整体盈亏。

### 盈亏率

盈亏率 = 盈亏金额 / 初始资金 × 100%

初始资金 = 所有模拟账户的 `initial_balance` 之和。

### 持仓方向

| 方向 | 标识 | 说明 |
|------|------|------|
| 多头 | `long` | 买入开仓，价格上涨盈利 |
| 空头 | `short` | 卖出开仓，价格下跌盈利 |

### 权益快照

系统通过 `record_equity_snapshot` 函数定期记录权益快照到 `portfolio_equity_history` 表，权益曲线数据即来源于此。

---

## 查看组合总览

### 操作步骤

1. 登录系统，获取 access token
2. 调用 `GET /api/v1/portfolio/summary` 获取组合汇总
3. 在前端展示总览卡片

### 示例

```bash
# 获取组合汇总
curl -s 'http://localhost:8080/api/v1/portfolio/summary' \
  -H 'Authorization: Bearer <your_access_token>'
```

响应示例：

```json
{
  "code": 0,
  "data": {
    "total_equity": "100000.00",
    "daily_pnl": "1234.56",
    "daily_pnl_rate": "1.25",
    "cumulative_pnl": "15000.00",
    "cumulative_pnl_rate": "17.65",
    "total_positions": 5,
    "updated_at": "2026-05-14T12:00:00+00:00"
  }
}
```

### 数据解读

| 字段 | 含义 | 前端展示建议 |
|------|------|-------------|
| `total_equity` | 总资产 | 显示在主卡片，带 `¥` 前缀和千分位分隔 |
| `daily_pnl` | 当日盈亏 | 正数绿色 `+¥1,234.56`，负数红色 `-¥1,234.56` |
| `daily_pnl_rate` | 当日盈亏率 | 正数 `+1.25%`，负数 `-1.25%`，与金额同色 |
| `cumulative_pnl` | 累计盈亏 | 同当日盈亏的着色规则 |
| `cumulative_pnl_rate` | 累计盈亏率 | 同当日盈亏率的着色规则 |
| `total_positions` | 持仓数 | 可作为 Badge 显示在持仓区域标题旁 |

> **数值格式约定**：所有金额字段为 string 类型，保留 2 位小数；百分比为 string 类型，保留 2 位小数（不含 `%` 符号，由前端拼接）。采用 string 类型是为了避免浮点精度问题（B4 契约）。

---

## 查询持仓列表

### 操作步骤

1. 调用 `GET /api/v1/portfolio/positions` 获取持仓列表
2. 可通过 `symbol` 参数按交易对筛选
3. 通过 `page` 和 `size` 参数控制分页

### 示例

```bash
# 获取全部持仓
curl -s 'http://localhost:8080/api/v1/portfolio/positions' \
  -H 'Authorization: Bearer <your_access_token>'

# 筛选 BTCUSDT 持仓
curl -s 'http://localhost:8080/api/v1/portfolio/positions?symbol=BTCUSDT' \
  -H 'Authorization: Bearer <your_access_token>'

# 分页查询
curl -s 'http://localhost:8080/api/v1/portfolio/positions?page=2&size=10' \
  -H 'Authorization: Bearer <your_access_token>'
```

### 响应解读

```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "symbol": "BTCUSDT",
        "side": "long",
        "quantity": "0.50000000",
        "avg_price": "65000.00000000",
        "current_price": "66000.00000000",
        "unrealized_pnl": "500.00",
        "unrealized_pnl_rate": "1.54"
      }
    ],
    "total": 5,
    "page": 1,
    "size": 20
  }
}
```

| 字段 | 含义 | 前端展示建议 |
|------|------|-------------|
| `symbol` | 交易对 | 表格首列，可点击跳转交易对详情 |
| `side` | 持仓方向 | 用 `el-tag` 显示：long → 绿色「多」，short → 红色「空」 |
| `quantity` | 数量 | 等宽字体，8 位小数（前端可截断尾部零） |
| `avg_price` | 均价 | 等宽字体，8 位小数 |
| `current_price` | 现价 | 等宽字体，8 位小数 |
| `unrealized_pnl` | 浮动盈亏 | 双行展示：金额在上（14px）+ 百分比在下（12px，较浅），正绿负红 |
| `unrealized_pnl_rate` | 浮动盈亏率 | 与 `unrealized_pnl` 同行展示 |

### 持仓方向筛选

> **当前限制**：后端暂不支持 `side` 筛选参数。如需前端实现方向筛选，请在客户端过滤：

```typescript
// 前端临时筛选方案（后端支持 side 参数后可移除）
const filteredPositions = positions.filter(p => 
  !selectedSide || p.side === selectedSide
)
```

### 分页参数

| 参数 | 默认值 | 范围 | 说明 |
|------|--------|------|------|
| `page` | 1 | ≥1 | 页码，从 1 开始 |
| `size` | 20 | 1–100 | 每页条数 |

> **注意**：后端默认 `size=20`，前端 composable 默认 `size=10`。建议前端在请求时显式传入 `size=10` 以保持一致。

---

## 策略绩效对比

### 操作步骤

1. 调用 `GET /api/v1/portfolio/performance` 获取所有策略绩效
2. 在策略绩效对比表中展示

### 示例

```bash
# 获取策略绩效对比
curl -s 'http://localhost:8080/api/v1/portfolio/performance' \
  -H 'Authorization: Bearer <your_access_token>'
```

### 响应解读

```json
{
  "code": 0,
  "data": {
    "strategies": [
      {
        "strategy_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "strategy_name": "双均线交叉",
        "total_pnl": "5000.00",
        "total_pnl_rate": "12.50",
        "max_drawdown": "3.45",
        "trade_count": 42,
        "win_rate": "65.00"
      }
    ]
  }
}
```

### 绩效指标说明

| 指标 | 说明 | 评估标准 |
|------|------|---------|
| `total_pnl` | 策略总盈亏 | 正数盈利，负数亏损 |
| `total_pnl_rate` | 总盈亏率 | 同上 |
| `max_drawdown` | 最大回撤 | 始终为正值，越小越好（<5% 优秀，>10% 需警惕） |
| `trade_count` | 交易次数 | 评估策略活跃度，过少可能不具统计意义 |
| `win_rate` | 胜率 | ≥60% 优秀（绿色），40–60% 一般（黄色），<40% 较差（红色） |

### 绩效卡片颜色规则

```typescript
// 胜率颜色阈值
function winRateColor(rate: number): string {
  if (rate >= 60) return 'var(--color-positive)'   // 绿色
  if (rate >= 40) return 'var(--color-warning)'    // 黄色
  return 'var(--color-negative)'                    // 红色
}

// 最大回撤颜色：始终红色
const maxDrawdownColor = 'var(--color-negative)'
```

> **当前限制**：后端仅返回各策略独立绩效，不包含组合整体指标（`max_drawdown`/`sharpe_ratio`/`win_rate`）。绩效卡片区域暂显示 `--`。

---

## 权益曲线分析

### 操作步骤

1. 调用 `GET /api/v1/portfolio/equity_curve` 获取权益曲线
2. 可选：指定日期范围和粒度
3. 在 ECharts 中渲染面积图

### 示例

```bash
# 默认日线权益曲线
curl -s 'http://localhost:8080/api/v1/portfolio/equity_curve' \
  -H 'Authorization: Bearer <your_access_token>'

# 最近 30 天日线
curl -s 'http://localhost:8080/api/v1/portfolio/equity_curve?start_date=2026-04-14&end_date=2026-05-14&granularity=day' \
  -H 'Authorization: Bearer <your_access_token>'

# 小时线（适合短线交易者）
curl -s 'http://localhost:8080/api/v1/portfolio/equity_curve?start_date=2026-05-14&granularity=hour' \
  -H 'Authorization: Bearer <your_access_token>'
```

### 粒度选择建议

| 粒度 | 适用场景 | 数据密度 |
|------|---------|---------|
| `hour` | 日内交易分析、短线策略 | 每小时一个数据点 |
| `day` | 日常监控、中长线策略（默认） | 每天一个数据点 |
| `week` | 月度/季度复盘 | 每周一个数据点（当前实现同 `day`） |

### 前端 ECharts 配置参考

```typescript
const chartOption = {
  xAxis: {
    type: 'category',
    data: points.map(p => p.timestamp),
    axisLabel: { fontFamily: 'JetBrains Mono, monospace' }
  },
  yAxis: {
    type: 'value',
    axisLabel: {
      formatter: (val: number) => '¥' + formatMoney(String(val)),
      fontFamily: 'JetBrains Mono, monospace'
    }
  },
  series: [{
    type: 'line',
    smooth: 0.3,
    lineStyle: { color: '#7170ff', width: 2 },
    areaStyle: {
      color: {
        type: 'linear',
        x: 0, y: 0, x2: 0, y2: 1,
        colorStops: [
          { offset: 0, color: 'rgba(113,112,255,0.20)' },
          { offset: 1, color: 'rgba(113,112,255,0.00)' }
        ]
      }
    },
    data: points.map(p => parseFloat(p.equity))
  }],
  tooltip: {
    trigger: 'axis',
    backgroundColor: '#212223',
    borderColor: 'rgba(255,255,255,0.08)',
    textStyle: { fontFamily: 'JetBrains Mono, monospace' },
    formatter: (params: any) => {
      const p = params[0]
      return `${p.name}<br/>权益: ¥${formatMoney(p.value)}`
    }
  },
  grid: { left: 64, right: 24, top: 16, bottom: 32 }
}
```

---

## 管理员操作

管理员（`role=admin`）可查看任意用户的组合数据，通过 `user_id` 查询参数指定目标用户。

### 查看指定用户组合

```bash
# 查看指定用户的组合汇总
curl -s 'http://localhost:8080/api/v1/portfolio/summary?user_id=550e8400-e29b-41d4-a716-446655440000' \
  -H 'Authorization: Bearer <admin_access_token>'

# 查看指定用户的持仓
curl -s 'http://localhost:8080/api/v1/portfolio/positions?user_id=550e8400-e29b-41d4-a716-446655440000' \
  -H 'Authorization: Bearer <admin_access_token>'
```

### 权限校验规则

| 请求者 | 目标 | 结果 |
|--------|------|------|
| 普通用户 | 自己 | ✅ 允许 |
| 普通用户 | 他人 | ❌ 40301 错误 |
| 管理员 | 自己 | ✅ 允许 |
| 管理员 | 他人 | ✅ 允许 |
| 未认证 | 任意 | ❌ 40101 错误 |

---

## 前端集成指南

### 项目文件结构

```
frontend/src/
├── types/portfolio.ts        # TypeScript 类型定义
├── api/portfolio.ts          # API 调用函数
└── views/portfolio/
    └── PortfolioView.vue     # 组合权益页面组件
```

### API 调用封装

前端已封装好 API 调用函数（`src/api/portfolio.ts`）：

```typescript
import client from './client'
import type {
  PortfolioSummary,
  PaginatedPositions,
  PortfolioPositionsQuery,
  PortfolioPerformance,
  EquityCurve,
  EquityCurveQuery,
} from '@/types/portfolio'

// 组合汇总
export function getPortfolioSummary(userId?: string): Promise<PortfolioSummary> {
  return client.get('/portfolio/summary', { params: { user_id: userId } })
}

// 持仓列表
export function listPortfolioPositions(params?: PortfolioPositionsQuery): Promise<PaginatedPositions> {
  return client.get('/portfolio/positions', { params })
}

// 策略绩效
export function getPortfolioPerformance(userId?: string): Promise<PortfolioPerformance> {
  return client.get('/portfolio/performance', { params: { user_id: userId } })
}

// 权益曲线
export function getEquityCurve(params?: EquityCurveQuery): Promise<EquityCurve> {
  return client.get('/portfolio/equity_curve', { params })
}
```

### Vue 组件集成示例

```vue
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getPortfolioSummary, listPortfolioPositions, getPortfolioPerformance, getEquityCurve } from '@/api/portfolio'
import type { PortfolioSummary, PortfolioPosition, StrategyPerformance, EquityCurvePoint } from '@/types/portfolio'

const summary = ref<PortfolioSummary | null>(null)
const positions = ref<PortfolioPosition[]>([])
const strategies = ref<StrategyPerformance[]>([])
const equityPoints = ref<EquityCurvePoint[]>([])
const loading = ref(true)

onMounted(async () => {
  try {
    const [summaryRes, positionsRes, perfRes, curveRes] = await Promise.all([
      getPortfolioSummary(),
      listPortfolioPositions({ page: 1, size: 10 }),
      getPortfolioPerformance(),
      getEquityCurve({ start_date: '2026-04-14', end_date: '2026-05-14', granularity: 'day' })
    ])
    summary.value = summaryRes
    positions.value = positionsRes.items
    strategies.value = perfRes.strategies
    equityPoints.value = curveRes.points
  } catch (error) {
    console.error('Failed to load portfolio data:', error)
  } finally {
    loading.value = false
  }
})
</script>
```

### 数值格式化工具

```typescript
// 金额格式化（千分位 + 2 位小数）
function formatMoney(value: string): string {
  const num = parseFloat(value)
  return num.toLocaleString('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  })
}

// 百分比格式化
function formatRate(value: string): string {
  const num = parseFloat(value)
  const prefix = num > 0 ? '+' : ''
  return `${prefix}${num.toFixed(2)}%`
}

// 盈亏颜色
function pnlColor(value: string): string {
  const num = parseFloat(value)
  if (num > 0) return 'var(--color-positive)'
  if (num < 0) return 'var(--color-negative)'
  return 'var(--color-neutral)'
}
```

### 错误处理最佳实践

```typescript
import { ElMessage } from 'element-plus'
import { useRouter } from 'vue-router'

const router = useRouter()

async function fetchWithErrorHandler(fn: () => Promise<any>) {
  try {
    return await fn()
  } catch (error: any) {
    const code = error?.response?.data?.code
    
    if (code === 40101 || code === 40102) {
      // 401: 跳转登录页
      ElMessage.error('登录已过期，请重新登录')
      router.push('/login')
    } else if (code === 40301) {
      // 403: 权限不足
      ElMessage.error('无权限查看该组合')
    } else {
      // 其他错误：显示错误信息
      ElMessage.error('加载失败，请稍后重试')
    }
    throw error
  }
}
```

---

## 已知限制

| 编号 | 限制 | 影响 | 建议规避方案 |
|------|------|------|-------------|
| BA-03 | `/performance` 缺少整体 `max_drawdown`/`sharpe_ratio`/`win_rate` | 绩效卡片显示 `--` | 前端显示占位符 `--`，后续版本后端补充后自动填充 |
| BA-05 | `/positions` 不支持 `side` 筛选 | 方向筛选前端失效 | 前端客户端过滤 `positions.filter(p => p.side === selectedSide)` |
| BA-08 | `current_price` 使用 `avg_entry_price` 占位 | 浮动盈亏精度不足 | 前端标注「数据仅供参考」，待接入行情服务 |
| BA-04 | `week` 粒度实现与 `day` 相同 | 周线权益曲线数据不准 | 使用 `day` 粒度替代，或前端自行聚合 |
| WS | WebSocket 实时更新未接入 | 数值无实时刷新 | 使用手动刷新按钮，后续版本接入 WS 推送 |

---

## 常见问题

### Q1: 为什么总资产和各账户余额之和不一致？

总资产（`total_equity`）是从 `paper_accounts` 聚合的，包含所有模拟账户的余额。如果用户有多个模拟账户（如不同策略使用不同账户），总资产是所有账户余额的总和。如果发现不一致，请检查是否有新创建的账户尚未产生交易。

### Q2: 当日盈亏为什么是 0？

可能原因：
- 当日没有已成交的订单
- 所有成交订单的盈亏恰好抵消
- 系统以 UTC 时区计算「当日」，请确认时区是否正确

### Q3: 浮动盈亏为什么和预期不一致？

当前版本 `current_price` 使用 `avg_entry_price` 作为占位值（BA-08），导致浮动盈亏约等于持仓记录中的 `unrealized_pnl` 字段值，而非实时计算的浮动盈亏。待接入行情服务后，`current_price` 将更新为实时价格。

### Q4: 如何获取历史某一天的权益快照？

使用 `/equity_curve` 端点指定日期范围：

```bash
curl -s 'http://localhost:8080/api/v1/portfolio/equity_curve?start_date=2026-05-01&end_date=2026-05-01&granularity=day' \
  -H 'Authorization: Bearer <your_access_token>'
```

### Q5: 管理员查看他人组合时返回 40301？

请确认：
1. 当前用户的 `role` 字段是否为 `admin`
2. `user_id` 参数是否为有效的 UUID 格式
3. 目标用户是否存在

### Q6: 权益曲线为空？

可能原因：
- 用户尚未产生任何权益快照（新用户或未运行策略）
- 日期范围内无数据（请扩大日期范围重试）
- 权益快照由 `record_equity_snapshot` 函数写入，需确认系统是否正常运行快照任务

### Q7: 分页参数 size 前后端默认值不一致？

后端 `size` 默认 20，前端 composable 默认 10。建议前端在请求时显式传入 `size=10` 以保持一致，避免展示条数与预期不符。

### Q8: 策略绩效数据为什么和回测结果不同？

`/performance` 端点的数据来源：
- `total_pnl` 和 `total_pnl_rate` 取自 `backtest_results.metrics.total_return`
- `max_drawdown` 和 `win_rate` 取自 `backtest_results.metrics` 中对应字段
- `trade_count` 统计的是**实盘**已成交订单数，而非回测交易次数

如果策略尚未在实盘运行，`trade_count` 可能为 0。
