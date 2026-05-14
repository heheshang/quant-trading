import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { usePriceFlash } from '@/composables/usePriceFlash'

describe('usePriceFlash', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('initializes with empty flashClass', () => {
    const { flashClass } = usePriceFlash()
    expect(flashClass.value).toBe('')
  })

  it('triggers flash-buy when price increases', () => {
    const { flashClass, triggerFlash } = usePriceFlash()
    triggerFlash(50100, 50000)
    expect(flashClass.value).toBe('flash-buy')
  })

  it('triggers flash-sell when price decreases', () => {
    const { flashClass, triggerFlash } = usePriceFlash()
    triggerFlash(49900, 50000)
    expect(flashClass.value).toBe('flash-sell')
  })

  it('does not flash when price is unchanged', () => {
    const { flashClass, triggerFlash } = usePriceFlash()
    triggerFlash(50000, 50000)
    expect(flashClass.value).toBe('')
  })

  it('clears flash after 500ms', () => {
    const { flashClass, triggerFlash } = usePriceFlash()
    triggerFlash(50100, 50000)
    expect(flashClass.value).toBe('flash-buy')

    vi.advanceTimersByTime(500)
    expect(flashClass.value).toBe('')
  })

  it('resets timer on rapid successive updates (debounce)', () => {
    const { flashClass, triggerFlash } = usePriceFlash()
    triggerFlash(50100, 50000)
    expect(flashClass.value).toBe('flash-buy')

    // 200ms later, another update - should reset the 500ms timer
    vi.advanceTimersByTime(200)
    triggerFlash(50200, 50100)
    expect(flashClass.value).toBe('flash-buy')

    // 300ms from second trigger (500ms from first) - should still be active
    vi.advanceTimersByTime(300)
    expect(flashClass.value).toBe('flash-buy')

    // 200ms more (500ms from second trigger) - should now clear
    vi.advanceTimersByTime(200)
    expect(flashClass.value).toBe('')
  })

  it('clearFlash immediately clears the flash', () => {
    const { flashClass, triggerFlash, clearFlash } = usePriceFlash()
    triggerFlash(50100, 50000)
    expect(flashClass.value).toBe('flash-buy')

    clearFlash()
    expect(flashClass.value).toBe('')
  })
})
