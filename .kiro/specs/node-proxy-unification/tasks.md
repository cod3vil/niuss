# Implementation Plan: Node-Proxy Unification

## Overview

This implementation plan consolidates node and proxy management by extending the nodes table with Clash-specific fields, migrating data from clash_proxies, updating the Clash configuration generator to read from nodes, and simplifying the admin UI. The implementation follows an incremental approach: database migration first, then backend API updates, then Clash config generation, and finally UI changes.

## Tasks

- [x] 1. Create database migration for schema unification
  - Create new migration file that adds include_in_clash and sort_order columns to nodes table
  - Implement data transfer logic from clash_proxies to nodes (matching by name, host, port, protocol)
  - Create backup table for clash_proxies before migration
  - Implement validation to ensure all proxies are transferred before dropping clash_proxies table
  - Add rollback logic for migration failures
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 1.1 Write property test for node field preservation during migration
  - **Property 1: Node field preservation during migration**
  - **Validates: Requirements 1.6**

- [x] 1.2 Write property test for clash proxy matching algorithm
  - **Property 2: Clash proxy matching algorithm**
  - **Validates: Requirements 6.5**

- [x] 1.3 Write unit tests for migration edge cases
  - Test empty clash_proxies table
  - Test all proxies match existing nodes
  - Test no proxies match existing nodes
  - Test migration rollback on failure
  - _Requirements: 1.3, 1.4, 6.2, 6.4_

- [x] 2. Update backend data models and database queries
  - Update Node struct in api/src/models.rs to include include_in_clash and sort_order fields
  - Update UpdateNodeRequest struct to accept new fields
  - Remove ClashProxy struct and related types
  - Update database query functions to work with extended Node model
  - Add validation for include_in_clash (boolean) and sort_order (non-negative integer)
  - _Requirements: 5.2, 5.5, 5.6_

- [x] 2.1 Write property test for API accepts Clash fields
  - **Property 11: Node update accepts Clash fields**
  - **Validates: Requirements 5.2**

- [x] 2.2 Write property test for sort order validation
  - **Property 13: Sort order validation**
  - **Validates: Requirements 5.5**

- [x] 2.3 Write property test for include-in-clash validation
  - **Property 14: Include-in-clash validation**
  - **Validates: Requirements 5.6**

- [x] 3. Implement protocol mapping and Clash config generation
  - [x] 3.1 Create protocol mapping function in api/src/clash.rs
    - Implement map_node_protocol_to_clash function (shadowsocks→ss, vmess→vmess, etc.)
    - Handle unsupported protocols by returning None and logging warning
    - _Requirements: 2.3, 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_
  
  - [x] 3.2 Write property test for protocol mapping correctness
    - **Property 5: Protocol mapping correctness**
    - **Validates: Requirements 2.3, 7.1-7.5**
  
  - [x] 3.3 Write property test for unsupported protocol handling
    - **Property 8: Unsupported protocol handling**
    - **Validates: Requirements 7.6**
  
  - [x] 3.4 Create node-to-proxy conversion function
    - Implement node_to_clash_proxy function that maps node fields to Clash proxy format
    - Map node.name → proxy.name, node.host → proxy.server, node.port → proxy.port
    - Implement merge_node_config to combine node.secret and node.config
    - Map secret to password (ss/trojan/hysteria2) or uuid (vmess/vless) based on protocol
    - _Requirements: 2.4, 8.1, 8.2, 8.3, 8.5, 8.6_
  
  - [x] 3.5 Write property test for field mapping correctness
    - **Property 6: Field mapping correctness**
    - **Validates: Requirements 2.4, 8.1-8.3**
  
  - [x] 3.6 Write property test for secret field mapping by protocol
    - **Property 9: Secret field mapping by protocol**
    - **Validates: Requirements 8.5**
  
  - [x] 3.7 Write property test for protocol-specific configuration preservation
    - **Property 10: Protocol-specific configuration preservation**
    - **Validates: Requirements 8.6**
  
  - [x] 3.8 Update generate_clash_config function
    - Query nodes with include_in_clash=true ordered by sort_order
    - Convert nodes to Clash proxies using node_to_clash_proxy
    - Maintain existing proxy groups and rules logic
    - Ensure config structure remains compatible (proxies, proxy-groups, rules keys)
    - _Requirements: 2.1, 2.2, 2.6_
  
  - [x] 3.9 Write property test for include-in-clash filtering
    - **Property 3: Include-in-clash filtering**
    - **Validates: Requirements 2.1**
  
  - [x] 3.10 Write property test for sort order preservation
    - **Property 4: Sort order preservation**
    - **Validates: Requirements 2.2**
  
  - [x] 3.11 Write property test for configuration structure compatibility
    - **Property 7: Configuration structure compatibility**
    - **Validates: Requirements 2.6**

