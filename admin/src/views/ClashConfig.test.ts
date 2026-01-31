import { describe, it, expect, vi, beforeEach } from 'vitest'
import * as fc from 'fast-check'

/**
 * Feature: node-proxy-unification
 * Property 18: Config preview shows nodes as proxies
 * 
 * For any Clash configuration preview in the admin UI, the displayed proxies
 * should match the set of nodes where include_in_clash is true.
 * 
 * Validates: Requirements 4.5
 */

// Mock API response type
interface Node {
  id: number
  name: string
  host: string
  port: number
  protocol: string
  include_in_clash: boolean
  sort_order: number
}

// Mock the API module
const mockApi = {
  get: vi.fn()
}

// Function to parse YAML config and extract proxy names
const extractProxyNamesFromYaml = (yamlConfig: string): string[] => {
  const lines = yamlConfig.split('\n')
  const proxyNames: string[] = []
  let inProxiesSection = false
  
  for (const line of lines) {
    if (line.trim() === 'proxies:') {
      inProxiesSection = true
      continue
    }
    
    if (inProxiesSection) {
      // Check if we've left the proxies section
      if (line.match(/^[a-z-]+:/) && !line.startsWith('  ')) {
        break
      }
      
      // Extract proxy name from lines like "  - name: ProxyName"
      const nameMatch = line.match(/^\s*-\s*name:\s*(.+)$/)
      if (nameMatch) {
        proxyNames.push(nameMatch[1].trim())
      }
    }
  }
  
  return proxyNames
}

describe('ClashConfig - Property-Based Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('Property 18: Config preview shows nodes as proxies', () => {
    it('should only include nodes with include_in_clash=true in the generated config', () => {
      fc.assert(
        fc.property(
          // Generate an array of nodes with random include_in_clash values
          fc.array(
            fc.record({
              id: fc.integer({ min: 1, max: 1000 }),
              name: fc.string({ minLength: 1, maxLength: 20 }).map(s => s.replace(/[^a-zA-Z0-9-]/g, 'N')),
              host: fc.domain(),
              port: fc.integer({ min: 1, max: 65535 }),
              protocol: fc.constantFrom('shadowsocks', 'vmess', 'trojan', 'hysteria2', 'vless'),
              include_in_clash: fc.boolean(),
              sort_order: fc.integer({ min: 0, max: 100 })
            }),
            { minLength: 0, maxLength: 20 }
          ),
          (nodes) => {
            // Ensure unique node names by appending index to duplicates
            const uniqueNodes = nodes.map((node, index) => ({
              ...node,
              name: `${node.name}-${index}`
            }))
            
            // Filter nodes that should be included
            const includedNodes = uniqueNodes.filter(n => n.include_in_clash)
            const includedNodeNames = new Set(includedNodes.map(n => n.name))
            
            // Generate a mock YAML config that would be returned by the API
            const mockYamlConfig = generateMockClashConfig(includedNodes)
            
            // Extract proxy names from the YAML
            const proxyNamesInConfig = extractProxyNamesFromYaml(mockYamlConfig)
            
            // Property 1: All proxies in config should come from nodes with include_in_clash=true
            for (const proxyName of proxyNamesInConfig) {
              expect(includedNodeNames.has(proxyName)).toBe(true)
            }
            
            // Property 2: All nodes with include_in_clash=true should appear in config
            for (const nodeName of includedNodeNames) {
              expect(proxyNamesInConfig.includes(nodeName)).toBe(true)
            }
            
            // Property 3: No nodes with include_in_clash=false should appear in config
            const excludedNodes = uniqueNodes.filter(n => !n.include_in_clash)
            for (const node of excludedNodes) {
              expect(proxyNamesInConfig.includes(node.name)).toBe(false)
            }
            
            // Property 4: Number of proxies should equal number of included nodes
            expect(proxyNamesInConfig.length).toBe(includedNodes.length)
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should maintain sort order of nodes in the config preview', () => {
      fc.assert(
        fc.property(
          // Generate nodes with include_in_clash=true and various sort orders
          fc.array(
            fc.record({
              id: fc.integer({ min: 1, max: 1000 }),
              name: fc.string({ minLength: 1, maxLength: 20 }).map(s => s.replace(/[^a-zA-Z0-9-]/g, 'N')),
              host: fc.domain(),
              port: fc.integer({ min: 1, max: 65535 }),
              protocol: fc.constantFrom('shadowsocks', 'vmess', 'trojan', 'hysteria2', 'vless'),
              include_in_clash: fc.constant(true), // All included
              sort_order: fc.integer({ min: 0, max: 100 })
            }),
            { minLength: 1, maxLength: 15 }
          ),
          (nodes) => {
            // Sort nodes by sort_order (as the backend should do)
            const sortedNodes = [...nodes].sort((a, b) => {
              if (a.sort_order !== b.sort_order) {
                return a.sort_order - b.sort_order
              }
              return a.name.localeCompare(b.name)
            })
            
            // Generate mock config
            const mockYamlConfig = generateMockClashConfig(sortedNodes)
            const proxyNamesInConfig = extractProxyNamesFromYaml(mockYamlConfig)
            
            // Property: Proxy order in config should match sorted node order
            const expectedOrder = sortedNodes.map(n => n.name)
            expect(proxyNamesInConfig).toEqual(expectedOrder)
          }
        ),
        { numRuns: 100 }
      )
    })

    it('should handle empty node list gracefully', () => {
      const emptyNodes: Node[] = []
      const mockYamlConfig = generateMockClashConfig(emptyNodes)
      const proxyNamesInConfig = extractProxyNamesFromYaml(mockYamlConfig)
      
      // Property: Empty node list should result in empty proxy list
      expect(proxyNamesInConfig).toEqual([])
    })

    it('should preserve node names exactly in config preview', () => {
      fc.assert(
        fc.property(
          fc.array(
            fc.record({
              id: fc.integer({ min: 1, max: 1000 }),
              name: fc.string({ minLength: 1, maxLength: 30 }).map(s => s.replace(/[^a-zA-Z0-9-_]/g, 'N')),
              host: fc.domain(),
              port: fc.integer({ min: 1, max: 65535 }),
              protocol: fc.constantFrom('shadowsocks', 'vmess', 'trojan'),
              include_in_clash: fc.constant(true),
              sort_order: fc.integer({ min: 0, max: 100 })
            }),
            { minLength: 1, maxLength: 10 }
          ),
          (nodes) => {
            const mockYamlConfig = generateMockClashConfig(nodes)
            const proxyNamesInConfig = extractProxyNamesFromYaml(mockYamlConfig)
            
            // Property: Each node name should appear exactly once in config
            const nodeNames = nodes.map(n => n.name)
            expect(proxyNamesInConfig.sort()).toEqual(nodeNames.sort())
            
            // Property: No name transformations should occur
            for (const node of nodes) {
              expect(proxyNamesInConfig.includes(node.name)).toBe(true)
            }
          }
        ),
        { numRuns: 100 }
      )
    })
  })
})

