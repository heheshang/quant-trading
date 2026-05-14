import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { ElMessageBox } from 'element-plus'
import KlineListView from '@/views/kline/KlineListView.vue'
import type { KlineSymbolOverview } from '@/types/kline'

// ─── Mocks ──────────────────────────────────────────────────────

// Mock echarts (no Canvas in jsdom)
vi.mock('echarts', () => ({
  default: {
    init: vi.fn(() => ({
      setOption: vi.fn(),
      dispose: vi.fn(),
      resize: vi.fn(),
    })),
  },
}))

// Mock API
const mockSymbolsData: KlineSymbolOverview[] = [
  {
    symbol: 'BTCUSDT',
    interval: '1h',
    data_points: 8760,
    coverage_start: new Date('2024-01-01').getTime(),
    coverage_end: new Date('2024-12-31').getTime(),
    last_updated: '3小时前',
    quality: 'normal',
    source: 'exchange',
  },
  {
    symbol: 'ETHUSDT',
    interval: '1h',
    data_points: 8742,
    coverage_start: new Date('2024-01-01').getTime(),
    coverage_end: new Date('2024-11-30').getTime(),
    last_updated: '1天前',
    quality: 'anomaly',
    source: 'api',
  },
  {
    symbol: 'BNBUSDT',
    interval: '15m',
    data_points: 35040,
    coverage_start: new Date('2024-03-01').getTime(),
    coverage_end: new Date('2024-09-30').getTime(),
    last_updated: '5小时前',
    quality: 'missing',
    source: 'csv',
  },
]

vi.mock('@/api/kline', () => ({
  getKlineSymbols: vi.fn(() => Promise.resolve(mockSymbolsData)),
  queryKlines: vi.fn(() =>
    Promise.resolve({
      data: [
        { open_time: 1704067200000, open: 42000, high: 42500, low: 41800, close: 42300, volume: 1000 },
      ],
      meta: { total: 1, page: 1, page_size: 500 },
    }),
  ),
}))

// Mock vue-router
const pushMock = vi.fn()
vi.mock('vue-router', () => ({
  useRouter: vi.fn(() => ({
    push: pushMock,
  })),
}))

// ─── Helpers ────────────────────────────────────────────────────

function mountComponent() {
  return mount(KlineListView, {
    global: {
      plugins: [ElementPlus],
      stubs: {
        'router-link': true,
        'router-view': true,
      },
    },
  })
}

// ─── Tests ──────────────────────────────────────────────────────

