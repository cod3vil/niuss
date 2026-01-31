<template>
  <a-layout style="min-height: 100vh">
    <a-layout-sider v-model:collapsed="collapsed" collapsible>
      <div class="logo">
        <h2 v-if="!collapsed">VPN 管理</h2>
        <h2 v-else>VPN</h2>
      </div>
      <a-menu
        v-model:selectedKeys="selectedKeys"
        theme="dark"
        mode="inline"
        @click="handleMenuClick"
      >
        <a-menu-item key="dashboard">
          <DashboardOutlined />
          <span>仪表板</span>
        </a-menu-item>
        <a-menu-item key="nodes">
          <CloudServerOutlined />
          <span>节点管理</span>
        </a-menu-item>
        <a-menu-item key="users">
          <UserOutlined />
          <span>用户管理</span>
        </a-menu-item>
        <a-menu-item key="orders">
          <ShoppingOutlined />
          <span>订单管理</span>
        </a-menu-item>
        <a-menu-item key="stats">
          <BarChartOutlined />
          <span>数据统计</span>
        </a-menu-item>
        <a-menu-item key="clash">
          <ApiOutlined />
          <span>Clash 配置</span>
        </a-menu-item>
        <a-menu-item key="access-logs">
          <FileTextOutlined />
          <span>访问日志</span>
        </a-menu-item>
      </a-menu>
    </a-layout-sider>
    
    <a-layout>
      <a-layout-header style="background: #fff; padding: 0 24px; display: flex; justify-content: space-between; align-items: center;">
        <a-breadcrumb>
          <a-breadcrumb-item>管理后台</a-breadcrumb-item>
          <a-breadcrumb-item v-if="currentRoute">{{ currentRoute.meta?.title }}</a-breadcrumb-item>
        </a-breadcrumb>
        
        <a-dropdown>
          <a class="ant-dropdown-link" @click.prevent>
            <UserOutlined />
            {{ authStore.user?.email }}
            <DownOutlined />
          </a>
          <template #overlay>
            <a-menu>
              <a-menu-item key="logout" @click="handleLogout">
                <LogoutOutlined />
                退出登录
              </a-menu-item>
            </a-menu>
          </template>
        </a-dropdown>
      </a-layout-header>
      
      <a-layout-content style="margin: 24px 16px; padding: 24px; background: #fff; min-height: 280px;">
        <router-view />
      </a-layout-content>
      
      <a-layout-footer style="text-align: center">
        VPN 管理后台 ©2024
      </a-layout-footer>
    </a-layout>
  </a-layout>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import {
  DashboardOutlined,
  CloudServerOutlined,
  UserOutlined,
  ShoppingOutlined,
  BarChartOutlined,
  ApiOutlined,
  FileTextOutlined,
  DownOutlined,
  LogoutOutlined
} from '@ant-design/icons-vue'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const collapsed = ref(false)
const selectedKeys = ref<string[]>([])

const currentRoute = computed(() => route)

// Update selected menu based on current route
watch(() => route.name, (newName) => {
  if (newName) {
    selectedKeys.value = [newName.toString().toLowerCase()]
  }
}, { immediate: true })

const handleMenuClick = ({ key }: { key: string }) => {
  router.push(`/admin/${key}`)
}

const handleLogout = () => {
  authStore.logout()
  router.push('/admin/login')
}
</script>

<style scoped>
.logo {
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: bold;
}

.logo h2 {
  color: #fff;
  margin: 0;
}

.ant-dropdown-link {
  color: rgba(0, 0, 0, 0.85);
  cursor: pointer;
}

.ant-dropdown-link:hover {
  color: #1890ff;
}
</style>
