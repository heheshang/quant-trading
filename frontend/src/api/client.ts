import axios, { AxiosError, type InternalAxiosRequestConfig } from 'axios'
import type { ApiResponse } from '@/types'

const client = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || '/api/v1',
  timeout: 15000,
  headers: {
    'Content-Type': 'application/json',
  },
})

// Track whether a token refresh is in progress to avoid multiple concurrent refresh attempts
let isRefreshing = false
let failedQueue: Array<{
  resolve: (token: string) => void
  reject: (error: unknown) => void
}> = []

function processQueue(error: unknown, token: string | null = null) {
  failedQueue.forEach(({ resolve, reject }) => {
    if (error) {
      reject(error)
    } else if (token) {
      resolve(token)
    }
  })
  failedQueue = []
}

// Request interceptor: inject Authorization header
client.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    const stored = sessionStorage.getItem('auth_tokens')
    if (stored) {
      try {
        const tokens = JSON.parse(stored)
        if (tokens.access_token) {
          config.headers.Authorization = `Bearer ${tokens.access_token}`
        }
      } catch {
        // ignore parse errors
      }
    }
    return config
  },
  (error) => Promise.reject(error),
)

// Response interceptor: unwrap data, handle 401 with token refresh
client.interceptors.response.use(
  (response) => {
    const body = response.data as ApiResponse<unknown>
    if (body.code !== 0) {
      return Promise.reject(new Error(body.message || 'Request failed'))
    }
    return body.data as any
  },
  async (error: AxiosError<ApiResponse<unknown>>) => {
    const originalRequest = error.config as InternalAxiosRequestConfig & { _retry?: boolean }

    // Only attempt refresh on 401 and if we haven't retried yet
    if (error.response?.status === 401 && !originalRequest._retry) {
      if (isRefreshing) {
        // Queue this request while refresh is in progress
        return new Promise<string>((resolve, reject) => {
          failedQueue.push({ resolve, reject })
        })
          .then((token) => {
            originalRequest.headers.Authorization = `Bearer ${token}`
            return client(originalRequest)
          })
          .catch((err) => Promise.reject(err))
      }

      originalRequest._retry = true
      isRefreshing = true

      try {
        const stored = sessionStorage.getItem('auth_tokens')
        if (!stored) {
          throw new Error('No refresh token available')
        }

        const tokens = JSON.parse(stored)
        const refreshTokenValue = tokens.refresh_token

        if (!refreshTokenValue) {
          throw new Error('No refresh token available')
        }

        const refreshResponse = await axios.post<ApiResponse<{ access_token: string; refresh_token: string }>>(
          `${client.defaults.baseURL}/auth/refresh`,
          { refresh_token: refreshTokenValue },
        )

        const { access_token, refresh_token } = refreshResponse.data.data

        // Update stored tokens
        const newTokens = { access_token, refresh_token }
        sessionStorage.setItem('auth_tokens', JSON.stringify(newTokens))

        // Update the original request header
        originalRequest.headers.Authorization = `Bearer ${access_token}`

        processQueue(null, access_token)

        return client(originalRequest)
      } catch (refreshError) {
        processQueue(refreshError, null)
        // Clear tokens and redirect to login
        sessionStorage.removeItem('auth_tokens')
        sessionStorage.removeItem('auth_user')
        window.location.href = '/login'
        return Promise.reject(refreshError)
      } finally {
        isRefreshing = false
      }
    }

    // For non-401 errors or failed retries, extract meaningful error message
    const message = error.response?.data?.message || error.message || 'Network error'
    return Promise.reject(new Error(message))
  },
)

export default client
