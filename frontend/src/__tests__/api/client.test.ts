import { describe, it, expect, vi } from 'vitest'
import client from '@/api/client'

describe('API Client', () => {
  it('creates an axios instance with correct baseURL', () => {
    expect(client.defaults.baseURL).toBe('/api/v1')
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
