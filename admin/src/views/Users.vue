<template>
  <div class="users">
    <a-card title="用户管理">
      <a-table
        :columns="columns"
        :data-source="usersStore.users"
        :loading="usersStore.loading"
        :pagination="{ pageSize: 10 }"
        row-key="id"
      >
        <template #bodyCell="{ column, record }">
          <template v-if="column.key === 'status'">
            <a-tag :color="record.status === 'active' ? 'green' : 'red'">
              {{ record.status === 'active' ? '正常' : '已禁用' }}
            </a-tag>
          </template>
          
          <template v-else-if="column.key === 'coin_balance'">
            <a-tag color="gold">{{ record.coin_balance }}</a-tag>
          </template>
          
          <template v-else-if="column.key === 'traffic'">
            <a-progress
              :percent="getTrafficPercent(record)"
              :status="getTrafficPercent(record) >= 90 ? 'exception' : 'normal'"
              size="small"
            />
            <div style="font-size: 12px; color: #666; margin-top: 4px;">
              {{ formatTraffic(record.traffic_used) }} / {{ formatTraffic(record.traffic_quota) }}
            </div>
          </template>
          
          <template v-else-if="column.key === 'created_at'">
            {{ formatDate(record.created_at) }}
          </template>
          
          <template v-else-if="column.key === 'action'">
            <a-space>
              <a-button size="small" @click="showUserDetail(record)">
                详情
              </a-button>
              <a-dropdown>
                <template #overlay>
                  <a-menu>
                    <a-menu-item key="status" @click="showStatusModal(record)">
                      {{ record.status === 'active' ? '禁用用户' : '启用用户' }}
                    </a-menu-item>
                    <a-menu-item key="balance" @click="showBalanceModal(record)">
                      调整金币
                    </a-menu-item>
                    <a-menu-item key="traffic" @click="showTrafficModal(record)">
                      调整流量
                    </a-menu-item>
                  </a-menu>
                </template>
                <a-button size="small">
                  操作 <DownOutlined />
                </a-button>
              </a-dropdown>
            </a-space>
          </template>
        </template>
      </a-table>
    </a-card>

    <!-- User Detail Modal -->
    <a-modal
      v-model:open="detailModalVisible"
      title="用户详情"
      :footer="null"
      width="600px"
    >
      <a-descriptions v-if="selectedUser" bordered :column="1">
        <a-descriptions-item label="用户 ID">{{ selectedUser.id }}</a-descriptions-item>
        <a-descriptions-item label="邮箱">{{ selectedUser.email }}</a-descriptions-item>
        <a-descriptions-item label="金币余额">{{ selectedUser.coin_balance }}</a-descriptions-item>
        <a-descriptions-item label="流量配额">{{ formatTraffic(selectedUser.traffic_quota) }}</a-descriptions-item>
        <a-descriptions-item label="已用流量">{{ formatTraffic(selectedUser.traffic_used) }}</a-descriptions-item>
        <a-descriptions-item label="推荐码">{{ selectedUser.referral_code || '无' }}</a-descriptions-item>
        <a-descriptions-item label="状态">
          <a-tag :color="selectedUser.status === 'active' ? 'green' : 'red'">
            {{ selectedUser.status === 'active' ? '正常' : '已禁用' }}
          </a-tag>
        </a-descriptions-item>
        <a-descriptions-item label="注册时间">{{ formatDate(selectedUser.created_at) }}</a-descriptions-item>
        <a-descriptions-item label="更新时间">{{ formatDate(selectedUser.updated_at) }}</a-descriptions-item>
      </a-descriptions>
    </a-modal>

    <!-- Status Modal -->
    <a-modal
      v-model:open="statusModalVisible"
      :title="selectedUser?.status === 'active' ? '禁用用户' : '启用用户'"
      :confirm-loading="usersStore.loading"
      @ok="handleStatusChange"
    >
      <p>确定要{{ selectedUser?.status === 'active' ? '禁用' : '启用' }}用户 {{ selectedUser?.email }} 吗？</p>
    </a-modal>

    <!-- Balance Modal -->
    <a-modal
      v-model:open="balanceModalVisible"
      title="调整金币余额"
      :confirm-loading="usersStore.loading"
      @ok="handleBalanceChange"
    >
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item label="当前余额">
          <a-input :value="selectedUser?.coin_balance" disabled />
        </a-form-item>
        <a-form-item label="调整金额" required>
          <a-input-number
            v-model:value="balanceAmount"
            :min="-999999"
            :max="999999"
            style="width: 100%"
            placeholder="正数为增加，负数为减少"
          />
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- Traffic Modal -->
    <a-modal
      v-model:open="trafficModalVisible"
      title="调整流量配额"
      :confirm-loading="usersStore.loading"
      @ok="handleTrafficChange"
    >
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item label="当前配额">
          <a-input :value="formatTraffic(selectedUser?.traffic_quota || 0)" disabled />
        </a-form-item>
        <a-form-item label="调整流量 (GB)" required>
          <a-input-number
            v-model:value="trafficAmount"
            :min="-999999"
            :max="999999"
            :precision="2"
            style="width: 100%"
            placeholder="正数为增加，负数为减少"
          />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { DownOutlined } from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { useUsersStore } from '@/stores/users'
