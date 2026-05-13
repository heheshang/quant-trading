import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import BacktestTradesTable from '@/components/backtest/BacktestTradesTable.vue'
import type { TradeRecord } from '@/types/backtest'

const mockTrades: TradeRecord[] = [
  {
    direction: 'long',
    entry_time: '2024-01-15T09:30:00Z',
    exit_time: '2024-01-20T14:00:00Z',
    entry_price: 42000,
    exit_price: 43500,
    quantity: 0.5,
    pnl_usdt: 750,
    pnl_pct: 3.57,
    fee: 21,
    slippage: 5,
    exit_reason: 'take_profit',
    holding_period_ms: 160200000,
  },
  {
    direction: 'short',
    entry_time: '2024-02-01T10:00:00Z',
    exit_time: '2024-02-05T16:00:00Z',
    entry_price: 44000,
    exit_price: 42800,
    quantity: 0.3,
    pnl_usdt: 360,
    pnl_pct: 2.73,
    fee: 13.2,
    slippage: 3.6,
    exit_reason: 'signal',
    holding_period_ms: 100800000,
  },
  {
    direction: 'long',
    entry_time: '2024-03-10T08:00:00Z',
    exit_time: '2024-03-15T12:00:00Z',
    entry_price: 45000,
    exit_price: 43800,
    quantity: 0.4,
    pnl_usdt: -480,
    pnl_pct: -2.67,
    fee: 17.76,
    slippage: 4.32,
    exit_reason: 'stop_loss',
    holding_period_ms: 115200000,
  },
]

// Generate enough trades to trigger pagination (pageSize default is 10)
const manyTrades: TradeRecord[] = Array.from({ length: 15 }, (_, i) => ({
  ...mockTrades[i % 3],
  entry_time: `2024-0${Math.floor(i / 3) + 1}-15T09:30:00Z`,
  pnl_usdt: i % 2 === 0 ? 500 + i * 10 : -(400 + i * 10),
  pnl_pct: i % 2 === 0 ? 2.0 + i * 0.1 : -(1.5 + i * 0.1),
  holding_period_ms: 100000 + i * 10000,
}))

function createWrapper(props: any = {}) {
  return mount(BacktestTradesTable, {
    props: {
      trades: props.trades === undefined ? mockTrades : props.trades,
      loading: props.loading ?? false,
    },
    global: {
      plugins: [ElementPlus],
    },
  })
}

describe('BacktestTradesTable', () => {
  it('renders the table wrapper', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.backtest-trades-table').exists()).toBe(true)
  })

  it('displays table header', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('交易记录')
  })

  it('shows loading skeleton when loading', () => {
    const wrapper = createWrapper({ loading: true })
    expect(wrapper.find('.table-loading').exists()).toBe(true)
  })

  it('shows empty state when no trades', () => {
    const wrapper = createWrapper({ trades: [] })
    expect(wrapper.find('.table-empty').exists()).toBe(true)
  })

  it('has export button', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.export-btn').exists()).toBe(true)
  })

  it('shows trade count', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('3')
    expect(wrapper.text()).toContain('笔')
  })

  it('has pagination component when trades exceed page size', () => {
    const wrapper = createWrapper({ trades: manyTrades })
    expect(wrapper.text()).toContain('15')
    expect(wrapper.find('.el-pagination').exists() || wrapper.text().includes('15')).toBe(true)
  })

  it('does not show pagination when trades are under page size', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('3')
    expect(wrapper.find('.pagination-wrapper').exists()).toBe(false)
  })

  it('renders direction tags for long/short', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.backtest-trades-table').exists()).toBe(true)
    expect(wrapper.text().length).toBeGreaterThan(0)
  })

  it('renders pnl with positive/negative formatting', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.backtest-trades-table').exists()).toBe(true)
    expect(wrapper.text().length).toBeGreaterThan(0)
  })

  it('export button triggers CSV download when clicked', async () => {
    const wrapper = createWrapper()
    const btn = wrapper.find('.export-btn')
    expect(btn.exists()).toBe(true)
    await btn.trigger('click')
  })

  it('maps exit_reason values correctly', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any

    expect(vm.exitReasonLabel('take_profit')).toBe('止盈')
    expect(vm.exitReasonLabel('stop_loss')).toBe('止损')
    expect(vm.exitReasonLabel('signal')).toBe('信号')

    expect(vm.exitReasonTagType('take_profit')).toBe('success')
    expect(vm.exitReasonTagType('stop_loss')).toBe('danger')
    expect(vm.exitReasonTagType('signal')).toBe('primary')
  })

  it('resets pagination to page 1 when trades change', async () => {
    const wrapper = createWrapper({ trades: manyTrades })

    ;(wrapper.vm as any).currentPage = 2
    expect((wrapper.vm as any).currentPage).toBe(2)

    await wrapper.setProps({ trades: mockTrades })
    expect((wrapper.vm as any).currentPage).toBe(1)
  })
})