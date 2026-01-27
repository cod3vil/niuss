<template>
  <div class="stats">
    <a-card title="数据统计">
      <template #extra>
        <a-space>
          <a-range-picker
            v-model:value="dateRange"
            format="YYYY-MM-DD"
            @change="handleDateChange"
          />
          <a-button type="primary" @click="handleRefresh">
            <ReloadOutlined />
            刷新
          </a-button>
        </a-space>
      </template>

      <a-tabs v-model:activeKey="activeTab">
        <a-tab-pane key="revenue" tab="收入统计">
          <a-card title="收入趋势图" :loading="statsStore.loading">
            <div v-if="revenueChartData.length > 0" class="chart-container">
              <div class="chart-header">
                <div class="chart-legend">
                  <span class="legend-item">
                    <span class="legend-color" style="background: #1890ff;"></span>
                    收入金额
                  </span>
                  <span class="legend-item">
                    <span class="legend-color" style="background: #52c41a;"></span>
                    订单数量
                  </span>
                </div>
              </div>
              
              <div class="chart-bars">
                <div v-for="item in revenueChartData" :key="item.date" class="chart-row">
                  <div class="chart-date">{{ formatDate(item.date) }}</div>
                  <div class="chart-data">
                    <div class="bar-group">
                      <div class="bar-label">收入</div>
                      <div class="bar-wrapper">
                        <div 
                          class="bar-fill revenue" 
                          :style="{ width: `${(item.revenue / maxRevenue) * 100}%` }"
                        >
                          {{ item.revenue }}
                        </div>
                      </div>
                    </div>
                    <div class="bar-group">
                      <div class="bar-label">订单</div>
                      <div class="bar-wrapper">
                        <div 
                          class="bar-fill orders" 
                          :style="{ width: `${(item.order_count / maxOrders) * 100}%` }"
                        >
                          {{ item.order_count }}
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <a-divider />

              <a-row :gutter="16">
                <a-col :span="8">
                  <a-statistic
                    title="总收入"
                    :value="totalRevenue"
                    suffix="金币"
                  />
                </a-col>
                <a-col :span="8">
                  <a-statistic
                    title="总订单数"
                    :value="totalOrders"
                  />
                </a-col>
                <a-col :span="8">
                  <a-statistic
                    title="平均订单金额"
                    :value="avgOrderAmount"
                    :precision="2"
                    suffix="金币"
                  />
                </a-col>
              </a-row>
            </div>
            <a-empty v-else description="暂无数据" />
          </a-card>
        </a-tab-pane>

        <a-tab-pane key="traffic" tab="流量统计">
          <a-card title="流量趋势图" :loading="statsStore.loading">
            <div v-if="trafficChartData.length > 0" class="chart-container">
              <div class="chart-header">
                <div class="chart-legend">
                  <span class="legend-item">
                    <span class="legend-color" style="background: #ff4d4f;"></span>
                    上传流量
                  </span>
                  <span class="legend-item">
                    <span class="legend-color" style="background: #52c41a;"></span>
                    下载流量
                  </span>
                  <span class="legend-item">
                    <span class="legend-color" style="background: #1890ff;"></span>
                    总流量
                  </span>
                </div>
              </div>
              
              <div class="chart-bars">
                <div v-for="item in trafficChartData" :key="item.date" class="chart-row">
                  <div class="chart-date">{{ formatDate(item.date) }}</div>
                  <div class="chart-data">
                    <div class="bar-group">
                      <div class="bar-label">上传</div>
                      <div class="bar-wrapper">
                        <div 
                          class="bar-fill upload" 
                          :style="{ width: `${(item.upload / maxTraffic) * 100}%` }"
                        >
                          {{ formatTraffic(item.upload) }} GB
                        </div>
                      </div>
                    </div>
                    <div class="bar-group">
                      <div class="bar-label">下载</div>
                      <div class="bar-wrapper">
                        <div 
                          class="bar-fill download" 
                          :style="{ width: `${(item.download / maxTraffic) * 100}%` }"
                        >
                          {{ formatTraffic(item.download) }} GB
                        </div>
                      </div>
                    </div>
                    <div class="bar-group">
                      <div class="bar-label">总计</div>
                      <div class="bar-wrapper">
                        <div 
                          class="bar-fill total" 
                          :style="{ width: `${(item.total / maxTraffic) * 100}%` }"
                        >
                          {{ formatTraffic(item.total) }} GB
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <a-divider />

              <a-row :gutter="16">
                <a-col :span="8">
                  <a-statistic
                    title="总上传流量"
                    :value="formatTraffic(totalUpload)"
                    suffix="GB"
                    :precision="2"
                  />
                </a-col>
                <a-col :span="8">
                  <a-statistic
                    title="总下载流量"
                    :value="formatTraffic(totalDownload)"
                    suffix="GB"
                    :precision="2"
                  />
                </a-col>
                <a-col :span="8">
                  <a-statistic
                    title="总流量"
                    :value="formatTraffic(totalTraffic)"
                    suffix="GB"
                    :precision="2"
                  />
                </a-col>
              </a-row>
            </div>
            <a-empty v-else description="暂无数据" />
          </a-card>
        </a-tab-pane>

        <a-tab-pane key="nodes" tab="节点统计">
          <a-card title="节点流量统计" :loading="nodesStore.loading">
            <a-table
              :columns="nodeColumns"
              :data-source="nodesStore.nodes"
              :pagination="false"
              row-key="id"
            >
              <template #bodyCell="{ column, record }">
                <template v-if="column.key === 'upload'">
                  {{ formatTraffic(record.total_upload) }} GB
                </template>
                <template v-else-if="column.key === 'download'">
                  {{ formatTraffic(record.total_download) }} GB
                </template>
                <template v-else-if="column.key === 'total'">
                  {{ formatTraffic(record.total_upload + record.total_download) }} GB
                </template>
                <template v-else-if="column.key === 'users'">
                  {{ record.current_users }} / {{ record.max_users }}
                </template>
              </template>
            </a-table>
          </a-card>
        </a-tab-pane>
      </a-tabs>
    </a-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { ReloadOutlined } from '@ant-design/icons-vue'
