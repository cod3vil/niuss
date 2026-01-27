import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'

export interface Order {
  id: number
  order_no: string
  user_id: number
  package_id: number
  amount: number
  status: string
  created_at: string
  completed_at: string | null
  user_email?: string
  package_name?: string
}

export interface OrdersResponse {
  orders: Order[]
  total: number
  limit: number
  offset: number
}

export const useOrdersStore = defineStore('orders', () => {
  const orders = ref<Order[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const fetchOrders = async (filters?: { status?: string; start_date?: string; end_date?: string; limit?: number; offset?: number }) => {
    loading.value = true
    error.value = null
    
    try {
      const params = new URLSearchParams()
      if (filters?.status) params.append('status', filters.status)
      if (filters?.start_date) params.append('start_date', filters.start_date)
      if (filters?.end_date) params.append('end_date', filters.end_date)
      if (filters?.limit) params.append('limit', filters.limit.toString())
      if (filters?.offset) params.append('offset', filters.offset.toString())
      
      const response = await api.get<OrdersResponse>(`/admin/orders?${params.toString()}`)
      orders.value = response.data.orders
      total.value = response.data.total
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取订单列表失败'
      console.error('Failed to fetch orders:', e)
    } finally {
      loading.value = false
    }
  }

  const getOrderById = async (id: number) => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.get<Order>(`/admin/orders/${id}`)
      return response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取订单详情失败'
      return null
    } finally {
      loading.value = false
    }
  }

  return {
    orders,
    total,
    loading,
    error,
    fetchOrders,
    getOrderById
  }
})
