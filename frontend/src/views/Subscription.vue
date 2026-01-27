<template>
  <div class="px-4 py-6 sm:px-0">
    <h2 class="text-2xl font-bold text-gray-900 mb-6">订阅管理</h2>

    <!-- Error Message -->
    <div v-if="subscriptionStore.error" class="mb-6 rounded-md bg-red-50 p-4">
      <div class="text-sm text-red-800">
        {{ subscriptionStore.error }}
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="subscriptionStore.loading" class="text-center py-12">
      <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600"></div>
      <p class="mt-2 text-gray-600">加载中...</p>
    </div>

    <!-- Subscription Link -->
    <div v-else-if="subscriptionStore.subscription" class="space-y-6">
      <!-- Subscription URL Card -->
      <div class="bg-white shadow rounded-lg p-6">
        <h3 class="text-lg font-medium text-gray-900 mb-4">订阅链接</h3>
        <p class="text-sm text-gray-600 mb-4">
          将此链接添加到 Clash 客户端即可使用 VPN 服务
        </p>
        
        <div class="flex items-center space-x-2">
          <input
            type="text"
            :value="subscriptionStore.subscription.url"
            readonly
            class="flex-1 px-3 py-2 border border-gray-300 rounded-md bg-gray-50 text-sm text-gray-700"
          />
          <button
            @click="copyToClipboard(subscriptionStore.subscription.url)"
            class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
          >
            <svg v-if="!copied" class="h-5 w-5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
            <svg v-else class="h-5 w-5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
            {{ copied ? '已复制' : '复制' }}
          </button>
        </div>

        <!-- QR Code (Optional - would need a QR code library) -->
        <div class="mt-4 p-4 bg-gray-50 rounded-md">
          <p class="text-xs text-gray-600">
            提示：在 Clash 客户端中选择"从 URL 导入配置"，粘贴上方链接即可
          </p>
        </div>
      </div>

      <!-- Usage Instructions -->
      <div class="bg-white shadow rounded-lg p-6">
        <h3 class="text-lg font-medium text-gray-900 mb-4">使用说明</h3>
        <ol class="list-decimal list-inside space-y-2 text-sm text-gray-700">
          <li>下载并安装 Clash 客户端（支持 Windows、macOS、Android、iOS）</li>
          <li>打开 Clash 客户端，选择"配置" → "从 URL 导入"</li>
          <li>粘贴上方的订阅链接，点击"确定"</li>
          <li>等待配置更新完成，选择一个节点</li>
          <li>开启系统代理，即可开始使用</li>
        </ol>
      </div>

      <!-- Available Nodes -->
      <div class="bg-white shadow rounded-lg p-6">
        <h3 class="text-lg font-medium text-gray-900 mb-4">节点信息</h3>
        
        <div class="text-sm text-gray-600 space-y-2">
          <p>✓ 所有可用节点已包含在订阅链接中</p>
          <p>✓ 导入订阅后，Clash 会自动获取最新的节点列表</p>
          <p>✓ 节点会根据您的套餐自动更新</p>
        </div>
      </div>

      <!-- Traffic Status -->
      <div class="bg-white shadow rounded-lg p-6">
        <h3 class="text-lg font-medium text-gray-900 mb-4">流量状态</h3>
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
          <div class="text-center">
            <p class="text-sm text-gray-600">总配额</p>
            <p class="text-2xl font-semibold text-gray-900 mt-1">
              {{ formatBytes(userStore.traffic.traffic_quota) }}
            </p>
          </div>
          <div class="text-center">
            <p class="text-sm text-gray-600">已使用</p>
            <p class="text-2xl font-semibold text-gray-900 mt-1">
              {{ formatBytes(userStore.traffic.traffic_used) }}
            </p>
          </div>
          <div class="text-center">
            <p class="text-sm text-gray-600">剩余</p>
            <p class="text-2xl font-semibold text-gray-900 mt-1">
              {{ formatBytes(userStore.traffic.traffic_remaining) }}
            </p>
          </div>
        </div>
        <div class="mt-4">
          <div class="w-full bg-gray-200 rounded-full h-2">
            <div
              class="h-2 rounded-full transition-all duration-300"
              :class="getProgressColor(userStore.traffic.percentage_used)"
              :style="{ width: `${Math.min(userStore.traffic.percentage_used, 100)}%` }"
            ></div>
          </div>
          <p class="mt-2 text-sm text-gray-600 text-center">
            已使用 {{ userStore.traffic.percentage_used.toFixed(1) }}%
          </p>
        </div>
      </div>
    </div>

    <!-- No Subscription -->
    <div v-else class="text-center py-12 bg-white rounded-lg shadow">
      <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
      </svg>
      <h3 class="mt-2 text-sm font-medium text-gray-900">暂无订阅</h3>
      <p class="mt-1 text-sm text-gray-500">请先购买套餐以获取订阅链接</p>
      <div class="mt-6">
        <router-link
          to="/dashboard/packages"
          class="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700"
        >
          购买套餐
        </router-link>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useSubscriptionStore } from '@/stores/subscription'
import { useUserStore } from '@/stores/user'

const subscriptionStore = useSubscriptionStore()
const userStore = useUserStore()
const copied = ref(false)

onMounted(() => {
  subscriptionStore.fetchSubscription()
  // Note: Removed fetchNodes() as it calls admin API
  // Nodes will be available through the subscription config
  userStore.fetchTraffic()
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

const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch (err) {
    console.error('Failed to copy:', err)
  }
}
</script>
