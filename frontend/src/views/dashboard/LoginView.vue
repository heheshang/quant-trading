<template>
  <div class="login-view">
    <div class="bg-grid" />

    <div class="login-card">
      <!-- Brand -->
      <div class="brand-section">
        <svg class="brand-icon" width="44" height="44" viewBox="0 0 44 44" fill="none">
          <rect width="44" height="44" rx="12" fill="#7C3AED" />
          <path d="M11 30V14L22 8L33 14V30L22 36L11 30Z" stroke="white" stroke-width="2" fill="none" />
          <path d="M16.5 24L20 27.5L27.5 20" stroke="white" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        <div class="brand-text">
          <h1 class="brand-name">Quant Trading</h1>
          <span class="brand-subtitle">QUANTITATIVE TRADING SYSTEM</span>
        </div>
      </div>

      <!-- Header -->
      <div class="form-header">
        <h2 class="form-title">Welcome back</h2>
        <p class="form-subtitle">Sign in to your account</p>
      </div>

      <!-- Login form -->
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-position="top"
        @submit.prevent="handleLogin"
        class="login-form"
      >
        <el-form-item prop="username" class="form-item">
          <template #label>
            <span class="field-label">Username</span>
          </template>
          <el-input
            v-model="form.username"
            placeholder="Enter your username"
            :disabled="loading"
            clearable
            @input="onFieldChange"
          >
            <template #prefix>
              <svg class="input-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                <circle cx="12" cy="7" r="4" />
              </svg>
            </template>
          </el-input>
        </el-form-item>

        <el-form-item prop="password" class="form-item">
          <template #label>
            <span class="field-label">Password</span>
          </template>
          <el-input
            v-model="form.password"
            type="password"
            placeholder="Enter your password"
            :disabled="loading"
            show-password
            clearable
            @input="onFieldChange"
          >
            <template #prefix>
              <svg class="input-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
                <path d="M7 11V7a5 5 0 0 1 10 0v4" />
              </svg>
            </template>
          </el-input>
        </el-form-item>

        <div class="form-options">
          <el-checkbox v-model="form.rememberMe" :disabled="loading">
            <span class="checkbox-label">Remember me</span>
          </el-checkbox>
          <a href="#" class="forgot-link" @click.prevent>Forgot password?</a>
        </div>

        <el-form-item class="form-item-btn">
          <el-button
            type="primary"
            native-type="submit"
            :loading="loading"
            :disabled="!formValid || loading"
            class="submit-btn"
          >
            {{ loading ? 'Signing in...' : 'Sign In' }}
          </el-button>
        </el-form-item>
      </el-form>

      <!-- Divider -->
      <div class="divider">
        <span class="divider-text">or continue with</span>
      </div>

      <!-- Social -->
      <div class="social-area">
        <button class="social-btn" disabled>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 12A10 10 0 1 1 12 2a10 10 0 0 1 10 10Z" />
            <path d="M12 6v6l4 2" />
          </svg>
          Google
        </button>
        <button class="social-btn" disabled>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M16 8a6 6 0 0 1 6 6v7h-4v-7a2 2 0 0 0-2-2 2 2 0 0 0-2 2v7h-4v-7a6 6 0 0 1 6-6z" />
            <rect x="2" y="9" width="4" height="12" />
            <circle cx="4" cy="4" r="2" />
          </svg>
          GitHub
        </button>
      </div>

      <!-- Register -->
      <div class="register-prompt">
        <span>Don't have an account?</span>
        <router-link to="/register" class="register-link">Create account</router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'

const router = useRouter()
const authStore = useAuthStore()
const formRef = ref<FormInstance>()
const loading = computed(() => authStore.loading)

const form = reactive({
  username: '',
  password: '',
  rememberMe: false,
})

const formValid = computed(() => {
  return form.username.trim().length >= 3 && form.password.trim().length >= 6
})

const rules: FormRules = {
  username: [
    { required: true, message: 'Please enter username', trigger: 'blur' },
    { min: 3, message: 'Username must be at least 3 characters', trigger: 'blur' },
    { max: 32, message: 'Username must be at most 32 characters', trigger: 'blur' },
  ],
  password: [
    { required: true, message: 'Please enter password', trigger: 'blur' },
    { min: 6, message: 'Password must be at least 6 characters', trigger: 'blur' },
  ],
}

