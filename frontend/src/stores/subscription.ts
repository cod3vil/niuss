import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'
import type { Subscription, Node } from '@/types'

export const useSubscriptionStore = defineStore('subscription', () => {
  const subscription = ref<Subscription | null>(null)
  const nodes = ref<Node[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  const fetchSubscription = async () => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.get('/subscription/link')
      const data = response.data
      // Backend returns subscription_url, map it to url for frontend
      subscription.value = {
        token: data.token,
        url: data.subscription_url
      }
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取订阅链接失败'
    } finally {
      loading.value = false
    }
  }

  const fetchNodes = async () => {
    try {
      const response = await api.get<Node[]>('/admin/nodes')
      // Filter only online nodes for user display
      nodes.value = response.data.filter(node => node.status === 'online')
    } catch (e: any) {
      console.error('Failed to fetch nodes:', e)
    }
  }

  return {
    subscription,
    nodes,
    loading,
    error,
    fetchSubscription,
    fetchNodes
  }
})
