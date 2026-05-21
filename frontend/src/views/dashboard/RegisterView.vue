<template>
  <div class="register-view">
    <h2 class="form-title">Create Account</h2>
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-position="top"
      @submit.prevent="handleRegister"
    >
      <el-form-item label="Username" prop="username">
        <el-input
          v-model="form.username"
          placeholder="Choose a username"
          :disabled="authStore.loading"
          clearable
        />
      </el-form-item>
      <el-form-item label="Email" prop="email">
        <el-input
          v-model="form.email"
          placeholder="Enter your email"
          :disabled="authStore.loading"
          clearable
        />
      </el-form-item>
      <el-form-item label="Password" prop="password">
        <el-input
          v-model="form.password"
          type="password"
          placeholder="At least 6 characters"
          :disabled="authStore.loading"
          show-password
          clearable
        />
      </el-form-item>
      <el-form-item label="Confirm Password" prop="confirmPassword">
        <el-input
          v-model="form.confirmPassword"
          type="password"
          placeholder="Re-enter your password"
          :disabled="authStore.loading"
          show-password
          clearable
        />
      </el-form-item>
      <el-form-item>
        <el-button
          type="primary"
          native-type="submit"
          :loading="authStore.loading"
          class="submit-btn"
        >
          Register
        </el-button>
      </el-form-item>
    </el-form>
    <div class="form-footer">
      <span>Already have an account?</span>
      <router-link to="/login" class="form-link">Sign In</router-link>
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
  email: '',
  password: '',
  confirmPassword: '',
})

const validateConfirm = (_rule: unknown, value: string, callback: (err?: Error) => void) => {
  if (value !== form.password) {
    callback(new Error('Passwords do not match'))
  } else {
    callback()
  }
}

const rules: FormRules = {
  username: [
    { required: true, message: 'Please choose a username', trigger: 'blur' },
    { min: 3, max: 32, message: 'Username must be 3-32 characters', trigger: 'blur' },
    { pattern: /^[a-zA-Z0-9_]+$/, message: 'Only letters, numbers, and underscores', trigger: 'blur' },
  ],
  email: [
    { required: true, message: 'Please enter your email', trigger: 'blur' },
    { type: 'email', message: 'Please enter a valid email', trigger: 'blur' },
  ],
  password: [
    { required: true, message: 'Please enter a password', trigger: 'blur' },
    { min: 6, message: 'Password must be at least 6 characters', trigger: 'blur' },
  ],
  confirmPassword: [
    { required: true, message: 'Please confirm your password', trigger: 'blur' },
    { validator: validateConfirm, trigger: 'blur' },
  ],
}

async function handleRegister() {
  if (!formRef.value) return
  const valid = await formRef.value.validate().catch(() => false)
  if (!valid) return

  authStore.clearError()
  const success = await authStore.register({
    username: form.username,
    email: form.email,
    password: form.password,
    confirmPassword: form.confirmPassword,
  })

  if (success) {
    ElMessage.success('Registration successful! Welcome aboard.')
    router.push('/dashboard')
  } else {
    ElMessage.error(authStore.error || 'Registration failed. Please try again.')
  }
}
</script>

<style scoped lang="scss">
.register-view {
  width: 100%;
}

.form-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--color-text-primary);
  margin: 0 0 24px;
  text-align: center;
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
    color: var(--color-accent);
    text-decoration: none;
    margin-left: 4px;
    transition: color 0.15s;

    &:hover {
      color: var(--color-accent-hover);
    }
  }
}
</style>
