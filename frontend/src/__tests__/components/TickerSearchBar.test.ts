import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import TickerSearchBar from '@/components/market/TickerSearchBar.vue'

describe('TickerSearchBar', () => {
  const globalStubs = {
    'el-input': {
      template: '<div class="el-input-stub"><slot /></div>',
      props: ['modelValue', 'placeholder', 'prefixIcon', 'clearable'],
      emits: ['input', 'clear', 'update:modelValue'],
    },
  }

  it('renders component with search class', () => {
    const wrapper = mount(TickerSearchBar, {
      global: { stubs: globalStubs },
    })
    expect(wrapper.find('.ticker-search-bar').exists()).toBe(true)
  })

  it('emits search event after 300ms debounce', async () => {
    vi.useFakeTimers()
    const wrapper = mount(TickerSearchBar, {
      global: { stubs: globalStubs },
    })

    // Directly test the onInput method
    wrapper.vm.searchValue = 'BTC'
    wrapper.vm.onInput()
    vi.advanceTimersByTime(300)

    expect(wrapper.emitted('search')).toBeTruthy()
    expect(wrapper.emitted('search')![0]).toEqual(['BTC'])
    vi.useRealTimers()
  })

  it('does not emit before debounce period', async () => {
    vi.useFakeTimers()
    const wrapper = mount(TickerSearchBar, {
      global: { stubs: globalStubs },
    })

    wrapper.vm.searchValue = 'ETH'
    wrapper.vm.onInput()
    vi.advanceTimersByTime(200) // Not yet 300ms

    expect(wrapper.emitted('search')).toBeFalsy()
    vi.useRealTimers()
  })

  it('emits empty string on clear', async () => {
    const wrapper = mount(TickerSearchBar, {
      global: { stubs: globalStubs },
    })
    await wrapper.vm.onClear()
    expect(wrapper.emitted('search')).toBeTruthy()
    expect(wrapper.emitted('search')![0]).toEqual([''])
  })
})
