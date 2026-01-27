import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'
import type { Package } from '@/types'

export const usePackageStore = defineStore('package', () => {
  const packages = ref<Package[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const purchasing = ref(false)

  const fetchPackages = async () => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.get<Package[]>('/packages')
      packages.value = response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取套餐列表失败'
    } finally {
      loading.value = false
    }
  }

  const purchasePackage = async (packageId: number) => {
    purchasing.value = true
    error.value = null
    
    try {
      await api.post(`/packages/${packageId}/purchase`)
      return true
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '购买失败'
      return false
    } finally {
      purchasing.value = false
    }
  }

  return {
    packages,
    loading,
    error,
    purchasing,
    fetchPackages,
    purchasePackage
  }
})
