# Requirements Document

## Introduction

This specification addresses the unification of node and proxy management in the VPN subscription platform. Currently, the system maintains two separate tables (nodes and clash_proxies) for essentially the same data, leading to duplication, manual synchronization, and potential inconsistencies. This feature will consolidate proxy server information into a single source of truth using the nodes table, eliminate the clash_proxies table, and simplify the admin UI.

## Glossary

- **Node**: A VPN server instance managed by the Node Agent that provides VPN services to users
- **Proxy**: A server configuration entry used in Clash configuration files for client connections
- **Clash_Config**: A YAML configuration file format used by Clash clients to connect to VPN servers
- **Node_Agent**: The service component that manages VPN server operations and monitors node status
- **Admin_UI**: The web-based administrative interface for managing the VPN platform
- **Clash_Proxy_Table**: The existing clash_proxies database table that stores proxy configurations
- **Nodes_Table**: The existing nodes database table that stores VPN server information
- **Migration**: A database schema change operation that modifies table structure or moves data

## Requirements

### Requirement 1: Database Schema Unification

**User Story:** As a system administrator, I want a single source of truth for proxy server information, so that I can avoid data duplication and synchronization issues.

#### Acceptance Criteria

1. THE Nodes_Table SHALL include a boolean field named "include_in_clash" to indicate whether a node should be included in Clash configuration generation
2. THE Nodes_Table SHALL include an integer field named "sort_order" to control the display order of nodes in Clash configurations
3. WHEN the database migration is executed, THE Migration SHALL transfer all existing data from Clash_Proxy_Table to Nodes_Table where corresponding nodes exist
4. WHEN the database migration is executed, THE Migration SHALL create new node entries for any clash_proxies that do not have corresponding nodes
5. WHEN the database migration completes successfully, THE Migration SHALL drop the Clash_Proxy_Table
6. THE Nodes_Table SHALL preserve all existing node fields including name, host, port, protocol, secret, config, status, max_users, current_users, and traffic statistics

### Requirement 2: Clash Configuration Generation

**User Story:** As a system operator, I want Clash configuration generation to read directly from the nodes table, so that proxy configurations stay synchronized with active nodes.

#### Acceptance Criteria

1. WHEN generating a Clash configuration, THE System SHALL query only nodes where include_in_clash is true
2. WHEN generating a Clash configuration, THE System SHALL order nodes by the sort_order field
3. WHEN generating a Clash configuration, THE System SHALL map node protocol types (shadowsocks, vmess, trojan, hysteria2, vless) to Clash proxy types (ss, vmess, trojan, hysteria2, vless)
4. WHEN generating a Clash configuration, THE System SHALL construct proxy entries using node fields: name, host, port, protocol, secret, and config
5. WHEN a node's include_in_clash field is false, THE System SHALL exclude that node from generated Clash configurations
6. THE System SHALL maintain backward compatibility with existing Clash configuration structure for proxy groups and rules

### Requirement 3: Admin UI Node Management

**User Story:** As an administrator, I want to manage node inclusion in Clash configs from the nodes interface, so that I have a unified management experience.

#### Acceptance Criteria

1. WHEN viewing the nodes list, THE Admin_UI SHALL display an "Include in Clash" toggle column for each node
2. WHEN an administrator toggles the "Include in Clash" field, THE Admin_UI SHALL update the node's include_in_clash value immediately
3. WHEN viewing the nodes list, THE Admin_UI SHALL display a "Sort Order" field that can be edited inline or in a modal
4. WHEN an administrator changes the sort_order value, THE Admin_UI SHALL update the node's sort_order value and reflect the change in the list ordering
5. THE Admin_UI SHALL allow administrators to filter nodes by include_in_clash status
6. THE Admin_UI SHALL display all existing node fields alongside the new Clash-related fields

### Requirement 4: Admin UI Clash Config Simplification

**User Story:** As an administrator, I want the Clash Config interface to focus on proxy groups and rules, so that I don't have duplicate proxy management interfaces.

#### Acceptance Criteria

1. WHEN viewing the Clash Config page, THE Admin_UI SHALL NOT display a "代理管理" (Proxy Management) tab
2. WHEN viewing the Clash Config page, THE Admin_UI SHALL continue to display proxy groups management functionality
3. WHEN viewing the Clash Config page, THE Admin_UI SHALL continue to display rules management functionality
4. THE Admin_UI SHALL provide a clear indication or link that proxy management is now handled in the Nodes interface
5. WHEN generating or previewing Clash configurations, THE Admin_UI SHALL show proxies sourced from the Nodes_Table

### Requirement 5: API Endpoint Updates

**User Story:** As a developer, I want API endpoints to reflect the unified data model, so that the backend correctly serves the new structure.

#### Acceptance Criteria

1. THE System SHALL remove API endpoints that specifically managed clash_proxies as a separate entity
2. THE System SHALL update node management endpoints to accept include_in_clash and sort_order fields
3. WHEN a client requests Clash configuration generation, THE System SHALL use the updated logic that reads from Nodes_Table
4. THE System SHALL maintain API backward compatibility for proxy groups and rules endpoints
5. WHEN updating a node via API, THE System SHALL validate that sort_order is a non-negative integer
6. WHEN updating a node via API, THE System SHALL validate that include_in_clash is a boolean value

### Requirement 6: Data Migration Safety

**User Story:** As a system administrator, I want the migration to be safe and reversible, so that I can recover if issues occur.

#### Acceptance Criteria

1. WHEN the migration begins, THE Migration SHALL create a backup table containing all clash_proxies data
2. IF the migration encounters an error, THEN THE Migration SHALL rollback all changes and preserve existing data
3. WHEN the migration completes, THE Migration SHALL log a summary of all data transfers and transformations
4. THE Migration SHALL validate that all clash_proxies have been successfully transferred before dropping the Clash_Proxy_Table
5. WHEN matching clash_proxies to nodes, THE Migration SHALL use name, server/host, port, and protocol as matching criteria
6. IF a clash_proxy cannot be matched to an existing node, THEN THE Migration SHALL create a new node entry with appropriate default values

### Requirement 7: Protocol Mapping Consistency

**User Story:** As a system operator, I want consistent protocol naming between nodes and Clash configs, so that configurations are generated correctly.

#### Acceptance Criteria

1. THE System SHALL map node protocol "shadowsocks" to Clash proxy type "ss"
2. THE System SHALL map node protocol "vmess" to Clash proxy type "vmess"
3. THE System SHALL map node protocol "trojan" to Clash proxy type "trojan"
4. THE System SHALL map node protocol "hysteria2" to Clash proxy type "hysteria2"
5. THE System SHALL map node protocol "vless" to Clash proxy type "vless"
6. WHEN a node has an unsupported protocol, THE System SHALL log a warning and exclude that node from Clash configuration generation

### Requirement 8: Configuration Field Mapping

**User Story:** As a system operator, I want node configuration fields to map correctly to Clash proxy configurations, so that clients can connect successfully.

#### Acceptance Criteria

1. WHEN generating Clash proxy entries, THE System SHALL map node.name to proxy.name
2. WHEN generating Clash proxy entries, THE System SHALL map node.host to proxy.server
3. WHEN generating Clash proxy entries, THE System SHALL map node.port to proxy.port
4. WHEN generating Clash proxy entries, THE System SHALL map node.protocol to proxy.type using the protocol mapping rules
5. WHEN generating Clash proxy entries, THE System SHALL merge node.secret and node.config into the appropriate Clash proxy configuration fields
6. THE System SHALL preserve all protocol-specific configuration parameters during the mapping process
