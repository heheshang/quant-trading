import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAppStore } from '@/stores/app'

describe('useAppStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
  })

  it('initializes with dark theme by default', () => {
    const store = useAppStore()
    expect(store.theme).toBe('dark')
  })

  it('toggles theme', () => {
    const store = useAppStore()
    expect(store.theme).toBe('dark')
    store.toggleTheme()
    expect(store.theme).toBe('light')
    store.toggleTheme()
    expect(store.theme).toBe('dark')
  })

  it('persists theme to localStorage', () => {
    const store = useAppStore()
    store.toggleTheme()
    expect(localStorage.getItem('app_theme')).toBe('light')
  })

  it('initializes sidebar not collapsed', () => {
    const store = useAppStore()
    expect(store.sidebarCollapsed).toBe(false)
  })

  it('toggles sidebar', () => {
    const store = useAppStore()
    store.toggleSidebar()
    expect(store.sidebarCollapsed).toBe(true)
    store.toggleSidebar()
    expect(store.sidebarCollapsed).toBe(false)
  })

  it('sets breadcrumbs', () => {
    const store = useAppStore()
    const crumbs = [{ label: '主页', path: '/dashboard' }, { label: '行情' }]
    store.setBreadcrumbs(crumbs)
    expect(store.breadcrumbs).toEqual(crumbs)
  })

  it('toggles mobile sidebar', () => {
    const store = useAppStore()
    store.toggleMobileSidebar()
    expect(store.sidebarMobileOpen).toBe(true)
    store.toggleMobileSidebar()
    expect(store.sidebarMobileOpen).toBe(false)
  })

  it('closes mobile sidebar', () => {
    const store = useAppStore()
    store.toggleMobileSidebar()
    expect(store.sidebarMobileOpen).toBe(true)
    store.closeMobileSidebar()
    expect(store.sidebarMobileOpen).toBe(false)
  })
})
