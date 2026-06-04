import { createApp } from 'vue'
import { createPinia } from 'pinia'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import 'element-plus/theme-chalk/dark/css-vars.css'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import router from '@/router'
import App from './App.vue'
import { useAuthStore } from '@/stores/auth'
import { useFeatureFlagStore } from '@/stores/featureFlag'
import './assets/styles/global.scss'

// Enable Element Plus dark mode
document.documentElement.classList.add('dark')

const app = createApp(App)

// Register all Element Plus icons globally
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}

app.use(createPinia())
app.use(ElementPlus)
app.use(router)
app.mount('#app')

// Bootstrap the feature-flag store. We fire-and-forget here because:
//   1. The endpoint requires auth — for an unauthenticated user, the
//      store's `load()` silently no-ops (401 is expected, see
//      stores/featureFlag.ts).
//   2. The store re-reads `sessionStorage` on every `useFeatureFlag()`
//      call, so a login from another tab will re-trigger the load via
//      the auth store's logout/login flow.
//   3. Components rendering on a fresh page need a quick value to avoid
//      a flash of "feature off"; awaiting here would block first paint.
// We also schedule a refresh after the auth store confirms login so the
// first authenticated render gets the right per-user flags.
const authStore = useAuthStore()
const ffStore = useFeatureFlagStore()
void ffStore.load()

// After a successful login, re-fetch the user's flags. The auth store
// already updates `isAuthenticated` synchronously, so this fires from
// the LoginView and is harmless if the call 401s.
authStore.$onAction(({ name, after }) => {
  if (name === 'login' || name === 'register' || name === 'fetchProfile') {
    after(() => {
      void ffStore.load()
    })
  }
  if (name === 'logout') {
    after(() => {
      ffStore.reset()
    })
  }
})
