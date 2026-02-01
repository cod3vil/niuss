<template>
  <div class="clash-config">
    <h1>Clash 配置管理</h1>
    
    <a-alert
      message="代理管理已迁移"
      description="代理管理功能已迁移至节点管理页面。请在节点页面使用「包含在 Clash 中」开关来控制哪些节点出现在 Clash 配置中。"
      type="info"
      show-icon
      closable
      style="margin-bottom: 16px"
    />
    
    <a-tabs v-model:activeKey="activeTab" type="card">
      <!-- 代理组管理 -->
      <a-tab-pane key="groups" tab="代理组管理">
        <div class="tab-header">
          <a-space>
            <a-button type="primary" @click="showGroupDialog('create')">添加代理组</a-button>
            <a-button @click="loadGroups">刷新</a-button>
          </a-space>
        </div>
        
        <a-table :dataSource="groups" :columns="groupColumns" :loading="loading" rowKey="id">
          <template #bodyCell="{ column, record }">
            <template v-if="column.key === 'proxies'">
              <a-tag v-for="proxy in record.proxies" :key="proxy" style="margin: 2px">
                {{ proxy }}
              </a-tag>
            </template>
            <template v-else-if="column.key === 'is_active'">
              <a-tag :color="record.is_active ? 'green' : 'default'">
                {{ record.is_active ? '启用' : '禁用' }}
              </a-tag>
            </template>
            <template v-else-if="column.key === 'action'">
              <a-space>
                <a-button size="small" @click="showGroupDialog('edit', record)">编辑</a-button>
                <a-popconfirm title="确定删除?" @confirm="deleteGroup(record.id)">
                  <a-button size="small" danger>删除</a-button>
                </a-popconfirm>
              </a-space>
            </template>
          </template>
        </a-table>
      </a-tab-pane>

      <!-- 规则管理 -->
      <a-tab-pane key="rules" tab="规则管理">
        <div class="tab-header">
          <a-space>
            <a-button type="primary" @click="showRuleDialog('create')">添加规则</a-button>
            <a-button @click="loadRules">刷新</a-button>
          </a-space>
        </div>
        
        <a-table :dataSource="rules" :columns="ruleColumns" :loading="loading" rowKey="id">
          <template #bodyCell="{ column, record }">
            <template v-if="column.key === 'is_active'">
              <a-tag :color="record.is_active ? 'green' : 'default'">
                {{ record.is_active ? '启用' : '禁用' }}
              </a-tag>
            </template>
            <template v-else-if="column.key === 'action'">
              <a-space>
                <a-button size="small" @click="showRuleDialog('edit', record)">编辑</a-button>
                <a-popconfirm title="确定删除?" @confirm="deleteRule(record.id)">
                  <a-button size="small" danger>删除</a-button>
                </a-popconfirm>
              </a-space>
            </template>
          </template>
        </a-table>
      </a-tab-pane>

      <!-- 生成配置 -->
      <a-tab-pane key="generate" tab="生成配置">
        <div class="generate-section">
          <a-button type="primary" size="large" @click="generateConfig" :loading="loading">
            生成 Clash 配置
          </a-button>
          <a-textarea
            v-if="generatedConfig"
            v-model:value="generatedConfig"
            :rows="20"
            readonly
            style="margin-top: 20px"
          />
        </div>
      </a-tab-pane>
    </a-tabs>

    <!-- 代理组对话框 -->
    <a-modal v-model:open="groupDialogVisible" :title="dialogTitle" width="600px" @ok="saveGroup">
      <a-form :model="groupForm" :label-col="{ span: 6 }">
        <a-form-item label="名称">
          <a-input v-model:value="groupForm.name" />
        </a-form-item>
        <a-form-item label="类型">
          <a-select v-model:value="groupForm.type">
            <a-select-option value="select">手动选择</a-select-option>
            <a-select-option value="url-test">自动测速</a-select-option>
            <a-select-option value="fallback">故障转移</a-select-option>
            <a-select-option value="load-balance">负载均衡</a-select-option>
            <a-select-option value="relay">链式代理</a-select-option>
          </a-select>
        </a-form-item>
        <a-form-item label="代理列表">
          <a-textarea v-model:value="groupForm.proxiesText" :rows="4" placeholder="每行一个代理名称" />
        </a-form-item>
        <a-form-item label="启用">
          <a-switch v-model:checked="groupForm.is_active" />
        </a-form-item>
        <a-form-item label="排序">
          <a-input-number v-model:value="groupForm.sort_order" :min="0" style="width: 100%" />
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 规则对话框 -->
    <a-modal v-model:open="ruleDialogVisible" :title="dialogTitle" width="600px" @ok="saveRule">
      <a-form :model="ruleForm" :label-col="{ span: 6 }">
        <a-form-item label="类型">
          <a-select v-model:value="ruleForm.rule_type">
            <a-select-option value="DOMAIN">DOMAIN</a-select-option>
            <a-select-option value="DOMAIN-SUFFIX">DOMAIN-SUFFIX</a-select-option>
            <a-select-option value="DOMAIN-KEYWORD">DOMAIN-KEYWORD</a-select-option>
            <a-select-option value="IP-CIDR">IP-CIDR</a-select-option>
            <a-select-option value="GEOIP">GEOIP</a-select-option>
            <a-select-option value="PROCESS-NAME">PROCESS-NAME</a-select-option>
            <a-select-option value="MATCH">MATCH</a-select-option>
          </a-select>
        </a-form-item>
        <a-form-item label="值">
          <a-input v-model:value="ruleForm.rule_value" placeholder="例如: google.com" />
        </a-form-item>
        <a-form-item label="代理组">
          <a-input v-model:value="ruleForm.proxy_group" />
        </a-form-item>
        <a-form-item label="描述">
          <a-input v-model:value="ruleForm.description" />
        </a-form-item>
        <a-form-item label="优先级">
          <a-input-number v-model:value="ruleForm.sort_order" :min="0" style="width: 100%" />
        </a-form-item>
        <a-form-item label="启用">
          <a-switch v-model:checked="ruleForm.is_active" />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { message } from 'ant-design-vue'
