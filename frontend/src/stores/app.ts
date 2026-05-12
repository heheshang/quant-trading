import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

const THEME_STORAGE_KEY = 'app_theme'
const SIDEBAR_STORAGE_KEY = 'app_sidebar'

export interface BreadcrumbItem {
  label: string
  path?: string
}

export const useAppStore = defineStore('app', () => {
  // State - initialize from localStorage
  const theme = ref<'dark' | 'light'>(
    (localStorage.getItem(THEME_STORAGE_KEY) as 'dark' | 'light') || 'dark',
  )
  const sidebarCollapsed = ref(
    localStorage.getItem(SIDEBAR_STORAGE_KEY) === 'true',
  )
  const sidebarMobileOpen = ref(false)
  const breadcrumbs = ref<BreadcrumbItem[]>([])

  // Persist theme changes
  watch(
    theme,
    (val) => {
      localStorage.setItem(THEME_STORAGE_KEY, val)
      try {
        document.documentElement.setAttribute('data-theme', val)
      } catch {
        // ignore in test environments
      }
    },
    { flush: 'sync' },
  )

  // Persist sidebar state
  watch(sidebarCollapsed, (val) => {
    localStorage.setItem(SIDEBAR_STORAGE_KEY, String(val))
  }, { flush: 'sync' })

  // Actions
  function toggleTheme() {
    theme.value = theme.value === 'dark' ? 'light' : 'dark'
  }

  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value
  }

  function setBreadcrumbs(items: BreadcrumbItem[]) {
    breadcrumbs.value = items
  }

  function toggleMobileSidebar() {
    sidebarMobileOpen.value = !sidebarMobileOpen.value
  }

  function closeMobileSidebar() {
    sidebarMobileOpen.value = false
  }

  return {
    // State
    theme,
    sidebarCollapsed,
    sidebarMobileOpen,
    breadcrumbs,
    // Actions
    toggleTheme,
    toggleSidebar,
    toggleMobileSidebar,
    closeMobileSidebar,
    setBreadcrumbs,
  }
})
