import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'
import type { Order } from '@/types'

export const useOrderStore = defineStore('order', () => {
  const orders = ref<Order[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  const fetchOrders = async () => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.get<Order[]>('/orders')
      orders.value = response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取订单列表失败'
    } finally {
      loading.value = false
    }
  }

  const fetchOrderById = async (id: number) => {
    try {
      const response = await api.get<Order>(`/orders/${id}`)
      return response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取订单详情失败'
      return null
    }
  }

  return {
    orders,
    loading,
    error,
    fetchOrders,
    fetchOrderById
  }
})
