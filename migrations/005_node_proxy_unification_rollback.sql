-- Rollback for Node-Proxy Unification Migration
-- This script restores the clash_proxies table and removes the new columns from nodes

BEGIN;

-- Step 1: Restore clash_proxies table from backup
CREATE TABLE clash_proxies (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    type VARCHAR(20) NOT NULL CHECK (type IN ('ss', 'vmess', 'trojan', 'hysteria2', 'vless')),
    server VARCHAR(255) NOT NULL,
    port INT NOT NULL CHECK (port > 0 AND port <= 65535),
    config JSONB NOT NULL,
    is_active BOOLEAN DEFAULT true,
    sort_order INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Step 2: Restore data from backup table
INSERT INTO clash_proxies SELECT * FROM clash_proxies_backup;

-- Step 3: Recreate indexes
CREATE INDEX idx_clash_proxies_is_active ON clash_proxies(is_active);
CREATE INDEX idx_clash_proxies_sort_order ON clash_proxies(sort_order);
CREATE INDEX idx_clash_proxies_type ON clash_proxies(type);

-- Step 4: Recreate trigger
CREATE TRIGGER update_clash_proxies_updated_at BEFORE UPDATE ON clash_proxies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Step 5: Remove new columns from nodes table
ALTER TABLE nodes DROP COLUMN IF EXISTS include_in_clash;
ALTER TABLE nodes DROP COLUMN IF EXISTS sort_order;

-- Step 6: Drop index
DROP INDEX IF EXISTS idx_nodes_clash_inclusion;

-- Step 7: Drop backup table
DROP TABLE IF EXISTS clash_proxies_backup;

COMMIT;
