<template>
  <div class="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100">
    <!-- Hero Section -->
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
      <div class="text-center">
        <h1 class="text-4xl font-extrabold text-gray-900 sm:text-5xl md:text-6xl">
          <span class="block">VPN 订阅服务平台</span>
          <span class="block text-indigo-600">安全、快速、稳定</span>
        </h1>
        <p class="mt-3 max-w-md mx-auto text-base text-gray-500 sm:text-lg md:mt-5 md:text-xl md:max-w-3xl">
          提供高速稳定的 VPN 服务，支持多种协议，全球节点覆盖
        </p>
        <div class="mt-5 max-w-md mx-auto sm:flex sm:justify-center md:mt-8">
          <div class="rounded-md shadow">
            <router-link
              to="/register"
              class="w-full flex items-center justify-center px-8 py-3 border border-transparent text-base font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 md:py-4 md:text-lg md:px-10"
            >
              立即注册
            </router-link>
          </div>
          <div class="mt-3 rounded-md shadow sm:mt-0 sm:ml-3">
            <router-link
              to="/login"
              class="w-full flex items-center justify-center px-8 py-3 border border-transparent text-base font-medium rounded-md text-indigo-600 bg-white hover:bg-gray-50 md:py-4 md:text-lg md:px-10"
            >
              登录
            </router-link>
          </div>
        </div>
      </div>

      <!-- Features -->
      <div class="mt-20">
        <div class="grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-3">
          <div class="bg-white rounded-lg shadow-lg p-6">
            <div class="flex items-center justify-center h-12 w-12 rounded-md bg-indigo-500 text-white">
              <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
            </div>
            <h3 class="mt-4 text-lg font-medium text-gray-900">高速稳定</h3>
            <p class="mt-2 text-base text-gray-500">
              采用高性能服务器，提供稳定快速的网络连接
            </p>
          </div>

          <div class="bg-white rounded-lg shadow-lg p-6">
            <div class="flex items-center justify-center h-12 w-12 rounded-md bg-indigo-500 text-white">
              <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
              </svg>
            </div>
            <h3 class="mt-4 text-lg font-medium text-gray-900">安全加密</h3>
            <p class="mt-2 text-base text-gray-500">
              支持多种加密协议，保护您的网络隐私和数据安全
            </p>
          </div>

          <div class="bg-white rounded-lg shadow-lg p-6">
            <div class="flex items-center justify-center h-12 w-12 rounded-md bg-indigo-500 text-white">
              <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <h3 class="mt-4 text-lg font-medium text-gray-900">全球节点</h3>
            <p class="mt-2 text-base text-gray-500">
              多个国家和地区的节点，满足不同的访问需求
            </p>
          </div>

          <div class="bg-white rounded-lg shadow-lg p-6">
            <div class="flex items-center justify-center h-12 w-12 rounded-md bg-indigo-500 text-white">
              <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <h3 class="mt-4 text-lg font-medium text-gray-900">灵活套餐</h3>
            <p class="mt-2 text-base text-gray-500">
              多种流量套餐可选，按需购买，性价比高
            </p>
          </div>

          <div class="bg-white rounded-lg shadow-lg p-6">
            <div class="flex items-center justify-center h-12 w-12 rounded-md bg-indigo-500 text-white">
              <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
              </svg>
            </div>
            <h3 class="mt-4 text-lg font-medium text-gray-900">推荐返利</h3>
            <p class="mt-2 text-base text-gray-500">
              邀请好友注册购买，获得丰厚返利奖励
            </p>
          </div>

          <div class="bg-white rounded-lg shadow-lg p-6">
            <div class="flex items-center justify-center h-12 w-12 rounded-md bg-indigo-500 text-white">
              <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
              </svg>
            </div>
            <h3 class="mt-4 text-lg font-medium text-gray-900">简单易用</h3>
            <p class="mt-2 text-base text-gray-500">
              一键订阅，支持 Clash 等主流客户端
            </p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
// Home page
</script>
