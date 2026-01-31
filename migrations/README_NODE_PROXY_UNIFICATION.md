# Node-Proxy Unification Migration

## Overview

This migration consolidates the dual management of VPN server information (nodes and clash_proxies tables) into a unified system using the nodes table as the single source of truth.

## Files Created

### Migration Files

1. **migrations/005_node_proxy_unification.sql**
   - Main migration script that adds `include_in_clash` and `sort_order` columns to the nodes table
   - Creates backup of clash_proxies table before migration
   - Matches and merges clash_proxies data into nodes based on name, host, port, and protocol
   - Creates new nodes for unmatched clash_proxies
   - Validates migration success before dropping clash_proxies table
   - Includes transaction management for rollback on failure

2. **migrations/005_node_proxy_unification_rollback.sql**
   - Rollback script to restore the system to pre-migration state
   - Recreates clash_proxies table from backup
   - Removes new columns from nodes table
   - Restores all indexes and triggers

### Test Files

3. **api/tests/node_proxy_unification_migration_test.rs**
   - Comprehensive test suite for migration validation
   - Includes property-based tests (PBT) and unit tests
   - Tests cover:
     - Node field preservation during migration (Property 1)
     - Clash proxy matching algorithm (Property 2)
     - Edge cases: empty tables, all matches, no matches, rollback scenarios

## Migration Details

### Schema Changes

The migration adds two new columns to the `nodes` table:

```sql
ALTER TABLE nodes ADD COLUMN include_in_clash BOOLEAN DEFAULT false;
ALTER TABLE nodes ADD COLUMN sort_order INTEGER DEFAULT 0;
CREATE INDEX idx_nodes_clash_inclusion ON nodes(include_in_clash, sort_order);
```

### Protocol Mapping

The migration maps clash_proxies protocol types to node protocols:

| Clash Proxy Type | Node Protocol |
|------------------|---------------|
| ss               | shadowsocks   |
| vmess            | vmess         |
| trojan           | trojan        |
| hysteria2        | hysteria2     |
| vless            | vless         |

### Matching Logic

Clash proxies are matched to existing nodes based on:
1. Name equality
2. Server/Host equality
3. Port equality
4. Protocol mapping (as shown above)

### Data Migration Steps

1. **Backup**: Create `clash_proxies_backup` table
2. **Add Columns**: Add `include_in_clash` and `sort_order` to nodes
3. **Update Existing Nodes**: Match and update nodes with clash_proxies data
4. **Create New Nodes**: Insert nodes for unmatched clash_proxies
5. **Validate**: Ensure all active proxies were migrated
6. **Drop Table**: Remove clash_proxies table

### Validation

The migration includes validation logic that:
- Counts active clash_proxies before migration
- Counts nodes with include_in_clash=true after migration
- Raises an exception if counts don't match
- Ensures no data loss during migration

## Running the Migration

### Prerequisites

- PostgreSQL database with existing schema (migrations 001-004 applied)
- Backup of production database (recommended)
- Database user with appropriate permissions

### Execution

```bash
# Run the migration
psql -U vpn_user -d vpn_platform -f migrations/005_node_proxy_unification.sql

# If rollback is needed
psql -U vpn_user -d vpn_platform -f migrations/005_node_proxy_unification_rollback.sql
```

### Verification

After running the migration, verify:

1. **Nodes table has new columns**:
   ```sql
   SELECT column_name, data_type 
   FROM information_schema.columns 
   WHERE table_name = 'nodes' 
   AND column_name IN ('include_in_clash', 'sort_order');
   ```

2. **Clash_proxies table is dropped**:
   ```sql
   SELECT table_name 
   FROM information_schema.tables 
   WHERE table_name = 'clash_proxies';
   -- Should return no rows
   ```

3. **Data was migrated**:
   ```sql
   SELECT COUNT(*) FROM nodes WHERE include_in_clash = true;
   -- Should match the number of active clash_proxies before migration
   ```

4. **Backup table exists**:
   ```sql
   SELECT COUNT(*) FROM clash_proxies_backup;
   -- Should match the original clash_proxies count
   ```

## Testing

### Running Tests

The test suite requires a running PostgreSQL database:

```bash
# Set database URL
export DATABASE_URL="postgres://vpn_user:vpn_password@localhost/vpn_platform"

# Run all migration tests
cd api
cargo test --test node_proxy_unification_migration_test -- --ignored --test-threads=1

# Run specific test
cargo test --test node_proxy_unification_migration_test test_migration_empty_clash_proxies_table -- --ignored
```

### Test Coverage

**Property-Based Tests (PBT)**:
- Property 1: Node field preservation during migration (100 test cases)
- Property 2: Clash proxy matching algorithm (100 test cases)
- Protocol mapping bijection test (50 test cases)

**Unit Tests**:
- Empty clash_proxies table scenario
- All proxies match existing nodes
- No proxies match existing nodes
- Migration rollback on failure

## Rollback Procedure

If issues are discovered after migration:

1. **Stop all services** that access the database
2. **Run rollback script**:
   ```bash
   psql -U vpn_user -d vpn_platform -f migrations/005_node_proxy_unification_rollback.sql
   ```
3. **Verify rollback**:
   - Check that clash_proxies table exists
   - Check that nodes table no longer has include_in_clash/sort_order columns
   - Verify data integrity
4. **Restart services**

## Post-Migration Tasks

After successful migration:

1. **Update application code** to use new node fields instead of clash_proxies
2. **Update API endpoints** to accept include_in_clash and sort_order parameters
3. **Update admin UI** to display and manage new fields
4. **Update Clash config generation** to read from nodes table
5. **Remove clash_proxies_backup table** after confirming stability (optional, recommended to keep for a while)

## Troubleshooting

### Migration Fails with Validation Error

If the migration fails with "Migration validation failed":
- Check the error message for expected vs actual counts
- Investigate which proxies were not migrated
- Verify protocol mapping is correct
- Check for data inconsistencies in clash_proxies table

### Rollback Fails

If rollback fails:
- Check that clash_proxies_backup table exists
- Verify database user has necessary permissions
- Check for foreign key constraints
- Review PostgreSQL logs for detailed error messages

### Performance Issues

If migration is slow on large datasets:
- Consider running during maintenance window
- Monitor database CPU and memory usage
- Check index creation time
- Consider batching the INSERT operations for very large datasets

## Requirements Validated

This migration implementation validates the following requirements:

- **Requirement 1.1**: Nodes table includes include_in_clash boolean field
- **Requirement 1.2**: Nodes table includes sort_order integer field
- **Requirement 1.3**: Data transfer from clash_proxies to nodes
- **Requirement 1.4**: New node entries for unmatched proxies
- **Requirement 1.5**: Clash_proxies table dropped after successful migration
- **Requirement 1.6**: All existing node fields preserved
- **Requirement 6.1**: Backup table created before migration
- **Requirement 6.2**: Rollback logic for migration failures
- **Requirement 6.3**: Migration summary logged
- **Requirement 6.4**: Validation before dropping clash_proxies
- **Requirement 6.5**: Matching criteria (name, host, port, protocol)

## Next Steps

After completing this migration, proceed with:

1. Task 2: Update backend data models and database queries
2. Task 3: Implement protocol mapping and Clash config generation
3. Task 4: Checkpoint - Run migration and test backend changes
4. Task 5: Update API endpoints and handlers
5. Task 6: Update admin UI - Nodes management view
6. Task 7: Update admin UI - Clash Config view
7. Task 8: Update TypeScript types and API client
8. Task 9: Final checkpoint - Integration testing and validation
