# ADR-017: 风控仪表盘前端

**ID**: ADR-017
**日期**: 2026-05-21
**状态**: Proposed
**关联**: ADR-013, ADR-016, PRD-Phase4-Risk-Management

---

## 页面布局

```
RiskDashboardView.vue
├── RiskStatusCard          # 连接状态 + 断线指示器
├── RiskRulesSummaryCard    # 规则状态（启用/暂停）
├── DailyLossGauge          # 当日亏损仪表盘
├── DrawdownChart           # 回撤曲线图
├── ActivePositionsPanel    # 当前持仓 + 止损状态
├── RiskLogTable            # 风控日志（分页）
└── EmergencyControls       # 紧急全平 + 暂停按钮（Admin）
```

---

## 核心组件

### 1. RiskStatusCard

```vue
<template>
  <div class="status-card" :class="connectionClass">
    <div class="status-indicator">
      <span class="dot" :class="connected ? 'green' : 'red'"></span>
      {{ connected ? 'Binance 已连接' : '断线中' }}
    </div>
    <div class="disconnect-time" v-if="!connected">
      断线时间: {{ elapsed }}s（超过 {{ threshold }}s 将暂停策略）
    </div>
    <div class="strategy-status">
      策略状态: {{ strategyPaused ? '⏸ 已暂停' : '✅ 运行中' }}
    </div>
  </div>
</template>
```

### 2. EmergencyControls

```vue
<script setup lang="ts">
// 紧急全平 — Admin only，二次确认
const handleEmergencyClose = async () => {
  if (!confirm('确定要紧急全平所有持仓吗？此操作不可撤销！')) return;
  if (!confirm('再次确认：紧急全平所有持仓？')) return;
  await riskApi.emergencyClose();
};
</script>
```

### 3. RiskLogTable

- 分页查询 `GET /api/v1/risk/logs`
- 展示：时间、规则类型、触发的阈值、实际值、执行动作
- 筛选：rule_type / 时间范围

### 4. RiskRulesEditor

- 字段：daily_loss_limit / single_trade_loss_ratio / max_drawdown_ratio
- ATR 配置：atr_period / atr_multiplier
- 支持热更新：`PUT /api/v1/risk/rules`

---

## API 集成

| 端点 | 用途 |
|------|------|
| GET /api/v1/risk/rules | 获取规则配置 |
| PUT /api/v1/risk/rules | 更新规则（热更新） |
| GET /api/v1/risk/logs | 风控日志 |
| POST /api/v1/risk/emergency-close | 紧急全平 |
| POST /api/v1/risk/pause-strategies | 暂停策略 |
| POST /api/v1/risk/resume-strategies | 恢复策略 |
| GET /api/v1/risk/connection-status | WebSocket 状态 |
| POST /api/v1/risk/check | 手动触发检查 |

---

## Composable

```typescript
// useRiskDashboard.ts
export const useRiskDashboard = () => {
  const rules = ref<RiskRules | null>(null);
  const logs = ref<RiskLog[]>([]);
  const connectionStatus = ref<ConnectionStatus | null>(null);
  const isAdmin = computed(() => userStore.role === 'admin');

  // WebSocket 实时更新连接状态
  useMarketWs().onTicker(() => { /* 刷新 connection status */ });

  return { rules, logs, connectionStatus, isAdmin };
};
```

---

## 路由

```typescript
// router.ts
{
  path: '/risk',
  component: RiskDashboardView,
  meta: { requiresAuth: true }
}
```
