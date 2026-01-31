import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'

export interface Node {
  id: number
  name: string
  host: string
  port: number
  protocol: string
  status: string
  max_users: number
  current_users: number
  total_upload: number
  total_download: number
  last_heartbeat: string | null
  created_at: string
  updated_at: string
  include_in_clash: boolean
  sort_order: number
  secret?: string
  config?: any
}

export interface CreateNodeRequest {
  name: string
  host: string
  port: number
  protocol: string
  secret: string
  config: any
  max_users?: number
  include_in_clash?: boolean
  sort_order?: number
}

export const useNodesStore = defineStore('nodes', () => {
  const nodes = ref<Node[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  const fetchNodes = async () => {
    loading.value = true
    error.value = null
    
    try {
      const response = await api.get<Node[]>('/admin/nodes')
      nodes.value = response.data
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '获取节点列表失败'
    } finally {
      loading.value = false
    }
  }

  const createNode = async (data: CreateNodeRequest) => {
    loading.value = true
    error.value = null
    
    try {
      await api.post('/admin/nodes', data)
      await fetchNodes()
      return true
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '创建节点失败'
      return false
    } finally {
      loading.value = false
    }
  }

  const updateNode = async (id: number, data: Partial<CreateNodeRequest>) => {
    loading.value = true
    error.value = null
    
    try {
      await api.put(`/admin/nodes/${id}`, data)
      await fetchNodes()
      return true
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '更新节点失败'
      return false
    } finally {
      loading.value = false
    }
  }

  const deleteNode = async (id: number) => {
    loading.value = true
    error.value = null
    
    try {
      await api.delete(`/admin/nodes/${id}`)
      await fetchNodes()
      return true
    } catch (e: any) {
      error.value = e.response?.data?.error?.message || '删除节点失败'
      return false
    } finally {
      loading.value = false
    }
  }

  return {
    nodes,
    loading,
    error,
    fetchNodes,
    createNode,
    updateNode,
    deleteNode
  }
})
