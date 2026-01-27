<template>
  <div class="nodes">
    <a-card title="节点管理">
      <template #extra>
        <a-button type="primary" @click="showCreateModal">
          <PlusOutlined />
          添加节点
        </a-button>
      </template>

      <a-table
        :columns="columns"
        :data-source="nodesStore.nodes"
        :loading="nodesStore.loading"
        :pagination="{ pageSize: 10 }"
        row-key="id"
      >
        <template #bodyCell="{ column, record }">
          <template v-if="column.key === 'status'">
            <a-tag :color="getStatusColor(record.status)">
              {{ getStatusText(record.status) }}
            </a-tag>
          </template>
          
          <template v-else-if="column.key === 'protocol'">
            <a-tag color="blue">{{ record.protocol.toUpperCase() }}</a-tag>
          </template>
          
          <template v-else-if="column.key === 'traffic'">
            {{ formatTraffic(record.total_upload + record.total_download) }}
          </template>
          
          <template v-else-if="column.key === 'users'">
            {{ record.current_users }} / {{ record.max_users }}
          </template>
          
          <template v-else-if="column.key === 'last_heartbeat'">
            {{ record.last_heartbeat ? formatTime(record.last_heartbeat) : '从未' }}
          </template>
          
          <template v-else-if="column.key === 'action'">
            <a-space>
              <a-button size="small" @click="showEditModal(record)">
                编辑
              </a-button>
              <a-popconfirm
                title="确定要删除这个节点吗？"
                ok-text="确定"
                cancel-text="取消"
                @confirm="handleDelete(record.id)"
              >
                <a-button size="small" danger>
                  删除
                </a-button>
              </a-popconfirm>
            </a-space>
          </template>
        </template>
      </a-table>
    </a-card>

    <!-- Create/Edit Modal -->
    <a-modal
      v-model:open="modalVisible"
      :title="isEdit ? '编辑节点' : '添加节点'"
      :confirm-loading="nodesStore.loading"
      @ok="handleSubmit"
      width="600px"
    >
      <a-form
        :model="formState"
        :label-col="{ span: 6 }"
        :wrapper-col="{ span: 18 }"
      >
        <a-form-item label="节点名称" required>
          <a-input v-model:value="formState.name" placeholder="例如：香港节点1" />
        </a-form-item>

        <a-form-item label="主机地址" required>
          <a-input v-model:value="formState.host" placeholder="例如：hk1.example.com" />
        </a-form-item>

        <a-form-item label="端口" required>
          <a-input-number v-model:value="formState.port" :min="1" :max="65535" style="width: 100%" />
        </a-form-item>

        <a-form-item label="协议" required>
          <a-select v-model:value="formState.protocol">
            <a-select-option value="shadowsocks">Shadowsocks</a-select-option>
            <a-select-option value="vmess">VMess</a-select-option>
            <a-select-option value="trojan">Trojan</a-select-option>
            <a-select-option value="hysteria2">Hysteria2</a-select-option>
            <a-select-option value="vless">VLESS-Reality</a-select-option>
          </a-select>
        </a-form-item>

        <a-form-item label="节点密钥" required>
          <a-input-password v-model:value="formState.secret" placeholder="用于 Node Agent 认证" />
        </a-form-item>

        <a-form-item label="最大用户数">
          <a-input-number v-model:value="formState.max_users" :min="1" style="width: 100%" />
        </a-form-item>

        <a-form-item label="协议配置">
          <a-textarea
            v-model:value="configJson"
            :rows="6"
            placeholder='{"method": "aes-256-gcm", "password": "your-password"}'
          />
          <div style="color: #999; font-size: 12px; margin-top: 4px;">
            JSON 格式的协议特定配置
          </div>
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { PlusOutlined } from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { useNodesStore } from '@/stores/nodes'
import type { Node } from '@/stores/nodes'

const nodesStore = useNodesStore()

const columns = [
  { title: 'ID', dataIndex: 'id', key: 'id', width: 60 },
  { title: '名称', dataIndex: 'name', key: 'name' },
  { title: '地址', dataIndex: 'host', key: 'host' },
  { title: '端口', dataIndex: 'port', key: 'port', width: 80 },
  { title: '协议', key: 'protocol', width: 120 },
  { title: '状态', key: 'status', width: 100 },
  { title: '用户数', key: 'users', width: 100 },
  { title: '总流量', key: 'traffic', width: 120 },
  { title: '最后心跳', key: 'last_heartbeat', width: 150 },
  { title: '操作', key: 'action', width: 150 }
]

const modalVisible = ref(false)
const isEdit = ref(false)
const editingId = ref<number | null>(null)
const configJson = ref('')

const formState = reactive({
  name: '',
  host: '',
  port: 443,
  protocol: 'vless',
  secret: '',
  max_users: 1000,
  config: {}
})

const getStatusColor = (status: string) => {
  const colors: Record<string, string> = {
    online: 'green',
    offline: 'red',
    maintenance: 'orange'
  }
  return colors[status] || 'default'
}

const getStatusText = (status: string) => {
  const texts: Record<string, string> = {
    online: '在线',
    offline: '离线',
    maintenance: '维护中'
  }
  return texts[status] || status
}

const formatTraffic = (bytes: number) => {
  const gb = bytes / (1024 * 1024 * 1024)
  return gb.toFixed(2) + ' GB'
}

const formatTime = (time: string) => {
  const date = new Date(time)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const minutes = Math.floor(diff / 60000)
  
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes}分钟前`
  if (minutes < 1440) return `${Math.floor(minutes / 60)}小时前`
  return `${Math.floor(minutes / 1440)}天前`
}

const showCreateModal = () => {
  isEdit.value = false
  editingId.value = null
  formState.name = ''
  formState.host = ''
  formState.port = 443
  formState.protocol = 'vless'
  formState.secret = ''
  formState.max_users = 1000
  configJson.value = '{}'
  modalVisible.value = true
}

const showEditModal = (node: Node) => {
  isEdit.value = true
  editingId.value = node.id
  formState.name = node.name
  formState.host = node.host
  formState.port = node.port
  formState.protocol = node.protocol
  formState.secret = node.secret
  formState.max_users = node.max_users
  configJson.value = JSON.stringify(node.config, null, 2)
  modalVisible.value = true
}

const handleSubmit = async () => {
  try {
    formState.config = JSON.parse(configJson.value)
  } catch (e) {
    message.error('配置 JSON 格式错误')
    return
  }

  let success = false
  if (isEdit.value && editingId.value) {
    success = await nodesStore.updateNode(editingId.value, formState)
  } else {
    success = await nodesStore.createNode(formState)
  }

  if (success) {
    message.success(isEdit.value ? '节点更新成功' : '节点创建成功')
    modalVisible.value = false
  } else {
    message.error(nodesStore.error || '操作失败')
  }
}

const handleDelete = async (id: number) => {
  const success = await nodesStore.deleteNode(id)
  if (success) {
    message.success('节点删除成功')
  } else {
    message.error(nodesStore.error || '删除失败')
  }
}

onMounted(() => {
  nodesStore.fetchNodes()
})
</script>

<style scoped>
.nodes {
  padding: 0;
}
</style>
