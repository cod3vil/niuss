import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAccessLogsStore } from './accessLogs'
import api from '@/api'

// Mock the api module
vi.mock('@/api', () => ({
  default: {
    get: vi.fn()
  }
}))

describe('useAccessLogsStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  describe('fetchAccessLogs', () => {
    it('should fetch access logs with no filters', async () => {
      const mockResponse = {
        data: {
          logs: [
            {
              id: 1,
              user_id: 100,
              user_email: 'test@example.com',
              subscription_token: 'token123',
              access_timestamp: '2024-01-01T00:00:00Z',
              ip_address: '192.168.1.1',
              user_agent: 'Mozilla/5.0',
              response_status: 'success'
            }
          ],
          total: 1,
          page: 1,
          page_size: 50,
          total_pages: 1
        }
      }

      vi.mocked(api.get).mockResolvedValue(mockResponse)

      const store = useAccessLogsStore()
      const result = await store.fetchAccessLogs()

      expect(api.get).toHaveBeenCalledWith('/admin/access-logs?page=1&page_size=50')
      expect(store.logs).toEqual(mockResponse.data.logs)
      expect(store.total).toBe(1)
      expect(store.page).toBe(1)
      expect(store.pageSize).toBe(50)
      expect(store.totalPages).toBe(1)
      expect(result).toEqual(mockResponse.data)
    })

    it('should fetch access logs with user_id filter', async () => {
      const mockResponse = {
        data: {
          logs: [],
          total: 0,
          page: 1,
          page_size: 50,
          total_pages: 0
        }
      }

      vi.mocked(api.get).mockResolvedValue(mockResponse)

      const store = useAccessLogsStore()
      await store.fetchAccessLogs({ userSearch: '123' })

      expect(api.get).toHaveBeenCalledWith('/admin/access-logs?user_id=123&page=1&page_size=50')
    })

    it('should fetch access logs with date range filter', async () => {
      const mockResponse = {
        data: {
          logs: [],
          total: 0,
          page: 1,
          page_size: 50,
          total_pages: 0
        }
      }

      vi.mocked(api.get).mockResolvedValue(mockResponse)

      const store = useAccessLogsStore()
      const startDate = '2024-01-01T00:00:00'
      const endDate = '2024-01-31T23:59:59'
      
      await store.fetchAccessLogs({ 
        startDate,
        endDate
      })

      const call = vi.mocked(api.get).mock.calls[0][0] as string
      expect(call).toContain('start_date=')
      expect(call).toContain('end_date=')
      expect(call).toContain('page=1')
      expect(call).toContain('page_size=50')
    })

    it('should fetch access logs with status filter', async () => {
      const mockResponse = {
        data: {
          logs: [],
          total: 0,
          page: 1,
          page_size: 50,
          total_pages: 0
        }
      }

      vi.mocked(api.get).mockResolvedValue(mockResponse)

      const store = useAccessLogsStore()
      await store.fetchAccessLogs({ status: 'success' })

      expect(api.get).toHaveBeenCalledWith('/admin/access-logs?status=success&page=1&page_size=50')
    })

    it('should fetch access logs with all filters combined', async () => {
      const mockResponse = {
        data: {
          logs: [],
          total: 0,
          page: 2,
          page_size: 25,
          total_pages: 0
        }
      }

      vi.mocked(api.get).mockResolvedValue(mockResponse)

      const store = useAccessLogsStore()
      const startDate = '2024-01-01T00:00:00'
      const endDate = '2024-01-31T23:59:59'
      
      await store.fetchAccessLogs({
        userSearch: '456',
        startDate,
        endDate,
        status: 'failed',
        page: 2,
        pageSize: 25
      })

      const call = vi.mocked(api.get).mock.calls[0][0] as string
      expect(call).toContain('user_id=456')
      expect(call).toContain('start_date=')
      expect(call).toContain('end_date=')
      expect(call).toContain('status=failed')
      expect(call).toContain('page=2')
      expect(call).toContain('page_size=25')
    })

    it('should handle non-numeric userSearch gracefully', async () => {
      const mockResponse = {
        data: {
          logs: [],
          total: 0,
          page: 1,
          page_size: 50,
          total_pages: 0
        }
      }

      vi.mocked(api.get).mockResolvedValue(mockResponse)

      const store = useAccessLogsStore()
      await store.fetchAccessLogs({ userSearch: 'not-a-number' })

      // Should not include user_id in params when userSearch is not a number
      expect(api.get).toHaveBeenCalledWith('/admin/access-logs?page=1&page_size=50')
    })

    it('should handle errors and update error state', async () => {
      const errorMessage = 'Failed to fetch access logs'
      const mockError = {
        response: {
          data: {
            error: {
              message: errorMessage
            }
          }
        }
      }

      vi.mocked(api.get).mockRejectedValue(mockError)

      const store = useAccessLogsStore()
      
      await expect(store.fetchAccessLogs()).rejects.toEqual(mockError)
      expect(store.error).toBe(errorMessage)
      expect(store.loading).toBe(false)
    })

    it('should handle errors without response data', async () => {
      const mockError = new Error('Network error')

      vi.mocked(api.get).mockRejectedValue(mockError)

      const store = useAccessLogsStore()
      
      await expect(store.fetchAccessLogs()).rejects.toEqual(mockError)
      expect(store.error).toBe('Failed to fetch access logs')
      expect(store.loading).toBe(false)
    })

    it('should manage loading state correctly', async () => {
      const mockResponse = {
        data: {
          logs: [],
          total: 0,
          page: 1,
          page_size: 50,
          total_pages: 0
        }
      }

      vi.mocked(api.get).mockImplementation(() => {
        return new Promise((resolve) => {
          setTimeout(() => resolve(mockResponse), 10)
        })
      })

      const store = useAccessLogsStore()
      
      expect(store.loading).toBe(false)
      
      const promise = store.fetchAccessLogs()
      expect(store.loading).toBe(true)
      
      await promise
      expect(store.loading).toBe(false)
    })

    it('should clear error state on successful fetch', async () => {
      const mockResponse = {
        data: {
          logs: [],
          total: 0,
          page: 1,
          page_size: 50,
          total_pages: 0
        }
      }

      const store = useAccessLogsStore()
      
      // Set an error first
      store.error = 'Previous error'
      
      vi.mocked(api.get).mockResolvedValue(mockResponse)
      await store.fetchAccessLogs()
      
      expect(store.error).toBeNull()
    })
  })
})
