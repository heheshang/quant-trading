import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAuthStore } from '@/stores/auth'

// Mock the auth API
vi.mock('@/api', () => ({
  authApi: {
    login: vi.fn(),
    register: vi.fn(),
    logout: vi.fn(),
    getProfile: vi.fn(),
  },
}))

import { authApi } from '@/api'

describe('useAuthStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    sessionStorage.clear()
  })

  it('initializes with no user or token', () => {
    const store = useAuthStore()
    expect(store.user).toBeNull()
    expect(store.token).toBeNull()
    expect(store.isAuthenticated).toBe(false)
    expect(store.isAdmin).toBe(false)
    expect(store.loading).toBe(false)
  })

  it('sets authenticated state after successful login', async () => {
    const mockUser = { id: 1, username: 'testuser', email: 'test@test.com', role: 'trader' as const, status: 'active', createdAt: '2026-01-01T00:00:00Z', lastLogin: null }
    const mockResponse = { access_token: 'mock-token', refresh_token: 'mock-refresh', user: mockUser, expires_in: 900 }

    vi.mocked(authApi.login).mockResolvedValue(mockResponse as any)

    const store = useAuthStore()
    const result = await store.login({ username: 'testuser', password: 'password123' })

    expect(result).toBe(true)
    expect(store.token).toBe('mock-token')
    expect(store.user).toEqual(mockUser)
    expect(store.isAuthenticated).toBe(true)
    expect(store.loading).toBe(false)
  })

  it('handles login failure', async () => {
    vi.mocked(authApi.login).mockRejectedValue(new Error('Invalid credentials'))

    const store = useAuthStore()
    const result = await store.login({ username: 'testuser', password: 'wrong' })

    expect(result).toBe(false)
    expect(store.token).toBeNull()
    expect(store.user).toBeNull()
    expect(store.isAuthenticated).toBe(false)
    expect(store.loading).toBe(false)
  })

  it('clears state on logout', async () => {
    vi.mocked(authApi.logout).mockResolvedValue(undefined as any)

    const store = useAuthStore()
    // Set initial state
    store.token = 'mock-token'
    store.user = { id: 1, username: 'testuser', email: 'test@test.com', role: 'trader' as const, status: 'active', createdAt: '2026-01-01T00:00:00Z', lastLogin: null }

    await store.logout()

    expect(store.token).toBeNull()
    expect(store.user).toBeNull()
    expect(store.isAuthenticated).toBe(false)
  })

  it('identifies admin users', async () => {
    const mockUser = { id: 1, username: 'admin', email: 'admin@test.com', role: 'admin', status: 'active', createdAt: '2026-01-01T00:00:00Z', lastLogin: null }
    const mockResponse = { access_token: 'admin-token', refresh_token: 'admin-refresh', user: mockUser, expires_in: 900 }

    vi.mocked(authApi.login).mockResolvedValue(mockResponse as any)

    const store = useAuthStore()
    await store.login({ username: 'admin', password: 'admin123' })

    expect(store.isAdmin).toBe(true)
    expect(store.userRole).toBe('admin')
  })

  it('clears error', () => {
    const store = useAuthStore()
    store.error = 'Some error'
    store.clearError()
    expect(store.error).toBeNull()
  })
})
