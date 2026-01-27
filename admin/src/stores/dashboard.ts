import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'

export interface DashboardStats {
  total_users: number
  active_users: number
  total_traffic: number
  total_revenue: number
}

export const useDashboardStore = defineStore('dashboard', () => {
  const stats = ref<DashboardStats>({
    total_users: 0,
    active_users: 0,
    total_traffic: 0,
    total_revenue: 0
  })
  const loading = ref(false)
  const error = ref<string | null>(null)

  const fetchStats = async () => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.get<DashboardStats>('/admin/stats/overview')
      stats.value = response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取统计数据失败'
    } finally {
      loading.value = false
    }
  }

  return {
    stats,
    loading,
    error,
    fetchStats
  }
})