function onFieldChange() {
  // Real-time: formValid computed will react
}

async function handleLogin() {
  if (!formRef.value) return
  const valid = await formRef.value.validate().catch(() => false)
  if (!valid) return

  authStore.clearError()
  const success = await authStore.login({
    username: form.username,
    password: form.password,
    rememberMe: form.rememberMe,
  })

  if (success) {
    ElMessage.success('Welcome back!')
    router.push('/dashboard')
  } else {
    ElMessage.error(authStore.error || 'Login failed. Please try again.')
  }
}
</script>

<style scoped lang="scss">
// ============================================================
// Color tokens (page-local, doesn't touch global variables)
// ============================================================
$purple:      #7C3AED;
$purple-dark: #6D28D9;
$error-red:   #F87171;
$bg-page:     #121212;
$bg-input:    #1E1E2E;
$bg-card:     #1A1A2A;
$border:      #333344;
$border-focus:#7C3AED;
$text-body:   #E0E0E0;
$text-label:  #D0D0E0;
$text-muted:  #888899;
$text-placeholder: #666677;

// ============================================================
// Page
// ============================================================
.login-view {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100%;
  background: $bg-page;
  overflow: hidden;
}

/* Subtle background grid */
.bg-grid {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(255,255,255,0.025) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255,255,255,0.025) 1px, transparent 1px);
  background-size: 40px 40px;
  mask-image: radial-gradient(ellipse 70% 55% at 50% 50%, black, transparent 70%);
  -webkit-mask-image: radial-gradient(ellipse 70% 55% at 50% 50%, black, transparent 70%);
}

// ============================================================
// Card
// ============================================================
.login-card {
  position: relative;
  z-index: 1;
  width: 100%;
  max-width: 384px;
  padding: 28px 28px 20px;
  background: $bg-card;
  border: 1px solid $border;
  border-radius: 16px;
  box-shadow: 0 4px 32px rgba(0,0,0,0.4);
}

// ============================================================
// Brand
// ============================================================
.brand-section {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 28px;
  padding-bottom: 24px;
  border-bottom: 1px solid $border;
}

.brand-icon {
  flex-shrink: 0;
}