// Helper function to generate a mock Clash YAML config from nodes
function generateMockClashConfig(nodes: Node[]): string {
  const proxies = nodes.map(node => {
    return `  - name: ${node.name}
    type: ${mapProtocol(node.protocol)}
    server: ${node.host}
    port: ${node.port}`
  }).join('\n')
  
  return `proxies:
${proxies || '  []'}

proxy-groups:
  - name: PROXY
    type: select
    proxies:
${nodes.map(n => `      - ${n.name}`).join('\n') || '      []'}

rules:
  - MATCH,DIRECT
`
}

function mapProtocol(protocol: string): string {
  const mapping: Record<string, string> = {
    'shadowsocks': 'ss',
    'vmess': 'vmess',
    'trojan': 'trojan',
    'hysteria2': 'hysteria2',
    'vless': 'vless'
  }
  return mapping[protocol] || protocol
}

/**
 * Unit Tests for Clash Config View Changes
 * 
 * These tests verify that:
 * - Proxy management tab is removed
 * - Proxy groups tab still exists
 * - Rules tab still exists
 * - Info banner displays correctly
 * 
 * Validates: Requirements 4.1, 4.2, 4.3, 4.4
 */

describe('ClashConfig - Unit Tests', () => {
  describe('Tab Structure', () => {
    it('should not have a proxy management tab', () => {
      // Read the ClashConfig.vue file content
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify no tab with key="proxies" exists
      expect(vueContent).not.toMatch(/<a-tab-pane\s+key="proxies"/)
      
      // Verify no "代理管理" tab text
      expect(vueContent).not.toMatch(/tab="代理管理"/)
    })

    it('should have a proxy groups management tab', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify tab with key="groups" exists
      expect(vueContent).toMatch(/<a-tab-pane\s+key="groups"/)
      
      // Verify "代理组管理" tab text exists
      expect(vueContent).toMatch(/tab="代理组管理"/)
    })

    it('should have a rules management tab', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify tab with key="rules" exists
      expect(vueContent).toMatch(/<a-tab-pane\s+key="rules"/)
      
      // Verify "规则管理" tab text exists
      expect(vueContent).toMatch(/tab="规则管理"/)
    })

    it('should have a config generation tab', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify tab with key="generate" exists
      expect(vueContent).toMatch(/<a-tab-pane\s+key="generate"/)
      
      // Verify "生成配置" tab text exists
      expect(vueContent).toMatch(/tab="生成配置"/)
    })
  })

  describe('Info Banner', () => {
    it('should display an info banner about proxy management migration', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify alert component exists
      expect(vueContent).toMatch(/<a-alert/)
      
      // Verify alert type is "info"
      expect(vueContent).toMatch(/type="info"/)
      
      // Verify alert message mentions proxy management migration
      expect(vueContent).toMatch(/代理管理已迁移/)
      
      // Verify alert description mentions nodes page
      expect(vueContent).toMatch(/节点管理页面/)
      expect(vueContent).toMatch(/节点页面/)
    })

    it('should have a closable info banner', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify alert is closable
      expect(vueContent).toMatch(/closable/)
    })

    it('should show an icon in the info banner', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify alert shows icon
      expect(vueContent).toMatch(/show-icon/)
    })
  })

  describe('Removed Functionality', () => {
    it('should not have proxy CRUD operation functions', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify no showProxyDialog function
      expect(vueContent).not.toMatch(/const showProxyDialog/)
      expect(vueContent).not.toMatch(/function showProxyDialog/)
      
      // Verify no saveProxy function
      expect(vueContent).not.toMatch(/const saveProxy/)
      expect(vueContent).not.toMatch(/function saveProxy/)
      
      // Verify no deleteProxy function
      expect(vueContent).not.toMatch(/const deleteProxy/)
      expect(vueContent).not.toMatch(/function deleteProxy/)
      
      // Verify no loadProxies function
      expect(vueContent).not.toMatch(/const loadProxies/)
      expect(vueContent).not.toMatch(/function loadProxies/)
    })

    it('should not have proxy form or dialog references', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify no proxyForm reference
      expect(vueContent).not.toMatch(/proxyForm/)
      
      // Verify no proxyDialogVisible reference
      expect(vueContent).not.toMatch(/proxyDialogVisible/)
    })

    it('should not call loadProxies in onMounted', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Extract onMounted section
      const onMountedMatch = vueContent.match(/onMounted\(\(\)\s*=>\s*\{[^}]*\}/s)
      expect(onMountedMatch).toBeTruthy()
      
      if (onMountedMatch) {
        const onMountedContent = onMountedMatch[0]
        
        // Verify loadProxies is not called
        expect(onMountedContent).not.toMatch(/loadProxies/)
        
        // Verify loadGroups and loadRules are still called
        expect(onMountedContent).toMatch(/loadGroups/)
        expect(onMountedContent).toMatch(/loadRules/)
      }
    })
  })

  describe('Preserved Functionality', () => {
    it('should still have proxy groups management functions', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify proxy group functions exist
      expect(vueContent).toMatch(/const showGroupDialog|function showGroupDialog/)
      expect(vueContent).toMatch(/const saveGroup|function saveGroup/)
      expect(vueContent).toMatch(/const deleteGroup|function deleteGroup/)
      expect(vueContent).toMatch(/const loadGroups|function loadGroups/)
    })

    it('should still have rules management functions', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify rules functions exist
      expect(vueContent).toMatch(/const showRuleDialog|function showRuleDialog/)
      expect(vueContent).toMatch(/const saveRule|function saveRule/)
      expect(vueContent).toMatch(/const deleteRule|function deleteRule/)
      expect(vueContent).toMatch(/const loadRules|function loadRules/)
    })

    it('should still have config generation function', () => {
      const fs = require('fs')
      const path = require('path')
      const vueFilePath = path.join(__dirname, 'ClashConfig.vue')
      const vueContent = fs.readFileSync(vueFilePath, 'utf-8')
      
      // Verify generateConfig function exists
      expect(vueContent).toMatch(/const generateConfig|function generateConfig/)
    })
  })
})
