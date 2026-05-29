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
        :ws-status="wsStatus"
        :on-ws-message="registerWsHandler"
      />
      <DepthView
        v-if="activeTab === 'depth'"
        ref="depthViewRef"
        :ws-status="wsStatus"
        :user-role="userRole"
        :on-ws-message="registerWsHandler"
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
  max-width: var(--content-max-width);
  padding: 0 var(--layout-padding);
  overflow: hidden;
  animation: fadeIn var(--transition-base);

  .page-header {
    margin-bottom: 24px;
  }

  .page-title {
    font-size: 26px;
    font-weight: 700;
    font-family: 'Outfit', 'Work Sans', var(--font-ui);
    color: var(--color-text-primary);
    margin: 0;
    letter-spacing: -0.5px;
    background: linear-gradient(135deg, var(--color-primary) 0%, var(--color-primary-hover) 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
}

.market-tabs {
  margin-bottom: 24px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 8px;
  box-shadow: var(--shadow-card);

  :deep(.el-tabs__nav-wrap::after) {
    display: none;
  }

  :deep(.el-tabs__nav) {
    border: none;
  }

  :deep(.el-tabs__item) {
    color: var(--color-text-tertiary);
    font-family: 'Work Sans', var(--font-ui);
    font-size: 14px;
    font-weight: 500;
    transition: all var(--transition-fast);
    padding: 0 20px;
    height: 40px;
    border-radius: var(--radius-md);
    border: 1px solid transparent;

    &:hover {
      color: var(--color-text-primary);
      background: rgba(59, 130, 246, 0.05);
    }

    &.is-active {
      color: #ffffff;
      font-weight: 600;
      background: var(--color-primary);
      box-shadow: var(--shadow-glow-primary);
      border-color: var(--color-primary);
    }
  }

  :deep(.el-tabs__active-bar) {
    display: none;
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
  gap: 10px;
  padding: 12px 16px;
  margin-bottom: 20px;
  background: rgba(59, 130, 246, 0.1);
  border: 1px solid rgba(59, 130, 246, 0.3);
  border-radius: var(--radius-lg);
  color: var(--color-primary);
  font-family: 'Work Sans', var(--font-ui);
  font-size: 13px;
  font-weight: 500;
  animation: slideUp var(--transition-base);

  .el-icon {
    font-size: 18px;
  }
}

.tab-content {
  animation: fadeIn var(--transition-base);
}
</style>