.brand-text {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.brand-name {
  font-size: 19px;
  font-weight: 700;
  color: $text-body;
  margin: 0;
  letter-spacing: -0.3px;
  line-height: 1.3;
}

.brand-subtitle {
  font-size: 11px;
  color: $text-muted;
  letter-spacing: 0.6px;
  text-transform: uppercase;
}

// ============================================================
// Form header
// ============================================================
.form-header {
  margin-bottom: 24px;
}

.form-title {
  font-size: 18px;
  font-weight: 600;
  color: $text-body;
  margin: 0 0 4px;
  letter-spacing: -0.2px;
}

.form-subtitle {
  font-size: 13px;
  color: $text-muted;
  margin: 0;
}

// ============================================================
// Form
// ============================================================
.login-form {
  :deep(.el-form-item) {
    margin-bottom: 18px;
  }

  :deep(.el-form-item__label) {
    padding-bottom: 6px;
    font-size: 14px;
    font-weight: 500;
    color: $text-label !important;
    line-height: 1.4;
  }

  :deep(.el-form-item.is-required .el-form-item__label::before) {
    content: none !important;
  }

  :deep(.el-form-item__error) {
    color: $error-red;
    font-size: 12px;
    padding-top: 3px;
  }

  :deep(.el-form-item.is-error .el-input__wrapper) {
    box-shadow: 0 0 0 1px $error-red inset !important;
  }

  :deep(.el-input__wrapper) {
    background: $bg-input !important;
    border-radius: 8px;
    padding: 1px 12px !important;
    box-shadow: 0 0 0 1px $border inset !important;
    transition: box-shadow 0.2s ease;
  }

  :deep(.el-input__wrapper.is-focus) {
    box-shadow:
      0 0 0 1px $border-focus inset,
      0 0 12px rgba(124, 58, 237, 0.15) !important;
  }

  :deep(.el-input__inner) {
    height: 38px;
    font-size: 14px;
    color: $text-body;
  }

  :deep(.el-input__inner::placeholder) {
    color: #666677 !important;
  }

  :deep(.el-input__prefix) {
    margin-right: 8px;
  }

  :deep(.el-input--disabled .el-input__wrapper) {
    opacity: 0.5;
  }
}

.field-label {
  font-size: 14px;
  font-weight: 500;
  color: $text-label;
  letter-spacing: 0.2px;
}

.input-icon {
  color: $text-muted;
  display: block;
}

// ============================================================
// Form options
// ============================================================
.form-options {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin: -4px 0 22px;

  :deep(.el-checkbox__inner) {
    background: $bg-input !important;
    border-color: $border !important;
    border-radius: 4px;
    transition: all 0.2s;
  }

  :deep(.el-checkbox__input.is-checked .el-checkbox__inner) {
    background: $purple !important;
    border-color: $purple !important;
  }

  :deep(.el-checkbox__input.is-checked .el-checkbox__inner::after) {
    border-color: white;
  }

  :deep(.el-checkbox__input.is-focus .el-checkbox__inner) {
    border-color: $purple;
  }

  .checkbox-label {
    font-size: 13px;
    color: $text-label;
  }

  .forgot-link {
    font-size: 13px;
    color: $purple;
    text-decoration: none;
    font-weight: 500;
    transition: color 0.15s;

    &:hover {
      color: lighten($purple, 10%);
      text-decoration: underline;
    }
  }
}

// ============================================================
// Submit button
// ============================================================
.form-item-btn {
  :deep(.el-form-item__content) {
    margin-bottom: 0;
  }
}

.submit-btn {
  width: 100%;
  height: 44px;
  font-size: 15px;
  font-weight: 600;
  border-radius: 10px;
  letter-spacing: 0.3px;
  border: none !important;
  transition: all 0.2s ease;

  // Active state — use !important to beat Element Plus internal CSS
  --el-button-bg-color: #7C3AED !important;
  --el-button-border-color: #7C3AED !important;
  --el-button-hover-bg-color: #6D28D9 !important;
  --el-button-hover-border-color: #6D28D9 !important;
  --el-button-active-bg-color: #6D28D9 !important;
  --el-button-active-border-color: #6D28D9 !important;
  --el-button-text-color: #ffffff !important;
  --el-button-hover-text-color: #ffffff !important;
  --el-button-active-text-color: #ffffff !important;

  // Disabled — direct style, not CSS var (Element Plus overrides vars)
  &.is-disabled {
    background: #2A2A3A !important;
    border-color: #2A2A3A !important;
    color: #666677 !important;
    opacity: 1;
  }

  &:not(.is-disabled) {
    background: #7C3AED !important;
    border-color: #7C3AED !important;
    color: #ffffff !important;
  }

  &:not(.is-disabled):hover {
    background: #6D28D9 !important;
    border-color: #6D28D9 !important;
    box-shadow: 0 0 20px rgba(124, 58, 237, 0.3);
    transform: translateY(-1px);
  }

  &:not(.is-disabled):active {
    transform: translateY(0);
    box-shadow: none;
  }
}

// ============================================================
// Divider
// ============================================================
.divider {
  display: flex;
  align-items: center;
  margin: 20px 0 16px;
  gap: 12px;

  &::before,
  &::after {
    content: '';
    flex: 1;
    height: 1px;
    background: #2A2A3A;
  }

  .divider-text {
    font-size: 11px;
    color: #555566;
    white-space: nowrap;
    text-transform: uppercase;
    letter-spacing: 0.8px;
  }
}

// ============================================================
// Social
// ============================================================
.social-area {
  display: flex;
  gap: 10px;
}

.social-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 40px;
  background: transparent;
  border: 1px solid #2A2A3A;
  border-radius: 10px;
  color: #555566;
  font-size: 13px;
  font-family: inherit;
  cursor: not-allowed;
  transition: all 0.2s;

  svg {
    display: block;
    flex-shrink: 0;
  }
}

// ============================================================
// Register prompt
// ============================================================
.register-prompt {
  margin-top: 20px;
  text-align: center;
  font-size: 13px;
  color: $text-muted;
  padding: 16px 0 0;
  border-top: 1px solid #2A2A3A;

  .register-link {
    color: $purple;
    text-decoration: none;
    font-weight: 500;
    margin-left: 4px;
    transition: color 0.15s;

    &:hover {
      color: lighten($purple, 10%);
      text-decoration: underline;
    }
  }
}
</style>
