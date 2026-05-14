<template>
  <el-input
    v-model="searchValue"
    placeholder="搜索交易对..."
    :prefix-icon="Search"
    clearable
    class="ticker-search-bar"
    @input="onInput"
    @clear="onClear"
  />
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { Search } from '@element-plus/icons-vue'

const emit = defineEmits<{
  search: [value: string]
}>()

const searchValue = ref('')
let debounceTimer: ReturnType<typeof setTimeout> | null = null

function onInput() {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    emit('search', searchValue.value)
    debounceTimer = null
  }, 300)  // debounce 300ms (PRD US-MD-05)
}

function onClear() {
  if (debounceTimer) clearTimeout(debounceTimer)
  searchValue.value = ''
  emit('search', '')
}
</script>

<style scoped lang="scss">
.ticker-search-bar {
  width: 240px;

  :deep(.el-input__wrapper) {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    height: 32px;
    box-shadow: none;
    transition: border-color 0.2s;

    &:hover {
      border-color: var(--color-border-hover);
    }
    &.is-focus {
      border-color: var(--color-accent);
    }
  }

  :deep(.el-input__inner) {
    color: var(--color-text-primary);
    font-size: 13px;

    &::placeholder {
      color: var(--color-text-tertiary);
    }
  }

  :deep(.el-input__prefix .el-icon) {
    color: var(--color-text-tertiary);
  }

  :deep(.el-input__suffix .el-icon) {
    color: var(--color-text-tertiary);
    cursor: pointer;

    &:hover {
      color: var(--color-text-secondary);
    }
  }
}

@media (max-width: 1023px) {
  .ticker-search-bar {
    width: 180px;
  }
}

@media (max-width: 767px) {
  .ticker-search-bar {
    width: 100%;
  }
}
</style>
