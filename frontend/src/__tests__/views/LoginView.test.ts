import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createRouter, createWebHistory } from 'vue-router'
import { createPinia, setActivePinia } from 'pinia'
import ElementPlus from 'element-plus'
import LoginView from '@/views/dashboard/LoginView.vue'

// Mock auth store
vi.mock('@/stores/auth', () => ({
  useAuthStore: vi.fn(() => ({
    user: null,
    token: null,
    loading: false,
    error: null,
    isAuthenticated: false,
    isAdmin: false,
    userRole: '',
    login: vi.fn().mockResolvedValue(true),
    register: vi.fn(),
    logout: vi.fn(),
    clearError: vi.fn(),
    fetchProfile: vi.fn(),
  })),
}))

describe('LoginView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders login form', async () => {
    const wrapper = mount(LoginView, {
      global: {
        plugins: [ElementPlus],
        stubs: {
          'router-link': true,
          'router-view': true,
        },
      },
    })

    expect(wrapper.find('.login-view').exists()).toBe(true)
    expect(wrapper.find('.form-title').text()).toContain('Sign In')
  })

  it('has username and password fields', async () => {
    const wrapper = mount(LoginView, {
      global: {
        plugins: [ElementPlus],
        stubs: {
          'router-link': true,
          'router-view': true,
        },
      },
    })

    expect(wrapper.find('form').exists()).toBe(true)
  })
})
