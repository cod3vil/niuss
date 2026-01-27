import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'

export interface RevenueStats {
  date: string
  revenue: number
  order_count: number
}

export interface TrafficStats {
  date: string
  upload: number
  download: number
  total: number
}

export interface NodeStats {
  node_id: number
  node_name: string
  total_upload: number
  total_download: number
  active_users: number
}

export const useStatsStore = defineStore('stats', () => {
  const revenueStats = ref<RevenueStats[]>([])
  const trafficStats = ref<TrafficStats[]>([])
  const nodeStats = ref<NodeStats[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  const fetchRevenueStats = async (startDate?: string, endDate?: string) => {
    loading.value = true
    error.value = null
    
    try {
      const params = new URLSearchParams()
      if (startDate) params.append('start_date', startDate)
      if (endDate) params.append('end_date', endDate)
      
      const response = await api.get<RevenueStats[]>(`/admin/stats/revenue?${params.toString()}`)
      revenueStats.value = response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取收入统计失败'
    } finally {
      loading.value = false
    }
  }

  const fetchTrafficStats = async (startDate?: string, endDate?: string) => {
    loading.value = true
    error.value = null
    
    try {
      const params = new URLSearchParams()
      if (startDate) params.append('start_date', startDate)
      if (endDate) params.append('end_date', endDate)
      
      const response = await api.get<TrafficStats[]>(`/admin/stats/traffic?${params.toString()}`)
      trafficStats.value = response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取流量统计失败'
    } finally {
      loading.value = false
    }
  }

  return {
    revenueStats,
    trafficStats,
    nodeStats,
    loading,
    error,
    fetchRevenueStats,
    fetchTrafficStats
  }
})