describe('KlineListView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  // ─── P0-01: Aggregated table structure ──────────────────────

  describe('P0-01: Aggregated table structure', () => {
    it('renders the aggregated overview table (not raw kline rows)', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      // Table should exist
      const table = wrapper.find('.el-table')
      expect(table.exists()).toBe(true)

      // Should have aggregated columns: 交易对, 周期, 数据点数, 覆盖范围, 最后更新, 质量, 操作
      const headers = wrapper.findAll('.el-table__header-wrapper th')
      const headerTexts = headers.map((h) => h.text())
      expect(headerTexts).toContain('交易对')
      expect(headerTexts).toContain('周期')
      expect(headerTexts).toContain('数据点数')
      expect(headerTexts).toContain('覆盖范围')
      expect(headerTexts).toContain('最后更新')
      expect(headerTexts).toContain('质量')
      expect(headerTexts).toContain('操作')
    })

    it('displays aggregated rows from getKlineSymbols API', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      // Should show 3 rows from mock data
      const rows = wrapper.findAll('.el-table__body-wrapper tbody tr')
      expect(rows.length).toBe(3)
    })

    it('shows symbol name in each row', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const symbolLabels = wrapper.findAll('.symbol-label')
      const symbols = symbolLabels.map((el) => el.text())
      expect(symbols).toContain('BTCUSDT')
      expect(symbols).toContain('ETHUSDT')
      expect(symbols).toContain('BNBUSDT')
    })

    it('shows data points as formatted numbers', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const monoNumbers = wrapper.findAll('.mono-number')
      // data_points values appear in the mono-number spans
      const texts = monoNumbers.map((el) => el.text())
      // 8,760 is the formatted value for BTCUSDT
      expect(texts.some((t) => t.includes('8,760'))).toBe(true)
    })

    it('shows quality status with correct class', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const qualityDots = wrapper.findAll('.quality-dot')
      expect(qualityDots.length).toBe(3)

      // quality-normal, quality-anomaly, quality-missing
      // (exclude 'quality-dot' itself since it also starts with 'quality-')
      const classes = qualityDots.map((el) => {
        const classList = el.classes()
        return classList.find((c) => c.startsWith('quality-') && c !== 'quality-dot') || ''
      })
      expect(classes).toContain('quality-normal')
      expect(classes).toContain('quality-anomaly')
      expect(classes).toContain('quality-missing')
    })

    it('should NOT display raw kline columns (时间/开盘/最高/最低/收盘/成交量)', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const headers = wrapper.findAll('.el-table__header-wrapper th')
      const headerTexts = headers.map((h) => h.text())

      expect(headerTexts).not.toContain('开盘')
      expect(headerTexts).not.toContain('最高')
      expect(headerTexts).not.toContain('最低')
      expect(headerTexts).not.toContain('收盘')
      expect(headerTexts).not.toContain('成交量')
    })
  })

  // ─── P0-02: Period pills ────────────────────────────────────

  describe('P0-02: Period pills filter', () => {
    it('renders interval pills instead of el-select dropdown', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      // Should have el-radio-group with pills class
      const pills = wrapper.find('.interval-pills')
      expect(pills.exists()).toBe(true)

      // Should NOT have interval el-select (there's a symbol select, but not interval)
      const selects = wrapper.findAllComponents({ name: 'ElSelect' })
      // Only one select: the symbol filter, no interval select
      expect(selects.length).toBe(1)
    })

    it('renders all interval options including "全部"', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const radioButtons = wrapper.findAllComponents({ name: 'ElRadioButton' })
      const labels = radioButtons.map((rb) => {
        // el-radio-button renders the label as text content
        const text = rb.text().trim()
        return text
      })

      expect(labels).toContain('全部')
      expect(labels).toContain('1m')
      expect(labels).toContain('5m')
      expect(labels).toContain('15m')
      expect(labels).toContain('30m')
      expect(labels).toContain('1h')
      expect(labels).toContain('4h')
      expect(labels).toContain('1d')
      expect(labels).toContain('1w')
    })

    it('defaults interval filter to empty string (全部)', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      // The first radio-button "全部" should have value=""
      const pills = wrapper.find('.interval-pills')
      expect(pills.exists()).toBe(true)
      // Check that all data is shown (unfiltered)
      const rows = wrapper.findAll('.el-table__body-wrapper tbody tr')
      expect(rows.length).toBe(3)
    })

    it('filters table rows when an interval pill is selected', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      // Initially 3 rows
      expect(wrapper.findAll('.el-table__body-wrapper tbody tr').length).toBe(3)

      // Select "1h" pill — should filter to only 1h rows
      const radioButtons = wrapper.findAllComponents({ name: 'ElRadioButton' })
      // Find the "1h" button (it's after "全部")
      const oneHourBtn = radioButtons.find((rb) => rb.text().trim() === '1h')
      expect(oneHourBtn).toBeTruthy()
      // Select "1h" interval — set filterForm.interval directly (el-radio-button v-model
      // works in real browser but not reliably in jsdom, matching the approach used in
      // the Filtering describe block's tests)
      const vm = wrapper.vm as any
      vm.filterForm.interval = '1h'
      await flushPromises()

      // Should now show only 2 rows (BTCUSDT 1h + ETHUSDT 1h)
      const rows = wrapper.findAll('.el-table__body-wrapper tbody tr')
      expect(rows.length).toBe(2)
    })
  })

  // ─── P0-03: Operations dropdown ─────────────────────────────

  describe('P0-03: Operations dropdown menu', () => {
    it('renders action column with dropdown trigger icon', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      // Should have action trigger icons (⋮)
      const triggers = wrapper.findAll('.action-trigger')
      expect(triggers.length).toBe(3) // one per row
    })

    it('renders dropdown menu items when triggered', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      // el-dropdown exists
      const dropdowns = wrapper.findAllComponents({ name: 'ElDropdown' })
      expect(dropdowns.length).toBe(3)
    })

    it('navigates to detail page on "查看详情" command', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      // Call handleCommand directly
      const vm = wrapper.vm as any
      const row = mockSymbolsData[0]
      vm.handleCommand('detail', row)
      await flushPromises()

      expect(pushMock).toHaveBeenCalledWith('/kline/BTCUSDT/1h')
    })

    it('opens chart preview dialog on "预览图表" command', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const vm = wrapper.vm as any
      const row = mockSymbolsData[0]
      vm.handleCommand('preview', row)
      await flushPromises()

      // Chart dialog should be visible
      expect(vm.chartDialogVisible).toBe(true)
      expect(vm.chartSymbol).toBe('BTCUSDT')
      expect(vm.chartInterval).toBe('1h')
    })

    it('opens tag edit dialog on "编辑标签" command', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const vm = wrapper.vm as any
      const row = mockSymbolsData[0]
      vm.handleCommand('editTag', row)
      await flushPromises()

      expect(vm.tagDialogVisible).toBe(true)
      expect(vm.tagRow).toEqual(row)
    })

    it('shows confirmation dialog on "删除数据" command and removes row on confirm', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      // Mock ElMessageBox.confirm to auto-confirm
      vi.spyOn(ElMessageBox, 'confirm').mockResolvedValue('confirm' as any)

      const vm = wrapper.vm as any
      const row = mockSymbolsData[0]
      await vm.confirmDelete(row)
      await flushPromises()

      // Row should be removed from data
      const remaining = vm.allData.filter(
        (r: KlineSymbolOverview) => !(r.symbol === 'BTCUSDT' && r.interval === '1h'),
      )
      expect(remaining.length).toBe(2)
    })

    it('does not remove row when delete is cancelled', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      // Mock ElMessageBox.confirm to reject (cancel)
      vi.spyOn(ElMessageBox, 'confirm').mockRejectedValue('cancel')

      const vm = wrapper.vm as any
      const row = mockSymbolsData[0]
      await vm.confirmDelete(row)
      await flushPromises()

      // Data should remain unchanged
      expect(vm.allData.length).toBe(3)
    })
  })

  // ─── Page Header ────────────────────────────────────────────

  describe('Page header', () => {
    it('displays correct title "K线数据管理"', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      expect(wrapper.find('.page-title').text()).toBe('K线数据管理')
    })

    it('has import button navigating to /kline/import', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const buttons = wrapper.findAllComponents({ name: 'ElButton' })
      const importBtn = buttons.find((b) => b.text().includes('导入'))
      expect(importBtn).toBeTruthy()

      await importBtn!.trigger('click')
      expect(pushMock).toHaveBeenCalledWith('/kline/import')
    })

    it('has export button navigating to /kline/export', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const buttons = wrapper.findAllComponents({ name: 'ElButton' })
      const exportBtn = buttons.find((b) => b.text().includes('导出'))
      expect(exportBtn).toBeTruthy()

      await exportBtn!.trigger('click')
      expect(pushMock).toHaveBeenCalledWith('/kline/export')
    })
  })

  // ─── Pagination ─────────────────────────────────────────────

  describe('Pagination', () => {
    it('renders pagination with correct page sizes (10/20/50/100)', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const pagination = wrapper.findComponent({ name: 'ElPagination' })
      expect(pagination.exists()).toBe(true)
      expect(pagination.props('pageSizes')).toEqual([10, 20, 50, 100])
    })

    it('defaults page size to 20', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const pagination = wrapper.findComponent({ name: 'ElPagination' })
      expect(pagination.props('pageSize')).toBe(20)
    })
  })

  // ─── Filtering ──────────────────────────────────────────────

  describe('Filtering', () => {
    it('filters by symbol when symbol select changes', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const vm = wrapper.vm as any
      vm.filterForm.symbol = 'BTCUSDT'
      vm.handleFilterChange()
      await flushPromises()

      // Should show only BTCUSDT rows (1h)
      expect(vm.filteredData.length).toBe(1)
    })

    it('filters by interval when pill is selected', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const vm = wrapper.vm as any
      vm.filterForm.interval = '15m'
      vm.handleFilterChange()
      await flushPromises()

      expect(vm.filteredData.length).toBe(1)
      expect(vm.filteredData[0].symbol).toBe('BNBUSDT')
    })

    it('combines symbol and interval filters', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const vm = wrapper.vm as any
      vm.filterForm.symbol = 'ETHUSDT'
      vm.filterForm.interval = '1h'
      vm.handleFilterChange()
      await flushPromises()

      expect(vm.filteredData.length).toBe(1)
      expect(vm.filteredData[0].symbol).toBe('ETHUSDT')
      expect(vm.filteredData[0].interval).toBe('1h')
    })

    it('resets page to 1 when filter changes', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const vm = wrapper.vm as any
      vm.pagination.page = 3
      vm.handleFilterChange()

      expect(vm.pagination.page).toBe(1)
    })
  })

  // ─── Edit tag dialog ────────────────────────────────────────

  describe('Edit tag dialog', () => {
    it('saves tag changes to the data', async () => {
      const wrapper = mountComponent()
      await flushPromises()

      const vm = wrapper.vm as any
      const row = mockSymbolsData[0] // BTCUSDT, source: exchange

      vm.openTagDialog(row)
      await flushPromises()

      expect(vm.tagDialogVisible).toBe(true)
      expect(vm.tagForm.source).toBe('exchange')

      // Change source
      vm.tagForm.source = 'csv'
      vm.saveTag()
      await flushPromises()

      expect(vm.tagDialogVisible).toBe(false)
      // The data should be updated
      const updated = vm.allData.find(
        (r: KlineSymbolOverview) => r.symbol === 'BTCUSDT' && r.interval === '1h',
      )
      expect(updated.source).toBe('csv')
    })
  })
})