import { useStatsStore } from '@/stores/stats'
import { useNodesStore } from '@/stores/nodes'
import type { Dayjs } from 'dayjs'

const statsStore = useStatsStore()
const nodesStore = useNodesStore()

const activeTab = ref('revenue')
const dateRange = ref<[Dayjs, Dayjs] | null>(null)

const nodeColumns = [
  { title: '节点名称', dataIndex: 'name', key: 'name' },
  { title: '上传流量', key: 'upload' },
  { title: '下载流量', key: 'download' },
  { title: '总流量', key: 'total' },
  { title: '当前用户', key: 'users' }
]

const revenueChartData = computed(() => statsStore.revenueStats)
const trafficChartData = computed(() => statsStore.trafficStats)

const maxRevenue = computed(() => {
  if (revenueChartData.value.length === 0) return 1
  return Math.max(...revenueChartData.value.map(item => item.revenue))
})

const maxOrders = computed(() => {
  if (revenueChartData.value.length === 0) return 1
  return Math.max(...revenueChartData.value.map(item => item.order_count))
})

const maxTraffic = computed(() => {
  if (trafficChartData.value.length === 0) return 1
  return Math.max(...trafficChartData.value.map(item => item.total))
})

const totalRevenue = computed(() => {
  return revenueChartData.value.reduce((sum, item) => sum + item.revenue, 0)
})

const totalOrders = computed(() => {
  return revenueChartData.value.reduce((sum, item) => sum + item.order_count, 0)
})

const avgOrderAmount = computed(() => {
  if (totalOrders.value === 0) return 0
  return totalRevenue.value / totalOrders.value
})

const totalUpload = computed(() => {
  return trafficChartData.value.reduce((sum, item) => sum + item.upload, 0)
})

const totalDownload = computed(() => {
  return trafficChartData.value.reduce((sum, item) => sum + item.download, 0)
})

const totalTraffic = computed(() => {
  return totalUpload.value + totalDownload.value
})

const formatTraffic = (bytes: number) => {
  return (bytes / (1024 * 1024 * 1024)).toFixed(2)
}

const formatDate = (dateStr: string) => {
  const date = new Date(dateStr)
  return `${date.getMonth() + 1}/${date.getDate()}`
}

const handleDateChange = () => {
  handleRefresh()
}

const handleRefresh = async () => {
  let startDate: string | undefined
  let endDate: string | undefined
  
  if (dateRange.value) {
    startDate = dateRange.value[0].format('YYYY-MM-DD')
    endDate = dateRange.value[1].format('YYYY-MM-DD')
  }
  
  await Promise.all([
    statsStore.fetchRevenueStats(startDate, endDate),
    statsStore.fetchTrafficStats(startDate, endDate),
    nodesStore.fetchNodes()
  ])
}

onMounted(() => {
  handleRefresh()
})
</script>

<style scoped>
.stats {
  padding: 0;
}

.chart-container {
  padding: 16px 0;
}

.chart-header {
  margin-bottom: 24px;
}

.chart-legend {
  display: flex;
  gap: 24px;
  justify-content: center;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
}

.legend-color {
  width: 16px;
  height: 16px;
  border-radius: 2px;
}

.chart-bars {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.chart-row {
  display: flex;
  gap: 16px;
}

.chart-date {
  width: 80px;
  font-weight: bold;
  color: #666;
  display: flex;
  align-items: center;
}

.chart-data {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.bar-group {
  display: flex;
  align-items: center;
  gap: 12px;
}

.bar-label {
  width: 50px;
  font-size: 12px;
  color: #666;
}

.bar-wrapper {
  flex: 1;
  height: 28px;
  background: #f0f0f0;
  border-radius: 4px;
  overflow: hidden;
}

.bar-fill {
  height: 100%;
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

.bar-fill.revenue {
  background: linear-gradient(90deg, #1890ff, #40a9ff);
}

.bar-fill.orders {
  background: linear-gradient(90deg, #52c41a, #73d13d);
}

.bar-fill.upload {
  background: linear-gradient(90deg, #ff4d4f, #ff7875);
}

.bar-fill.download {
  background: linear-gradient(90deg, #52c41a, #73d13d);
}

.bar-fill.total {
  background: linear-gradient(90deg, #1890ff, #40a9ff);
}
</style>
