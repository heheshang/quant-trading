import { ref, watch, type Ref } from 'vue'
import type { Depth, DepthLevel, WsMessage } from '@/types'
import { getDepth } from '@/api/market'

/**
 * useDepth - 深度数据管理 (Design §7.2)
 * 
 * 职责：快照加载 + 增量合并
 * - REST API 获取初始快照
 * - WS depth 消息替换完整快照
 * - WS depth_update 消息增量合并
 * - quantity=0 删除该价位
 * - total 由前端本地重算
 */

export function useDepth(symbol: Ref<string>, levels: Ref<number>) {
  const depth = ref<Depth | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  /** 重新计算累计量 */
  function recalculateTotals(levels: DepthLevel[], direction: 'asc' | 'desc'): void {
    if (direction === 'asc') {
      // 卖盘：从卖一价（最优价 = 价格最低）开始累加
      let total = 0
      for (const level of levels) {
        total += level.quantity
        level.total = total
      }
    } else {
      // 买盘：从买一价（最优价 = 价格最高）开始累加
      let total = 0
      for (const level of levels) {
        total += level.quantity
        level.total = total
      }
    }
  }

  /** 增量合并 (Design §7.2 mergeDepthLevels) */
  function mergeDepthLevels(existing: DepthLevel[], updates: DepthLevel[]): void {
    for (const u of updates) {
      const idx = existing.findIndex(l => l.price === u.price)
      if (idx >= 0) {
        if (u.quantity === 0) {
          existing.splice(idx, 1)  // 删除该价位
        } else {
          existing[idx].quantity = u.quantity  // 更新数量
        }
      } else if (u.quantity > 0) {
        existing.push({ ...u })  // 新增价位
      }
    }
    // 保持排序（按价格升序）
    existing.sort((a, b) => a.price - b.price)
  }

  /** 处理 WS depth 消息（完整快照） */
  function handleSnapshot(msg: WsMessage): void {
    const data = msg.data as Depth
    depth.value = {
      bids: data.bids.map(b => ({ ...b })),
      asks: data.asks.map(a => ({ ...a })),
      timestamp: data.timestamp,
    }
    // 确保排序
    depth.value.bids.sort((a, b) => b.price - a.price)  // 买盘降序
    depth.value.asks.sort((a, b) => a.price - b.price)  // 卖盘升序
    // 重算累计量
    recalculateTotals(depth.value.bids, 'desc')
    recalculateTotals(depth.value.asks, 'asc')
  }

  /** 处理 WS depth_update 消息（增量） */
  function handleUpdate(msg: WsMessage): void {
    if (!depth.value) return

    const update = msg.data as Depth
    // 合并 bids
    if (update.bids?.length) {
      mergeDepthLevels(depth.value.bids, update.bids)
      depth.value.bids.sort((a, b) => b.price - a.price)  // 买盘降序
      recalculateTotals(depth.value.bids, 'desc')
    }
    // 合并 asks
    if (update.asks?.length) {
      mergeDepthLevels(depth.value.asks, update.asks)
      depth.value.asks.sort((a, b) => a.price - b.price)  // 卖盘升序
      recalculateTotals(depth.value.asks, 'asc')
    }
    // 更新 timestamp
    depth.value.timestamp = update.timestamp || Date.now()
  }

  /** 通过 REST API 加载初始数据 */
  async function fetchDepth(): Promise<void> {
    loading.value = true
    error.value = null
    try {
      const data = await getDepth(symbol.value, levels.value)
      depth.value = {
        bids: data.bids.map(b => ({ ...b })),
        asks: data.asks.map(a => ({ ...a })),
        timestamp: data.timestamp,
      }
      depth.value.bids.sort((a, b) => b.price - a.price)
      depth.value.asks.sort((a, b) => a.price - b.price)
      recalculateTotals(depth.value.bids, 'desc')
      recalculateTotals(depth.value.asks, 'asc')
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  // 监听 symbol/levels 变化重新加载
  watch([symbol, levels], () => {
    fetchDepth()
  })

  return {
    depth,
    loading,
    error,
    fetchDepth,
    handleSnapshot,
    handleUpdate,
  }
}
