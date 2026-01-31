-- Rollback Migration for Clash Access Logs Table
-- This script removes the clash_access_logs table and all associated indexes

-- Drop indexes first
DROP INDEX IF EXISTS idx_clash_access_logs_subscription_token;
DROP INDEX IF EXISTS idx_clash_access_logs_response_status;
DROP INDEX IF EXISTS idx_clash_access_logs_access_timestamp;
DROP INDEX IF EXISTS idx_clash_access_logs_user_id;

-- Drop the table (CASCADE will handle foreign key constraints)
DROP TABLE IF EXISTS clash_access_logs;
