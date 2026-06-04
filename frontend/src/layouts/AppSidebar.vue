<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { computed } from 'vue'

const router = useRouter()
const route = useRoute()
const appStore = useAppStore()

interface NavItem {
  icon: string
  label: string
  route: string
}

const navItems: NavItem[] = [
  { icon: 'data-board', label: '仪表盘', route: '/dashboard' },
  { icon: 'trend-charts', label: '行情', route: '/market' },
  { icon: 'cpu', label: '策略', route: '/strategies' },
  { icon: 'odometer', label: '回测', route: '/backtest' },
  { icon: 'switch', label: '交易', route: '/trading' },
  { icon: 'briefcase', label: '持仓', route: '/portfolio' },
  // §6-1: PAMM (Percent Allocation Management Module).
  { icon: 'coin', label: 'PAMM 基金', route: '/pamm' },
  // §6-2: Copy Trading (trader list, subscribe, fan-out, profit share).
  { icon: 'copy-document', label: '跟单交易', route: '/copy' },
  { icon: 'setting', label: '系统管理', route: '/admin' },
  { icon: 'operation', label: '策略审核', route: '/strategy-review' },
]

const activeRoute = computed(() => route.path)

function handleSelect(index: string) {
  router.push(index)
  if (appStore.sidebarMobileOpen) {
    appStore.closeMobileSidebar()
  }
}
</script>

<template>
  <aside
    class="app-sidebar"
    :class="{ 'is-collapsed': appStore.sidebarCollapsed, 'is-mobile-open': appStore.sidebarMobileOpen }"
  >
    <!-- Overlay for mobile -->
    <div
      v-if="appStore.sidebarMobileOpen"
      class="sidebar-overlay"
      @click="appStore.closeMobileSidebar()"
    />

    <!-- Sidebar inner -->
    <div class="sidebar-inner">
      <!-- Logo -->
      <div class="sidebar-logo">
        <div class="logo-icon">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M3 21V3H21V21H3Z" stroke="currentColor" stroke-width="2" stroke-linejoin="round" />
            <path d="M7 17L10 13L13 15L17 9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
            <path d="M17 9H14M17 9V12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </div>
        <span v-if="!appStore.sidebarCollapsed" class="logo-text">Quant Trading</span>
      </div>

      <!-- Navigation -->
      <nav class="sidebar-nav">
        <div
          v-for="item in navItems"
          :key="item.route"
          class="nav-item"
          :class="{ 'is-active': activeRoute === item.route }"
          @click="handleSelect(item.route)"
        >
          <el-icon class="nav-icon">
            <component :is="item.icon" />
          </el-icon>
          <span v-if="!appStore.sidebarCollapsed" class="nav-label">{{ item.label }}</span>
        </div>
      </nav>
    </div>
  </aside>
</template>

<style scoped lang="scss">
.app-sidebar {
  position: fixed;
  top: 0;
  left: 0;
  width: var(--sidebar-width);
  height: 100vh;
  background: rgba(8, 9, 10, 0.98);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-right: 1px solid var(--color-border);
  z-index: 100;
  display: flex;
  flex-direction: column;
  transition:
    width var(--transition-slow),
    transform var(--transition-slow);
  box-shadow: var(--shadow-md);

  &.is-collapsed {
    width: 64px;

    .nav-label {
      display: none;
    }
  }

  .sidebar-overlay {
    display: none;
  }

  .sidebar-inner {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .sidebar-logo {
    height: var(--header-height);
    display: flex;
    align-items: center;
    padding: 0 16px;
    gap: 12px;
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    overflow: hidden;
    background: var(--gradient-surface);

    .logo-icon {
      flex-shrink: 0;
      width: 26px;
      height: 26px;
      color: var(--color-primary);
      display: flex;
      align-items: center;
      justify-content: center;
      filter: drop-shadow(0 0 8px var(--color-primary-glow));
      transition: all var(--transition-base);
    }

    .logo-text {
      font-size: 15px;
      font-weight: 600;
      font-family: var(--font-heading);
      color: var(--color-text-primary);
      white-space: nowrap;
      letter-spacing: -0.3px;
    }
  }

  .sidebar-nav {
    padding: 12px 8px;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;

    .nav-item {
      height: 40px;
      padding: 0 12px;
      border-radius: var(--radius-md);
      display: flex;
      align-items: center;
      gap: 12px;
      cursor: pointer;
      transition: all var(--transition-fast);
      color: var(--color-text-secondary);
      white-space: nowrap;
      overflow: hidden;
      position: relative;

      .nav-icon {
        flex-shrink: 0;
        width: 20px;
        height: 20px;
        font-size: 20px;
        transition: all var(--transition-fast);
      }

      .nav-label {
        font-size: 13px;
        font-weight: 500;
        transition: all var(--transition-fast);
      }

      &::before {
        content: '';
        position: absolute;
        left: 0;
        top: 50%;
        transform: translateY(-50%);
        width: 3px;
        height: 0;
        background: var(--gradient-primary);
        border-radius: 0 2px 2px 0;
        transition: height var(--transition-fast);
      }

      &:hover {
        background: rgba(255, 255, 255, 0.04);
        color: var(--color-text-primary);

        .nav-icon {
          transform: scale(1.05);
        }
      }

      &.is-active {
        background: var(--color-primary-light);
        color: var(--color-primary);
        font-weight: 600;

        &::before {
          height: 60%;
        }

        .nav-icon {
          color: var(--color-primary);
          filter: drop-shadow(0 0 6px var(--color-primary-glow));
        }

        .nav-label {
          color: var(--color-primary);
        }
      }
    }
  }
}

// Mobile responsive
@media (max-width: 768px) {
  .app-sidebar {
    transform: translateX(-100%);

    &.is-mobile-open {
      transform: translateX(0);
    }

    .sidebar-overlay {
      display: block;
      position: fixed;
      top: 0;
      left: 0;
      width: 100vw;
      height: 100vh;
      background: rgba(0, 0, 0, 0.6);
      backdrop-filter: blur(4px);
      -webkit-backdrop-filter: blur(4px);
      z-index: -1;
      animation: fadeIn var(--transition-base);
    }
  }
}
</style>
