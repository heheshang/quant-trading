import { ref } from 'vue'

/**
 * usePriceFlash - 价格闪烁动画 (Design §2.5)
 * 
 * 当价格变化时，触发买入/卖出闪烁动画
 * 上涨：绿色闪烁 500ms / 下跌：红色闪烁 500ms
 * 防抖：500ms 内连续更新重置动画
 */
export function usePriceFlash() {
  const flashClass = ref<'flash-buy' | 'flash-sell' | ''>('')
  let flashTimer: ReturnType<typeof setTimeout> | null = null

  function triggerFlash(newPrice: number, oldPrice: number) {
    if (newPrice === oldPrice) return

    // 清除之前的定时器（防抖：重置动画）
    if (flashTimer) {
      clearTimeout(flashTimer)
    }

    // 设置闪烁 class
    flashClass.value = newPrice > oldPrice ? 'flash-buy' : 'flash-sell'

    // 500ms 后自动清除
    flashTimer = setTimeout(() => {
      flashClass.value = ''
      flashTimer = null
    }, 500)
  }

  function clearFlash() {
    if (flashTimer) {
      clearTimeout(flashTimer)
      flashTimer = null
    }
    flashClass.value = ''
  }

  return { flashClass, triggerFlash, clearFlash }
}
