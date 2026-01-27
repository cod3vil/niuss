import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import api from '@/api'
import type { User, LoginRequest, RegisterRequest, AuthResponse } from '@/types'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const token = ref<string | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const isAuthenticated = computed(() => !!token.value && !!user.value)

  // Initialize from localStorage
  const init = () => {
    const storedToken = localStorage.getItem('token')
    const storedUser = localStorage.getItem('user')
    
    if (storedToken && storedUser) {
      token.value = storedToken
      try {
        user.value = JSON.parse(storedUser)
      } catch (e) {
        localStorage.removeItem('token')
        localStorage.removeItem('user')
      }
    }
  }

  const login = async (credentials: LoginRequest) => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.post<AuthResponse>('/auth/login', credentials)
      const { token: authToken, user: userData } = response.data
      
      token.value = authToken
      user.value = userData
      
      localStorage.setItem('token', authToken)
      localStorage.setItem('user', JSON.stringify(userData))
      
      return true
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '登录失败'
      return false
    } finally {
      loading.value = false
    }
  }

  const register = async (data: RegisterRequest) => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.post<AuthResponse>('/auth/register', data)
      const { token: authToken, user: userData } = response.data
      
      token.value = authToken
      user.value = userData
      
      localStorage.setItem('token', authToken)
      localStorage.setItem('user', JSON.stringify(userData))
      
      return true
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '注册失败'
      return false
    } finally {
      loading.value = false
    }
  }

  const logout = () => {
    token.value = null
    user.value = null
    localStorage.removeItem('token')
    localStorage.removeItem('user')
  }

  const refreshUser = async () => {
    try {
      const response = await api.get<User>('/user/profile')
      user.value = response.data
      localStorage.setItem('user', JSON.stringify(response.data))
    } catch (e) {
      console.error('Failed to refresh user data:', e)
    }
  }

  // Auto-refresh token (simplified - in production use refresh tokens)
  const setupTokenRefresh = () => {
    setInterval(() => {
      if (isAuthenticated.value) {
        refreshUser()
      }
    }, 5 * 60 * 1000) // Refresh every 5 minutes
  }

  return {
    user,
    token,
    loading,
    error,
    isAuthenticated,
    init,
    login,
    register,
    logout,
    refreshUser,
    setupTokenRefresh
  }
})
