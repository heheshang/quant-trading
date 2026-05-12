<template>
  <div class="login-view">
    <h2 class="form-title">Sign In</h2>
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-position="top"
      @submit.prevent="handleLogin"
    >
      <el-form-item label="Username" prop="username">
        <el-input
          v-model="form.username"
          placeholder="Enter your username"
          :disabled="authStore.loading"
          clearable
        />
      </el-form-item>
      <el-form-item label="Password" prop="password">
        <el-input
          v-model="form.password"
          type="password"
          placeholder="Enter your password"
          :disabled="authStore.loading"
          show-password
          clearable
        />
      </el-form-item>
      <el-form-item>
        <div class="form-options">
          <el-checkbox v-model="form.rememberMe" :disabled="authStore.loading">Remember me</el-checkbox>
          <router-link to="/register" class="form-link">Create account</router-link>
        </div>
      </el-form-item>
      <el-form-item>
        <el-button
          type="primary"
          native-type="submit"
          :loading="authStore.loading"
          class="submit-btn"
        >
          Sign In
        </el-button>
      </el-form-item>
    </el-form>
    <div class="form-footer">
      <span>Don't have an account?</span>
      <router-link to="/register" class="form-link">Register</router-link>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'

const router = useRouter()
const authStore = useAuthStore()
const formRef = ref<FormInstance>()

const form = reactive({
  username: '',
  password: '',
  rememberMe: false,
})

const rules: FormRules = {
  username: [
    { required: true, message: 'Please enter username', trigger: 'blur' },
    { min: 3, message: 'Username must be at least 3 characters', trigger: 'blur' },
  ],
  password: [
    { required: true, message: 'Please enter password', trigger: 'blur' },
    { min: 6, message: 'Password must be at least 6 characters', trigger: 'blur' },
  ],
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
.login-view {
  width: 100%;
}

.form-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0 0 24px;
  text-align: center;
}

.form-options {
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;
}

.form-link {
  color: var(--color-accent);
  text-decoration: none;
  font-size: 13px;
  transition: color 0.15s;

  &:hover {
    color: var(--color-accent-hover);
  }
}

.submit-btn {
  width: 100%;
  height: 40px;
}

.form-footer {
  text-align: center;
  margin-top: 16px;
  font-size: 13px;
  color: var(--color-text-tertiary);

  .form-link {
    margin-left: 4px;
  }
}
</style>