import api from '@/api'

const activeTab = ref('groups')
const loading = ref(false)

// 表格列定义
const groupColumns = [
  { title: '名称', dataIndex: 'name', key: 'name', width: 150 },
  { title: '类型', dataIndex: 'type', key: 'type', width: 120 },
  { title: '代理列表', key: 'proxies' },
  { title: '状态', key: 'is_active', width: 80 },
  { title: '操作', key: 'action', width: 200 }
]

const ruleColumns = [
  { title: '类型', dataIndex: 'rule_type', key: 'rule_type', width: 150 },
  { title: '值', dataIndex: 'rule_value', key: 'rule_value', width: 200 },
  { title: '代理组', dataIndex: 'proxy_group', key: 'proxy_group', width: 150 },
  { title: '描述', dataIndex: 'description', key: 'description' },
  { title: '优先级', dataIndex: 'sort_order', key: 'sort_order', width: 80 },
  { title: '状态', key: 'is_active', width: 80 },
  { title: '操作', key: 'action', width: 200 }
]

// 数据
const groups = ref([])
const rules = ref([])
const generatedConfig = ref('')

// 对话框
const groupDialogVisible = ref(false)
const ruleDialogVisible = ref(false)
const dialogTitle = ref('')
const dialogMode = ref<'create' | 'edit'>('create')

// 表单
const groupForm = ref({
  id: null,
  name: '',
  type: 'select',
  proxiesText: '',
  is_active: true,
  sort_order: 0
})

const ruleForm = ref({
  id: null,
  rule_type: 'DOMAIN-SUFFIX',
  rule_value: '',
  proxy_group: '',
  description: '',
  sort_order: 0,
  is_active: true
})

