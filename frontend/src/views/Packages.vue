<template>
  <div class="px-4 py-6 sm:px-0">
    <h2 class="text-2xl font-bold text-gray-900 mb-6">购买套餐</h2>

    <!-- Current Balance -->
    <div class="mb-6 bg-blue-50 border border-blue-200 rounded-lg p-4">
      <div class="flex items-center">
        <svg class="h-5 w-5 text-blue-400 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span class="text-sm text-blue-800">
          当前金币余额：<span class="font-semibold">{{ userStore.balance }}</span> 金币
        </span>
      </div>
    </div>

    <!-- Error Message -->
    <div v-if="packageStore.error" class="mb-6 rounded-md bg-red-50 p-4">
      <div class="text-sm text-red-800">
        {{ packageStore.error }}
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="packageStore.loading" class="text-center py-12">
      <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600"></div>
      <p class="mt-2 text-gray-600">加载中...</p>
    </div>

    <!-- Packages Grid -->
    <div v-else class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
      <div
        v-for="pkg in packageStore.packages"
        :key="pkg.id"
        class="bg-white overflow-hidden shadow rounded-lg hover:shadow-lg transition-shadow"
      >
        <div class="p-6">
          <h3 class="text-lg font-semibold text-gray-900 mb-2">
            {{ pkg.name }}
          </h3>
          <p v-if="pkg.description" class="text-sm text-gray-600 mb-4">
            {{ pkg.description }}
          </p>
          
          <div class="space-y-2 mb-6">
            <div class="flex items-center text-sm text-gray-700">
              <svg class="h-5 w-5 text-green-500 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
              </svg>
              流量：{{ formatBytes(pkg.traffic_amount) }}
            </div>
            <div class="flex items-center text-sm text-gray-700">
              <svg class="h-5 w-5 text-green-500 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
              </svg>
              有效期：{{ pkg.duration_days }} 天
            </div>
          </div>

          <div class="flex items-baseline mb-4">
            <span class="text-3xl font-bold text-gray-900">{{ pkg.price }}</span>
            <span class="ml-2 text-sm text-gray-600">金币</span>
          </div>

          <button
            @click="handlePurchase(pkg)"
            :disabled="packageStore.purchasing || userStore.balance < pkg.price"
            class="w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <span v-if="userStore.balance < pkg.price">余额不足</span>
            <span v-else-if="packageStore.purchasing">购买中...</span>
            <span v-else>立即购买</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Confirmation Modal -->
    <div
      v-if="showConfirmModal"
      class="fixed z-10 inset-0 overflow-y-auto"
      aria-labelledby="modal-title"
      role="dialog"
      aria-modal="true"
    >
      <div class="flex items-end justify-center min-h-screen pt-4 px-4 pb-20 text-center sm:block sm:p-0">
        <div class="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity" @click="showConfirmModal = false"></div>

        <span class="hidden sm:inline-block sm:align-middle sm:h-screen" aria-hidden="true">&#8203;</span>

        <div class="inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full">
          <div class="bg-white px-4 pt-5 pb-4 sm:p-6 sm:pb-4">
            <div class="sm:flex sm:items-start">
              <div class="mx-auto flex-shrink-0 flex items-center justify-center h-12 w-12 rounded-full bg-indigo-100 sm:mx-0 sm:h-10 sm:w-10">
                <svg class="h-6 w-6 text-indigo-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 11V7a4 4 0 00-8 0v4M5 9h14l1 12H4L5 9z" />
                </svg>
              </div>
              <div class="mt-3 text-center sm:mt-0 sm:ml-4 sm:text-left">
                <h3 class="text-lg leading-6 font-medium text-gray-900" id="modal-title">
                  确认购买
                </h3>
                <div class="mt-2">
                  <p class="text-sm text-gray-500">
                    您确定要购买 <span class="font-semibold">{{ selectedPackage?.name }}</span> 吗？
                  </p>
                  <div class="mt-4 space-y-2 text-sm text-gray-700">
                    <p>流量：{{ formatBytes(selectedPackage?.traffic_amount || 0) }}</p>
                    <p>有效期：{{ selectedPackage?.duration_days }} 天</p>
                    <p>价格：<span class="font-semibold text-indigo-600">{{ selectedPackage?.price }} 金币</span></p>
                    <p>购买后余额：<span class="font-semibold">{{ userStore.balance - (selectedPackage?.price || 0) }} 金币</span></p>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div class="bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse">
            <button
              type="button"
              @click="confirmPurchase"
              :disabled="packageStore.purchasing"
              class="w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-indigo-600 text-base font-medium text-white hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 sm:ml-3 sm:w-auto sm:text-sm disabled:opacity-50"
            >
              确认购买
            </button>
            <button
              type="button"
              @click="showConfirmModal = false"
              :disabled="packageStore.purchasing"
              class="mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 sm:mt-0 sm:ml-3 sm:w-auto sm:text-sm"
            >
              取消
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { usePackageStore } from '@/stores/package'
import { useUserStore } from '@/stores/user'
import { useAuthStore } from '@/stores/auth'
import type { Package } from '@/types'

const router = useRouter()
const packageStore = usePackageStore()
const userStore = useUserStore()
const authStore = useAuthStore()

const showConfirmModal = ref(false)
const selectedPackage = ref<Package | null>(null)

onMounted(() => {
  packageStore.fetchPackages()
  userStore.fetchBalance()
})

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const handlePurchase = (pkg: Package) => {
  selectedPackage.value = pkg
  showConfirmModal.value = true
}

const confirmPurchase = async () => {
  if (!selectedPackage.value) return
  
  const success = await packageStore.purchasePackage(selectedPackage.value.id)
  if (success) {
    showConfirmModal.value = false
    selectedPackage.value = null
    
    // Refresh user data
    await userStore.refresh()
    await authStore.refreshUser()
    
    // Redirect to subscription page
    router.push('/dashboard/subscription')
  }
}
</script>
