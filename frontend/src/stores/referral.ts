import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'
import type { ReferralStats } from '@/types'

export const useReferralStore = defineStore('referral', () => {
  const stats = ref<ReferralStats | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  const fetchReferralStats = async () => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.get<ReferralStats>('/user/referral/stats')
      stats.value = response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取推荐统计失败'
    } finally {
      loading.value = false
    }
  }

  const fetchReferralLink = async () => {
    try {
      const response = await api.get<{ referral_link: string }>('/user/referral')
      return response.data.referral_link
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取推荐链接失败'
      return null
    }
  }

  return {
    stats,
    loading,
    error,
    fetchReferralStats,
    fetchReferralLink
  }
})
