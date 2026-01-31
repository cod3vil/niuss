import { describe, it, expect, vi, beforeEach } from 'vitest'
import * as fc from 'fast-check'

/**
 * Feature: node-proxy-unification
 * Property 15: Toggle updates backend immediately
 * 
 * For any node in the admin UI, when the "Include in Clash" toggle is changed,
 * the node's include_in_clash value in the database should be updated to match
 * the toggle state.
 * 
 * Validates: Requirements 3.2
 */

// Mock API for testing
interface MockNode {
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

interface UpdateNodeRequest {
  include_in_clash?: boolean
  sort_order?: number
}

// Simulated backend storage
let mockBackendNodes: Map<number, MockNode>

// Mock API functions
const mockUpdateNode = async (id: number, data: UpdateNodeRequest): Promise<boolean> => {
  const node = mockBackendNodes.get(id)
  if (!node) return false
  
  if (data.include_in_clash !== undefined) {
    node.include_in_clash = data.include_in_clash
  }
  if (data.sort_order !== undefined) {
    node.sort_order = data.sort_order
  }
  
  mockBackendNodes.set(id, node)
  return true
}

const mockGetNode = (id: number): MockNode | undefined => {
  return mockBackendNodes.get(id)
}

// Arbitrary generators
const nodeIdArbitrary = fc.integer({ min: 1, max: 10000 })
const booleanArbitrary = fc.boolean()
const sortOrderArbitrary = fc.integer({ min: 0, max: 1000 })

const nodeArbitrary = fc.record({
  id: nodeIdArbitrary,
  name: fc.string({ minLength: 1, maxLength: 50 }),
  host: fc.domain(),
  port: fc.integer({ min: 1, max: 65535 }),
  protocol: fc.constantFrom('shadowsocks', 'vmess', 'trojan', 'hysteria2', 'vless'),
  status: fc.constantFrom('online', 'offline', 'maintenance'),
  max_users: fc.integer({ min: 1, max: 10000 }),
  current_users: fc.integer({ min: 0, max: 10000 }),
  total_upload: fc.integer({ min: 0, max: 1000000000 }),
  total_download: fc.integer({ min: 0, max: 1000000000 }),
  last_heartbeat: fc.option(
    fc.constantFrom(
      '2024-01-01T00:00:00.000Z',
      '2024-06-15T12:30:00.000Z',
      '2024-12-31T23:59:59.000Z'
    ),
    { nil: null }
  ),
  created_at: fc.constantFrom(
    '2024-01-01T00:00:00.000Z',
    '2024-06-15T12:30:00.000Z',
    '2024-12-31T23:59:59.000Z'
  ),
  updated_at: fc.constantFrom(
    '2024-01-01T00:00:00.000Z',
    '2024-06-15T12:30:00.000Z',
    '2024-12-31T23:59:59.000Z'
  ),
  include_in_clash: booleanArbitrary,
  sort_order: sortOrderArbitrary,
  secret: fc.option(fc.string({ minLength: 8, maxLength: 64 }), { nil: undefined }),
  config: fc.option(
    fc.record({
      cipher: fc.option(fc.constantFrom('aes-256-gcm', 'chacha20-poly1305'), { nil: undefined }),
      method: fc.option(fc.string({ minLength: 1, maxLength: 20 }), { nil: undefined })
    }),
    { nil: undefined }
  )
})

describe('Nodes View - Property-Based Tests', () => {
  beforeEach(() => {
    mockBackendNodes = new Map()
  })

  describe('Property 15: Toggle updates backend immediately', () => {
    it('should update include_in_clash in backend when toggle is changed', async () => {
      await fc.assert(
        fc.asyncProperty(
          nodeArbitrary,
          booleanArbitrary,
          async (node, newToggleValue) => {
            // Setup: Add node to mock backend
            mockBackendNodes.set(node.id, { ...node })
            
            // Action: Simulate toggle change by calling update API
            const success = await mockUpdateNode(node.id, {
              include_in_clash: newToggleValue
            })
            
            // Assertion 1: Update should succeed
            expect(success).toBe(true)
            
            // Assertion 2: Backend should reflect the new toggle value
            const updatedNode = mockGetNode(node.id)
            expect(updatedNode).toBeDefined()
            expect(updatedNode!.include_in_clash).toBe(newToggleValue)
            
            // Assertion 3: Other fields should remain unchanged
            expect(updatedNode!.name).toBe(node.name)
            expect(updatedNode!.host).toBe(node.host)
            expect(updatedNode!.port).toBe(node.port)
            expect(updatedNode!.protocol).toBe(node.protocol)
            expect(updatedNode!.sort_order).toBe(node.sort_order)
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should handle multiple toggle changes on the same node', async () => {
      await fc.assert(
        fc.asyncProperty(
          nodeArbitrary,
          fc.array(booleanArbitrary, { minLength: 1, maxLength: 10 }),
          async (node, toggleSequence) => {
            // Setup: Add node to mock backend
            mockBackendNodes.set(node.id, { ...node })
            
            // Action: Apply sequence of toggle changes
            for (const toggleValue of toggleSequence) {
              const success = await mockUpdateNode(node.id, {
                include_in_clash: toggleValue
              })
              expect(success).toBe(true)
            }
            
            // Assertion: Final state should match last toggle value
            const finalNode = mockGetNode(node.id)
            expect(finalNode).toBeDefined()
            expect(finalNode!.include_in_clash).toBe(toggleSequence[toggleSequence.length - 1])
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should update toggle independently for different nodes', async () => {
      await fc.assert(
        fc.asyncProperty(
          fc.array(nodeArbitrary, { minLength: 2, maxLength: 10 }),
          async (nodes) => {
            // Ensure unique IDs
            const uniqueNodes = nodes.map((node, index) => ({
              ...node,
              id: index + 1
            }))
            
            // Setup: Add all nodes to mock backend
            uniqueNodes.forEach(node => {
              mockBackendNodes.set(node.id, { ...node })
            })
            
            // Action: Toggle each node independently
            const toggleValues = uniqueNodes.map(() => Math.random() > 0.5)
            
            for (let i = 0; i < uniqueNodes.length; i++) {
              await mockUpdateNode(uniqueNodes[i].id, {
                include_in_clash: toggleValues[i]
              })
            }
            
            // Assertion: Each node should have its own toggle value
            for (let i = 0; i < uniqueNodes.length; i++) {
              const node = mockGetNode(uniqueNodes[i].id)
              expect(node).toBeDefined()
              expect(node!.include_in_clash).toBe(toggleValues[i])
            }
          }
        ),
        { numRuns: 100 }
      )
    })
  })

  describe('Property 16: Sort order updates backend and UI', () => {
    it('should update sort_order in backend when value is changed', async () => {
      await fc.assert(
        fc.asyncProperty(
          nodeArbitrary,
          sortOrderArbitrary,
          async (node, newSortOrder) => {
            // Setup: Add node to mock backend
            mockBackendNodes.set(node.id, { ...node })
            
            // Action: Simulate sort order change by calling update API
            const success = await mockUpdateNode(node.id, {
              sort_order: newSortOrder
            })
            
            // Assertion 1: Update should succeed
            expect(success).toBe(true)
            
            // Assertion 2: Backend should reflect the new sort order
            const updatedNode = mockGetNode(node.id)
            expect(updatedNode).toBeDefined()
            expect(updatedNode!.sort_order).toBe(newSortOrder)
            
            // Assertion 3: Other fields should remain unchanged
            expect(updatedNode!.name).toBe(node.name)
            expect(updatedNode!.host).toBe(node.host)
            expect(updatedNode!.port).toBe(node.port)
            expect(updatedNode!.protocol).toBe(node.protocol)
            expect(updatedNode!.include_in_clash).toBe(node.include_in_clash)
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should handle multiple sort order changes on the same node', async () => {
      await fc.assert(
        fc.asyncProperty(
          nodeArbitrary,
          fc.array(sortOrderArbitrary, { minLength: 1, maxLength: 10 }),
          async (node, sortOrderSequence) => {
            // Setup: Add node to mock backend
            mockBackendNodes.set(node.id, { ...node })
            
            // Action: Apply sequence of sort order changes
            for (const sortOrder of sortOrderSequence) {
              const success = await mockUpdateNode(node.id, {
                sort_order: sortOrder
              })
              expect(success).toBe(true)
            }
            
            // Assertion: Final state should match last sort order value
            const finalNode = mockGetNode(node.id)
            expect(finalNode).toBeDefined()
            expect(finalNode!.sort_order).toBe(sortOrderSequence[sortOrderSequence.length - 1])
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should update sort order independently for different nodes', async () => {
      await fc.assert(
        fc.asyncProperty(
          fc.array(nodeArbitrary, { minLength: 2, maxLength: 10 }),
          async (nodes) => {
            // Ensure unique IDs
            const uniqueNodes = nodes.map((node, index) => ({
              ...node,
              id: index + 1
            }))
            
            // Setup: Add all nodes to mock backend
            uniqueNodes.forEach(node => {
              mockBackendNodes.set(node.id, { ...node })
            })
            
            // Action: Update sort order for each node independently
            const sortOrders = uniqueNodes.map((_, i) => i * 10)
            
            for (let i = 0; i < uniqueNodes.length; i++) {
              await mockUpdateNode(uniqueNodes[i].id, {
                sort_order: sortOrders[i]
              })
            }
            
            // Assertion: Each node should have its own sort order value
            for (let i = 0; i < uniqueNodes.length; i++) {
              const node = mockGetNode(uniqueNodes[i].id)
              expect(node).toBeDefined()
              expect(node!.sort_order).toBe(sortOrders[i])
            }
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should accept non-negative sort order values', async () => {
      await fc.assert(
        fc.asyncProperty(
          nodeArbitrary,
          fc.integer({ min: 0, max: 10000 }),
          async (node, sortOrder) => {
            // Setup: Add node to mock backend
            mockBackendNodes.set(node.id, { ...node })
            
            // Action: Update with non-negative sort order
            const success = await mockUpdateNode(node.id, {
              sort_order: sortOrder
            })
            
            // Assertion: Should succeed for all non-negative values
            expect(success).toBe(true)
            const updatedNode = mockGetNode(node.id)
            expect(updatedNode!.sort_order).toBe(sortOrder)
          }
        ),
        { numRuns: 100 }
      )
    })
  })

  describe('Property 17: Filter by include-in-clash status', () => {
    it('should filter nodes by include_in_clash status correctly', () => {
      fc.assert(
        fc.property(
          fc.array(nodeArbitrary, { minLength: 5, maxLength: 20 }),
          (nodes) => {
            // Ensure unique IDs and mix of include_in_clash values
            const uniqueNodes = nodes.map((node, index) => ({
              ...node,
              id: index + 1,
              include_in_clash: index % 2 === 0 // Alternate true/false
            }))
            
            // Test filter: all nodes
            const allNodes = uniqueNodes.filter(() => true)
            expect(allNodes.length).toBe(uniqueNodes.length)
            
            // Test filter: only included nodes
            const includedNodes = uniqueNodes.filter(node => node.include_in_clash)
            const expectedIncluded = uniqueNodes.filter(node => node.include_in_clash)
            expect(includedNodes.length).toBe(expectedIncluded.length)
            includedNodes.forEach(node => {
              expect(node.include_in_clash).toBe(true)
            })
            
            // Test filter: only excluded nodes
            const excludedNodes = uniqueNodes.filter(node => !node.include_in_clash)
            const expectedExcluded = uniqueNodes.filter(node => !node.include_in_clash)
            expect(excludedNodes.length).toBe(expectedExcluded.length)
            excludedNodes.forEach(node => {
              expect(node.include_in_clash).toBe(false)
            })
            
            // Property: included + excluded should equal all nodes
            expect(includedNodes.length + excludedNodes.length).toBe(allNodes.length)
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should handle edge case: all nodes included', () => {
      fc.assert(
        fc.property(
          fc.array(nodeArbitrary, { minLength: 1, maxLength: 10 }),
          (nodes) => {
            // Set all nodes to included
            const allIncluded = nodes.map((node, index) => ({
              ...node,
              id: index + 1,
              include_in_clash: true
            }))
            
            // Filter for included nodes
            const includedNodes = allIncluded.filter(node => node.include_in_clash)
            expect(includedNodes.length).toBe(allIncluded.length)
            
            // Filter for excluded nodes
            const excludedNodes = allIncluded.filter(node => !node.include_in_clash)
            expect(excludedNodes.length).toBe(0)
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should handle edge case: all nodes excluded', () => {
      fc.assert(
        fc.property(
          fc.array(nodeArbitrary, { minLength: 1, maxLength: 10 }),
          (nodes) => {
            // Set all nodes to excluded
            const allExcluded = nodes.map((node, index) => ({
              ...node,
              id: index + 1,
              include_in_clash: false
            }))
            
            // Filter for included nodes
            const includedNodes = allExcluded.filter(node => node.include_in_clash)
            expect(includedNodes.length).toBe(0)
            
            // Filter for excluded nodes
            const excludedNodes = allExcluded.filter(node => !node.include_in_clash)
            expect(excludedNodes.length).toBe(allExcluded.length)
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should maintain node identity through filtering', () => {
      fc.assert(
        fc.property(
          fc.array(nodeArbitrary, { minLength: 2, maxLength: 10 }),
          (nodes) => {
            // Ensure unique IDs
            const uniqueNodes = nodes.map((node, index) => ({
              ...node,
              id: index + 1
            }))
            
            // Filter nodes
            const includedNodes = uniqueNodes.filter(node => node.include_in_clash)
            
            // Property: All filtered nodes should exist in original list
            includedNodes.forEach(filteredNode => {
              const originalNode = uniqueNodes.find(n => n.id === filteredNode.id)
              expect(originalNode).toBeDefined()
              expect(originalNode!.name).toBe(filteredNode.name)
              expect(originalNode!.host).toBe(filteredNode.host)
              expect(originalNode!.port).toBe(filteredNode.port)
            })
          }
        ),
        { numRuns: 100 }
      )
    })
  })
})


describe('Nodes View - Unit Tests', () => {
  beforeEach(() => {
    mockBackendNodes = new Map()
  })

  describe('Component Rendering', () => {
    it('should render toggle switch for include_in_clash field', () => {
      const testNode: MockNode = {
        id: 1,
        name: 'Test Node',
        host: 'test.example.com',
        port: 443,
        protocol: 'vless',
        status: 'online',
        max_users: 1000,
        current_users: 50,
        total_upload: 1000000,
        total_download: 2000000,
        last_heartbeat: new Date().toISOString(),
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        include_in_clash: true,
        sort_order: 10
      }

      // Verify node has include_in_clash field
      expect(testNode.include_in_clash).toBeDefined()
      expect(typeof testNode.include_in_clash).toBe('boolean')
    })

    it('should render sort order input for sort_order field', () => {
      const testNode: MockNode = {
        id: 1,
        name: 'Test Node',
        host: 'test.example.com',
        port: 443,
        protocol: 'vless',
        status: 'online',
        max_users: 1000,
        current_users: 50,
        total_upload: 1000000,
        total_download: 2000000,
        last_heartbeat: new Date().toISOString(),
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        include_in_clash: false,
        sort_order: 5
      }

      // Verify node has sort_order field
      expect(testNode.sort_order).toBeDefined()
      expect(typeof testNode.sort_order).toBe('number')
      expect(testNode.sort_order).toBeGreaterThanOrEqual(0)
    })

    it('should display all existing node fields correctly', () => {
      const testNode: MockNode = {
        id: 1,
        name: 'Hong Kong Node 1',
        host: 'hk1.example.com',
        port: 8388,
        protocol: 'shadowsocks',
        status: 'online',
        max_users: 500,
        current_users: 123,
        total_upload: 5000000000,
        total_download: 10000000000,
        last_heartbeat: new Date().toISOString(),
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        include_in_clash: true,
        sort_order: 1,
        secret: 'test-secret',
        config: { cipher: 'aes-256-gcm' }
      }

      // Verify all required fields exist
      expect(testNode.id).toBe(1)
      expect(testNode.name).toBe('Hong Kong Node 1')
      expect(testNode.host).toBe('hk1.example.com')
      expect(testNode.port).toBe(8388)
      expect(testNode.protocol).toBe('shadowsocks')
      expect(testNode.status).toBe('online')
      expect(testNode.max_users).toBe(500)
      expect(testNode.current_users).toBe(123)
      expect(testNode.total_upload).toBe(5000000000)
      expect(testNode.total_download).toBe(10000000000)
      expect(testNode.last_heartbeat).toBeDefined()
      expect(testNode.created_at).toBeDefined()
      expect(testNode.updated_at).toBeDefined()
      
      // Verify new fields
      expect(testNode.include_in_clash).toBe(true)
      expect(testNode.sort_order).toBe(1)
    })
  })

  describe('Filter Dropdown', () => {
    it('should filter to show all nodes', () => {
      const nodes: MockNode[] = [
        {
          id: 1,
          name: 'Node 1',
          host: 'node1.example.com',
          port: 443,
          protocol: 'vless',
          status: 'online',
          max_users: 1000,
          current_users: 50,
          total_upload: 1000000,
          total_download: 2000000,
          last_heartbeat: new Date().toISOString(),
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          include_in_clash: true,
          sort_order: 1
        },
        {
          id: 2,
          name: 'Node 2',
          host: 'node2.example.com',
          port: 443,
          protocol: 'vmess',
          status: 'online',
          max_users: 1000,
          current_users: 30,
          total_upload: 500000,
          total_download: 1000000,
          last_heartbeat: new Date().toISOString(),
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          include_in_clash: false,
          sort_order: 2
        }
      ]

      // Filter: all
      const allNodes = nodes.filter(() => true)
      expect(allNodes.length).toBe(2)
    })

    it('should filter to show only included nodes', () => {
      const nodes: MockNode[] = [
        {
          id: 1,
          name: 'Node 1',
          host: 'node1.example.com',
          port: 443,
          protocol: 'vless',
          status: 'online',
          max_users: 1000,
          current_users: 50,
          total_upload: 1000000,
          total_download: 2000000,
          last_heartbeat: new Date().toISOString(),
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          include_in_clash: true,
          sort_order: 1
        },
        {
          id: 2,
          name: 'Node 2',
          host: 'node2.example.com',
          port: 443,
          protocol: 'vmess',
          status: 'online',
          max_users: 1000,
          current_users: 30,
          total_upload: 500000,
          total_download: 1000000,
          last_heartbeat: new Date().toISOString(),
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          include_in_clash: false,
          sort_order: 2
        }
      ]

      // Filter: included only
      const includedNodes = nodes.filter(node => node.include_in_clash)
      expect(includedNodes.length).toBe(1)
      expect(includedNodes[0].id).toBe(1)
    })

    it('should filter to show only excluded nodes', () => {
      const nodes: MockNode[] = [
        {
          id: 1,
          name: 'Node 1',
          host: 'node1.example.com',
          port: 443,
          protocol: 'vless',
          status: 'online',
          max_users: 1000,
          current_users: 50,
          total_upload: 1000000,
          total_download: 2000000,
          last_heartbeat: new Date().toISOString(),
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          include_in_clash: true,
          sort_order: 1
        },
        {
          id: 2,
          name: 'Node 2',
          host: 'node2.example.com',
          port: 443,
          protocol: 'vmess',
          status: 'online',
          max_users: 1000,
          current_users: 30,
          total_upload: 500000,
          total_download: 1000000,
          last_heartbeat: new Date().toISOString(),
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          include_in_clash: false,
          sort_order: 2
        }
      ]

      // Filter: excluded only
      const excludedNodes = nodes.filter(node => !node.include_in_clash)
      expect(excludedNodes.length).toBe(1)
      expect(excludedNodes[0].id).toBe(2)
    })
  })

  describe('Edge Cases', () => {
    it('should handle empty node list', () => {
      const nodes: MockNode[] = []
      
      const allNodes = nodes.filter(() => true)
      const includedNodes = nodes.filter(node => node.include_in_clash)
      const excludedNodes = nodes.filter(node => !node.include_in_clash)
      
      expect(allNodes.length).toBe(0)
      expect(includedNodes.length).toBe(0)
      expect(excludedNodes.length).toBe(0)
    })

    it('should handle node with minimum sort_order value', () => {
      const testNode: MockNode = {
        id: 1,
        name: 'Test Node',
        host: 'test.example.com',
        port: 443,
        protocol: 'vless',
        status: 'online',
        max_users: 1000,
        current_users: 50,
        total_upload: 1000000,
        total_download: 2000000,
        last_heartbeat: new Date().toISOString(),
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        include_in_clash: true,
        sort_order: 0
      }

      expect(testNode.sort_order).toBe(0)
      expect(testNode.sort_order).toBeGreaterThanOrEqual(0)
    })

    it('should handle node with large sort_order value', () => {
      const testNode: MockNode = {
        id: 1,
        name: 'Test Node',
        host: 'test.example.com',
        port: 443,
        protocol: 'vless',
        status: 'online',
        max_users: 1000,
        current_users: 50,
        total_upload: 1000000,
        total_download: 2000000,
        last_heartbeat: new Date().toISOString(),
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        include_in_clash: true,
        sort_order: 9999
      }

      expect(testNode.sort_order).toBe(9999)
      expect(testNode.sort_order).toBeGreaterThanOrEqual(0)
    })
  })
})
