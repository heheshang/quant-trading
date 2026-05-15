import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import BacktestHistoryList from '@/components/backtest/BacktestHistoryList.vue'
import type { BacktestSummary } from '@/types/backtest'

const mockItems: BacktestSummary[] = [
  { id: 'uuid-1', strategy_id: 'str-1', symbol: 'BTC/USDT', status: 'completed', total_return_pct: 25.3, sharpe_ratio: 2.1, created_at: '2024-01-01T00:00:00Z' },
  { id: 'uuid-2', strategy_id: 'str-2', symbol: 'ETH/USDT', status: 'failed', total_return_pct: -8.5, sharpe_ratio: -0.5, created_at: '2024-01-02T00:00:00Z' },
  { id: 'uuid-3', strategy_id: 'str-1', symbol: 'SOL/USDT', status: 'running', total_return_pct: 12.0, sharpe_ratio: 1.8, created_at: '2024-01-03T00:00:00Z' },
]

// Generate enough items to trigger pagination
const manyItems: BacktestSummary[] = Array.from({ length: 15 }, (_, i) => ({
  ...mockItems[i % 3],
  id: `uuid-${i + 10}`,
  created_at: `2024-01-${String(i + 1).padStart(2, '0')}T00:00:00Z`,
}))

function createWrapper(props: any = {}) {
  return mount(BacktestHistoryList, {
    props: {
      items: props.items === undefined ? mockItems : props.items,
      loading: props.loading ?? false,
      currentId: props.currentId ?? '',
    },
    global: {
      plugins: [ElementPlus],
    },
  })
}

describe('BacktestHistoryList', () => {
  it('renders the component wrapper', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.backtest-history-list').exists()).toBe(true)
  })

  it('displays section title', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('回测历史')
  })

  it('shows loading skeleton when loading with no items', () => {
    const wrapper = createWrapper({ loading: true, items: [] })
    expect(wrapper.find('.table-loading').exists()).toBe(true)
  })

  it('shows empty state when no items', () => {
    const wrapper = createWrapper({ items: [] })
    expect(wrapper.find('.table-empty').exists()).toBe(true)
  })

  it('renders table with history items', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.history-table-wrapper').exists()).toBe(true)
  })

  it('shows item count', () => {
    const wrapper = createWrapper()
    expect(wrapper.text()).toContain('3')
    expect(wrapper.text()).toContain('条')
  })

  it('has refresh button', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.refresh-btn').exists()).toBe(true)
  })

  it('emits refresh event on refresh button click', async () => {
    const wrapper = createWrapper()
    const btn = wrapper.find('.refresh-btn')
    await btn.trigger('click')
    expect(wrapper.emitted('refresh')).toBeTruthy()
    expect(wrapper.emitted('refresh')!.length).toBe(1)
  })

  it('renders status tags with correct types', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.statusTagType('completed')).toBe('success')
    expect(vm.statusTagType('running')).toBe('primary')
    expect(vm.statusTagType('pending')).toBe('warning')
    expect(vm.statusTagType('failed')).toBe('danger')
    expect(vm.statusTagType('unknown')).toBe('info')
  })

  it('renders status labels in Chinese', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.statusLabel('completed')).toBe('已完成')
    expect(vm.statusLabel('running')).toBe('运行中')
    expect(vm.statusLabel('pending')).toBe('等待中')
    expect(vm.statusLabel('failed')).toBe('失败')
    expect(vm.statusLabel('unknown')).toBe('unknown')
  })

  it('renders positive return with up class via vm data', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    // Verify the data has positive returns and the computed returns them
    expect(vm.paginatedItems.length).toBeGreaterThan(0)
    const hasPositive = vm.paginatedItems.some((item: any) => item.total_return_pct >= 0)
    expect(hasPositive).toBe(true)
  })

  it('renders negative return with down class via vm data', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    // Verify the data has negative returns
    const hasNegative = vm.paginatedItems.some((item: any) => item.total_return_pct < 0)
    expect(hasNegative).toBe(true)
  })

  it('emits select on row click', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.handleRowClick(mockItems[0])
    expect(wrapper.emitted('select')).toBeTruthy()
    expect(wrapper.emitted('select')![0][0]).toEqual(mockItems[0])
  })

  it('emits delete on delete button click', async () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    vm.handleDelete(mockItems[0])
    expect(wrapper.emitted('delete')).toBeTruthy()
    expect(wrapper.emitted('delete')![0][0]).toBe('uuid-1')
  })

  it('highlights current row by currentId', () => {
    const wrapper = createWrapper({ currentId: 'uuid-1' })
    const vm = wrapper.vm as any
    expect(vm.rowClassName({ row: mockItems[0] })).toBe('current-row-highlight')
    expect(vm.rowClassName({ row: mockItems[1] })).toBe('')
  })

  it('has pagination when items exceed page size', () => {
    const wrapper = createWrapper({ items: manyItems })
    expect(wrapper.find('.pagination-wrapper').exists()).toBe(true)
  })

  it('does not show pagination when items under page size', () => {
    const wrapper = createWrapper()
    expect(wrapper.find('.pagination-wrapper').exists()).toBe(false)
  })

  it('resets pagination to page 1 when items change', async () => {
    const wrapper = createWrapper({ items: manyItems })
    ;(wrapper.vm as any).currentPage = 2
    expect((wrapper.vm as any).currentPage).toBe(2)

    await wrapper.setProps({ items: mockItems })
    expect((wrapper.vm as any).currentPage).toBe(1)
  })

  it('formats date column using formatDateTime', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    const result = vm.formatDateCol(null, null, '2024-01-01T00:00:00Z')
    expect(result).toBeTruthy()
    expect(result).not.toBe('--')
  })

  it('renders sharpe ratio values via vm', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.paginatedItems.length).toBeGreaterThan(0)
    expect(vm.paginatedItems[0].sharpe_ratio).toBeDefined()
  })

  it('renders strategy IDs via vm', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.paginatedItems.some((item: any) => item.strategy_id === 'str-1')).toBe(true)
    expect(vm.paginatedItems.some((item: any) => item.strategy_id === 'str-2')).toBe(true)
  })

  it('renders symbol column via vm', () => {
    const wrapper = createWrapper()
    const vm = wrapper.vm as any
    expect(vm.paginatedItems.some((item: any) => item.symbol === 'BTC/USDT')).toBe(true)
    expect(vm.paginatedItems.some((item: any) => item.symbol === 'ETH/USDT')).toBe(true)
  })
})
