import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

import MainLayout from '@/layouts/MainLayout.vue'
import AuthLayout from '@/layouts/AuthLayout.vue'

declare module 'vue-router' {
  interface RouteMeta {
    requiresAuth?: boolean
    title?: string
    roles?: string[]
  }
}

const routes: RouteRecordRaw[] = [
  {
    path: '/login',
    component: AuthLayout,
    children: [
      {
        path: '',
        name: 'Login',
        component: () => import('@/views/dashboard/LoginView.vue'),
        meta: { requiresAuth: false, title: '登录' },
      },
    ],
  },
  {
    path: '/register',
    component: AuthLayout,
    children: [
      {
        path: '',
        name: 'Register',
        component: () => import('@/views/dashboard/RegisterView.vue'),
        meta: { requiresAuth: false, title: '注册' },
      },
    ],
  },
  {
    path: '/',
    redirect: '/dashboard',
  },
  {
    path: '/',
    component: MainLayout,
    children: [
      {
        path: 'dashboard',
        name: 'Dashboard',
        component: () => import('@/views/dashboard/DashboardView.vue'),
        meta: { requiresAuth: true, title: '仪表盘' },
      },
      {
        path: 'kline/:symbol/:interval',
        name: 'KlineDetail',
        component: () => import('@/views/kline/KlineDetailView.vue'),
        meta: { requiresAuth: true, title: 'K线详情' },
      },
      {
        path: 'market',
        name: 'Market',
        component: () => import('@/views/market/MarketView.vue'),
        meta: { requiresAuth: true, title: '行情' },
      },
      {
        path: 'strategies',
        name: 'Strategies',
        component: () => import('@/views/strategy/StrategiesView.vue'),
        meta: { requiresAuth: true, title: '策略' },
      },
      {
        path: 'strategies/create',
        name: 'StrategyCreate',
        component: () => import('@/views/strategy/StrategyCreateView.vue'),
        meta: { requiresAuth: true, title: '新建策略' },
      },
      {
        path: 'strategies/:id/edit',
        name: 'StrategyEdit',
        component: () => import('@/views/strategy/StrategyEditView.vue'),
        meta: { requiresAuth: true, title: '编辑策略' },
      },
      {
        path: 'strategies/templates',
        name: 'StrategyTemplates',
        component: () => import('@/views/strategy/StrategyTemplateView.vue'),
        meta: { requiresAuth: true, title: '策略模板市场' },
      },
      {
        path: 'backtest',
        name: 'Backtest',
        component: () => import('@/views/backtest/BacktestView.vue'),
        meta: { requiresAuth: true, title: '回测' },
      },
      {
        path: 'trading',
        name: 'Trading',
        component: () => import('@/views/trade/TradingView.vue'),
        meta: { requiresAuth: true, title: '交易' },
      },
      {
        path: 'arbitrage',
        name: 'Arbitrage',
        component: () => import('@/views/arbitrage/ArbitrageView.vue'),
        meta: { requiresAuth: true, title: '套利' },
      },
      {
        path: 'orders',
        name: 'Orders',
        component: () => import('@/views/order/OrderManagementView.vue'),
        meta: { requiresAuth: true, title: '订单管理' },
      },
      {
        path: 'portfolio',
        name: 'Portfolio',
        component: () => import('@/views/portfolio/PortfolioView.vue'),
        meta: { requiresAuth: true, title: '持仓' },
      },
      {
        path: 'risk',
        name: 'RiskDashboard',
        component: () => import('@/views/risk/RiskDashboardView.vue'),
        meta: { requiresAuth: true, title: '风控面板' },
      },
      {
        path: 'admin',
        name: 'Admin',
        component: () => import('@/views/system/SystemAdminView.vue'),
        meta: { requiresAuth: true, title: '系统管理' },
      },
      {
        path: 'admin/feature-flags',
        name: 'AdminFeatureFlags',
        component: () => import('@/views/admin/FeatureFlagView.vue'),
        meta: { requiresAuth: true, title: 'Feature Flags', roles: ['admin'] },
      },
      {
        path: 'api-keys',
        name: 'ApiKeyManagement',
        component: () => import('@/views/ApiKeyManagementView.vue'),
        meta: { requiresAuth: true, title: 'API密钥管理' },
      },
      {
        path: 'strategy-review',
        name: 'StrategyReview',
        component: () => import('@/views/strategy/StrategyReviewView.vue'),
        meta: { requiresAuth: true, title: '策略审核' },
      },
    ],
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/dashboard',
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

router.beforeEach((to, _from, next) => {
  const authStore = useAuthStore()
  const requiresAuth = to.meta.requiresAuth

  if (requiresAuth === false) {
    // Public routes (login, register) - redirect to dashboard if already logged in
    if (authStore.isAuthenticated) {
      next({ name: 'Dashboard' })
    } else {
      next()
    }
  } else if (requiresAuth) {
    // Protected routes
    if (!authStore.isAuthenticated) {
      next({ name: 'Login' })
    } else {
      // Check roles if specified
      const roles = to.meta.roles as string[] | undefined
      if (roles && roles.length > 0) {
        const userRole = authStore.userRole
        if (!userRole || !roles.includes(userRole)) {
          next({ name: 'Dashboard' })
          return
        }
      }
      next()
    }
  } else {
    // No meta (redirect routes) - just proceed
    next()
  }
})

export default router
