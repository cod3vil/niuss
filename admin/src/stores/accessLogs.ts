import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'

export interface AccessLog {
  id: number
  user_id: number
  user_email: string
  subscription_token: string
  access_timestamp: string
  ip_address: string
  user_agent: string | null
  response_status: string
}

export interface AccessLogFilters {
  userSearch?: string
  startDate?: string
  endDate?: string
  status?: string
  page?: number
  pageSize?: number
}

export interface AccessLogListResponse {
  logs: AccessLog[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

export const useAccessLogsStore = defineStore('accessLogs', () => {
  const logs = ref<AccessLog[]>([])
  const total = ref(0)
  const page = ref(1)
  const pageSize = ref(50)
  const totalPages = ref(1)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const fetchAccessLogs = async (filters: AccessLogFilters = {}): Promise<AccessLogListResponse> => {
    loading.value = true
    error.value = null

    try {
      const params = new URLSearchParams()

      if (filters.userSearch) {
        // Try to parse as user ID, otherwise search by email
        const userId = parseInt(filters.userSearch)
        if (!isNaN(userId)) {
          params.append('user_id', userId.toString())
        }
        // Note: Email search would require backend support
      }

      if (filters.startDate) {
        params.append('start_date', new Date(filters.startDate).toISOString())
      }

      if (filters.endDate) {
        params.append('end_date', new Date(filters.endDate).toISOString())
      }

      if (filters.status) {
        params.append('status', filters.status)
      }

      params.append('page', (filters.page || 1).toString())
      params.append('page_size', (filters.pageSize || 50).toString())

      const response = await api.get<AccessLogListResponse>(`/admin/access-logs?${params.toString()}`)

      logs.value = response.data.logs
      total.value = response.data.total
      page.value = response.data.page
      pageSize.value = response.data.page_size
      totalPages.value = response.data.total_pages

      return response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || 'Failed to fetch access logs'
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    logs,
    total,
    page,
    pageSize,
    totalPages,
    loading,
    error,
    fetchAccessLogs
  }
})
