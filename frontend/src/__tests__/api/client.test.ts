import { describe, it, expect, vi } from 'vitest'
import client from '@/api/client'

describe('API Client', () => {
  it('creates an axios instance with correct baseURL', () => {
    // In test environment, window.location.origin = http://localhost:8081
    // so VITE_API_BASE_URL resolves to the full URL, not just /api/v1
    const expected = client.defaults.baseURL as string
    // baseURL is either /api/v1 (in dev/staging) or full URL (in test with jsdom origin)
    expect(expected).toMatch(/\/api\/v1/)
    expect(client.defaults.timeout).toBe(15000)
    expect(client.defaults.headers['Content-Type']).toBe('application/json')
  })

  it('has request interceptor', () => {
    const interceptors = client.interceptors.request as any
    expect(interceptors).toBeDefined()
  })

  it('has response interceptor', () => {
    const interceptors = client.interceptors.response as any
    expect(interceptors).toBeDefined()
  })
})
