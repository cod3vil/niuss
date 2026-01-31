# Design Document: Node-Proxy Unification

## Overview

This design consolidates the dual management of VPN server information (nodes and clash_proxies tables) into a unified system using the nodes table as the single source of truth. The solution eliminates data duplication, removes manual synchronization requirements, and simplifies the admin interface while maintaining full functionality for Clash configuration generation.

The key architectural change is extending the nodes table with Clash-specific metadata (include_in_clash, sort_order) and modifying the Clash configuration generation logic to read directly from nodes instead of a separate clash_proxies table. This approach preserves the existing Node Agent functionality while streamlining proxy management.

## Architecture

### Current Architecture

```
┌─────────────────┐         ┌──────────────────┐
│  Nodes Table    │         │ Clash_Proxies    │
│  - Node Agent   │         │ - Clash Config   │
│    data         │         │   generation     │
└─────────────────┘         └──────────────────┘
        │                            │
        │                            │
        ▼                            ▼
┌─────────────────┐         ┌──────────────────┐
│  Node Agent     │         │ Clash Config     │
│  Management     │         │ Generator        │
└─────────────────┘         └──────────────────┘
```

**Problems:**
- Data duplication between two tables
- Manual synchronization required
- Two separate admin UI interfaces
- Risk of inconsistency

### Proposed Architecture

```
┌─────────────────────────────────────┐
│  Unified Nodes Table                │
│  - Node Agent data                  │
│  - Clash metadata (include, order)  │
└─────────────────────────────────────┘
        │                    │
        │                    │
        ▼                    ▼
┌─────────────────┐  ┌──────────────────┐
│  Node Agent     │  │ Clash Config     │
│  Management     │  │ Generator        │
└─────────────────┘  └──────────────────┘
```

**Benefits:**
- Single source of truth
- Automatic synchronization
- Unified admin interface
- Guaranteed consistency

## Components and Interfaces

### 1. Database Schema Changes

#### Extended Nodes Table

```sql
-- New fields added to existing nodes table
ALTER TABLE nodes ADD COLUMN include_in_clash BOOLEAN DEFAULT false;
ALTER TABLE nodes ADD COLUMN sort_order INTEGER DEFAULT 0;
CREATE INDEX idx_nodes_clash_inclusion ON nodes(include_in_clash, sort_order);
```

**Fields:**
- `include_in_clash`: Boolean flag indicating if node should appear in Clash configs
- `sort_order`: Integer for controlling display order in Clash configurations (lower values first)

#### Migration Strategy

The migration will follow these steps:

1. **Add new columns** to nodes table
2. **Create backup** of clash_proxies table
3. **Match and merge** clash_proxies data into nodes:
   - Match by: name, server/host, port, protocol
   - Set include_in_clash = is_active
   - Set sort_order from clash_proxies.sort_order
4. **Create new nodes** for unmatched clash_proxies
5. **Validate** all data transferred successfully
6. **Drop** clash_proxies table

```sql
-- Migration pseudocode
BEGIN TRANSACTION;

-- Backup
CREATE TABLE clash_proxies_backup AS SELECT * FROM clash_proxies;

-- Add new columns
ALTER TABLE nodes ADD COLUMN include_in_clash BOOLEAN DEFAULT false;
ALTER TABLE nodes ADD COLUMN sort_order INTEGER DEFAULT 0;

-- Match and update existing nodes
UPDATE nodes n
SET 
  include_in_clash = cp.is_active,
  sort_order = cp.sort_order
FROM clash_proxies cp
WHERE 
  n.name = cp.name 
  AND n.host = cp.server 
  AND n.port = cp.port
  AND n.protocol = cp.type;

-- Create nodes for unmatched proxies
INSERT INTO nodes (name, host, port, protocol, secret, config, include_in_clash, sort_order, status)
SELECT 
  cp.name,
  cp.server,
  cp.port,
  cp.type,
  '', -- empty secret, will need configuration
  cp.config,
  cp.is_active,
  cp.sort_order,
  'inactive' -- default status for newly created nodes
FROM clash_proxies cp
WHERE NOT EXISTS (
  SELECT 1 FROM nodes n
  WHERE n.name = cp.name 
    AND n.host = cp.server 
    AND n.port = cp.port
    AND n.protocol = cp.type
);

-- Validate migration
DO $$
DECLARE
  proxy_count INTEGER;
  migrated_count INTEGER;
BEGIN
  SELECT COUNT(*) INTO proxy_count FROM clash_proxies;
  SELECT COUNT(*) INTO migrated_count FROM nodes WHERE include_in_clash = true;
  
  IF migrated_count < proxy_count THEN
    RAISE EXCEPTION 'Migration validation failed: expected % proxies, found % nodes', proxy_count, migrated_count;
  END IF;
END $$;

-- Drop old table
DROP TABLE clash_proxies;

COMMIT;
```

