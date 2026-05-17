import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import PositionPanel from '@/components/order/PositionPanel.vue'
import type { Position, PaperAccount } from '@/types/order'

const router = createRouter({
  history: createWebHistory(),
  routes: [{ path: '/', component: { template: '<div />' } }],
})

// Mock closePosition API
vi.mock('@/api/order', () => ({
  closePosition: vi.fn().mockResolvedValue(undefined),
}))

function mockPosition(overrides?: Partial<Position>): Position {
  return {
    id: '1',
    user_id: '1',
    symbol: 'BTC/USDT',
    side: 'long',
    quantity: '0.3000',
    available_quantity: '0.2000',
    avg_entry_price: '49000.00',
    unrealized_pnl: '300.00',
    realized_pnl: '0',
    mode: 'paper',
    created_at: '2026-05-14T06:00:00Z',
    updated_at: '2026-05-14T06:00:00Z',
    ...overrides,
  }
}

function mockAccount(overrides?: Partial<PaperAccount>): PaperAccount {
  return {
    user_id: '1',
    balance: '55000.00',
    frozen_balance: '4950.00',
    initial_balance: '100000.00',
    total_pnl: '-40450.00',
    equity: '60050.00',
    positions_count: 2,
    active_orders_count: 5,
    ...overrides,
  }
}

function createWrapper(props = {}) {
  return mount(PositionPanel, {
    props: {
      positions: [],
      account: mockAccount(),
      currentPrices: { 'BTC/USDT': '50000.00' },
      loading: false,
      ...props,
    },
    global: {
      plugins: [ElementPlus, router],
    },
  })
}

describe('PositionPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ===== Rendering =====
  it('should render position summary bar when account is provided', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.position-summary').exists()).toBe(true)
  })

  it('should not render summary bar when account is null', () => {
    const wrapper = createWrapper({ account: null })
    expect(wrapper.find('.position-summary').exists()).toBe(false)
  })

  it('should render filter bar', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.position-filter').exists()).toBe(true)
  })

  it('should show loading skeleton when loading', () => {
    const wrapper = createWrapper({ loading: true })
    expect(wrapper.find('.skeleton-container').exists()).toBe(true)
  })

  // ===== Computed properties =====
  it('should compute filteredPositions with side filter', () => {
    const wrapper = createWrapper({
      positions: [
        mockPosition({ id: '1', side: 'long' }),
        mockPosition({ id: '2', side: 'short', symbol: 'ETH/USDT' }),
      ],
    })
    const vm = wrapper.vm as any
    vm.filterSide = 'long'
    expect(vm.filteredPositions.length).toBe(1)
    expect(vm.filteredPositions[0].side).toBe('long')
  })

  it('should compute filteredPositions with symbol filter', () => {
    const wrapper = createWrapper({
      positions: [
        mockPosition({ id: '1', symbol: 'BTC/USDT' }),
        mockPosition({ id: '2', symbol: 'ETH/USDT' }),
      ],
    })
    const vm = wrapper.vm as any
    vm.filterSymbol = 'btc'
    expect(vm.filteredPositions.length).toBe(1)
    expect(vm.filteredPositions[0].symbol).toBe('BTC/USDT')
  })

  it('should return all positions when no filters', () => {
    const wrapper = createWrapper({
      positions: [
        mockPosition({ id: '1' }),
        mockPosition({ id: '2', symbol: 'ETH/USDT' }),
      ],
    })
    const vm = wrapper.vm as any
    expect(vm.filteredPositions.length).toBe(2)
  })

  // ===== getCurrentPrice =====
  it('should get current price from currentPrices map', () => {
    const wrapper = createWrapper({ currentPrices: { 'BTC/USDT': '50000.00' } })
    const vm = wrapper.vm as any
    expect(vm.getCurrentPrice('BTC/USDT')).toContain('50,000')
  })

  it('should return empty string for unknown symbol', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.getCurrentPrice('UNKNOWN')).toBe('')
  })

  // ===== Close position =====
  it('should emit close-success when position is closed', async () => {
    const wrapper = createWrapper({ positions: [mockPosition()] })
    const vm = wrapper.vm as any
    vm.closeTarget = mockPosition()
    vm.closeQuantity = '0.3000'
    await vm.onConfirmClose()
    await flushPromises()
    expect(wrapper.emitted('close-success')).toBeDefined()
  })

  it('should set closeDialogVisible for full close', async () => {
    const wrapper = createWrapper({ positions: [mockPosition()] })
    const vm = wrapper.vm as any
    vm.onClosePosition(mockPosition())
    await flushPromises()
    expect(vm.closeDialogVisible).toBe(true)
    expect(vm.isPartialClose).toBe(false)
  })

  it('should set closeDialogVisible for partial close', async () => {
    const wrapper = createWrapper({ positions: [mockPosition()] })
    const vm = wrapper.vm as any
    vm.onPartialClose(mockPosition())
    await flushPromises()
    expect(vm.closeDialogVisible).toBe(true)
    expect(vm.isPartialClose).toBe(true)
  })

  it('should warn when partial close quantity is empty', async () => {
    const wrapper = createWrapper({ positions: [mockPosition()] })
    const vm = wrapper.vm as any
    vm.closeTarget = mockPosition()
    vm.isPartialClose = true
    vm.closeQuantity = ''
    await vm.onConfirmClose()
    // Should not emit close-success
    expect(wrapper.emitted('close-success')).toBeUndefined()
  })

  // ===== Formatters =====
  it('should format money with thousand separators', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatMoney('55000.00')).toContain('55,000')
  })

  it('should format PnL with plus prefix for profit', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatPnl('300.00')).toContain('+¥')
  })

  it('should format PnL with minus prefix for loss', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.formatPnl('-100.00')).toContain('-¥')
  })

  it('should format PnL percent correctly', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    const pct = vm.formatPnlPercent(mockPosition())
    expect(pct).toContain('%')
    expect(pct).toContain('+')
  })

  it('should return pnlClass for positive values', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.pnlClass('300.00')).toBe('text-profit')
  })

  it('should return pnlClass for negative values', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.pnlClass('-100.00')).toBe('text-loss')
  })

  it('should return empty pnlClass for zero', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.pnlClass('0')).toBe('')
  })

  // ===== baseCurrency =====
  it('should extract base currency from position symbol', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.baseCurrency(mockPosition())).toBe('BTC')
  })

  it('should handle null position for baseCurrency', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.baseCurrency(null)).toBe('')
  })

  // ===== Emit events =====
  it('should emit filter-change', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.onFilterChange()
    expect(wrapper.emitted('filter-change')).toBeDefined()
  })
})
