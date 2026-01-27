<template>
  <div class="px-4 py-6 sm:px-0">
    <h2 class="text-2xl font-bold text-gray-900 mb-6">仪表板</h2>

    <!-- Stats Cards -->
    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
      <!-- Coin Balance Card -->
      <div class="bg-white overflow-hidden shadow rounded-lg">
        <div class="p-5">
          <div class="flex items-center">
            <div class="flex-shrink-0">
              <svg class="h-6 w-6 text-yellow-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <div class="ml-5 w-0 flex-1">
              <dl>
                <dt class="text-sm font-medium text-gray-500 truncate">
                  金币余额
                </dt>
                <dd class="flex items-baseline">
                  <div class="text-2xl font-semibold text-gray-900">
                    {{ userStore.balance }}
                  </div>
                  <div class="ml-2 text-sm text-gray-500">
                    金币
                  </div>
                </dd>
              </dl>
            </div>
          </div>
        </div>
        <div class="bg-gray-50 px-5 py-3">
          <router-link to="/dashboard/packages" class="text-sm font-medium text-indigo-600 hover:text-indigo-500">
            购买套餐 →
          </router-link>
        </div>
      </div>

      <!-- Traffic Quota Card -->
      <div class="bg-white overflow-hidden shadow rounded-lg">
        <div class="p-5">
          <div class="flex items-center">
            <div class="flex-shrink-0">
              <svg class="h-6 w-6 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
              </svg>
            </div>
            <div class="ml-5 w-0 flex-1">
              <dl>
                <dt class="text-sm font-medium text-gray-500 truncate">
                  剩余流量
                </dt>
                <dd class="flex items-baseline">
                  <div class="text-2xl font-semibold text-gray-900">
                    {{ formatBytes(userStore.traffic.traffic_remaining) }}
                  </div>
                </dd>
              </dl>
            </div>
          </div>
        </div>
        <div class="bg-gray-50 px-5 py-3">
          <div class="text-sm text-gray-500">
            已使用 {{ userStore.traffic.percentage_used.toFixed(1) }}%
          </div>
        </div>
      </div>

      <!-- Traffic Usage Card -->
      <div class="bg-white overflow-hidden shadow rounded-lg">
        <div class="p-5">
          <div class="flex items-center">
            <div class="flex-shrink-0">
              <svg class="h-6 w-6 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
              </svg>
            </div>
            <div class="ml-5 w-0 flex-1">
              <dl>
                <dt class="text-sm font-medium text-gray-500 truncate">
                  已使用流量
                </dt>
                <dd class="flex items-baseline">
                  <div class="text-2xl font-semibold text-gray-900">
                    {{ formatBytes(userStore.traffic.traffic_used) }}
                  </div>
                </dd>
              </dl>
            </div>
          </div>
        </div>
        <div class="bg-gray-50 px-5 py-3">
          <div class="text-sm text-gray-500">
            总配额 {{ formatBytes(userStore.traffic.traffic_quota) }}
          </div>
        </div>
      </div>
    </div>

    <!-- Traffic Progress Bar -->
    <div class="mt-6 bg-white shadow rounded-lg p-6">
      <h3 class="text-lg font-medium text-gray-900 mb-4">流量使用情况</h3>
      <div class="w-full bg-gray-200 rounded-full h-4">
        <div
          class="h-4 rounded-full transition-all duration-300"
          :class="getProgressColor(userStore.traffic.percentage_used)"
          :style="{ width: `${Math.min(userStore.traffic.percentage_used, 100)}%` }"
        ></div>
      </div>
      <div class="mt-2 flex justify-between text-sm text-gray-600">
        <span>{{ formatBytes(userStore.traffic.traffic_used) }} / {{ formatBytes(userStore.traffic.traffic_quota) }}</span>
        <span>{{ userStore.traffic.percentage_used.toFixed(1) }}%</span>
      </div>
    </div>

    <!-- Quick Actions -->
    <div class="mt-6 bg-white shadow rounded-lg p-6">
      <h3 class="text-lg font-medium text-gray-900 mb-4">快速操作</h3>
      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <router-link
          to="/dashboard/packages"
          class="flex items-center justify-center px-4 py-3 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
        >
          购买套餐
        </router-link>
        <router-link
          to="/dashboard/subscription"
          class="flex items-center justify-center px-4 py-3 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
        >
          获取订阅
        </router-link>
        <router-link
          to="/dashboard/orders"
          class="flex items-center justify-center px-4 py-3 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
        >
          查看订单
        </router-link>
        <router-link
          to="/dashboard/referral"
          class="flex items-center justify-center px-4 py-3 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
        >
          推荐返利
        </router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useUserStore } from '@/stores/user'

const userStore = useUserStore()

onMounted(() => {
  userStore.refresh()
})

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const getProgressColor = (percentage: number): string => {
  if (percentage < 50) return 'bg-green-500'
  if (percentage < 80) return 'bg-yellow-500'
  return 'bg-red-500'
}
</script>
