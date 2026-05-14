<template>
  <div class="depth-level-selector">
    <span class="selector-label">档位:</span>
    <div class="level-buttons">
      <button
        v-for="opt in levelOptions"
        :key="opt.value"
        class="level-btn"
        :class="{
          active: modelValue === opt.value,
          disabled: opt.disabled
        }"
        :disabled="opt.disabled"
        @click="opt.disabled ? null : emit('update:modelValue', opt.value)"
      >
        {{ opt.value }}
        <el-icon v-if="opt.disabled" class="lock-icon"><Lock /></el-icon>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Lock } from '@element-plus/icons-vue'

const props = defineProps<{
  modelValue: number
  userRole?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: number]
}>()

const levelOptions = computed(() => {
  const isPro = props.userRole === 'pro-trader' || props.userRole === 'admin'
  return [
    { value: 5, disabled: false },
    { value: 10, disabled: false },
    { value: 20, disabled: false },
    { value: 50, disabled: !isPro },  // D4: 50档仅 pro-trader/admin
  ]
})
</script>

<style scoped lang="scss">
.depth-level-selector {
  display: flex;
  align-items: center;
  gap: 8px;
}

.selector-label {
  font-size: 13px;
  color: var(--color-text-secondary);
}

.level-buttons {
  display: flex;
  gap: 4px;
  background: var(--color-surface);
  border-radius: 6px;
  padding: 2px;
}

.level-btn {
  padding: 4px 12px;
  font-size: 13px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
  background: transparent;
  color: var(--color-text-tertiary);
  display: flex;
  align-items: center;
  gap: 2px;

  &:hover:not(.disabled) {
    background: var(--color-surface-elevated);
    color: var(--color-text-secondary);
  }

  &.active {
    background: var(--color-accent);
    color: #fff;
  }

  &.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .lock-icon {
    font-size: 12px;
  }
}
</style>