- [x] 4. Checkpoint - Run migration and test backend changes
  - Run database migration on test database
  - Verify nodes table has new columns
  - Verify clash_proxies table is dropped
  - Test Clash config generation with migrated data
  - Ensure all backend tests pass
  - Ask the user if questions arise

- [-] 5. Update API endpoints and handlers
  - Update node management endpoints in api/src/handlers.rs to accept include_in_clash and sort_order
  - Remove clash_proxies management endpoints (GET/POST/PUT/DELETE /api/admin/clash/proxies)
  - Ensure proxy groups and rules endpoints remain unchanged
  - Add endpoint validation for new fields
  - _Requirements: 5.1, 5.2, 5.4, 5.5, 5.6_

- [x] 5.1 Write property test for API backward compatibility
  - **Property 12: Proxy groups and rules API compatibility**
  - **Validates: Requirements 5.4**

- [ ] 5.2 Write unit tests for API endpoint changes
  - Test removed endpoints return 404
  - Test node creation with new fields
  - Test node update with new fields
  - Test validation errors for invalid field types
  - _Requirements: 5.1, 5.2, 5.5, 5.6_

- [x] 6. Update admin UI - Nodes management view
  - [x] 6.1 Add new columns to Nodes table in admin/src/views/Nodes.vue
    - Add "Include in Clash" toggle switch column
    - Add "Sort Order" editable number input column
    - Implement updateNodeClashStatus function to handle toggle changes
    - Implement updateNodeSortOrder function to handle sort order changes
    - _Requirements: 3.1, 3.2, 3.3, 3.4_
  
  - [x] 6.2 Write property test for toggle updates backend
    - **Property 15: Toggle updates backend immediately**
    - **Validates: Requirements 3.2**
  
  - [x] 6.3 Write property test for sort order updates
    - **Property 16: Sort order updates backend and UI**
    - **Validates: Requirements 3.4**
  
  - [x] 6.4 Add filter functionality for include_in_clash status
    - Add filter dropdown to show all/included/excluded nodes
    - Implement filter logic to display only matching nodes
    - _Requirements: 3.5_
  
  - [x] 6.5 Write property test for filter functionality
    - **Property 17: Filter by include-in-clash status**
    - **Validates: Requirements 3.5**
  
  - [x] 6.6 Write unit tests for Nodes view components
    - Test toggle switch renders correctly
    - Test sort order input renders correctly
    - Test filter dropdown works
    - Test all existing node fields display correctly
    - _Requirements: 3.1, 3.3, 3.6_

- [x] 7. Update admin UI - Clash Config view
  - [x] 7.1 Remove proxy management tab from admin/src/views/ClashConfig.vue
    - Remove "代理管理" tab and all proxy CRUD operations
    - Keep proxy groups and rules tabs unchanged
    - Add info banner explaining proxy management moved to Nodes page
    - _Requirements: 4.1, 4.2, 4.3, 4.4_
  
  - [x] 7.2 Update config preview to show proxies from nodes
    - Ensure preview displays proxies sourced from nodes with include_in_clash=true
    - Verify preview maintains correct structure
    - _Requirements: 4.5_
  
  - [x] 7.3 Write property test for config preview accuracy
    - **Property 18: Config preview shows nodes as proxies**
    - **Validates: Requirements 4.5**
  
  - [x] 7.4 Write unit tests for Clash Config view changes
    - Test proxy management tab is removed
    - Test proxy groups tab still exists
    - Test rules tab still exists
    - Test info banner displays correctly
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [x] 8. Update TypeScript types and API client
  - Update Node interface in frontend to include include_in_clash and sort_order
  - Remove ClashProxy interface and related types
  - Update API client functions to work with new node structure
  - Remove API client functions for clash_proxies endpoints
  - _Requirements: 5.1, 5.2_

- [x] 9. Final checkpoint - Integration testing and validation
  - Run full test suite (unit tests and property tests)
  - Test complete workflow: create node → toggle include → change sort order → preview config
  - Verify Clash config generation works end-to-end
  - Test with real Clash client to ensure configs are valid
  - Verify no data loss from migration
  - Ensure all tests pass, ask the user if questions arise

## Notes

- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at key milestones
- Property tests validate universal correctness properties across all inputs
- Unit tests validate specific examples, edge cases, and error conditions
- The migration should be tested thoroughly on a copy of production data before running in production
- Consider creating a rollback migration in case issues are discovered after deployment
