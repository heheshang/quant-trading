// useFeatureFlag — small reactive wrapper around the FeatureFlag store.
//
// Usage in a component:
//   import { useFeatureFlag } from '@/composables/useFeatureFlag'
//   const icebergEnabled = useFeatureFlag('iceberg_order')
//   <div v-if="icebergEnabled">Iceberg orders are ENABLED for you</div>
//
// Returns a `ComputedRef<boolean>` so the template auto-tracks the underlying
// store. Unknown flags default to `false` (off) — see `store.isEnabled`.
//
// The store must have been bootstrapped (call `useFeatureFlagStore().load()`
// after auth) before this composable is used, otherwise everything resolves
// to `false` until the first response lands.

import { computed, type ComputedRef } from 'vue'
import { useFeatureFlagStore } from '@/stores/featureFlag'

export function useFeatureFlag(key: string): ComputedRef<boolean> {
  const store = useFeatureFlagStore()
  return computed(() => store.isEnabled(key))
}
