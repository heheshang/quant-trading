import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import OrderBookTable from '@/components/market/OrderBookTable.vue'
import type { Depth } from '@/types'

const mockDepth: Depth = {
  bids: [
    { price: 50000, quantity: 1.5, total: 6.0 },
    { price: 49999, quantity: 2.0, total: 4.5 },
    { price: 49998, quantity: 1.0, total: 2.5 },
    { price: 49997, quantity: 0.9, total: 1.5 },
    { price: 49996, quantity: 0.6, total: 0.6 },
  ],
  asks: [
    { price: 50001, quantity: 0.8, total: 0.8 },
    { price: 50002, quantity: 1.2, total: 2.0 },
    { price: 50003, quantity: 0.5, total: 2.5 },
    { price: 50004, quantity: 0.3, total: 2.8 },
    { price: 50005, quantity: 2.0, total: 4.8 },
  ],
  timestamp: 1715640000000,
}

describe('OrderBookTable', () => {
  it('renders asks and bids sides', () => {
    const wrapper = mount(OrderBookTable, {
      props: {
        depth: mockDepth,
        lastPrice: 50000,
        change: 150,
        changePercent: 0.3,
      },
    })
    expect(wrapper.find('.asks-side').exists()).toBe(true)
    expect(wrapper.find('.bids-side').exists()).toBe(true)
  })

  it('renders last price center with price direction', () => {
    const wrapper = mount(OrderBookTable, {
      props: {
        depth: mockDepth,
        lastPrice: 50000,
        change: 150,
        changePercent: 0.3,
      },
    })
    const center = wrapper.find('.last-price-center')
    expect(center.exists()).toBe(true)
    expect(center.find('.price-main').classes()).toContain('up')
  })

  it('shows down class when change is negative', () => {
    const wrapper = mount(OrderBookTable, {
      props: {
        depth: mockDepth,
        lastPrice: 50000,
        change: -40,
        changePercent: -0.8,
      },
    })
    const center = wrapper.find('.last-price-center')
    expect(center.find('.price-main').classes()).toContain('down')
  })

  it('renders spread indicator', () => {
    const wrapper = mount(OrderBookTable, {
      props: {
        depth: mockDepth,
        lastPrice: 50000,
        change: 150,
        changePercent: 0.3,
      },
    })
    const spreadRow = wrapper.find('.spread-row')
    expect(spreadRow.exists()).toBe(true)
    // Spread = ask - bid = 50001 - 50000 = 1
    expect(spreadRow.text()).toContain('1.00')
  })

  it('displays depth bar backgrounds', () => {
    const wrapper = mount(OrderBookTable, {
      props: {
        depth: mockDepth,
        lastPrice: 50000,
        change: 150,
        changePercent: 0.3,
      },
    })
    expect(wrapper.findAll('.depth-bar').length).toBeGreaterThan(0)
    expect(wrapper.find('.sell-bar').exists()).toBe(true)
    expect(wrapper.find('.buy-bar').exists()).toBe(true)
  })

  it('highlights row when highlightedPrice matches', () => {
    const wrapper = mount(OrderBookTable, {
      props: {
        depth: mockDepth,
        lastPrice: 50000,
        change: 0,
        changePercent: 0,
        highlightedPrice: 50001,
      },
    })
    const highlightedRows = wrapper.findAll('.highlighted')
    expect(highlightedRows.length).toBeGreaterThan(0)
  })
})
