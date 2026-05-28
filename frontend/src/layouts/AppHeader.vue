<script setup lang="ts">
import { useRoute } from 'vue-router'
import { useAppStore } from '@/stores/app'
import { useAuthStore } from '@/stores/auth'
import { ref, computed } from 'vue'
import { Sunny, Moon, Expand, Fold, User, SwitchButton } from '@element-plus/icons-vue'

const route = useRoute()
const appStore = useAppStore()
const authStore = useAuthStore()

const breadcrumbItems = computed(() => {
  const titles: Record<string, string> = {
    '/dashboard': '仪表盘',
    '/market': '行情',
    '/strategies': '策略',
    '/backtest': '回测',
    '/trading': '交易',
    '/portfolio': '持仓',
    '/admin': '系统管理',
  }
  const path = route.path
  const segments = path.split('/').filter(Boolean)
  const items: { label: string; path?: string }[] = [{ label: '主页', path: '/dashboard' }]

  let currentPath = ''
  for (const seg of segments) {
    currentPath += '/' + seg
    const label = titles[currentPath]
    if (label) {
      items.push({ label, path: currentPath === path ? undefined : currentPath })
    }
  }

  // If no title mapping found, use the last segment as label
  if (items.length === 1 && segments.length > 0) {
    items.push({ label: segments[segments.length - 1] })
  }

  return items
})

const dropdownVisible = ref(false)

function handleLogout() {
  dropdownVisible.value = false
  authStore.logout()
  // Router guard will redirect to login
}
</script>

<template>
  <header class="app-header">
    <div class="header-left">
      <!-- Mobile menu toggle -->
      <button class="mobile-menu-btn" @click="appStore.toggleMobileSidebar()">
        <el-icon :size="20">
          <Fold v-if="appStore.sidebarMobileOpen" />
          <Expand v-else />
        </el-icon>
      </button>

      <!-- Breadcrumb -->
      <nav class="breadcrumb">
        <template v-for="(item, index) in breadcrumbItems" :key="index">
          <span v-if="index > 0" class="breadcrumb-separator">&gt;</span>
          <router-link
            v-if="item.path"
            :to="item.path"
            class="breadcrumb-link"
          >
            {{ item.label }}
          </router-link>
          <span v-else class="breadcrumb-current">
            {{ item.label }}
          </span>
        </template>
      </nav>
    </div>

    <div class="header-right">
      <!-- Theme toggle -->
      <button class="theme-toggle-btn" @click="appStore.toggleTheme()" :title="appStore.theme === 'dark' ? '切换亮色模式' : '切换暗色模式'">
        <el-icon :size="18">
          <Sunny v-if="appStore.theme === 'dark'" />
          <Moon v-else />
        </el-icon>
      </button>

      <!-- User avatar dropdown -->
      <el-dropdown
        trigger="click"
        @visible-change="(v: boolean) => dropdownVisible = v"
      >
        <div class="user-avatar">
          {{ authStore.userInitial }}
        </div>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item :icon="User">
              <span style="font-size:13px">个人信息</span>
            </el-dropdown-item>
            <el-dropdown-item :icon="SwitchButton" divided @click="handleLogout">
              <span style="font-size:13px">退出登录</span>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </header>
</template>

<style scoped lang="scss">
.app-header {
  height: var(--header-height);
  background: rgba(8, 9, 10, 0.95);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-bottom: 1px solid var(--color-border);
  padding: 0 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
  position: sticky;
  top: 0;
  z-index: 50;
  transition: all var(--transition-base);
  box-shadow: var(--shadow-sm);

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .mobile-menu-btn {
    display: none;
    background: none;
    border: none;
    color: var(--color-text-tertiary);
    cursor: pointer;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);

    &:hover {
      background: rgba(255, 255, 255, 0.06);
      color: var(--color-text-secondary);
      transform: scale(1.05);
    }

    &:active {
      transform: scale(0.95);
    }

    @media (max-width: 768px) {
      display: flex;
    }
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 0;
    font-size: 13px;

    .breadcrumb-separator {
      margin: 0 8px;
      color: var(--color-text-tertiary);
      font-size: 12px;
      opacity: 0.6;
    }

    .breadcrumb-link {
      color: var(--color-text-tertiary);
      font-weight: 400;
      text-decoration: none;
      transition: all var(--transition-fast);
      padding: 4px 8px;
      border-radius: 4px;

      &:hover {
        color: var(--color-text-primary);
        background: rgba(255, 255, 255, 0.04);
      }
    }

    .breadcrumb-current {
      color: var(--color-text-primary);
      font-weight: 600;
      padding: 4px 8px;
      background: var(--color-accent-light);
      border-radius: 4px;
    }
  }

  .theme-toggle-btn {
    background: none;
    border: none;
    color: var(--color-text-tertiary);
    cursor: pointer;
    width: 32px;
    height: 32px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);

    &:hover {
      background: rgba(255, 255, 255, 0.06);
      color: var(--color-accent);
      transform: rotate(15deg) scale(1.1);
    }

    &:active {
      transform: scale(0.95);
    }
  }

  .user-avatar {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    background: var(--gradient-accent);
    color: #ffffff;
    font-size: 13px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    user-select: none;
    transition: all var(--transition-fast);
    box-shadow: var(--shadow-sm);
    border: 2px solid transparent;

    &:hover {
      transform: scale(1.08);
      box-shadow: var(--shadow-glow);
      border-color: rgba(113, 112, 255, 0.3);
    }

    &:active {
      transform: scale(0.95);
    }
  }
}
</style>
