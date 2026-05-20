# TechDesign-Frontend-P0-F1-API-Signing-20260519

## 前端技术设计方案：P0-F1 API 签名加密

---

## 1. 页面结构

```
frontend/src/
├── views/
│   └── exchange/
│       └── ApiKeyManageView.vue   # [新增] API Key 管理页
├── components/
│   └── exchange/
│       ├── ApiKeyForm.vue         # [新增] 添加/编辑 Key 表单
│       ├── ApiKeyList.vue         # [新增] Key 列表
│       └── ConnectionTest.vue      # [新增] 连通性测试组件
├── api/
│   └── exchange.ts                # [新增] /api/v1/exchange/* 调用
├── stores/
│   └── exchange.ts                # [新增] Exchange Store（Pinia）
└── types/
    └── exchange.ts                 # [新增] API Key 类型定义
```

---

## 2. 页面路由

```typescript
// router/index.ts
{
  path: '/exchange/apikeys',
  component: () => import('@/views/exchange/ApiKeyManageView.vue'),
  meta: { requiresAuth: true, roles: ['trader', 'admin'] }
}
```

---

## 3. 核心组件

### `ApiKeyManageView.vue`
- 布局：左右分栏（左侧表单 / 右侧列表）
- 顶部：连接状态指示（绿色=已配置 / 红色=未配置）
- 操作：添加 Key、测试连通性、删除 Key、启用/禁用

### `ApiKeyForm.vue`
```
字段：
- API Key（输入框，粘贴模式）
- Secret Key（输入框，密码模式）
- 权限勾选（read / trade / spot）
- 测试连通性按钮（POST /exchange/ping）
```

### `ApiKeyList.vue`
```
列：
- 交易所（图标 + binance）
- Key 标识（前6后4，如 BTC***DT）
- 权限（tag）
- 状态（启用/禁用 switch）
- 最后使用时间
- 操作（测试/编辑/删除）
```

---

## 4. API 层

```typescript
// api/exchange.ts
export const exchangeApi = {
  // API Key 管理
  listApiKeys: () => get('/api/v1/exchange/keys'),
  createApiKey: (data: CreateApiKeyRequest) => post('/api/v1/exchange/keys', data),
  deleteApiKey: (id: string) => delete(`/api/v1/exchange/keys/${id}`),
  toggleApiKey: (id: string, active: boolean) => patch(`/api/v1/exchange/keys/${id}`, { is_active: active }),
  testConnection: () => get('/api/v1/exchange/ping'),

  // 交易（需签名）
  getAccount: () => get('/api/v1/exchange/account'),
  placeOrder: (data: OrderRequest) => post('/api/v1/exchange/order', data),
  cancelOrder: (orderId: string) => delete(`/api/v1/exchange/order/${orderId}`),
  getRateLimit: () => get('/api/v1/exchange/rate-limit'),
}
```

---

## 5. 类型定义

```typescript
// types/exchange.ts
export interface ExchangeApiKey {
  id: string
  exchange: 'binance'
  api_key_masked: string  // "BTC***DT"
  permissions: ('read' | 'trade' | 'spot')[]
  is_active: boolean
  last_used_at: string | null
  created_at: string
}

export interface CreateApiKeyRequest {
  api_key: string
  secret_key: string     // 仅创建时传输，不存储明文
  permissions: string[]
}

export interface AccountBalance {
  asset: string
  free: string
  locked: string
}
```

---

## 6. 状态管理

```typescript
// stores/exchange.ts
export const useExchangeStore = defineStore('exchange', () => {
  const apiKeys = ref<ExchangeApiKey[]>([])
  const isConnected = ref(false)

  async function fetchApiKeys() { /* ... */ }
  async function testConnection() { /* ... */ }

  return { apiKeys, isConnected, fetchApiKeys, testConnection }
})
```

---

## 7. 安全注意事项

1. **Secret Key 不存储**：前端只做转发，Secret Key 一次性传输到后端加密存储
2. **Key 遮蔽显示**：列表 API 返回 `api_key_masked`（前6后4），不返回明文
3. **HTTPS Only**：所有 API 通信强制 HTTPS
4. **Input Sanitization**：API Key/Secret 输入框禁止 HTML 注入
