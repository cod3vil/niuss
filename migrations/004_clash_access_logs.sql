-- Clash Access Logs Table
-- This migration adds a table for tracking access attempts to Clash subscription URLs

-- Create clash_access_logs table
CREATE TABLE clash_access_logs (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subscription_token VARCHAR(64) NOT NULL,
    access_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address VARCHAR(45) NOT NULL,
    user_agent TEXT,
    response_status VARCHAR(20) NOT NULL CHECK (response_status IN ('success', 'failed', 'quota_exceeded', 'expired', 'disabled')),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX idx_clash_access_logs_user_id ON clash_access_logs(user_id);
CREATE INDEX idx_clash_access_logs_access_timestamp ON clash_access_logs(access_timestamp);
CREATE INDEX idx_clash_access_logs_response_status ON clash_access_logs(response_status);
CREATE INDEX idx_clash_access_logs_subscription_token ON clash_access_logs(subscription_token);

COMMENT ON TABLE clash_access_logs IS 'Clash订阅访问日志表';
COMMENT ON COLUMN clash_access_logs.user_id IS '用户ID';
COMMENT ON COLUMN clash_access_logs.subscription_token IS '订阅令牌';
COMMENT ON COLUMN clash_access_logs.access_timestamp IS '访问时间戳';
COMMENT ON COLUMN clash_access_logs.ip_address IS '客户端IP地址（支持IPv4和IPv6）';
COMMENT ON COLUMN clash_access_logs.user_agent IS '客户端User-Agent';
COMMENT ON COLUMN clash_access_logs.response_status IS '响应状态：success-成功, failed-失败, quota_exceeded-流量超限, expired-已过期, disabled-已禁用';

-- Rollback migration
-- To rollback this migration, run the following commands:
-- DROP INDEX IF EXISTS idx_clash_access_logs_subscription_token;
-- DROP INDEX IF EXISTS idx_clash_access_logs_response_status;
-- DROP INDEX IF EXISTS idx_clash_access_logs_access_timestamp;
-- DROP INDEX IF EXISTS idx_clash_access_logs_user_id;
-- DROP TABLE IF EXISTS clash_access_logs;
