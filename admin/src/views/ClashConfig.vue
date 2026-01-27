<template>
  <div class="clash-config">
    <h1>Clash 配置管理</h1>
    
    <el-tabs v-model="activeTab" type="border-card">
      <!-- 代理管理 -->
      <el-tab-pane label="代理管理" name="proxies">
        <div class="tab-header">
          <el-button type="primary" @click="showProxyDialog('create')">添加代理</el-button>
          <el-button @click="loadProxies">刷新</el-button>
        </div>
        
        <el-table :data="proxies" style="width: 100%" v-loading="loading">
          <el-table-column prop="name" label="名称" width="150" />
          <el-table-column prop="type" label="类型" width="100" />
          <el-table-column prop="server" label="服务器" width="200" />
          <el-table-column prop="port" label="端口" width="80" />
          <el-table-column prop="is_active" label="状态" width="80">
            <template #default="{ row }">
              <el-tag :type="row.is_active ? 'success' : 'info'">
                {{ row.is_active ? '启用' : '禁用' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="sort_order" label="排序" width="80" />
          <el-table-column label="操作" width="200">
            <template #default="{ row }">
              <el-button size="small" @click="showProxyDialog('edit', row)">编辑</el-button>
              <el-button size="small" type="danger" @click="deleteProxy(row.id)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 代理组管理 -->
      <el-tab-pane label="代理组管理" name="groups">
        <div class="tab-header">
          <el-button type="primary" @click="showGroupDialog('create')">添加代理组</el-button>
          <el-button @click="loadGroups">刷新</el-button>
        </div>
        
        <el-table :data="groups" style="width: 100%" v-loading="loading">
          <el-table-column prop="name" label="名称" width="150" />
          <el-table-column prop="type" label="类型" width="120" />
          <el-table-column label="代理列表" min-width="200">
            <template #default="{ row }">
              <el-tag v-for="proxy in row.proxies" :key="proxy" size="small" style="margin: 2px">
                {{ proxy }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="is_active" label="状态" width="80">
            <template #default="{ row }">
              <el-tag :type="row.is_active ? 'success' : 'info'">
                {{ row.is_active ? '启用' : '禁用' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="200">
            <template #default="{ row }">
              <el-button size="small" @click="showGroupDialog('edit', row)">编辑</el-button>
              <el-button size="small" type="danger" @click="deleteGroup(row.id)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 规则管理 -->
      <el-tab-pane label="规则管理" name="rules">
        <div class="tab-header">
          <el-button type="primary" @click="showRuleDialog('create')">添加规则</el-button>
          <el-button @click="loadRules">刷新</el-button>
        </div>
        
        <el-table :data="rules" style="width: 100%" v-loading="loading">
          <el-table-column prop="rule_type" label="类型" width="150" />
          <el-table-column prop="rule_value" label="值" width="200" />
          <el-table-column prop="proxy_group" label="代理组" width="150" />
          <el-table-column prop="description" label="描述" min-width="200" />
          <el-table-column prop="sort_order" label="优先级" width="80" />
          <el-table-column prop="is_active" label="状态" width="80">
            <template #default="{ row }">
              <el-tag :type="row.is_active ? 'success' : 'info'">
                {{ row.is_active ? '启用' : '禁用' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="200">
            <template #default="{ row }">
              <el-button size="small" @click="showRuleDialog('edit', row)">编辑</el-button>
              <el-button size="small" type="danger" @click="deleteRule(row.id)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <!-- 生成配置 -->
      <el-tab-pane label="生成配置" name="generate">
        <div class="generate-section">
          <el-button type="primary" size="large" @click="generateConfig">生成 Clash 配置</el-button>
          <el-input
            v-if="generatedConfig"
            v-model="generatedConfig"
            type="textarea"
            :rows="20"
            readonly
            style="margin-top: 20px"
          />
        </div>
      </el-tab-pane>
    </el-tabs>

    <!-- 代理对话框 -->
    <el-dialog v-model="proxyDialogVisible" :title="dialogTitle" width="600px">
      <el-form :model="proxyForm" label-width="100px">
        <el-form-item label="名称">
          <el-input v-model="proxyForm.name" />
        </el-form-item>
        <el-form-item label="类型">
          <el-select v-model="proxyForm.type">
            <el-option label="Shadowsocks" value="ss" />
            <el-option label="VMess" value="vmess" />
            <el-option label="Trojan" value="trojan" />
            <el-option label="Hysteria2" value="hysteria2" />
            <el-option label="VLESS" value="vless" />
          </el-select>
        </el-form-item>
        <el-form-item label="服务器">
          <el-input v-model="proxyForm.server" />
        </el-form-item>
        <el-form-item label="端口">
          <el-input-number v-model="proxyForm.port" :min="1" :max="65535" />
        </el-form-item>
        <el-form-item label="配置">
          <el-input v-model="proxyForm.configJson" type="textarea" :rows="6" placeholder='{"password": "xxx"}' />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="proxyForm.is_active" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="proxyForm.sort_order" :min="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="proxyDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveProxy">保存</el-button>
      </template>
    </el-dialog>

    <!-- 代理组对话框 -->
    <el-dialog v-model="groupDialogVisible" :title="dialogTitle" width="600px">
      <el-form :model="groupForm" label-width="100px">
        <el-form-item label="名称">
          <el-input v-model="groupForm.name" />
        </el-form-item>
        <el-form-item label="类型">
          <el-select v-model="groupForm.type">
            <el-option label="手动选择" value="select" />
            <el-option label="自动测速" value="url-test" />
            <el-option label="故障转移" value="fallback" />
            <el-option label="负载均衡" value="load-balance" />
            <el-option label="链式代理" value="relay" />
          </el-select>
        </el-form-item>
        <el-form-item label="代理列表">
          <el-input v-model="groupForm.proxiesText" type="textarea" :rows="4" placeholder="每行一个代理名称" />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="groupForm.is_active" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="groupForm.sort_order" :min="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="groupDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveGroup">保存</el-button>
      </template>
    </el-dialog>

    <!-- 规则对话框 -->
    <el-dialog v-model="ruleDialogVisible" :title="dialogTitle" width="600px">
      <el-form :model="ruleForm" label-width="100px">
        <el-form-item label="类型">
          <el-select v-model="ruleForm.rule_type">
            <el-option label="DOMAIN" value="DOMAIN" />
            <el-option label="DOMAIN-SUFFIX" value="DOMAIN-SUFFIX" />
            <el-option label="DOMAIN-KEYWORD" value="DOMAIN-KEYWORD" />
            <el-option label="IP-CIDR" value="IP-CIDR" />
            <el-option label="GEOIP" value="GEOIP" />
            <el-option label="PROCESS-NAME" value="PROCESS-NAME" />
            <el-option label="MATCH" value="MATCH" />
          </el-select>
        </el-form-item>
        <el-form-item label="值">
          <el-input v-model="ruleForm.rule_value" placeholder="例如: google.com" />
        </el-form-item>
        <el-form-item label="代理组">
          <el-input v-model="ruleForm.proxy_group" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="ruleForm.description" />
        </el-form-item>
        <el-form-item label="优先级">
          <el-input-number v-model="ruleForm.sort_order" :min="0" />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="ruleForm.is_active" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="ruleDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveRule">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import api from '@/api'

const activeTab = ref('proxies')
const loading = ref(false)

// 数据
const proxies = ref([])
const groups = ref([])
const rules = ref([])
const generatedConfig = ref('')

// 对话框
const proxyDialogVisible = ref(false)
const groupDialogVisible = ref(false)
const ruleDialogVisible = ref(false)
const dialogTitle = ref('')
const dialogMode = ref<'create' | 'edit'>('create')

// 表单
const proxyForm = ref({
  id: null,
  name: '',
  type: 'trojan',
  server: '',
  port: 443,
  configJson: '{}',
  is_active: true,
  sort_order: 0
})

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
const loadProxies = async () => {
  loading.value = true
  try {
    const res = await api.get('/admin/clash/proxies')
    proxies.value = res.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.error?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

const loadGroups = async () => {
  loading.value = true
  try {
    const res = await api.get('/admin/clash/proxy-groups')
    groups.value = res.data
  } catch (error: any) {
    ElMessage.error(error.response?.data?.error?.message || '加载失败')
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
    ElMessage.error(error.response?.data?.error?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

// 代理操作
const showProxyDialog = (mode: 'create' | 'edit', row?: any) => {
  dialogMode.value = mode
  dialogTitle.value = mode === 'create' ? '添加代理' : '编辑代理'
  
  if (mode === 'edit' && row) {
    proxyForm.value = {
      id: row.id,
      name: row.name,
      type: row.type,
      server: row.server,
      port: row.port,
      configJson: JSON.stringify(row.config, null, 2),
      is_active: row.is_active,
      sort_order: row.sort_order
    }
  } else {
    proxyForm.value = {
      id: null,
      name: '',
      type: 'trojan',
      server: '',
      port: 443,
      configJson: '{"password": "", "udp": true, "skip-cert-verify": true}',
      is_active: true,
      sort_order: 0
    }
  }
  
  proxyDialogVisible.value = true
}

const saveProxy = async () => {
  try {
    const config = JSON.parse(proxyForm.value.configJson)
    const data = {
      name: proxyForm.value.name,
      type: proxyForm.value.type,
      server: proxyForm.value.server,
      port: proxyForm.value.port,
      config,
      is_active: proxyForm.value.is_active,
      sort_order: proxyForm.value.sort_order
    }
    
    if (dialogMode.value === 'create') {
      await api.post('/admin/clash/proxies', data)
      ElMessage.success('添加成功')
    } else {
      await api.put(`/admin/clash/proxies/${proxyForm.value.id}`, data)
      ElMessage.success('更新成功')
    }
    
    proxyDialogVisible.value = false
    loadProxies()
  } catch (error: any) {
    ElMessage.error(error.response?.data?.error?.message || '保存失败')
  }
}

const deleteProxy = async (id: number) => {
  try {
    await ElMessageBox.confirm('确定要删除这个代理吗？', '提示', {
      type: 'warning'
    })
    await api.delete(`/admin/clash/proxies/${id}`)
    ElMessage.success('删除成功')
    loadProxies()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.response?.data?.error?.message || '删除失败')
    }
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
      ElMessage.success('添加成功')
    } else {
      await api.put(`/admin/clash/proxy-groups/${groupForm.value.id}`, data)
      ElMessage.success('更新成功')
    }
    
    groupDialogVisible.value = false
    loadGroups()
  } catch (error: any) {
    ElMessage.error(error.response?.data?.error?.message || '保存失败')
  }
}

const deleteGroup = async (id: number) => {
  try {
    await ElMessageBox.confirm('确定要删除这个代理组吗？', '提示', {
      type: 'warning'
    })
    await api.delete(`/admin/clash/proxy-groups/${id}`)
    ElMessage.success('删除成功')
    loadGroups()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.response?.data?.error?.message || '删除失败')
    }
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
      ElMessage.success('添加成功')
    } else {
      await api.put(`/admin/clash/rules/${ruleForm.value.id}`, data)
      ElMessage.success('更新成功')
    }
    
    ruleDialogVisible.value = false
    loadRules()
  } catch (error: any) {
    ElMessage.error(error.response?.data?.error?.message || '保存失败')
  }
}

const deleteRule = async (id: number) => {
  try {
    await ElMessageBox.confirm('确定要删除这个规则吗？', '提示', {
      type: 'warning'
    })
    await api.delete(`/admin/clash/rules/${id}`)
    ElMessage.success('删除成功')
    loadRules()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.response?.data?.error?.message || '删除失败')
    }
  }
}

// 生成配置
const generateConfig = async () => {
  loading.value = true
  try {
    const res = await api.get('/admin/clash/generate')
    generatedConfig.value = res.data
    ElMessage.success('配置生成成功')
  } catch (error: any) {
    ElMessage.error(error.response?.data?.error?.message || '生成失败')
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadProxies()
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
