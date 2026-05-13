import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

import MainLayout from '@/layouts/MainLayout.vue'
import AuthLayout from '@/layouts/AuthLayout.vue'

import DashboardView from '@/views/dashboard/DashboardView.vue'
import MarketView from '@/views/market/MarketView.vue'
import StrategiesView from '@/views/strategy/StrategiesView.vue'
import StrategyCreateView from '@/views/strategy/StrategyCreateView.vue'
import StrategyEditView from '@/views/strategy/StrategyEditView.vue'
import StrategyTemplateView from '@/views/strategy/StrategyTemplateView.vue'
import BacktestView from '@/views/backtest/BacktestView.vue'
import TradingView from '@/views/trade/TradingView.vue'
import PortfolioView from '@/views/portfolio/PortfolioView.vue'
import SystemAdminView from '@/views/system/SystemAdminView.vue'
import LoginView from '@/views/dashboard/LoginView.vue'
import RegisterView from '@/views/dashboard/RegisterView.vue'

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
        component: LoginView,
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
        component: RegisterView,
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
        component: DashboardView,
        meta: { requiresAuth: true, title: '仪表盘' },
      },
      {
        path: 'market',
        name: 'Market',
        component: MarketView,
        meta: { requiresAuth: true, title: '行情' },
      },
      {
        path: 'strategies',
        name: 'Strategies',
        component: StrategiesView,
        meta: { requiresAuth: true, title: '策略' },
      },
      {
        path: 'strategies/create',
        name: 'StrategyCreate',
        component: StrategyCreateView,
        meta: { requiresAuth: true, title: '新建策略' },
      },
      {
        path: 'strategies/:id/edit',
        name: 'StrategyEdit',
        component: StrategyEditView,
        meta: { requiresAuth: true, title: '编辑策略' },
      },
      {
        path: 'strategies/templates',
        name: 'StrategyTemplates',
        component: StrategyTemplateView,
        meta: { requiresAuth: true, title: '策略模板市场' },
      },
      {
        path: 'backtest',
        name: 'Backtest',
        component: BacktestView,
        meta: { requiresAuth: true, title: '回测' },
      },
      {
        path: 'trading',
        name: 'Trading',
        component: TradingView,
        meta: { requiresAuth: true, title: '交易' },
      },
      {
        path: 'portfolio',
        name: 'Portfolio',
        component: PortfolioView,
        meta: { requiresAuth: true, title: '持仓' },
      },
      {
        path: 'admin',
        name: 'Admin',
        component: SystemAdminView,
        meta: { requiresAuth: true, title: '系统管理', roles: ['admin'] },
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
        const userRole = authStore.user?.role
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
