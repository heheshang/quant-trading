import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { useMarketWs } from '@/composables/useMarketWs'

describe('useMarketWs', () => {
  it('initializes with idle status', () => {
    const { status } = useMarketWs()
    expect(status.value).toBe('idle')
  })

  it('disconnects and sets status to idle', () => {
    const { status, disconnect } = useMarketWs()
    disconnect()
    expect(status.value).toBe('idle')
  })

  it('exposes subscribe and unsubscribe methods', () => {
    const { subscribe, unsubscribe } = useMarketWs()
    expect(typeof subscribe).toBe('function')
    expect(typeof unsubscribe).toBe('function')
  })

  it('exposes connect method', () => {
    const { connect } = useMarketWs()
    expect(typeof connect).toBe('function')
  })

  it('registers message handlers via onMessage', () => {
    const { onMessage } = useMarketWs()
    expect(typeof onMessage).toBe('function')

    const handler = vi.fn()
    onMessage(handler)
    // Handler is registered without error
  })

  it('registers kick handlers via onKick', () => {
    const { onKick } = useMarketWs()
    expect(typeof onKick).toBe('function')

    const handler = vi.fn()
    onKick(handler)
    // Handler is registered without error
  })

  it('subscriptions set is initially empty', () => {
    const { subscriptions } = useMarketWs()
    expect(subscriptions.value.size).toBe(0)
  })
})
