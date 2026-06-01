import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import AIPredictPanel from '@/components/ai/AIPredictPanel.vue'
import type { AIPredictResponse } from '@/types/ai'

const mockPrediction: AIPredictResponse = {
  symbol: 'BTCUSDT',
  interval: '1h',
  direction: 'long',
  confidence: 0.78,
  signal: 'buy',
  analysis: '根据技术指标分析，1h级别呈现上升趋势，RSI 55处于多头区域。',
  price_target: 74500,
  indicators: {
    rsi_14: 55.3,
    macd: 63.64,
    macd_signal: 90.36,
    macd_hist: -26.72,
    sma_7: 74022.3,
    sma_25: 73933.91,
    ema_20: 73946.89,
    bb_upper: 74168.85,
    bb_middle: 73997.35,
    bb_lower: 73825.85,
    atr_14: 168.44,
    volume_ma_20: 256.69,
    current_price: 73907.37,
  },
  generated_at: '2026-05-31T12:00:00Z',
}

describe('AIPredictPanel', () => {
  it('renders empty state when prediction is null', () => {
    const wrapper = mount(AIPredictPanel, {
      props: { prediction: null, status: 'idle' },
    })
    expect(wrapper.find('.empty-state').exists()).toBe(true)
    expect(wrapper.find('.ws-text').text()).toBe('未连接')
  })

  it('renders prediction data correctly', () => {
    const wrapper = mount(AIPredictPanel, {
      props: { prediction: mockPrediction, status: 'connected' },
    })
    expect(wrapper.find('.signal-tag.long').exists()).toBe(true)
    expect(wrapper.find('.signal-tag.long').text()).toBe('买入')
    expect(wrapper.find('.confidence-value').text()).toBe('78%')
    expect(wrapper.find('.analysis-text').text()).toContain('上升趋势')
  })

  it('renders price target when available', () => {
    const wrapper = mount(AIPredictPanel, {
      props: { prediction: mockPrediction, status: 'connected' },
    })
    expect(wrapper.find('.pt-value').text()).toBe('74,500.00')
  })

  it('hides price target when null', () => {
    const noTarget: AIPredictResponse = { ...mockPrediction, price_target: null }
    const wrapper = mount(AIPredictPanel, {
      props: { prediction: noTarget, status: 'connected' },
    })
    expect(wrapper.find('.price-target').exists()).toBe(false)
  })

  it('shows connecting status', () => {
    const wrapper = mount(AIPredictPanel, {
      props: { prediction: null, status: 'connecting' },
    })
    expect(wrapper.find('.ws-text').text()).toBe('连接中...')
    expect(wrapper.find('.empty-text').text()).toBe('正在连接 AI 服务...')
  })

  it('shows disconnected status', () => {
    const wrapper = mount(AIPredictPanel, {
      props: { prediction: null, status: 'disconnected' },
    })
    expect(wrapper.find('.ws-text').text()).toBe('连接中断')
  })

  it('renders short direction correctly', () => {
    const shortPred: AIPredictResponse = { ...mockPrediction, direction: 'short', signal: 'sell' }
    const wrapper = mount(AIPredictPanel, {
      props: { prediction: shortPred, status: 'connected' },
    })
    expect(wrapper.find('.signal-tag.short').exists()).toBe(true)
    expect(wrapper.find('.signal-tag.short').text()).toBe('卖出')
  })

  it('renders neutral direction correctly', () => {
    const neutralPred: AIPredictResponse = { ...mockPrediction, direction: 'neutral', signal: 'neutral' }
    const wrapper = mount(AIPredictPanel, {
      props: { prediction: neutralPred, status: 'connected' },
    })
    expect(wrapper.find('.signal-tag.neutral').exists()).toBe(true)
    expect(wrapper.find('.signal-tag.neutral').text()).toBe('中性')
  })

  it('shows indicators toggle button', () => {
    const wrapper = mount(AIPredictPanel, {
      props: { prediction: mockPrediction, status: 'connected' },
    })
    expect(wrapper.find('.indicators-toggle').exists()).toBe(true)
  })

  it('toggles indicators visibility', async () => {
    const wrapper = mount(AIPredictPanel, {
      props: { prediction: mockPrediction, status: 'connected' },
    })
    expect(wrapper.find('.indicators-grid').exists()).toBe(false)
    await wrapper.find('.indicators-toggle').trigger('click')
    expect(wrapper.find('.indicators-grid').isVisible()).toBe(true)
  })
})