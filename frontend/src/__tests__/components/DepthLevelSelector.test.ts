import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import DepthLevelSelector from '@/components/market/DepthLevelSelector.vue'

describe('DepthLevelSelector', () => {
  it('renders 4 level options (5/10/20/50)', () => {
    const wrapper = mount(DepthLevelSelector, {
      props: { modelValue: 10, userRole: 'trader' },
    })
    const buttons = wrapper.findAll('.level-btn')
    expect(buttons).toHaveLength(4)
    expect(buttons[0].text()).toContain('5')
    expect(buttons[1].text()).toContain('10')
    expect(buttons[2].text()).toContain('20')
    expect(buttons[3].text()).toContain('50')
  })

  it('marks the selected level as active', () => {
    const wrapper = mount(DepthLevelSelector, {
      props: { modelValue: 10, userRole: 'trader' },
    })
    const buttons = wrapper.findAll('.level-btn')
    expect(buttons[1].classes()).toContain('active')
  })

  it('disables 50-level for trader role (D4 RBAC)', () => {
    const wrapper = mount(DepthLevelSelector, {
      props: { modelValue: 10, userRole: 'trader' },
    })
    const buttons = wrapper.findAll('.level-btn')
    expect(buttons[3].classes()).toContain('disabled')
    expect(buttons[3].find('.lock-icon').exists()).toBe(true)
  })

  it('enables 50-level for pro-trader role', () => {
    const wrapper = mount(DepthLevelSelector, {
      props: { modelValue: 10, userRole: 'pro-trader' },
    })
    const buttons = wrapper.findAll('.level-btn')
    expect(buttons[3].classes()).not.toContain('disabled')
  })

  it('enables 50-level for admin role', () => {
    const wrapper = mount(DepthLevelSelector, {
      props: { modelValue: 10, userRole: 'admin' },
    })
    const buttons = wrapper.findAll('.level-btn')
    expect(buttons[3].classes()).not.toContain('disabled')
  })

  it('emits update:modelValue when clicking a level', async () => {
    const wrapper = mount(DepthLevelSelector, {
      props: { modelValue: 10, userRole: 'trader' },
    })
    const buttons = wrapper.findAll('.level-btn')
    await buttons[0].trigger('click')
    expect(wrapper.emitted('update:modelValue')).toBeTruthy()
    expect(wrapper.emitted('update:modelValue')![0]).toEqual([5])
  })

  it('does not emit when clicking disabled level', async () => {
    const wrapper = mount(DepthLevelSelector, {
      props: { modelValue: 10, userRole: 'trader' },
    })
    const buttons = wrapper.findAll('.level-btn')
    await buttons[3].trigger('click') // 50 is disabled for trader
    expect(wrapper.emitted('update:modelValue')).toBeFalsy()
  })
})
