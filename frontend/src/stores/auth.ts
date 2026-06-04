import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { User, LoginPayload, RegisterPayload } from '@/types'
import { authApi } from '@/api'
import client from '@/api/client'
import router from '@/router'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const token = ref<string | null>(null)
  const refreshTokenVal = ref<string | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Initialize from sessionStorage
  function initFromStorage() {
    const storedTokens = sessionStorage.getItem('auth_tokens')
    if (storedTokens) {
      try {
        const tokens = JSON.parse(storedTokens)
        token.value = tokens.access_token || null
        refreshTokenVal.value = tokens.refresh_token || null
      } catch {
        sessionStorage.removeItem('auth_tokens')
      }
    }
    const storedUser = sessionStorage.getItem('auth_user')
    if (storedUser) {
      try {
        user.value = JSON.parse(storedUser)
      } catch {
        sessionStorage.removeItem('auth_user')
      }
    }
  }

  // Run init
  initFromStorage()

  const isAuthenticated = computed(() => !!token.value && !!user.value)
  const isAdmin = computed(() => user.value?.role === 'admin')
  const userRole = computed(() => user.value?.role || '')
  const userInitial = computed(() => user.value?.username?.charAt(0)?.toUpperCase() || 'U')

  /**
   * Current user's id (UUID, server-side). The `User` type in
   * `types/index.ts` declares `id: number` (a pre-existing type bug
   * — the backend's `UserResponse.id` is a `Uuid`), so we coerce to
   * string here to keep the call sites safe regardless of the
   * declared type.
   */
  const userId = computed<string | null>(() => {
    const u = user.value as unknown as { id?: unknown } | null
    if (!u) return null
    if (typeof u.id === 'string') return u.id
    if (u.id != null) return String(u.id)
    return null
  })

  function saveTokens(accessToken: string, refreshTokenStr: string) {
    token.value = accessToken
    refreshTokenVal.value = refreshTokenStr
    sessionStorage.setItem('auth_tokens', JSON.stringify({
      access_token: accessToken,
      refresh_token: refreshTokenStr,
    }))
  }

  async function login(payload: LoginPayload) {
    loading.value = true
    error.value = null
    try {
      const res = await authApi.login(payload)
      saveTokens(res.access_token, res.refresh_token)
      user.value = res.user
      sessionStorage.setItem('auth_user', JSON.stringify(res.user))
      return true
    } catch (e: unknown) {
      error.value = (e as { response?: { data?: { message?: string }; message?: string }; message?: string })?.response?.data?.message || (e as { message?: string })?.message || 'Login failed'
      return false
    } finally {
      loading.value = false
    }
  }

  async function register(payload: RegisterPayload) {
    loading.value = true
    error.value = null
    try {
      const res = await authApi.register(payload)
      saveTokens(res.access_token, res.refresh_token)
      user.value = res.user
      sessionStorage.setItem('auth_user', JSON.stringify(res.user))
      return true
    } catch (e: unknown) {
      error.value = (e as { response?: { data?: { message?: string }; message?: string }; message?: string })?.response?.data?.message || (e as { message?: string })?.message || 'Registration failed'
      return false
    } finally {
      loading.value = false
    }
  }

  async function logout() {
    try {
      await authApi.logout()
    } catch {
      // ignore
    }
    token.value = null
    refreshTokenVal.value = null
    user.value = null
    sessionStorage.removeItem('auth_tokens')
    sessionStorage.removeItem('auth_user')
    router.push('/login')
  }

  async function fetchProfile() {
    try {
      const profile = await authApi.getProfile()
      user.value = profile
      sessionStorage.setItem('auth_user', JSON.stringify(profile))
    } catch {
      // ignore
    }
  }

  function clearError() {
    error.value = null
  }

  return {
    user,
    token,
    refreshToken: refreshTokenVal,
    loading,
    error,
    isAuthenticated,
    isAdmin,
    userRole,
    userId,
    userInitial,
    login,
    register,
    logout,
    fetchProfile,
    clearError,
    initFromStorage,
  }
})