### 2. Backend Data Models

#### Updated Node Model (Rust)

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Node {
    pub id: i32,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub protocol: String, // shadowsocks, vmess, trojan, hysteria2, vless
    pub secret: String,
    pub config: serde_json::Value,
    pub status: String,
    pub max_users: Option<i32>,
    pub current_users: Option<i32>,
    pub traffic_up: Option<i64>,
    pub traffic_down: Option<i64>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    
    // New fields for Clash integration
    pub include_in_clash: bool,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNodeRequest {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub protocol: Option<String>,
    pub secret: Option<String>,
    pub config: Option<serde_json::Value>,
    pub status: Option<String>,
    pub max_users: Option<i32>,
    
    // New fields
    pub include_in_clash: Option<bool>,
    pub sort_order: Option<i32>,
}
```

#### Removed ClashProxy Model

The `ClashProxy` struct and related database operations will be removed entirely.

### 3. Clash Configuration Generator

#### Protocol Mapping

```rust
fn map_node_protocol_to_clash(protocol: &str) -> Option<&str> {
    match protocol {
        "shadowsocks" => Some("ss"),
        "vmess" => Some("vmess"),
        "trojan" => Some("trojan"),
        "hysteria2" => Some("hysteria2"),
        "vless" => Some("vless"),
        _ => None,
    }
}
```

#### Configuration Generation Logic

```rust
pub async fn generate_clash_config(
    pool: &PgPool,
    user_id: i32,
) -> Result<ClashConfig, Error> {
    // Query nodes that should be included in Clash config
    let nodes = sqlx::query_as::<_, Node>(
        "SELECT * FROM nodes 
         WHERE include_in_clash = true 
         ORDER BY sort_order ASC, name ASC"
    )
    .fetch_all(pool)
    .await?;
    
    // Convert nodes to Clash proxy format
    let proxies: Vec<ClashProxy> = nodes
        .iter()
        .filter_map(|node| node_to_clash_proxy(node))
        .collect();
    
    // Get proxy groups and rules (unchanged)
    let proxy_groups = get_proxy_groups(pool).await?;
    let rules = get_rules(pool).await?;
    
    Ok(ClashConfig {
        proxies,
        proxy_groups,
        rules,
        // ... other config fields
    })
}

fn node_to_clash_proxy(node: &Node) -> Option<ClashProxy> {
    let proxy_type = map_node_protocol_to_clash(&node.protocol)?;
    
    Some(ClashProxy {
        name: node.name.clone(),
        proxy_type: proxy_type.to_string(),
        server: node.host.clone(),
        port: node.port,
        config: merge_node_config(node),
    })
}

fn merge_node_config(node: &Node) -> serde_json::Value {
    let mut config = node.config.clone();
    
    // Add protocol-specific fields from node.secret
    match node.protocol.as_str() {
        "shadowsocks" => {
            config["password"] = json!(node.secret);
        },
        "vmess" => {
            config["uuid"] = json!(node.secret);
        },
        "trojan" => {
            config["password"] = json!(node.secret);
        },
        "hysteria2" => {
            config["password"] = json!(node.secret);
        },
        "vless" => {
            config["uuid"] = json!(node.secret);
        },
        _ => {},
    }
    
    config
}
```

### 4. API Endpoints

#### Updated Endpoints

**Node Management:**
```
GET    /api/admin/nodes              - List all nodes (includes Clash fields)
GET    /api/admin/nodes/:id          - Get node details
POST   /api/admin/nodes              - Create node (includes Clash fields)
PUT    /api/admin/nodes/:id          - Update node (includes Clash fields)
DELETE /api/admin/nodes/:id          - Delete node
```

**Removed Endpoints:**
```
GET    /api/admin/clash/proxies      - REMOVED
POST   /api/admin/clash/proxies      - REMOVED
PUT    /api/admin/clash/proxies/:id  - REMOVED
DELETE /api/admin/clash/proxies/:id  - REMOVED
```

**Unchanged Endpoints:**
```
GET    /api/admin/clash/groups       - Proxy groups management
POST   /api/admin/clash/groups       - Create proxy group
GET    /api/admin/clash/rules        - Rules management
POST   /api/admin/clash/rules        - Create rule
GET    /api/clash/config/:token      - Generate Clash config (updated logic)
```

#### Request/Response Examples

**Update Node with Clash Fields:**
```json
PUT /api/admin/nodes/123
{
  "name": "HK-Node-01",
  "host": "hk1.example.com",
  "port": 8388,
  "protocol": "shadowsocks",
  "secret": "mypassword123",
  "config": {
    "cipher": "aes-256-gcm"
  },
  "include_in_clash": true,
  "sort_order": 10
}
```

**Response:**
```json
{
  "id": 123,
  "name": "HK-Node-01",
  "host": "hk1.example.com",
  "port": 8388,
  "protocol": "shadowsocks",
  "secret": "mypassword123",
  "config": {
    "cipher": "aes-256-gcm"
  },
  "status": "active",
  "max_users": 100,
  "current_users": 45,
  "include_in_clash": true,
  "sort_order": 10,
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

### 5. Admin UI Changes

#### Nodes Management View (admin/src/views/Nodes.vue)

**New Columns:**
- "Include in Clash" - Toggle switch column
- "Sort Order" - Editable number input

**New Features:**
- Filter by "Include in Clash" status
- Inline editing for sort_order
- Bulk toggle for include_in_clash

**Component Structure:**
```vue
<template>
  <div class="nodes-management">
    <a-table :dataSource="nodes" :columns="columns">
      <!-- Existing columns: name, host, port, protocol, status, etc. -->
      
      <!-- New column: Include in Clash -->
      <template #includeInClash="{ record }">
        <a-switch 
          v-model:checked="record.include_in_clash"
          @change="updateNodeClashStatus(record)"
        />
      </template>
      
      <!-- New column: Sort Order -->
      <template #sortOrder="{ record }">
        <a-input-number
          v-model:value="record.sort_order"
          :min="0"
          @change="updateNodeSortOrder(record)"
        />
      </template>
    </a-table>
  </div>
</template>

<script setup lang="ts">
const columns = [
  { title: 'Name', dataIndex: 'name', key: 'name' },
  { title: 'Host', dataIndex: 'host', key: 'host' },
  { title: 'Port', dataIndex: 'port', key: 'port' },
  { title: 'Protocol', dataIndex: 'protocol', key: 'protocol' },
  { title: 'Status', dataIndex: 'status', key: 'status' },
  { title: 'Include in Clash', key: 'includeInClash', slots: { customRender: 'includeInClash' } },
  { title: 'Sort Order', key: 'sortOrder', slots: { customRender: 'sortOrder' } },
  // ... other columns
];

async function updateNodeClashStatus(node: Node) {
  await api.updateNode(node.id, {
    include_in_clash: node.include_in_clash
  });
}

async function updateNodeSortOrder(node: Node) {
  await api.updateNode(node.id, {
    sort_order: node.sort_order
  });
}
</script>
```

#### Clash Config View (admin/src/views/ClashConfig.vue)

**Removed:**
- "代理管理" (Proxy Management) tab
- All proxy CRUD operations

**Kept:**
- Proxy Groups management tab
- Rules management tab
- Config preview/generation

**Added:**
- Info banner: "Proxy management has been moved to the Nodes page. Use the 'Include in Clash' toggle to control which nodes appear in Clash configurations."

```vue
<template>
  <div class="clash-config">
    <a-alert
      message="Proxy Management Update"
      description="Proxy management has been moved to the Nodes page. Use the 'Include in Clash' toggle to control which nodes appear in Clash configurations."
      type="info"
      show-icon
      closable
      style="margin-bottom: 16px"
    />
    
    <a-tabs v-model:activeKey="activeTab">
      <!-- Removed: Proxy Management tab -->
      
      <a-tab-pane key="groups" tab="Proxy Groups">
        <!-- Existing proxy groups management -->
      </a-tab-pane>
      
      <a-tab-pane key="rules" tab="Rules">
        <!-- Existing rules management -->
      </a-tab-pane>
      
      <a-tab-pane key="preview" tab="Config Preview">
        <!-- Shows generated config with proxies from nodes -->
      </a-tab-pane>
    </a-tabs>
  </div>
</template>
```

## Data Models

### Node (Extended)

```typescript
interface Node {
  id: number;
  name: string;
  host: string;
  port: number;
  protocol: 'shadowsocks' | 'vmess' | 'trojan' | 'hysteria2' | 'vless';
  secret: string;
  config: Record<string, any>;
  status: 'active' | 'inactive' | 'maintenance';
  max_users?: number;
  current_users?: number;
  traffic_up?: number;
  traffic_down?: number;
  created_at: string;
  updated_at: string;
  
  // New fields
  include_in_clash: boolean;
  sort_order: number;
}
```

### ClashProxy (Generated from Node)

```typescript
interface ClashProxy {
  name: string;
  type: 'ss' | 'vmess' | 'trojan' | 'hysteria2' | 'vless';
  server: string;
  port: number;
  password?: string;  // For ss, trojan, hysteria2
  uuid?: string;      // For vmess, vless
  cipher?: string;    // For ss
  // ... other protocol-specific fields from node.config
}
```

### ClashConfig (Unchanged Structure)

```typescript
interface ClashConfig {
  proxies: ClashProxy[];
  'proxy-groups': ProxyGroup[];
  rules: string[];
  // ... other Clash config fields
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Migration Properties

Property 1: Node field preservation during migration
*For any* node that exists before migration, all original fields (name, host, port, protocol, secret, config, status, max_users, current_users, traffic statistics) should remain unchanged after migration completes
**Validates: Requirements 1.6**

Property 2: Clash proxy matching algorithm
*For any* clash_proxy and node pair, they should be considered a match if and only if their name, server/host, port, and protocol fields are equal
**Validates: Requirements 6.5**

### Clash Configuration Generation Properties

Property 3: Include-in-clash filtering
*For any* generated Clash configuration, all proxies in the configuration should come from nodes where include_in_clash is true, and no proxies should come from nodes where include_in_clash is false
**Validates: Requirements 2.1**

Property 4: Sort order preservation
*For any* generated Clash configuration, the proxies should be ordered by their sort_order field in ascending order (with ties broken by name)
**Validates: Requirements 2.2**

Property 5: Protocol mapping correctness
*For any* node with protocol in {shadowsocks, vmess, trojan, hysteria2, vless}, the generated Clash proxy should have type {ss, vmess, trojan, hysteria2, vless} respectively
**Validates: Requirements 2.3, 7.1, 7.2, 7.3, 7.4, 7.5**

Property 6: Field mapping correctness
*For any* node included in Clash configuration, the generated proxy should have: proxy.name = node.name, proxy.server = node.host, and proxy.port = node.port
**Validates: Requirements 2.4, 8.1, 8.2, 8.3**

Property 7: Configuration structure compatibility
*For any* generated Clash configuration, it should contain the keys "proxies", "proxy-groups", and "rules", maintaining the same structure as configurations generated before the unification
**Validates: Requirements 2.6**

Property 8: Unsupported protocol handling
*For any* node with a protocol not in {shadowsocks, vmess, trojan, hysteria2, vless}, that node should be excluded from the generated Clash configuration and a warning should be logged
**Validates: Requirements 7.6**

Property 9: Secret field mapping by protocol
*For any* node with protocol in {shadowsocks, trojan, hysteria2}, the generated proxy config should have password = node.secret; for any node with protocol in {vmess, vless}, the generated proxy config should have uuid = node.secret
**Validates: Requirements 8.5**

Property 10: Protocol-specific configuration preservation
*For any* node, all key-value pairs in node.config should appear in the generated Clash proxy configuration
**Validates: Requirements 8.6**

### API Properties

Property 11: Node update accepts Clash fields
*For any* valid node update request that includes include_in_clash or sort_order fields, the API should accept the request and persist the values to the database
**Validates: Requirements 5.2**

Property 12: Proxy groups and rules API compatibility
*For any* API request to proxy groups or rules endpoints, the response and behavior should be identical to the behavior before the unification
**Validates: Requirements 5.4**

Property 13: Sort order validation
*For any* node update request with sort_order field, if the value is negative, the API should reject the request with a validation error
**Validates: Requirements 5.5**

Property 14: Include-in-clash validation
*For any* node update request with include_in_clash field, if the value is not a boolean, the API should reject the request with a validation error
**Validates: Requirements 5.6**

### UI Properties

Property 15: Toggle updates backend immediately
*For any* node in the admin UI, when the "Include in Clash" toggle is changed, the node's include_in_clash value in the database should be updated to match the toggle state
**Validates: Requirements 3.2**

Property 16: Sort order updates backend and UI
*For any* node in the admin UI, when the sort_order value is changed, the node's sort_order value in the database should be updated and the node list should be re-ordered accordingly
**Validates: Requirements 3.4**

Property 17: Filter by include-in-clash status
*For any* filter state (showing all nodes, only included nodes, or only excluded nodes), the displayed node list should contain only nodes matching the filter criteria
**Validates: Requirements 3.5**

Property 18: Config preview shows nodes as proxies
*For any* Clash configuration preview in the admin UI, the displayed proxies should match the set of nodes where include_in_clash is true
**Validates: Requirements 4.5**

## Error Handling

### Migration Errors

1. **Matching Failures**: If a clash_proxy cannot be matched to an existing node, create a new node with status='inactive' and log the creation
2. **Validation Failures**: If migration validation fails (proxy count mismatch), rollback the entire transaction and preserve all original data
3. **Constraint Violations**: If adding new columns or dropping tables fails, rollback and log the error with details

### Configuration Generation Errors

1. **Unsupported Protocols**: Log warning and exclude node from configuration (don't fail entire generation)
2. **Missing Required Fields**: Log error and exclude node from configuration
3. **Invalid Configuration JSON**: Log error with node details and exclude from configuration
4. **Database Connection Errors**: Return 500 error with appropriate message

### API Errors

1. **Invalid Field Types**: Return 400 Bad Request with validation error details
2. **Negative sort_order**: Return 400 Bad Request with message "sort_order must be non-negative"
3. **Non-boolean include_in_clash**: Return 400 Bad Request with message "include_in_clash must be a boolean"
4. **Node Not Found**: Return 404 Not Found
5. **Database Errors**: Return 500 Internal Server Error

### UI Error Handling

1. **API Request Failures**: Show error notification with retry option
2. **Network Errors**: Show connection error message and retry button
3. **Validation Errors**: Show inline validation messages on form fields
4. **Optimistic Update Failures**: Revert UI state and show error notification

## Testing Strategy

### Unit Tests

Unit tests should focus on specific examples, edge cases, and error conditions:

**Migration Tests:**
- Test migration with empty clash_proxies table
- Test migration with clash_proxies that all match existing nodes
- Test migration with clash_proxies that have no matching nodes
- Test migration rollback on validation failure
- Test backup table creation

**Protocol Mapping Tests:**
- Test each specific protocol mapping (shadowsocks→ss, vmess→vmess, etc.)
- Test unsupported protocol returns None
- Test case sensitivity in protocol names

**Configuration Generation Tests:**
- Test empty node list generates empty proxies array
- Test single node with include_in_clash=true
- Test single node with include_in_clash=false
- Test multiple nodes with mixed include_in_clash values
- Test sort_order with ties (same sort_order, different names)

**API Tests:**
- Test node creation with new fields
- Test node update with only include_in_clash
- Test node update with only sort_order
- Test node update with both new fields
- Test validation errors for invalid field types
- Test removed endpoints return 404

**UI Component Tests:**
- Test toggle switch renders correctly
- Test sort order input renders correctly
- Test filter dropdown works
- Test info banner displays on Clash Config page
- Test proxy management tab is removed

### Property-Based Tests

Property-based tests should verify universal properties across all inputs. Each test should run a minimum of 100 iterations.

**Property Test 1: Node field preservation**
- Generate random nodes with all fields populated
- Run migration simulation
- Verify all original fields unchanged
- **Tag: Feature: node-proxy-unification, Property 1: Node field preservation during migration**

**Property Test 2: Clash proxy matching**
- Generate random nodes and clash_proxies
- Test matching algorithm
- Verify matches only when all four fields equal
- **Tag: Feature: node-proxy-unification, Property 2: Clash proxy matching algorithm**

**Property Test 3: Include-in-clash filtering**
- Generate random nodes with random include_in_clash values
- Generate Clash config
- Verify all proxies come from nodes with include_in_clash=true
- **Tag: Feature: node-proxy-unification, Property 3: Include-in-clash filtering**

**Property Test 4: Sort order preservation**
- Generate random nodes with random sort_order values
- Generate Clash config
- Verify proxies ordered by sort_order ascending
- **Tag: Feature: node-proxy-unification, Property 4: Sort order preservation**

**Property Test 5: Protocol mapping**
- Generate random nodes with supported protocols
- Generate Clash config
- Verify protocol mapping correctness for all nodes
- **Tag: Feature: node-proxy-unification, Property 5: Protocol mapping correctness**

**Property Test 6: Field mapping**
- Generate random nodes
- Generate Clash config
- Verify name, host, port mapped correctly for all proxies
- **Tag: Feature: node-proxy-unification, Property 6: Field mapping correctness**

**Property Test 7: Configuration structure**
- Generate random nodes and proxy groups/rules
- Generate Clash config
- Verify structure contains required keys
- **Tag: Feature: node-proxy-unification, Property 7: Configuration structure compatibility**

**Property Test 8: Unsupported protocol handling**
- Generate random nodes including some with invalid protocols
- Generate Clash config
- Verify invalid protocol nodes excluded
- **Tag: Feature: node-proxy-unification, Property 8: Unsupported protocol handling**

**Property Test 9: Secret field mapping**
- Generate random nodes with various protocols
- Generate Clash config
- Verify secret mapped to password or uuid based on protocol
- **Tag: Feature: node-proxy-unification, Property 9: Secret field mapping by protocol**

**Property Test 10: Config preservation**
- Generate random nodes with random config JSON
- Generate Clash config
- Verify all config key-value pairs present in proxy config
- **Tag: Feature: node-proxy-unification, Property 10: Protocol-specific configuration preservation**

**Property Test 11: API accepts Clash fields**
- Generate random node update requests with new fields
- Send to API
- Verify values persisted correctly
- **Tag: Feature: node-proxy-unification, Property 11: Node update accepts Clash fields**

**Property Test 12: API backward compatibility**
- Generate random proxy group and rule requests
- Send to API
- Verify responses match pre-unification behavior
- **Tag: Feature: node-proxy-unification, Property 12: Proxy groups and rules API compatibility**

**Property Test 13: Sort order validation**
- Generate random node updates with negative sort_order
- Send to API
- Verify all rejected with validation error
- **Tag: Feature: node-proxy-unification, Property 13: Sort order validation**

**Property Test 14: Include-in-clash validation**
- Generate random node updates with non-boolean include_in_clash
- Send to API
- Verify all rejected with validation error
- **Tag: Feature: node-proxy-unification, Property 14: Include-in-clash validation**

**Property Test 15: Toggle updates backend**
- Generate random nodes
- Simulate toggle changes in UI
- Verify database updated correctly for all nodes
- **Tag: Feature: node-proxy-unification, Property 15: Toggle updates backend immediately**

**Property Test 16: Sort order updates**
- Generate random nodes
- Simulate sort_order changes in UI
- Verify database and UI ordering updated correctly
- **Tag: Feature: node-proxy-unification, Property 16: Sort order updates backend and UI**

**Property Test 17: Filter functionality**
- Generate random nodes with mixed include_in_clash values
- Apply various filter states
- Verify displayed nodes match filter criteria
- **Tag: Feature: node-proxy-unification, Property 17: Filter by include-in-clash status**

**Property Test 18: Config preview accuracy**
- Generate random nodes with mixed include_in_clash values
- Generate config preview
- Verify preview proxies match nodes with include_in_clash=true
- **Tag: Feature: node-proxy-unification, Property 18: Config preview shows nodes as proxies**

### Integration Tests

- Test full migration process with real database
- Test Clash config generation end-to-end
- Test admin UI workflows (create node → toggle include → preview config)
- Test API endpoints with real database
- Test backward compatibility with existing Clash clients

### Testing Tools

- **Backend**: Rust with `cargo test`, `sqlx` for database tests, `proptest` for property-based testing
- **Frontend**: Vitest for unit tests, Vue Test Utils for component tests, `fast-check` for property-based testing
- **Integration**: Actix-web test utilities, test database with migrations
- **E2E**: Playwright or Cypress for full user workflows