// 加载数据
const loadGroups = async () => {
  loading.value = true
  try {
    const res = await api.get('/admin/clash/proxy-groups')
    groups.value = res.data
  } catch (error: any) {
    message.error(error.response?.data?.error?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

const loadRules = async () => {
  loading.value = true
  try {
    const res = await api.get('/admin/clash/rules')
    rules.value = res.data
  } catch (error: any) {
    message.error(error.response?.data?.error?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

// 代理组操作
const showGroupDialog = (mode: 'create' | 'edit', row?: any) => {
  dialogMode.value = mode
  dialogTitle.value = mode === 'create' ? '添加代理组' : '编辑代理组'
  
  if (mode === 'edit' && row) {
    groupForm.value = {
      id: row.id,
      name: row.name,
      type: row.type,
      proxiesText: row.proxies.join('\n'),
      is_active: row.is_active,
      sort_order: row.sort_order
    }
  } else {
    groupForm.value = {
      id: null,
      name: '',
      type: 'select',
      proxiesText: '',
      is_active: true,
      sort_order: 0
    }
  }
  
  groupDialogVisible.value = true
}

const saveGroup = async () => {
  try {
    const proxies = groupForm.value.proxiesText.split('\n').filter(p => p.trim())
    const data = {
      name: groupForm.value.name,
      type: groupForm.value.type,
      proxies,
      is_active: groupForm.value.is_active,
      sort_order: groupForm.value.sort_order
    }
    
    if (dialogMode.value === 'create') {
      await api.post('/admin/clash/proxy-groups', data)
      message.success('添加成功')
    } else {
      await api.put(`/admin/clash/proxy-groups/${groupForm.value.id}`, data)
      message.success('更新成功')
    }
    
    groupDialogVisible.value = false
    loadGroups()
  } catch (error: any) {
    message.error(error.response?.data?.error?.message || '保存失败')
  }
}

const deleteGroup = async (id: number) => {
  try {
    await api.delete(`/admin/clash/proxy-groups/${id}`)
    message.success('删除成功')
    loadGroups()
  } catch (error: any) {
    message.error(error.response?.data?.error?.message || '删除失败')
  }
}

// 规则操作
const showRuleDialog = (mode: 'create' | 'edit', row?: any) => {
  dialogMode.value = mode
  dialogTitle.value = mode === 'create' ? '添加规则' : '编辑规则'
  
  if (mode === 'edit' && row) {
    ruleForm.value = { ...row }
  } else {
    ruleForm.value = {
      id: null,
      rule_type: 'DOMAIN-SUFFIX',
      rule_value: '',
      proxy_group: '',
      description: '',
      sort_order: 0,
      is_active: true
    }
  }
  
  ruleDialogVisible.value = true
}

const saveRule = async () => {
  try {
    const data = {
      rule_type: ruleForm.value.rule_type,
      rule_value: ruleForm.value.rule_value || null,
      proxy_group: ruleForm.value.proxy_group,
      description: ruleForm.value.description,
      sort_order: ruleForm.value.sort_order,
      is_active: ruleForm.value.is_active
    }
    
    if (dialogMode.value === 'create') {
      await api.post('/admin/clash/rules', data)
      message.success('添加成功')
    } else {
      await api.put(`/admin/clash/rules/${ruleForm.value.id}`, data)
      message.success('更新成功')
    }
    
    ruleDialogVisible.value = false
    loadRules()
  } catch (error: any) {
    message.error(error.response?.data?.error?.message || '保存失败')
  }
}

const deleteRule = async (id: number) => {
  try {
    await api.delete(`/admin/clash/rules/${id}`)
    message.success('删除成功')
    loadRules()
  } catch (error: any) {
    message.error(error.response?.data?.error?.message || '删除失败')
  }
}

// 生成配置
const generateConfig = async () => {
  loading.value = true
  try {
    const res = await api.get('/admin/clash/generate')
    generatedConfig.value = res.data
    message.success('配置生成成功')
  } catch (error: any) {
    message.error(error.response?.data?.error?.message || '生成失败')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadGroups()
  loadRules()
})
</script>

<style scoped>
.clash-config {
  padding: 20px;
}

.tab-header {
  margin-bottom: 20px;
}

.generate-section {
  padding: 20px;
}
</style>
