import { ref } from 'vue'

export function useFormat() {
  function formatCurrency(value: number | null | undefined, compact = false): string {
    if (value === null || value === undefined) return '--'
    if (compact && Math.abs(value) >= 100_000_000) {
      return `${(value / 100_000_000).toFixed(2)}亿`
    }
    if (compact && Math.abs(value) >= 10_000) {
      return `${(value / 10_000).toFixed(2)}万`
    }
    return new Intl.NumberFormat('en-US', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value)
  }

  function formatPercent(value: number | null | undefined): string {
    if (value === null || value === undefined) return '--'
    const sign = value > 0 ? '+' : ''
    return `${sign}${value.toFixed(2)}%`
  }

  function formatNumber(value: number | null | undefined, decimals = 2): string {
    if (value === null || value === undefined) return '--'
    return new Intl.NumberFormat('en-US', {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }).format(value)
  }

  function formatPrice(value: number | null | undefined): string {
    if (value === null || value === undefined) return '--'
    let decimals = 2
    if (value < 0.001) decimals = 8
    else if (value < 1) decimals = 6
    else if (value < 1000) decimals = 4
    else decimals = 2
    return new Intl.NumberFormat('en-US', {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }).format(value)
  }

  function formatDate(dateStr: string | null | undefined): string {
    if (!dateStr) return '--'
    const d = new Date(dateStr)
    return d.toLocaleDateString('en-US', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    })
  }

  function formatDateTime(dateStr: string | null | undefined): string {
    if (!dateStr) return '--'
    const d = new Date(dateStr)
    return d.toLocaleDateString('en-US', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  return {
    formatCurrency,
    formatPercent,
    formatNumber,
    formatPrice,
    formatDate,
    formatDateTime,
  }
}