import type { User } from '@/stores/users'

const usersStore = useUsersStore()

const columns = [
  { title: 'ID', dataIndex: 'id', key: 'id', width: 60 },
  { title: '邮箱', dataIndex: 'email', key: 'email' },
  { title: '金币', key: 'coin_balance', width: 100 },
  { title: '流量使用', key: 'traffic', width: 200 },
  { title: '状态', key: 'status', width: 100 },
  { title: '注册时间', key: 'created_at', width: 150 },
  { title: '操作', key: 'action', width: 180 }
]

const detailModalVisible = ref(false)
const statusModalVisible = ref(false)
const balanceModalVisible = ref(false)
const trafficModalVisible = ref(false)
const selectedUser = ref<User | null>(null)
const balanceAmount = ref(0)
const trafficAmount = ref(0)

const getTrafficPercent = (user: User) => {
  if (user.traffic_quota === 0) return 0
  return Math.round((user.traffic_used / user.traffic_quota) * 100)
}

const formatTraffic = (bytes: number) => {
  const gb = bytes / (1024 * 1024 * 1024)
  return gb.toFixed(2) + ' GB'
}

const formatDate = (dateStr: string) => {
  const date = new Date(dateStr)
  return date.toLocaleString('zh-CN')
}

const showUserDetail = (user: User) => {
  selectedUser.value = user
  detailModalVisible.value = true
}

const showStatusModal = (user: User) => {
  selectedUser.value = user
  statusModalVisible.value = true
}

const showBalanceModal = (user: User) => {
  selectedUser.value = user
  balanceAmount.value = 0
  balanceModalVisible.value = true
}

const showTrafficModal = (user: User) => {
  selectedUser.value = user
  trafficAmount.value = 0
  trafficModalVisible.value = true
}

const handleStatusChange = async () => {
  if (!selectedUser.value) return
  
  const newStatus = selectedUser.value.status === 'active' ? 'disabled' : 'active'
  const success = await usersStore.updateUserStatus(selectedUser.value.id, newStatus)
  
  if (success) {
    message.success('用户状态更新成功')
    statusModalVisible.value = false
  } else {
    message.error(usersStore.error || '操作失败')
  }
}

const handleBalanceChange = async () => {
  if (!selectedUser.value || balanceAmount.value === 0) {
    message.warning('请输入调整金额')
    return
  }
  
  const success = await usersStore.updateUserBalance(selectedUser.value.id, balanceAmount.value)
  
  if (success) {
    message.success('金币余额调整成功')
    balanceModalVisible.value = false
  } else {
    message.error(usersStore.error || '操作失败')
  }
}

const handleTrafficChange = async () => {
  if (!selectedUser.value || trafficAmount.value === 0) {
    message.warning('请输入调整流量')
    return
  }
  
  const trafficBytes = trafficAmount.value * 1024 * 1024 * 1024
  const success = await usersStore.updateUserTraffic(selectedUser.value.id, trafficBytes)
  
  if (success) {
    message.success('流量配额调整成功')
    trafficModalVisible.value = false
  } else {
    message.error(usersStore.error || '操作失败')
  }
}

onMounted(() => {
  usersStore.fetchUsers()
})
</script>

<style scoped>
.users {
  padding: 0;
}
</style>
