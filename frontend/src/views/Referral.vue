<template>
  <div class="px-4 py-6 sm:px-0">
    <h2 class="text-2xl font-bold text-gray-900 mb-6">推荐返利</h2>

    <!-- Error Message -->
    <div v-if="referralStore.error" class="mb-6 rounded-md bg-red-50 p-4">
      <div class="text-sm text-red-800">
        {{ referralStore.error }}
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="referralStore.loading" class="text-center py-12">
      <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600"></div>
      <p class="mt-2 text-gray-600">加载中...</p>
    </div>

    <!-- Referral Content -->
    <div v-else class="space-y-6">
      <!-- Stats Cards -->
      <div class="grid grid-cols-1 gap-5 sm:grid-cols-2">
        <!-- Referral Count Card -->
        <div class="bg-white overflow-hidden shadow rounded-lg">
          <div class="p-5">
            <div class="flex items-center">
              <div class="flex-shrink-0">
                <svg class="h-6 w-6 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
                </svg>
              </div>
              <div class="ml-5 w-0 flex-1">
                <dl>
                  <dt class="text-sm font-medium text-gray-500 truncate">
                    推荐人数
                  </dt>
                  <dd class="flex items-baseline">
                    <div class="text-2xl font-semibold text-gray-900">
                      {{ referralStore.stats?.referral_count || 0 }}
                    </div>
                    <div class="ml-2 text-sm text-gray-500">
                      人
                    </div>
                  </dd>
                </dl>
              </div>
            </div>
          </div>
        </div>

        <!-- Total Commission Card -->
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
                    累计返利
                  </dt>
                  <dd class="flex items-baseline">
                    <div class="text-2xl font-semibold text-gray-900">
                      {{ referralStore.stats?.total_commission || 0 }}
                    </div>
                    <div class="ml-2 text-sm text-gray-500">
                      金币
                    </div>
                  </dd>
                </dl>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Referral Link Card -->
      <div class="bg-white shadow rounded-lg p-6">
        <h3 class="text-lg font-medium text-gray-900 mb-4">我的推荐链接</h3>
        <p class="text-sm text-gray-600 mb-4">
          分享此链接给好友，好友注册并首次购买后，您将获得返利奖励
        </p>
        
        <div class="flex items-center space-x-2">
          <input
            type="text"
            :value="referralStore.stats?.referral_link || ''"
            readonly
            class="flex-1 px-3 py-2 border border-gray-300 rounded-md bg-gray-50 text-sm text-gray-700"
          />
          <button
            @click="copyToClipboard(referralStore.stats?.referral_link || '')"
            class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
          >
            <svg v-if="!copied" class="h-5 w-5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
            <svg v-else class="h-5 w-5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
            {{ copied ? '已复制' : '复制链接' }}
          </button>
        </div>
      </div>

      <!-- How It Works -->
      <div class="bg-white shadow rounded-lg p-6">
        <h3 class="text-lg font-medium text-gray-900 mb-4">返利规则</h3>
        <div class="space-y-4">
          <div class="flex items-start">
            <div class="flex-shrink-0">
              <div class="flex items-center justify-center h-8 w-8 rounded-full bg-indigo-100 text-indigo-600 font-semibold">
                1
              </div>
            </div>
            <div class="ml-4">
              <h4 class="text-sm font-medium text-gray-900">分享推荐链接</h4>
              <p class="mt-1 text-sm text-gray-600">
                复制您的专属推荐链接，分享给好友
              </p>
            </div>
          </div>

          <div class="flex items-start">
            <div class="flex-shrink-0">
              <div class="flex items-center justify-center h-8 w-8 rounded-full bg-indigo-100 text-indigo-600 font-semibold">
                2
              </div>
            </div>
            <div class="ml-4">
              <h4 class="text-sm font-medium text-gray-900">好友注册</h4>
              <p class="mt-1 text-sm text-gray-600">
                好友通过您的链接注册账号
              </p>
            </div>
          </div>

          <div class="flex items-start">
            <div class="flex-shrink-0">
              <div class="flex items-center justify-center h-8 w-8 rounded-full bg-indigo-100 text-indigo-600 font-semibold">
                3
              </div>
            </div>
            <div class="ml-4">
              <h4 class="text-sm font-medium text-gray-900">好友首次购买</h4>
              <p class="mt-1 text-sm text-gray-600">
                好友完成首次套餐购买
              </p>
            </div>
          </div>

          <div class="flex items-start">
            <div class="flex-shrink-0">
              <div class="flex items-center justify-center h-8 w-8 rounded-full bg-indigo-100 text-indigo-600 font-semibold">
                4
              </div>
            </div>
            <div class="ml-4">
              <h4 class="text-sm font-medium text-gray-900">获得返利</h4>
              <p class="mt-1 text-sm text-gray-600">
                您将自动获得返利金币，可用于购买套餐
              </p>
            </div>
          </div>
        </div>
      </div>

      <!-- Tips -->
      <div class="bg-blue-50 border border-blue-200 rounded-lg p-4">
        <div class="flex">
          <div class="flex-shrink-0">
            <svg class="h-5 w-5 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <div class="ml-3">
            <h3 class="text-sm font-medium text-blue-800">
              温馨提示
            </h3>
            <div class="mt-2 text-sm text-blue-700">
              <ul class="list-disc list-inside space-y-1">
                <li>返利金币将在好友首次购买后自动到账</li>
                <li>返利金额根据好友购买的套餐价格计算</li>
                <li>不支持自我推荐</li>
                <li>推荐人数和返利金额实时更新</li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useReferralStore } from '@/stores/referral'

const referralStore = useReferralStore()
const copied = ref(false)

onMounted(() => {
  referralStore.fetchReferralStats()
})

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
