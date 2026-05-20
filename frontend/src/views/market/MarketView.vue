<template>
  <div class="market-view">
    <!-- Page Header -->
    <div class="page-header">
      <h1 class="page-title">行情中心</h1>
    </div>

    <!-- Tabs -->
    <el-tabs v-model="activeTab" class="market-tabs" @tab-change="onTabChange">
      <el-tab-pane label="Ticker 概览" name="ticker">
        <template #label>
          <span class="tab-label">
            <el-icon><TrendCharts /></el-icon>
            Ticker 概览
          </span>
        </template>
      </el-tab-pane>
      <el-tab-pane label="深度盘口" name="depth">
        <template #label>
          <span class="tab-label">
            <el-icon><DataLine /></el-icon>
            深度盘口
          </span>
        </template>
      </el-tab-pane>
    </el-tabs>

    <!-- WS Disconnection Banner -->
    <div v-if="wsStatus === 'disconnected' || wsStatus === 'reconnecting'" class="ws-banner">
      <el-icon><WarningFilled /></el-icon>
      <span>{{ wsStatus === 'reconnecting' ? '正在尝试重新连接...' : '连接中断，正在尝试重连...' }}</span>
    </div>

    <!-- Tab Content -->
    <div class="tab-content">
      <TickerListView
        v-if="activeTab === 'ticker'"
        ref="tickerListRef"
        :wsStatus="wsStatus"
        :onWsMessage="registerWsHandler"
      />
      <DepthView
        v-if="activeTab === 'depth'"
        ref="depthViewRef"
        :wsStatus="wsStatus"
        :userRole="userRole"
        :onWsMessage="registerWsHandler"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { TrendCharts, DataLine, WarningFilled } from '@element-plus/icons-vue'
import type { WsMessage, WsStatus } from '@/types'
import { useMarketWs } from '@/composables/useMarketWs'
import { useAuthStore } from '@/stores/auth'
import TickerListView from '@/views/market/TickerListView.vue'
import DepthView from '@/views/market/DepthView.vue'
import { ElMessage } from 'element-plus'

const activeTab = ref('ticker')
const tickerListRef = ref<InstanceType<typeof TickerListView>>()
const depthViewRef = ref<InstanceType<typeof DepthView>>()

const { status: wsStatus, connect, disconnect, subscribe, unsubscribe, onMessage, onKick } = useMarketWs()

const authStore = useAuthStore()
const userRole = computed(() => authStore.user?.role || 'trader')

// WS Message routing
const wsHandlers: ((msg: WsMessage) => void)[] = []

function registerWsHandler(handler: (msg: WsMessage) => void) {
  wsHandlers.push(handler)
}

onMessage((msg: WsMessage) => {
  wsHandlers.forEach(h => h(msg))
})

onKick((reason?: string) => {
  ElMessage.warning(reason ? `WebSocket 连接被断开: ${reason}` : 'WebSocket 连接被断开')
})

function onTabChange(tab: string) {
  // Subscribe/unsubscribe channels based on active tab
  if (tab === 'ticker') {
    // Subscribe to ticker channels for all visible symbols
    subscribe(['market:ticker:BTCUSDT', 'market:ticker:ETHUSDT', 'market:ticker:BNBUSDT'])
  } else if (tab === 'depth') {
    subscribe(['market:depth:BTCUSDT'])
  }
}

onMounted(() => {
  // Connect WebSocket
  const token = authStore.token
  if (token) {
    connect(token)
    // Subscribe to ticker by default
    setTimeout(() => {
      subscribe(['market:ticker:BTCUSDT', 'market:ticker:ETHUSDT', 'market:ticker:BNBUSDT'])
    }, 1000)
  }
})

onBeforeUnmount(() => {
  disconnect()
})
</script>

<style scoped lang="scss">
.market-view {
  max-width: 1344px;
  padding: 0 24px;

  .page-header {
    margin-bottom: 20px;
  }

  .page-title {
    font-size: 24px;
    font-weight: 600;
    color: var(--color-text-primary);
    margin: 0;
  }
}

.market-tabs {
  margin-bottom: 20px;

  :deep(.el-tabs__nav-wrap::after) {
    background-color: var(--color-border);
  }

  :deep(.el-tabs__item) {
    color: var(--color-text-tertiary);
    font-size: 14px;
    transition: color 0.2s;

    &:hover {
      color: var(--color-text-secondary);
    }

    &.is-active {
      color: var(--color-text-primary);
    }
  }

  :deep(.el-tabs__active-bar) {
    background-color: var(--color-accent);
    height: 2px;
  }

  .tab-label {
    display: flex;
    align-items: center;
    gap: 6px;
  }
}

.ws-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  background: rgba(229, 72, 77, 0.12);
  border: 1px solid rgba(229, 72, 77, 0.3);
  border-radius: 6px;
  margin-bottom: 16px;
  font-size: 13px;
  color: var(--color-error);
}

.tab-content {
  min-height: 400px;
}

@media (max-width: 767px) {
  .market-view {
    padding: 0 16px;
  }
}
</style>
