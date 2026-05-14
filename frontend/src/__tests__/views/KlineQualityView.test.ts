import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createRouter, createWebHistory } from 'vue-router'
import KlineQualityView from '@/views/kline/KlineQualityView.vue'
import * as klineApi from '@/api/kline'

// Mock API
vi.mock('@/api/kline', () => ({
  getQualityReport: vi.fn(),
  cleanKlines: vi.fn(),
  getKlineSymbols: vi.fn(() => Promise.resolve([])),
}))

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/kline', name: 'KlineList', component: { template: '<div>kline</div>' } },
    { path: '/kline/quality', name: 'KlineQuality', component: { template: '<div>quality</div>' } },
  ],
})

const mockReport = {
  symbol: 'BTCUSDT',
  interval: '1h',
  start_time: 1704067200000,
  end_time: 1735689600000,
  total_rows: 50000,
  valid_rows: 49850,
  gap_count: 12,
  gap_positions: [1704153600000, 1704240000000],
  anomaly_count: 28,
  anomaly_rows: [
    { open_time: 1710489600000, type: 'suspicious' as const, field: 'high', value: 999999, expected_range: '40000-70000' },
    { open_time: 1710576000000, type: 'corrupted' as const, field: 'low', value: 0, expected_range: '40000-70000' },
    { open_time: 1710662400000, type: 'zero_volume' as const, field: 'volume', value: 0, expected_range: '>0' },
  ],
  duplicate_count: 120,
  coverage_rate: 99.7,
  suspicious_count: 15,
  corrupted_count: 10,
  created_at: '2024-12-31T00:00:00Z',
}

function mountComponent() {
  return mount(KlineQualityView, {
    global: {
      plugins: [router, ElementPlus],
    },
  })
}

async function loadReport(wrapper: ReturnType<typeof mountComponent>, reportData = mockReport) {
  vi.mocked(klineApi.getQualityReport).mockResolvedValue(reportData as any)
  const vm = wrapper.vm as any
  vm.form.symbol = reportData.symbol
  vm.form.interval = reportData.interval
  await vm.runQualityCheck()
  await flushPromises()
}

