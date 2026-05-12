<template>
  <StrategyCreateView :strategy-id="strategyId" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import StrategyCreateView from '@/views/strategy/StrategyCreateView.vue'

const route = useRoute()
const strategyId = computed(() => Number(route.params.id))
</script>
