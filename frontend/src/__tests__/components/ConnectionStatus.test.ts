import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import ConnectionStatus from '@/components/market/ConnectionStatus.vue'

describe('ConnectionStatus', () => {
  it('shows connected status with green dot', () => {
    const wrapper = mount(ConnectionStatus, {
      props: { status: 'connected' },
    })
    expect(wrapper.find('.status-dot').classes()).toContain('connected')
    expect(wrapper.find('.status-text').text()).toBe('推送正常')
  })

  it('shows disconnected status with red dot', () => {
    const wrapper = mount(ConnectionStatus, {
      props: { status: 'disconnected' },
    })
    expect(wrapper.find('.status-dot').classes()).toContain('disconnected')
    expect(wrapper.find('.status-text').text()).toBe('连接中断')
  })

  it('shows reconnecting status with yellow dot and pulse animation', () => {
    const wrapper = mount(ConnectionStatus, {
      props: { status: 'reconnecting' },
    })
    expect(wrapper.find('.status-dot').classes()).toContain('reconnecting')
    expect(wrapper.find('.status-text').text()).toBe('重新连接...')
  })

  it('shows idle status with hollow dot', () => {
    const wrapper = mount(ConnectionStatus, {
      props: { status: 'idle' },
    })
    expect(wrapper.find('.status-dot').classes()).toContain('idle')
    expect(wrapper.find('.status-text').text()).toBe('未连接')
  })
})