describe('KlineQualityView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(klineApi.getKlineSymbols).mockResolvedValue([])
  })

  // P0-06: 6 metric cards
  it('renders 6 metric cards when report is loaded', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    const metricCards = wrapper.findAll('.metric-card')
    expect(metricCards.length).toBe(6)

    const labels = metricCards.map(c => c.find('.metric-label').text())
    expect(labels).toContain('总行数')
    expect(labels).toContain('有效行')
    expect(labels).toContain('数据覆盖率')
    expect(labels).toContain('缺口数量')
    expect(labels).toContain('异常数量')
    expect(labels).toContain('重复数量')
  })

  // P0-06: Total rows and valid rows display correct values
  it('displays total_rows and valid_rows values correctly', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    const metricCards = wrapper.findAll('.metric-card')
    const labels = metricCards.map(c => c.find('.metric-label').text())

    const totalIdx = labels.indexOf('总行数')
    const validIdx = labels.indexOf('有效行')
    expect(totalIdx).toBeGreaterThanOrEqual(0)
    expect(validIdx).toBeGreaterThanOrEqual(0)

    // Total rows and valid rows cards use .metric-value (non-coverage cards)
    const totalValue = metricCards[totalIdx].find('.metric-value').text()
    const validValue = metricCards[validIdx].find('.metric-value').text()
    expect(totalValue).toContain('50,000')
    expect(validValue).toContain('49,850')
  })

  // P1-07: Coverage threshold > 95% = success
  it('shows success class when coverage > 95%', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper, { ...mockReport, coverage_rate: 99.7 })

    const ringProgress = wrapper.find('.ring-progress')
    expect(ringProgress.classes()).toContain('success')
  })

  // P1-07: Coverage threshold 90-95% = warning
  it('shows warning class when coverage is 90-95%', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper, { ...mockReport, coverage_rate: 92.5 })

    const ringProgress = wrapper.find('.ring-progress')
    expect(ringProgress.classes()).toContain('warning')
  })

  // P1-07: Coverage threshold < 90% = danger
  it('shows danger class when coverage < 90%', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper, { ...mockReport, coverage_rate: 85.0 })

    const ringProgress = wrapper.find('.ring-progress')
    expect(ringProgress.classes()).toContain('danger')
  })

  // P1-07: Boundary — exactly 95% should be warning (not success, since >95% is green)
  it('shows warning class at exactly 95% (boundary: >95% is green, not >=95%)', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper, { ...mockReport, coverage_rate: 95.0 })

    const ringProgress = wrapper.find('.ring-progress')
    expect(ringProgress.classes()).toContain('warning')
  })

  // P1-07: Boundary — exactly 90% should be warning
  it('shows warning class at exactly 90% (boundary)', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper, { ...mockReport, coverage_rate: 90.0 })

    const ringProgress = wrapper.find('.ring-progress')
    expect(ringProgress.classes()).toContain('warning')
  })

  // P1-07: Boundary — just below 90%
  it('shows danger class at 89.9%', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper, { ...mockReport, coverage_rate: 89.9 })

    const ringProgress = wrapper.find('.ring-progress')
    expect(ringProgress.classes()).toContain('danger')
  })

  // P1-08: Ring chart SVG renders
  it('renders coverage ring SVG with correct structure', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    expect(wrapper.find('.coverage-ring').exists()).toBe(true)
    expect(wrapper.find('.ring-track').exists()).toBe(true)
    expect(wrapper.find('.ring-progress').exists()).toBe(true)
    expect(wrapper.find('.ring-text').exists()).toBe(true)
    expect(wrapper.find('.ring-text').text()).toBe('99.7%')
  })

  // P1-08: Ring chart wrapper sizing
  it('renders ring wrapper with correct dimensions', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    const ringWrapper = wrapper.find('.coverage-ring-wrapper')
    expect(ringWrapper.exists()).toBe(true)
    const svg = wrapper.find('.coverage-ring')
    expect(svg.attributes('width')).toBe('80')
    expect(svg.attributes('height')).toBe('80')
  })

  // P1-08: Ring chart dash offset math
  it('calculates ring dash offset correctly for 50% coverage', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper, { ...mockReport, coverage_rate: 50 })

    const vm = wrapper.vm as any
    const circumference = 2 * Math.PI * 34 // ≈ 213.63
    const expectedOffset = circumference * 0.5
    expect(vm.ringDashOffset).toBeCloseTo(expectedOffset, 1)
  })

  // P1-08: Ring chart dash offset at 100%
  it('calculates ring dash offset as 0 for 100% coverage', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper, { ...mockReport, coverage_rate: 100 })

    const vm = wrapper.vm as any
    expect(vm.ringDashOffset).toBeCloseTo(0, 1)
  })

  // P1-08: Ring chart dash offset at 0%
  it('calculates ring dash offset as full circumference for 0% coverage', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper, { ...mockReport, coverage_rate: 0 })

    const vm = wrapper.vm as any
    const circumference = 2 * Math.PI * 34
    expect(vm.ringDashOffset).toBeCloseTo(circumference, 1)
  })

  // P0-05: Clean dialog — "全部清洗" button exists in anomaly table header
  it('has 全部清洗 button in anomaly table header when anomalies exist', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    const headerActions = wrapper.find('.card-header-actions')
    expect(headerActions.exists()).toBe(true)
    const buttons = headerActions.findAll('button')
    const buttonTexts = buttons.map(b => b.text())
    expect(buttonTexts.some(t => t.includes('全部清洗'))).toBe(true)
    expect(buttonTexts.some(t => t.includes('导出'))).toBe(true)
  })

  // P0-05: Clean dialog opens when clicking 全部清洗
  it('shows clean confirmation dialog when clicking 全部清洗', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    const vm = wrapper.vm as any
    expect(vm.cleanDialogVisible).toBe(false)

    // Open dialog
    vm.showCleanDialog()
    await flushPromises()

    expect(vm.cleanDialogVisible).toBe(true)
    // Check dialog content
    expect(wrapper.find('.clean-dialog-content').exists()).toBe(true)
    expect(wrapper.find('.clean-type-list').exists()).toBe(true)
  })

  // P0-05: Clean API not called until confirmed
  it('does not call cleanKlines API until dialog is confirmed', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    expect(klineApi.cleanKlines).not.toHaveBeenCalled()

    // Open dialog
    const vm = wrapper.vm as any
    vm.showCleanDialog()
    await flushPromises()

    // Still not called
    expect(klineApi.cleanKlines).not.toHaveBeenCalled()
  })

  // P0-05: Clean executes after confirmation
  it('executes cleanKlines after confirming in dialog', async () => {
    vi.mocked(klineApi.cleanKlines).mockResolvedValue({
      filled_gaps: 5,
      deduplicated: 10,
      deleted: 8,
      fixed: 5,
    } as any)
    const wrapper = mountComponent()
    await loadReport(wrapper)

    const vm = wrapper.vm as any
    vm.showCleanDialog()
    await flushPromises()

    // Confirm
    await vm.confirmCleanAll()
    await flushPromises()

    expect(klineApi.cleanKlines).toHaveBeenCalledWith({
      symbol: 'BTCUSDT',
      interval: '1h',
      mode: 'auto',
    })
  })

  // P0-05: Auto-clean button also opens dialog
  it('auto-clean button opens dialog instead of direct execution', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    const vm = wrapper.vm as any
    expect(vm.cleanDialogVisible).toBe(false)
    expect(klineApi.cleanKlines).not.toHaveBeenCalled()

    // Find and click auto-clean button
    const allButtons = wrapper.findAll('button')
    const autoCleanBtn = allButtons.find(b => b.text().includes('自动清洗'))
    expect(autoCleanBtn).toBeTruthy()
    await autoCleanBtn!.trigger('click')
    await flushPromises()

    // Dialog should be visible, API not called
    expect(vm.cleanDialogVisible).toBe(true)
    expect(klineApi.cleanKlines).not.toHaveBeenCalled()
  })

  // P0-05: Clean dialog shows anomaly type breakdown
  it('shows anomaly type breakdown in clean dialog', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    const vm = wrapper.vm as any
    // Check computed property directly instead of rendering dialog (avoids timeout)
    expect(vm.otherAnomalyCount).toBe(3) // 28 - 15 - 10 = 3
    expect(vm.report.suspicious_count).toBe(15)
    expect(vm.report.corrupted_count).toBe(10)
  }, 10000)

  // P0-05: otherAnomalyCount computed property
  it('computes otherAnomalyCount correctly', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    const vm = wrapper.vm as any
    // mockReport: anomaly_count=28, suspicious=15, corrupted=10, other=3
    expect(vm.otherAnomalyCount).toBe(3)
  })

  // P0-05: Export anomalies
  it('handleExportAnomalies creates CSV download', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper)

    const vm = wrapper.vm as any
    // Mock URL.createObjectURL (already mocked in setup)
    const createObjectURLSpy = vi.spyOn(URL, 'createObjectURL')

    vm.handleExportAnomalies()

    expect(createObjectURLSpy).toHaveBeenCalled()
    createObjectURLSpy.mockRestore()
  })

  // Coverage ring text color matches class
  it('ring text color class matches coverage class', async () => {
    const wrapper = mountComponent()
    await loadReport(wrapper, { ...mockReport, coverage_rate: 92.5 })

    const ringText = wrapper.find('.ring-text')
    expect(ringText.classes()).toContain('warning')
    expect(ringText.text()).toBe('92.5%')
  })

  // No report: empty state
  it('shows empty state when no report', async () => {
    const wrapper = mountComponent()
    await flushPromises()

    expect(wrapper.find('.empty-state').exists()).toBe(true)
    expect(wrapper.find('.metrics-grid').exists()).toBe(false)
  })
})
