import { describe, it, expect, beforeEach } from 'vitest'
import { useFormat } from '@/composables/useFormat'

describe('useFormat', () => {
  let f: ReturnType<typeof useFormat>

  beforeEach(() => {
    f = useFormat()
  })

  describe('formatCurrency', () => {
    it('formats positive numbers', () => {
      expect(f.formatCurrency(1234.56)).toBe('1,234.56')
    })

    it('formats negative numbers', () => {
      expect(f.formatCurrency(-500.25)).toBe('-500.25')
    })

    it('returns -- for null/undefined', () => {
      expect(f.formatCurrency(null)).toBe('--')
      expect(f.formatCurrency(undefined)).toBe('--')
    })

    it('formats zero', () => {
      expect(f.formatCurrency(0)).toBe('0.00')
    })
  })

  describe('formatPercent', () => {
    it('formats positive percent with + sign', () => {
      expect(f.formatPercent(12.34)).toBe('+12.34%')
    })

    it('formats negative percent with - sign', () => {
      expect(f.formatPercent(-5.67)).toBe('-5.67%')
    })

    it('returns -- for null/undefined', () => {
      expect(f.formatPercent(null)).toBe('--')
    })
  })

  describe('formatNumber', () => {
    it('formats with default 2 decimals', () => {
      expect(f.formatNumber(1234.5678)).toBe('1,234.57')
    })

    it('formats with custom decimals', () => {
      expect(f.formatNumber(1234.5678, 4)).toBe('1,234.5678')
    })

    it('returns -- for null', () => {
      expect(f.formatNumber(null)).toBe('--')
    })
  })

  describe('formatPrice', () => {
    it('formats large prices with 2 decimals', () => {
      expect(f.formatPrice(45678.90)).toBe('45,678.90')
    })

    it('formats small prices with more decimals', () => {
      const result = f.formatPrice(0.1234)
      expect(result).toContain('0.')
      expect(result.split('.')[1]?.length).toBe(6)
    })

    it('returns -- for null', () => {
      expect(f.formatPrice(null)).toBe('--')
    })
  })

  describe('formatDate', () => {
    it('formats ISO date string', () => {
      const result = f.formatDate('2026-05-12T10:30:00Z')
      expect(result).toContain('05')
      expect(result).toContain('2026')
    })

    it('returns -- for null/undefined', () => {
      expect(f.formatDate(null)).toBe('--')
    })
  })
})
