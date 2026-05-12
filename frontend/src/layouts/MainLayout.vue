<script setup lang="ts">
import AppSidebar from './AppSidebar.vue'
import AppHeader from './AppHeader.vue'
import { useAppStore } from '@/stores/app'

const appStore = useAppStore()
</script>

<template>
  <div class="main-layout" :class="{ 'sidebar-collapsed': appStore.sidebarCollapsed }">
    <AppSidebar />

    <div class="layout-main">
      <AppHeader />

      <main class="layout-content">
        <router-view />
      </main>
    </div>
  </div>
</template>

<style scoped lang="scss">
.main-layout {
  display: flex;
  min-height: 100vh;

  .layout-main {
    flex: 1;
    margin-left: $sidebar-width;
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    transition: margin-left 0.3s ease;
  }

  .layout-content {
    flex: 1;
    padding: $layout-padding-desktop;
    max-width: $content-max-width;
    width: 100%;
    overflow-y: auto;
  }

  &.sidebar-collapsed {
    .layout-main {
      margin-left: 64px;
    }
  }
}

@media (max-width: 768px) {
  .main-layout {
    .layout-main {
      margin-left: 0;
    }

    .layout-content {
      padding: $layout-padding-mobile;
    }
  }
}
</style>
