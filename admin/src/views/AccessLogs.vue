<template>
  <div class="access-logs">
    <a-card title="访问日志">
      <template #extra>
        <a-space>
          <a-button @click="showFilterModal">
            <FilterOutlined />
            筛选
          </a-button>
          <a-button @click="clearFilters">
            <ClearOutlined />
            清除筛选
          </a-button>
          <a-button @click="handleRefresh">
            <ReloadOutlined />
            刷新
          </a-button>
        </a-space>
      </template>

      <a-table
        :columns="columns"
        :data-source="accessLogsStore.logs"
        :loading="accessLogsStore.loading"
        :pagination="{
          current: accessLogsStore.page,
          pageSize: accessLogsStore.pageSize,
          total: accessLogsStore.total,
          showTotal: (total: number) => `共 ${total} 条记录`,
          onChange: handlePageChange
        }"
        row-key="id"
      >
        <template #bodyCell="{ column, record }">
          <template v-if="column.key === 'access_timestamp'">
            {{ formatTimestamp(record.access_timestamp) }}
          </template>
          
          <template v-else-if="column.key === 'user_email'">
            <a-typography-text>{{ record.user_email }}</a-typography-text>
            <div style="font-size: 12px; color: #999;">ID: {{ record.user_id }}</div>
          </template>
          
          <template v-else-if="column.key === 'ip_address'">
            <a-typography-text copyable>{{ record.ip_address }}</a-typography-text>
          </template>
          
          <template v-else-if="column.key === 'user_agent'">
            <a-tooltip :title="record.user_agent || 'N/A'">
              <div class="user-agent-cell">
                {{ record.user_agent || 'N/A' }}
              </div>
            </a-tooltip>
          </template>
          
          <template v-else-if="column.key === 'response_status'">
            <a-tag :color="statusClass(record.response_status)">
              {{ getStatusText(record.response_status) }}
            </a-tag>
          </template>
        </template>
      </a-table>
    </a-card>

    <!-- Filter Modal -->
    <a-modal
      v-model:open="filterModalVisible"
      title="筛选访问日志"
      @ok="applyFilters"
    >
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item label="用户搜索">
          <a-input
            v-model:value="filterForm.userSearch"
            placeholder="输入用户 ID 或邮箱"
            allow-clear
          />
        </a-form-item>

        <a-form-item label="开始时间">
          <a-date-picker
            v-model:value="filterForm.startDate"
            show-time
            style="width: 100%"
            format="YYYY-MM-DD HH:mm:ss"
            placeholder="选择开始时间"
          />
        </a-form-item>

        <a-form-item label="结束时间">
          <a-date-picker
            v-model:value="filterForm.endDate"
            show-time
            style="width: 100%"
            format="YYYY-MM-DD HH:mm:ss"
            placeholder="选择结束时间"
          />
        </a-form-item>

        <a-form-item label="访问状态">
          <a-select v-model:value="filterForm.status" allow-clear placeholder="全部">
            <a-select-option value="">全部</a-select-option>
            <a-select-option value="success">成功</a-select-option>
            <a-select-option value="failed">失败</a-select-option>
            <a-select-option value="quota_exceeded">流量超限</a-select-option>
            <a-select-option value="expired">已过期</a-select-option>
            <a-select-option value="disabled">已禁用</a-select-option>
          </a-select>
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { FilterOutlined, ClearOutlined, ReloadOutlined } from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { useAccessLogsStore } from '@/stores/accessLogs'
import type { Dayjs } from 'dayjs'

const accessLogsStore = useAccessLogsStore()

const columns = [
  { title: '访问时间', key: 'access_timestamp', width: 180 },
  { title: '用户', key: 'user_email', width: 200 },
  { title: 'IP 地址', key: 'ip_address', width: 150 },
  { title: 'User Agent', key: 'user_agent', width: 300 },
  { title: '状态', key: 'response_status', width: 120 }
]

const filterModalVisible = ref(false)

const filterForm = reactive<{
  userSearch: string
  startDate: Dayjs | null
  endDate: Dayjs | null
  status: string
}>({
  userSearch: '',
  startDate: null,
  endDate: null,
  status: ''
})

const formatTimestamp = (timestamp: string): string => {
  const date = new Date(timestamp)
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  })
}

const statusClass = (status: string): string => {
  const classes: Record<string, string> = {
    success: 'green',
    failed: 'red',
    quota_exceeded: 'orange',
    expired: 'orange',
    disabled: 'red'
  }
  return classes[status] || 'default'
}

const getStatusText = (status: string): string => {
  const texts: Record<string, string> = {
    success: '成功',
    failed: '失败',
    quota_exceeded: '流量超限',
    expired: '已过期',
    disabled: '已禁用'
  }
  return texts[status] || status
}

const showFilterModal = () => {
  filterModalVisible.value = true
}

const loadLogs = async () => {
  try {
    const filters: any = {
      page: accessLogsStore.page,
      pageSize: accessLogsStore.pageSize
    }
    
    if (filterForm.userSearch) {
      filters.userSearch = filterForm.userSearch
    }
    
    if (filterForm.startDate) {
      filters.startDate = filterForm.startDate.toISOString()
    }
    
    if (filterForm.endDate) {
      filters.endDate = filterForm.endDate.toISOString()
    }
    
    if (filterForm.status) {
      filters.status = filterForm.status
    }
    
    await accessLogsStore.fetchAccessLogs(filters)
  } catch (error) {
    message.error('加载访问日志失败')
  }
}

const applyFilters = async () => {
  // Reset to page 1 when applying filters
  accessLogsStore.page = 1
  await loadLogs()
  filterModalVisible.value = false
  message.success('筛选完成')
}

const clearFilters = async () => {
  filterForm.userSearch = ''
  filterForm.startDate = null
  filterForm.endDate = null
  filterForm.status = ''
  accessLogsStore.page = 1
  await loadLogs()
  message.success('筛选已清除')
}

const handleRefresh = async () => {
  await loadLogs()
  message.success('刷新成功')
}

const handlePageChange = async (page: number, pageSize: number) => {
  accessLogsStore.page = page
  accessLogsStore.pageSize = pageSize
  await loadLogs()
}

onMounted(() => {
  loadLogs()
})
</script>

<style scoped>
.access-logs {
  padding: 0;
}

.user-agent-cell {
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
