import client from './client'
import type { LoginPayload, RegisterPayload, User } from '@/types'

export const authApi = {
  login(payload: LoginPayload): Promise<{ access_token: string; refresh_token: string; user: User; expires_in: number }> {
    return client.post('/auth/login', payload)
  },
  register(payload: RegisterPayload): Promise<{ access_token: string; refresh_token: string; user: User; expires_in: number }> {
    return client.post('/auth/register', payload)
  },
  logout(): Promise<void> {
    return client.post('/auth/logout')
  },
  getProfile(): Promise<User> {
    return client.get('/auth/me')
  },
  refreshToken(refreshToken: string): Promise<{ access_token: string; refresh_token: string }> {
    return client.post('/auth/refresh', { refresh_token: refreshToken })
  },
}
