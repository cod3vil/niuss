import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '@/api'

export interface AdminUser {
  id: number
  email: string
  is_admin: boolean
  status: string
  coin_balance?: number
  traffic_quota?: number
  traffic_used?: number
  referral_code?: string | null
  created_at?: string
}

export interface LoginRequest {
  email: string
  password: string
}

export interface AuthResponse {
  token: string
  user: AdminUser
}

export const useAuthStore = defineStore('auth', () => {
  const user = ref<AdminUser | null>(null)
  const token = ref<string | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const isAuthenticated = computed(() => !!token.value && !!user.value)
  const isAdmin = computed(() => user.value?.is_admin === true)

  // Initialize from localStorage
  const init = () => {
    const storedToken = localStorage.getItem('admin_token')
    const storedUser = localStorage.getItem('admin_user')
    
    if (storedToken && storedUser) {
      token.value = storedToken
      try {
        user.value = JSON.parse(storedUser)
      } catch (e) {
        localStorage.removeItem('admin_token')
        localStorage.removeItem('admin_user')
      }
    }
  }

  const login = async (credentials: LoginRequest) => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.post<AuthResponse>('/auth/login', credentials)
      const { token: authToken, user: userData } = response.data
      
      // Verify admin role
      if (!userData.is_admin) {
        error.value = '无管理员权限'
        return false
      }
      
      token.value = authToken
      user.value = userData
      
      localStorage.setItem('admin_token', authToken)
      localStorage.setItem('admin_user', JSON.stringify(userData))
      
      return true
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '登录失败'
      return false
    } finally {
      loading.value = false
    }
  }

  const logout = () => {
    token.value = null
    user.value = null
    localStorage.removeItem('admin_token')
    localStorage.removeItem('admin_user')
  }

  return {
    user,
    token,
    loading,
    error,
    isAuthenticated,
    isAdmin,
    init,
    login,
    logout
  }
})
