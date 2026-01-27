import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'
import type { TrafficInfo } from '@/types'

export const useUserStore = defineStore('user', () => {
  const balance = ref(0)
  const traffic = ref<TrafficInfo>({
    traffic_quota: 0,
    traffic_used: 0,
    traffic_remaining: 0,
    percentage_used: 0
  })
  const loading = ref(false)

  const fetchBalance = async () => {
    try {
      const response = await api.get('/user/balance')
      balance.value = response.data.coin_balance
    } catch (e) {
      console.error('Failed to fetch balance:', e)
    }
  }

  const fetchTraffic = async () => {
    try {
      const response = await api.get('/user/traffic')
      traffic.value = response.data
    } catch (e) {
      console.error('Failed to fetch traffic:', e)
    }
  }

  const refresh = async () => {
    loading.value = true
    try {
      await Promise.all([fetchBalance(), fetchTraffic()])
    } finally {
      loading.value = false
    }
  }

  return {
    balance,
    traffic,
    loading,
    fetchBalance,
    fetchTraffic,
    refresh
  }
})
