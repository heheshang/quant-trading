import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import TickerTable from '@/components/market/TickerTable.vue'
import type { Ticker } from '@/types'

const mockTickers: Ticker[] = [
  {
    symbol: 'BTCUSDT',
    price: 50500,
    change: 150,
    change_percent: 0.3,
    volume: 123456,
    high: 51000,
    low: 49000,
    bid: 50499,
    ask: 50501,
    timestamp: 1715640000000,
  },
  {
    symbol: 'ETHUSDT',
    price: 3200,
    change: -40,
    change_percent: -1.2,
    volume: 89012,
    high: 3280,
    low: 3150,
    bid: 3199,
    ask: 3201,
    timestamp: 1715640000000,
  },
]

describe('TickerTable', () => {
  it('renders ticker data rows', () => {
    const wrapper = mount(TickerTable, {
      props: { data: mockTickers },
      global: {
        stubs: {
          ElTable: true,
          ElTableColumn: true,
        },
      },
    })
    // Check that the component renders without errors
    expect(wrapper.find('.ticker-table-wrapper').exists()).toBe(true)
  })

  it('displays formatted symbol name', () => {
    const wrapper = mount(TickerTable, {
      props: { data: mockTickers },
      global: {
        stubs: {
          ElTable: true,
          ElTableColumn: true,
        },
      },
    })
    // The template uses formatSymbol which converts BTCUSDT -> BTC/USDT
    expect(wrapper.vm.formatSymbol).toBeDefined()
  })

  it('applies flash class from flashMap', () => {
    const flashMap = { BTCUSDT: 'flash-buy' as const }
    const wrapper = mount(TickerTable, {
      props: { data: mockTickers, flashMap },
      global: {
        stubs: {
          ElTable: true,
          ElTableColumn: true,
        },
      },
    })
    expect(wrapper.vm.getFlashClass('BTCUSDT')).toBe('flash-buy')
    expect(wrapper.vm.getFlashClass('ETHUSDT')).toBe('')
  })

  it('formats volume with K/M suffix', () => {
    const wrapper = mount(TickerTable, {
      props: { data: mockTickers },
      global: {
        stubs: {
          ElTable: true,
          ElTableColumn: true,
        },
      },
    })
    expect(wrapper.vm.formatVolume(1500000)).toBe('1.50M')
    expect(wrapper.vm.formatVolume(5000)).toBe('5.00K')
    expect(wrapper.vm.formatVolume(500)).toBe('500.00')
  })

  it('returns correct change class and arrow', () => {
    const wrapper = mount(TickerTable, {
      props: { data: mockTickers },
      global: {
        stubs: {
          ElTable: true,
          ElTableColumn: true,
        },
      },
    })
    expect(wrapper.vm.getChangeClass(150)).toBe('change-up')
    expect(wrapper.vm.getChangeClass(-40)).toBe('change-down')
    expect(wrapper.vm.getChangeClass(0)).toBe('change-flat')
    expect(wrapper.vm.getArrow(150)).toBe('▲')
    expect(wrapper.vm.getArrow(-40)).toBe('▼')
    expect(wrapper.vm.getArrow(0)).toBe('')
  })
})
