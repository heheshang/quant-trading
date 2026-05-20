<template>
  <div class="kdj-overlay">
    <div v-if="showPanel" class="kdj-panel">
      <div class="kdj-title">KDJ 指标</div>
      <div class="kdj-controls">
        <el-radio-group v-model="activeInterval" size="small">
          <el-radio-button value="none">关闭</el-radio-button>
          <el-radio-button value="inline">叠加</el-radio-button>
        </el-radio-group>
      </div>
      <div v-if="activeInterval === 'inline'" class="kdj-params">
        <el-input-number v-model="n" :min="1" :max="100" size="small" title="N (RSV周期)" />
        <el-input-number v-model="m1" :min="1" :max="100" size="small" title="M1 (K平滑)" />
        <el-input-number v-model="m2" :min="1" :max="100" size="small" title="M2 (D平滑)" />
        <el-button size="small" type="primary" @click="loadKdj">计算</el-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import type { KdjBar } from '@/types/indicator'

const props = defineProps<{
  symbol: string
  interval: string
  showPanel?: boolean
}>()

const emit = defineEmits<{
  (e: 'kdj-loaded', data: KdjBar[]): void
}>()

const activeInterval = ref<'none' | 'inline'>('none')
const n = ref(9)
const m1 = ref(3)
const m2 = ref(3)

async function loadKdj() {
  if (!props.symbol || !props.interval) return
  try {
    const { getKdj } = await import('@/api/indicator')
    const res = await getKdj({
      symbol: props.symbol.toLowerCase(),
      interval: props.interval,
      n: n.value,
      m1: m1.value,
      m2: m2.value,
    })
    emit('kdj-loaded', res.data)
  } catch (err) {
    console.error('KDJ load failed:', err)
  }
}

watch(activeInterval, (val) => {
  if (val === 'inline') {
    loadKdj()
  }
})
</script>

<script lang="ts">
// expose for parent component usage
export default {
  methods: {
    reload() {
      ;(this as any).loadKdj?.()
    },
  },
}
</script>

<style scoped>
.kdj-overlay {
  display: flex;
  flex-direction: column;
}

.kdj-panel {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: var(--el-fill-color-light);
  border-radius: 6px;
  margin-bottom: 8px;
}

.kdj-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--el-text-color-regular);
}

.kdj-controls {
  display: flex;
  gap: 8px;
}

.kdj-params {
  display: flex;
  align-items: center;
  gap: 6px;
}

.kdj-params :deep(.el-input-number) {
  width: 80px;
}
</style>
