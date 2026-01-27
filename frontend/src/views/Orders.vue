<template>
  <div class="px-4 py-6 sm:px-0">
    <h2 class="text-2xl font-bold text-gray-900 mb-6">订单历史</h2>

    <!-- Error Message -->
    <div v-if="orderStore.error" class="mb-6 rounded-md bg-red-50 p-4">
      <div class="text-sm text-red-800">
        {{ orderStore.error }}
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="orderStore.loading" class="text-center py-12">
      <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600"></div>
      <p class="mt-2 text-gray-600">加载中...</p>
    </div>

    <!-- Empty State -->
    <div v-else-if="orderStore.orders.length === 0" class="text-center py-12 bg-white rounded-lg shadow">
      <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
      <h3 class="mt-2 text-sm font-medium text-gray-900">暂无订单</h3>
      <p class="mt-1 text-sm text-gray-500">您还没有购买任何套餐</p>
      <div class="mt-6">
        <router-link
          to="/dashboard/packages"
          class="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700"
        >
          购买套餐
        </router-link>
      </div>
    </div>

    <!-- Orders List -->
    <div v-else class="bg-white shadow overflow-hidden sm:rounded-md">
      <ul class="divide-y divide-gray-200">
        <li v-for="order in orderStore.orders" :key="order.id">
          <div class="px-4 py-4 sm:px-6 hover:bg-gray-50 cursor-pointer" @click="toggleOrderDetails(order.id)">
            <div class="flex items-center justify-between">
              <div class="flex-1">
                <div class="flex items-center justify-between">
                  <p class="text-sm font-medium text-indigo-600 truncate">
                    订单号：{{ order.order_no }}
                  </p>
                  <div class="ml-2 flex-shrink-0 flex">
                    <p
                      class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full"
                      :class="getStatusClass(order.status)"
                    >
                      {{ getStatusText(order.status) }}
                    </p>
                  </div>
                </div>
                <div class="mt-2 sm:flex sm:justify-between">
                  <div class="sm:flex">
                    <p class="flex items-center text-sm text-gray-500">
                      <svg class="flex-shrink-0 mr-1.5 h-5 w-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 11V7a4 4 0 00-8 0v4M5 9h14l1 12H4L5 9z" />
                      </svg>
                      {{ order.package?.name || '套餐' }}
                    </p>
                    <p class="mt-2 flex items-center text-sm text-gray-500 sm:mt-0 sm:ml-6">
                      <svg class="flex-shrink-0 mr-1.5 h-5 w-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                      </svg>
                      {{ order.amount }} 金币
                    </p>
                  </div>
                  <div class="mt-2 flex items-center text-sm text-gray-500 sm:mt-0">
                    <svg class="flex-shrink-0 mr-1.5 h-5 w-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" />
                    </svg>
                    {{ formatDate(order.created_at) }}
                  </div>
                </div>
              </div>
              <div class="ml-5 flex-shrink-0">
                <svg
                  class="h-5 w-5 text-gray-400 transition-transform"
                  :class="{ 'transform rotate-180': expandedOrders.has(order.id) }"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                </svg>
              </div>
            </div>

            <!-- Order Details (Expanded) -->
            <div v-if="expandedOrders.has(order.id)" class="mt-4 border-t border-gray-200 pt-4">
              <dl class="grid grid-cols-1 gap-x-4 gap-y-4 sm:grid-cols-2">
                <div v-if="order.package">
                  <dt class="text-sm font-medium text-gray-500">套餐信息</dt>
                  <dd class="mt-1 text-sm text-gray-900">
                    <p>{{ order.package.name }}</p>
                    <p class="text-gray-600">流量：{{ formatBytes(order.package.traffic_amount) }}</p>
                    <p class="text-gray-600">有效期：{{ order.package.duration_days }} 天</p>
                  </dd>
                </div>
                <div>
                  <dt class="text-sm font-medium text-gray-500">支付金额</dt>
                  <dd class="mt-1 text-sm text-gray-900">{{ order.amount }} 金币</dd>
                </div>
                <div>
                  <dt class="text-sm font-medium text-gray-500">创建时间</dt>
                  <dd class="mt-1 text-sm text-gray-900">{{ formatDate(order.created_at) }}</dd>
                </div>
                <div v-if="order.completed_at">
                  <dt class="text-sm font-medium text-gray-500">完成时间</dt>
                  <dd class="mt-1 text-sm text-gray-900">{{ formatDate(order.completed_at) }}</dd>
                </div>
              </dl>
            </div>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useOrderStore } from '@/stores/order'

const orderStore = useOrderStore()
const expandedOrders = ref(new Set<number>())

onMounted(() => {
  orderStore.fetchOrders()
})

const formatDate = (dateString: string): string => {
  return new Date(dateString).toLocaleString('zh-CN')
}

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const getStatusText = (status: string): string => {
  const statusMap: Record<string, string> = {
    pending: '待支付',
    completed: '已完成',
    failed: '失败'
  }
  return statusMap[status] || status
}

const getStatusClass = (status: string): string => {
  const classMap: Record<string, string> = {
    pending: 'bg-yellow-100 text-yellow-800',
    completed: 'bg-green-100 text-green-800',
    failed: 'bg-red-100 text-red-800'
  }
  return classMap[status] || 'bg-gray-100 text-gray-800'
}

const toggleOrderDetails = (orderId: number) => {
  if (expandedOrders.value.has(orderId)) {
    expandedOrders.value.delete(orderId)
  } else {
    expandedOrders.value.add(orderId)
  }
}
</script>
