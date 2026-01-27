<template>
  <div class="login-container">
    <a-card class="login-card" title="VPN 管理后台">
      <a-form
        :model="formState"
        :rules="rules"
        @finish="handleLogin"
        layout="vertical"
      >
        <a-form-item label="邮箱" name="email">
          <a-input
            v-model:value="formState.email"
            placeholder="请输入管理员邮箱"
            size="large"
            :disabled="authStore.loading"
          >
            <template #prefix>
              <UserOutlined />
            </template>
          </a-input>
        </a-form-item>

        <a-form-item label="密码" name="password">
          <a-input-password
            v-model:value="formState.password"
            placeholder="请输入密码"
            size="large"
            :disabled="authStore.loading"
          >
            <template #prefix>
              <LockOutlined />
            </template>
          </a-input-password>
        </a-form-item>

        <a-form-item v-if="authStore.error">
          <a-alert :message="authStore.error" type="error" show-icon closable />
        </a-form-item>

        <a-form-item>
          <a-button
            type="primary"
            html-type="submit"
            size="large"
            block
            :loading="authStore.loading"
          >
            登录
          </a-button>
        </a-form-item>
      </a-form>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { useRouter } from 'vue-router'
import { UserOutlined, LockOutlined } from '@ant-design/icons-vue'
import { useAuthStore } from '@/stores/auth'
import type { Rule } from 'ant-design-vue/es/form'

const router = useRouter()
const authStore = useAuthStore()

const formState = reactive({
  email: '',
  password: ''
})

const rules: Record<string, Rule[]> = {
  email: [
    { required: true, message: '请输入邮箱', trigger: 'blur' },
    { type: 'email', message: '请输入有效的邮箱地址', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 8, message: '密码至少8个字符', trigger: 'blur' }
  ]
}

const handleLogin = async () => {
  const success = await authStore.login(formState)
  if (success) {
    router.push('/admin/dashboard')
  }
}
</script>

<style scoped>
.login-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.login-card {
  width: 100%;
  max-width: 400px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.login-card :deep(.ant-card-head) {
  text-align: center;
  font-size: 24px;
  font-weight: bold;
}
</style>
