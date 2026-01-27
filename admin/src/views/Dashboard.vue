<template>
  <div class="dashboard">
    <a-row :gutter="16">
      <a-col :xs="24" :sm="12" :lg="6">
        <a-card>
          <a-statistic
            title="用户总数"
            :value="dashboardStore.stats.total_users"
            :prefix="h(UserOutlined)"
          />
        </a-card>
      </a-col>
      
      <a-col :xs="24" :sm="12" :lg="6">
        <a-card>
          <a-statistic
            title="活跃用户"
            :value="dashboardStore.stats.active_users"
            :prefix="h(TeamOutlined)"
            :value-style="{ color: '#3f8600' }"
          />
        </a-card>
      </a-col>
      
      <a-col :xs="24" :sm="12" :lg="6">
        <a-card>
          <a-statistic
            title="总流量 (GB)"
            :value="formatTraffic(dashboardStore.stats.total_traffic)"
            :prefix="h(CloudDownloadOutlined)"
            :precision="2"
          />
        </a-card>
      </a-col>
      
      <a-col :xs="24" :sm="12" :lg="6">
        <a-card>
          <a-statistic
            title="总收入 (金币)"
            :value="dashboardStore.stats.total_revenue"
            :prefix="h(DollarOutlined)"
            :value-style="{ color: '#cf1322' }"
          />
        </a-card>
      </a-col>
    </a-row>

    <a-row :gutter="16" style="margin-top: 16px;">
      <a-col :xs="24" :lg="12">
        <a-card title="收入趋势" :loading="statsStore.loading">
          <div v-if="revenueChartData.length > 0" class="chart-container">
            <div v-for="item in revenueChartData" :key="item.date" class="chart-bar">
              <div class="bar-label">{{ formatDate(item.date) }}</div>
              <div class="bar-wrapper">
                <div 
                  class="bar-fill" 
                  :style="{ width: `${(item.revenue / maxRevenue) * 100}%` }"
                >
                  {{ item.revenue }}
                </div>
              </div>
            </div>
          </div>
          <a-empty v-else description="暂无数据" />
        </a-card>
      </a-col>
      
      <a-col :xs="24" :lg="12">
        <a-card title="流量趋势" :loading="statsStore.loading">
          <div v-if="trafficChartData.length > 0" class="chart-container">
            <div v-for="item in trafficChartData" :key="item.date" class="chart-bar">
              <div class="bar-label">{{ formatDate(item.date) }}</div>
              <div class="bar-wrapper">
                <div 
                  class="bar-fill traffic" 
                  :style="{ width: `${(item.total / maxTraffic) * 100}%` }"
                >
                  {{ formatTraffic(item.total) }} GB
                </div>
              </div>
            </div>
          </div>
          <a-empty v-else description="暂无数据" />
        </a-card>
      </a-col>
    </a-row>

    <a-row :gutter="16" style="margin-top: 16px;">
      <a-col :span="24">
        <a-card title="快速操作">
          <a-space>
            <a-button type="primary" @click="router.push('/admin/nodes')">
              <CloudServerOutlined />
              管理节点
            </a-button>
            <a-button @click="router.push('/admin/users')">
              <UserOutlined />
              管理用户
            </a-button>
            <a-button @click="router.push('/admin/orders')">
              <ShoppingOutlined />
              查看订单
            </a-button>
            <a-button @click="refreshData">
              <ReloadOutlined />
              刷新数据
            </a-button>
          </a-space>
        </a-card>
      </a-col>
    </a-row>
  </div>
</template>

<script setup lang="ts">
import { h, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import {
  UserOutlined,
  TeamOutlined,
  CloudDownloadOutlined,
  DollarOutlined,
  CloudServerOutlined,
  ShoppingOutlined,
  ReloadOutlined
} from '@ant-design/icons-vue'
import { useDashboardStore } from '@/stores/dashboard'
import { useStatsStore } from '@/stores/stats'

const router = useRouter()
const dashboardStore = useDashboardStore()
const statsStore = useStatsStore()

const revenueChartData = computed(() => statsStore.revenueStats.slice(-7))
const trafficChartData = computed(() => statsStore.trafficStats.slice(-7))

const maxRevenue = computed(() => {
  if (revenueChartData.value.length === 0) return 1
  return Math.max(...revenueChartData.value.map(item => item.revenue))
})

const maxTraffic = computed(() => {
  if (trafficChartData.value.length === 0) return 1
  return Math.max(...trafficChartData.value.map(item => item.total))
})

const formatTraffic = (bytes: number) => {
  return (bytes / (1024 * 1024 * 1024)).toFixed(2)
}

const formatDate = (dateStr: string) => {
  const date = new Date(dateStr)
  return `${date.getMonth() + 1}/${date.getDate()}`
}

const refreshData = async () => {
  await Promise.all([
    dashboardStore.fetchStats(),
    statsStore.fetchRevenueStats(),
    statsStore.fetchTrafficStats()
  ])
}

onMounted(() => {
  refreshData()
})
</script>

<style scoped>
.dashboard {
  padding: 0;
}

.chart-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.chart-bar {
  display: flex;
  align-items: center;
  gap: 12px;
}

.bar-label {
  width: 60px;
  font-size: 12px;
  color: #666;
}

.bar-wrapper {
  flex: 1;
  height: 32px;
  background: #f0f0f0;
  border-radius: 4px;
  overflow: hidden;
}

.bar-fill {
  height: 100%;
  background: linear-gradient(90deg, #1890ff, #40a9ff);
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding-right: 8px;
  color: white;
  font-size: 12px;
  font-weight: bold;
  min-width: 60px;
  transition: width 0.3s ease;
}

.bar-fill.traffic {
  background: linear-gradient(90deg, #52c41a, #73d13d);
}
</style>
