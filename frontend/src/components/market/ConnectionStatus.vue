<template>
  <span class="connection-status" :class="statusClass">
    <span class="status-dot" :class="statusClass"></span>
    <span class="status-text">{{ statusText }}</span>
  </span>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { WsStatus } from '@/types'

const props = defineProps<{
  status: WsStatus
}>()

const statusClass = computed(() => props.status)

const statusText = computed(() => {
  const map: Record<WsStatus, string> = {
    connected: '推送正常',
    disconnected: '连接中断',
    reconnecting: '重新连接...',
    idle: '未连接',
  }
  return map[props.status] || '未连接'
})
</script>

<style scoped lang="scss">
.connection-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--color-text-tertiary);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;

  &.connected {
    background: var(--color-success);
  }
  &.disconnected {
    background: var(--color-error);
  }
  &.reconnecting {
    background: var(--color-warning);
    animation: pulse 1.5s ease-in-out infinite;
  }
  &.idle {
    background: transparent;
    border: 1.5px solid var(--color-text-tertiary);
  }
}

.status-text {
  &.connected { color: var(--color-success); }
  &.disconnected { color: var(--color-error); }
  &.reconnecting { color: var(--color-warning); }
  &.idle { color: var(--color-text-tertiary); }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
</style>
