-- Node-Proxy Unification Migration
-- This migration consolidates clash_proxies into the nodes table
-- by adding Clash-specific fields to nodes and migrating data

BEGIN;

-- Step 1: Create backup table for clash_proxies
CREATE TABLE clash_proxies_backup AS SELECT * FROM clash_proxies;

-- Step 2: Add new columns to nodes table
ALTER TABLE nodes ADD COLUMN include_in_clash BOOLEAN DEFAULT false;
ALTER TABLE nodes ADD COLUMN sort_order INTEGER DEFAULT 0;

-- Step 3: Create index for Clash-related queries
CREATE INDEX idx_nodes_clash_inclusion ON nodes(include_in_clash, sort_order);

-- Step 4: Match and update existing nodes with clash_proxies data
-- Match by: name, server/host, port, and protocol mapping
UPDATE nodes n
SET 
  include_in_clash = cp.is_active,
  sort_order = cp.sort_order
FROM clash_proxies cp
WHERE 
  n.name = cp.name 
  AND n.host = cp.server 
  AND n.port = cp.port
  AND (
    (n.protocol = 'shadowsocks' AND cp.type = 'ss') OR
    (n.protocol = 'vmess' AND cp.type = 'vmess') OR
    (n.protocol = 'trojan' AND cp.type = 'trojan') OR
    (n.protocol = 'hysteria2' AND cp.type = 'hysteria2') OR
    (n.protocol = 'vless' AND cp.type = 'vless')
  );

-- Step 5: Create nodes for unmatched clash_proxies
-- These are proxies that don't have corresponding nodes
INSERT INTO nodes (name, host, port, protocol, secret, config, include_in_clash, sort_order, status, max_users, current_users, total_upload, total_download)
SELECT 
  cp.name,
  cp.server,
  cp.port,
  CASE cp.type
    WHEN 'ss' THEN 'shadowsocks'
    WHEN 'vmess' THEN 'vmess'
    WHEN 'trojan' THEN 'trojan'
    WHEN 'hysteria2' THEN 'hysteria2'
    WHEN 'vless' THEN 'vless'
    ELSE cp.type
  END,
  COALESCE(cp.config->>'password', cp.config->>'uuid', ''), -- Extract secret from config
  cp.config,
  cp.is_active,
  cp.sort_order,
  'offline', -- Default status for newly created nodes
  1000, -- Default max_users
  0, -- Default current_users
  0, -- Default total_upload
  0  -- Default total_download
FROM clash_proxies cp
WHERE NOT EXISTS (
  SELECT 1 FROM nodes n
  WHERE n.name = cp.name 
    AND n.host = cp.server 
    AND n.port = cp.port
    AND (
      (n.protocol = 'shadowsocks' AND cp.type = 'ss') OR
      (n.protocol = 'vmess' AND cp.type = 'vmess') OR
      (n.protocol = 'trojan' AND cp.type = 'trojan') OR
      (n.protocol = 'hysteria2' AND cp.type = 'hysteria2') OR
      (n.protocol = 'vless' AND cp.type = 'vless')
    )
);

-- Step 6: Validate migration
-- Ensure all active clash_proxies have been transferred
DO $$
DECLARE
  proxy_count INTEGER;
  migrated_count INTEGER;
BEGIN
  SELECT COUNT(*) INTO proxy_count FROM clash_proxies WHERE is_active = true;
  SELECT COUNT(*) INTO migrated_count FROM nodes WHERE include_in_clash = true;
  
  IF migrated_count < proxy_count THEN
    RAISE EXCEPTION 'Migration validation failed: expected at least % active proxies, found % nodes with include_in_clash=true', proxy_count, migrated_count;
  END IF;
  
  RAISE NOTICE 'Migration validation passed: % active proxies migrated to % nodes', proxy_count, migrated_count;
END $$;

-- Step 7: Drop clash_proxies table
DROP TABLE clash_proxies;

COMMIT;

-- Rollback instructions (to be used manually if needed):
-- BEGIN;
-- CREATE TABLE clash_proxies AS SELECT * FROM clash_proxies_backup;
-- ALTER TABLE nodes DROP COLUMN include_in_clash;
-- ALTER TABLE nodes DROP COLUMN sort_order;
-- DROP INDEX IF EXISTS idx_nodes_clash_inclusion;
-- DROP TABLE clash_proxies_backup;
-- COMMIT;
