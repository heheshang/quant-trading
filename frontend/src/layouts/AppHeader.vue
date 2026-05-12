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
  background: var(--color-bg);
  border-bottom: 1px solid var(--color-border);
  padding: 0 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
  position: sticky;
  top: 0;
  z-index: 50;
  transition: background-color 0.3s ease;

  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 12px;
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
    transition: all 0.15s ease;

    &:hover {
      background: rgba(255, 255, 255, 0.04);
      color: var(--color-text-secondary);
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
    }

    .breadcrumb-link {
      color: var(--color-text-tertiary);
      font-weight: 400;
      text-decoration: none;
      transition: color 0.15s ease;

      &:hover {
        color: var(--color-text-secondary);
      }
    }

    .breadcrumb-current {
      color: var(--color-text-primary);
      font-weight: 510;
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
    transition: all 0.15s ease;

    &:hover {
      background: rgba(255, 255, 255, 0.04);
      color: var(--color-text-secondary);
    }
  }

  .user-avatar {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: var(--color-accent);
    color: #ffffff;
    font-size: 13px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    user-select: none;
    transition: opacity 0.15s ease;

    &:hover {
      opacity: 0.85;
    }
  }
}
</style>
