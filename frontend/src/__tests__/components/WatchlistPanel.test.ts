import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import WatchlistPanel from '@/components/market/WatchlistPanel.vue'
import type { WatchlistItem } from '@/components/market/WatchlistPanel.vue'

const mockWatchlistItems: WatchlistItem[] = [
  { symbol: 'BTCUSDT', price: 50500, change: 150, change_percent: 0.3 },
  { symbol: 'ETHUSDT', price: 3200, change: -40, change_percent: -1.2 },
  { symbol: 'BNBUSDT', price: 580, change: 5, change_percent: 0.87 },
]

describe('WatchlistPanel', () => {
  it('renders the watchlist panel', () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    expect(wrapper.find('.watchlist-panel').exists()).toBe(true)
  })

  it('renders header with title', () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
      },
      global: {
        stubs: {
          ElIcon: { template: '<i class="el-icon" />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    expect(wrapper.find('.panel-title').text()).toContain('自选列表')
  })

  it('renders watchlist items when provided', () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    expect(wrapper.findAll('.watchlist-item')).toHaveLength(3)
  })

  it('renders empty state when no items', () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: [],
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    expect(wrapper.find('.empty-state').exists()).toBe(true)
    expect(wrapper.find('.empty-text').text()).toBe('暂无自选交易对')
  })

  it('displays formatted symbol names', () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    // formatSymbol('BTCUSDT') => 'BTC/USDT'
    const firstSymbol = wrapper.find('.item-symbol')
    expect(firstSymbol.text()).toBe('BTC/USDT')
  })

  it('displays symbol full names from SYMBOL_NAMES', () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    // BTCUSDT -> Bitcoin from SYMBOL_NAMES
    const itemNames = wrapper.findAll('.item-name')
    expect(itemNames[0].text()).toBe('Bitcoin')
    expect(itemNames[1].text()).toBe('Ethereum')
  })

  it('applies correct price change class', () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    expect(wrapper.vm.getPriceClass(150)).toBe('price-up')
    expect(wrapper.vm.getPriceClass(-40)).toBe('price-down')
    expect(wrapper.vm.getPriceClass(0)).toBe('')
  })

  it('highlights active symbol', () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
        activeSymbol: 'ETHUSDT',
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    const activeItem = wrapper.find('.watchlist-item.active')
    expect(activeItem.exists()).toBe(true)
  })

  it('emits select event when item clicked', async () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    await wrapper.find('.watchlist-item').trigger('click')
    expect(wrapper.emitted()).toHaveProperty('select')
    expect(wrapper.emitted('select')?.[0]).toEqual(['BTCUSDT'])
  })

  it('emits remove event when remove button clicked', async () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    // Hover over first item to show remove button
    const firstItem = wrapper.find('.watchlist-item')
    await firstItem.trigger('mouseenter')
    await wrapper.find('.remove-btn').trigger('click')
    expect(wrapper.emitted()).toHaveProperty('remove')
    expect(wrapper.emitted('remove')?.[0]).toEqual(['BTCUSDT'])
  })

  it('opens add dialog when add button clicked', async () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    await wrapper.find('.add-btn').trigger('click')
    expect(wrapper.vm.showAddDialog).toBe(true)
  })

  it('emits add event when symbol added via dialog', async () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: mockWatchlistItems,
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: true,
          ElDialog: {
            template: '<div class="el-dialog"><slot /><slot name="footer" /></div>',
            props: ['modelValue'],
            emits: ['update:modelValue'],
          },
          ElSelect: {
            template: '<select><option value="SOLUSDT">Solana</option></select>',
            props: ['modelValue'],
            emits: ['update:modelValue'],
          },
          ElOption: { template: '<option />' },
        },
      },
    })

    wrapper.vm.showAddDialog = true
    wrapper.vm.selectedSymbol = 'SOLUSDT'
    await wrapper.vm.addSymbol()

    expect(wrapper.emitted()).toHaveProperty('add')
    expect(wrapper.emitted('add')?.[0]).toEqual(['SOLUSDT'])
    expect(wrapper.vm.showAddDialog).toBe(false)
  })

  it('exposes formatSymbol, getSymbolName, formatNumber, formatPercent methods', () => {
    const wrapper = mount(WatchlistPanel, {
      props: {
        items: [],
      },
      global: {
        stubs: {
          ElIcon: { template: '<i />' },
          ElButton: { template: '<button />' },
          ElDialog: { template: '<div class="el-dialog" />' },
          ElSelect: { template: '<div class="el-select" />' },
          ElOption: { template: '<option />' },
        },
      },
    })
    expect(wrapper.vm.formatSymbol('BTCUSDT')).toBe('BTC/USDT')
    expect(wrapper.vm.getSymbolName('ETHUSDT')).toBe('Ethereum')
    expect(wrapper.vm.formatNumber(12345.678)).toBe('12345.68')
    expect(wrapper.vm.formatPercent(5.5)).toBe('5.50%')
  })
})
