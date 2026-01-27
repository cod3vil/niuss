<template>
  <div class="orders">
    <a-card title="订单管理">
      <template #extra>
        <a-space>
          <a-button @click="showFilterModal">
            <FilterOutlined />
            筛选
          </a-button>
          <a-button @click="handleRefresh">
            <ReloadOutlined />
            刷新
          </a-button>
        </a-space>
      </template>

      <a-table
        :columns="columns"
        :data-source="ordersStore.orders"
        :loading="ordersStore.loading"
        :pagination="{ pageSize: 10 }"
        row-key="id"
      >
        <template #bodyCell="{ column, record }">
          <template v-if="column.key === 'order_no'">
            <a-typography-text copyable>{{ record.order_no }}</a-typography-text>
          </template>
          
          <template v-else-if="column.key === 'status'">
            <a-tag :color="getStatusColor(record.status)">
              {{ getStatusText(record.status) }}
            </a-tag>
          </template>
          
          <template v-else-if="column.key === 'amount'">
            <a-tag color="gold">{{ record.amount }}</a-tag>
          </template>
          
          <template v-else-if="column.key === 'created_at'">
            {{ formatDate(record.created_at) }}
          </template>
          
          <template v-else-if="column.key === 'action'">
            <a-button size="small" @click="showOrderDetail(record)">
              查看详情
            </a-button>
          </template>
        </template>
      </a-table>
    </a-card>

    <!-- Filter Modal -->
    <a-modal
      v-model:open="filterModalVisible"
      title="筛选订单"
      @ok="handleFilter"
    >
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item label="订单状态">
          <a-select v-model:value="filterForm.status" allow-clear placeholder="全部">
            <a-select-option value="">全部</a-select-option>
            <a-select-option value="pending">待处理</a-select-option>
            <a-select-option value="completed">已完成</a-select-option>
            <a-select-option value="failed">失败</a-select-option>
          </a-select>
        </a-form-item>

        <a-form-item label="开始日期">
          <a-date-picker
            v-model:value="filterForm.start_date"
            style="width: 100%"
            format="YYYY-MM-DD"
          />
        </a-form-item>

        <a-form-item label="结束日期">
          <a-date-picker
            v-model:value="filterForm.end_date"
            style="width: 100%"
            format="YYYY-MM-DD"
          />
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- Order Detail Modal -->
    <a-modal
      v-model:open="detailModalVisible"
      title="订单详情"
      :footer="null"
      width="600px"
    >
      <a-descriptions v-if="selectedOrder" bordered :column="1">
        <a-descriptions-item label="订单号">
          <a-typography-text copyable>{{ selectedOrder.order_no }}</a-typography-text>
        </a-descriptions-item>
        <a-descriptions-item label="用户 ID">{{ selectedOrder.user_id }}</a-descriptions-item>
        <a-descriptions-item label="用户邮箱">{{ selectedOrder.user_email || '未知' }}</a-descriptions-item>
        <a-descriptions-item label="套餐 ID">{{ selectedOrder.package_id }}</a-descriptions-item>
        <a-descriptions-item label="套餐名称">{{ selectedOrder.package_name || '未知' }}</a-descriptions-item>
        <a-descriptions-item label="金额">
          <a-tag color="gold">{{ selectedOrder.amount }} 金币</a-tag>
        </a-descriptions-item>
        <a-descriptions-item label="状态">
          <a-tag :color="getStatusColor(selectedOrder.status)">
            {{ getStatusText(selectedOrder.status) }}
          </a-tag>
        </a-descriptions-item>
        <a-descriptions-item label="创建时间">{{ formatDate(selectedOrder.created_at) }}</a-descriptions-item>
        <a-descriptions-item label="完成时间">
          {{ selectedOrder.completed_at ? formatDate(selectedOrder.completed_at) : '未完成' }}
        </a-descriptions-item>
      </a-descriptions>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { FilterOutlined, ReloadOutlined } from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { useOrdersStore } from '@/stores/orders'
import type { Order } from '@/stores/orders'
import type { Dayjs } from 'dayjs'

const ordersStore = useOrdersStore()

const columns = [
  { title: 'ID', dataIndex: 'id', key: 'id', width: 60 },
  { title: '订单号', key: 'order_no', width: 200 },
  { title: '用户 ID', dataIndex: 'user_id', key: 'user_id', width: 80 },
  { title: '套餐 ID', dataIndex: 'package_id', key: 'package_id', width: 80 },
  { title: '金额', key: 'amount', width: 100 },
  { title: '状态', key: 'status', width: 100 },
  { title: '创建时间', key: 'created_at', width: 180 },
  { title: '操作', key: 'action', width: 120 }
]

const filterModalVisible = ref(false)
const detailModalVisible = ref(false)
const selectedOrder = ref<Order | null>(null)

const filterForm = reactive<{
  status: string
  start_date: Dayjs | null
  end_date: Dayjs | null
}>({
  status: '',
  start_date: null,
  end_date: null
})

const getStatusColor = (status: string) => {
  const colors: Record<string, string> = {
    pending: 'orange',
    completed: 'green',
    failed: 'red'
  }
  return colors[status] || 'default'
}

const getStatusText = (status: string) => {
  const texts: Record<string, string> = {
    pending: '待处理',
    completed: '已完成',
    failed: '失败'
  }
  return texts[status] || status
}

const formatDate = (dateStr: string) => {
  const date = new Date(dateStr)
  return date.toLocaleString('zh-CN')
}

const showFilterModal = () => {
  filterModalVisible.value = true
}

const handleFilter = async () => {
  const filters: any = {}
  
  if (filterForm.status) {
    filters.status = filterForm.status
  }
  
  if (filterForm.start_date) {
    filters.start_date = filterForm.start_date.format('YYYY-MM-DD')
  }
  
  if (filterForm.end_date) {
    filters.end_date = filterForm.end_date.format('YYYY-MM-DD')
  }
  
  await ordersStore.fetchOrders(filters)
  filterModalVisible.value = false
  message.success('筛选完成')
}

const handleRefresh = async () => {
  filterForm.status = ''
  filterForm.start_date = null
  filterForm.end_date = null
  await ordersStore.fetchOrders()
  message.success('刷新成功')
}

const showOrderDetail = async (order: Order) => {
  const detail = await ordersStore.getOrderById(order.id)
  if (detail) {
    selectedOrder.value = detail
    detailModalVisible.value = true
  } else {
    message.error('获取订单详情失败')
  }
}

onMounted(() => {
  ordersStore.fetchOrders()
})
</script>

<style scoped>
.orders {
  padding: 0;
}
</style>
