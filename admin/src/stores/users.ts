import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'

export interface User {
  id: number
  email: string
  coin_balance: number
  traffic_quota: number
  traffic_used: number
  referral_code: string | null
  status: string
  created_at: string
  updated_at: string
}

export const useUsersStore = defineStore('users', () => {
  const users = ref<User[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  const fetchUsers = async () => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.get<User[]>('/admin/users')
      users.value = response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取用户列表失败'
    } finally {
      loading.value = false
    }
  }

  const updateUserStatus = async (id: number, status: string) => {
    loading.value = true
    error.value = null
    
    try {
      await api.put(`/admin/users/${id}/status`, { status })
      await fetchUsers()
      return true
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '更新用户状态失败'
      return false
    } finally {
      loading.value = false
    }
  }

  const updateUserBalance = async (id: number, amount: number) => {
    loading.value = true
    error.value = null
    
    try {
      await api.put(`/admin/users/${id}/balance`, { amount })
      await fetchUsers()
      return true
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '调整用户金币失败'
      return false
    } finally {
      loading.value = false
    }
  }

  const updateUserTraffic = async (id: number, traffic: number) => {
    loading.value = true
    error.value = null
    
    try {
      await api.put(`/admin/users/${id}/traffic`, { traffic })
      await fetchUsers()
      return true
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '调整用户流量失败'
      return false
    } finally {
      loading.value = false
    }
  }

  return {
    users,
    loading,
    error,
    fetchUsers,
    updateUserStatus,
    updateUserBalance,
    updateUserTraffic
  }
})
