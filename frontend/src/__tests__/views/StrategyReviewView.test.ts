import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import StrategyReviewView from '@/views/strategy/StrategyReviewView.vue'
import { listPendingReviews, approveStrategy, rejectStrategy } from '@/api/review'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { ReviewResponse } from '@/api/review'

vi.mock('@/api/review', () => ({
  listPendingReviews: vi.fn(),
  approveStrategy: vi.fn(),
  rejectStrategy: vi.fn(),
}))

vi.mock('element-plus', async () => {
  const actual = await vi.importActual('element-plus')
  return {
    ...actual,
    ElMessage: { error: vi.fn(), success: vi.fn(), warning: vi.fn() },
    ElMessageBox: { confirm: vi.fn() },
  }
})

const mockReviews: ReviewResponse[] = [
  {
    id: '1',
    strategy_id: '550e8400-e29b-41d4-a716-446655440001',
    review_status: 'pending_review',
    rejection_reason: null,
    submitted_at: '2025-05-20T10:00:00Z',
    reviewed_at: null,
    reviewed_by: null,
  },
]

describe('StrategyReviewView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders page header and calls listPendingReviews on mount', async () => {
    vi.mocked(listPendingReviews).mockResolvedValue({
      data: { items: [], page: 1, size: 20, total: 0 },
    } as any)

    const wrapper = mount(StrategyReviewView, {
      global: { plugins: [ElementPlus] },
    })
    await flushPromises()

    expect(wrapper.find('.page-header').exists()).toBe(true)
    expect(listPendingReviews).toHaveBeenCalled()
  })

  it('shows loading state while fetching', async () => {
    vi.mocked(listPendingReviews).mockReturnValue(new Promise(() => {}) as any)

    const wrapper = mount(StrategyReviewView, {
      global: { plugins: [ElementPlus] },
    })

    expect(wrapper.find('.el-table').exists()).toBe(true)
  })

  it('shows error message on API failure', async () => {
    vi.mocked(listPendingReviews).mockRejectedValue(new Error('Network error'))

    mount(StrategyReviewView, {
      global: { plugins: [ElementPlus] },
    })
    await flushPromises()

    expect(ElMessage.error).toHaveBeenCalled()
  })

  it('approve button calls approveStrategy after confirmation', async () => {
    vi.mocked(listPendingReviews).mockResolvedValue({
      data: { items: mockReviews, page: 1, size: 20, total: 1 },
    } as any)
    vi.mocked(approveStrategy).mockResolvedValue({} as any)
    vi.mocked(ElMessageBox.confirm).mockResolvedValue(true as any)

    const wrapper = mount(StrategyReviewView, {
      global: { plugins: [ElementPlus] },
    })
    await flushPromises()

    const buttons = wrapper.findAll('.el-button')
    const approveBtn = buttons.find(b => b.text() === '批准')
    await approveBtn!.trigger('click')
    await flushPromises()

    expect(approveStrategy).toHaveBeenCalledWith({ strategy_id: mockReviews[0].strategy_id })
    expect(ElMessage.success).toHaveBeenCalledWith('策略已批准')
  })

  it('reject button shows rejection dialog', async () => {
    vi.mocked(listPendingReviews).mockResolvedValue({
      data: { items: mockReviews, page: 1, size: 20, total: 1 },
    } as any)

    const wrapper = mount(StrategyReviewView, {
      global: { plugins: [ElementPlus] },
    })
    await flushPromises()

    const buttons = wrapper.findAll('.el-button')
    const rejectBtn = buttons.find(b => b.text() === '拒绝')
    await rejectBtn!.trigger('click')
    await flushPromises()

    expect(wrapper.find('.el-dialog__header').exists()).toBe(true)
  })
})
