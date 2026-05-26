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
  background: var(--color-bg);
  border-right: 1px solid var(--color-border);
  z-index: 100;
  display: flex;
  flex-direction: column;
  transition:
    width 0.3s ease,
    transform 0.3s ease;

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
  }

  .sidebar-logo {
    height: var(--header-height);
    display: flex;
    align-items: center;
    padding: 0 16px;
    gap: 10px;
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    overflow: hidden;

    .logo-icon {
      flex-shrink: 0;
      width: 22px;
      height: 22px;
      color: var(--color-accent);
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .logo-text {
      font-size: 15px;
      font-weight: 600;
      color: var(--color-text-primary);
      white-space: nowrap;
      letter-spacing: -0.3px;
    }
  }

  .sidebar-nav {
    padding: 12px 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;

    .nav-item {
      height: 40px;
      margin: 0 8px;
      padding: 0 16px;
      border-radius: 6px;
      display: flex;
      align-items: center;
      gap: 12px;
      cursor: pointer;
      transition: all 0.15s ease;
      color: var(--color-text-tertiary);
      white-space: nowrap;
      overflow: hidden;

      .nav-icon {
        flex-shrink: 0;
        width: 20px;
        height: 20px;
        font-size: 20px;
      }

      .nav-label {
        font-size: 14px;
        font-weight: 510;
      }

      &:hover {
        background: rgba(255, 255, 255, 0.04);
        color: var(--color-text-secondary);
      }

      &.is-active {
        background: var(--color-sidebar-active);
        color: var(--color-accent);

        .nav-icon {
          color: var(--color-accent);
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
      background: rgba(0, 0, 0, 0.5);
      z-index: -1;
    }
  }
}
</style>
