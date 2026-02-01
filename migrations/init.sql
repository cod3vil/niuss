-- ========================================
-- VPN Subscription Platform - Complete Database Schema
-- ========================================
-- This file consolidates all migrations into a single file
-- Generated: 2026-02-01
-- 
-- Migrations included:
-- - 001_init.sql: Initial database schema
-- - 002_seed_test_data.sql: Test data (optional, for development only)
-- - 003_clash_config_management.sql: Clash configuration tables
-- - 004_clash_access_logs.sql: Clash access logging
-- - 005_node_proxy_unification.sql: Node-proxy unification
-- ========================================

-- ========================================
-- MIGRATION 001: Initial Database Schema
-- ========================================

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    coin_balance BIGINT DEFAULT 0 CHECK (coin_balance >= 0),
    traffic_quota BIGINT DEFAULT 0 CHECK (traffic_quota >= 0),
    traffic_used BIGINT DEFAULT 0 CHECK (traffic_used >= 0),
    referral_code VARCHAR(32) UNIQUE,
    referred_by BIGINT REFERENCES users(id) ON DELETE SET NULL,
    status VARCHAR(20) DEFAULT 'active' CHECK (status IN ('active', 'disabled')),
    is_admin BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_referral_code ON users(referral_code);
CREATE INDEX idx_users_status ON users(status);

-- Packages table
CREATE TABLE packages (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    traffic_amount BIGINT NOT NULL CHECK (traffic_amount > 0),
    price BIGINT NOT NULL CHECK (price >= 0),
    duration_days INT NOT NULL CHECK (duration_days > 0),
    description TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_packages_is_active ON packages(is_active);

-- Orders table
CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    order_no VARCHAR(64) UNIQUE NOT NULL,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    package_id BIGINT NOT NULL REFERENCES packages(id) ON DELETE RESTRICT,
    amount BIGINT NOT NULL CHECK (amount >= 0),
    status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'completed', 'failed')),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_orders_user_id ON orders(user_id);
CREATE INDEX idx_orders_created_at ON orders(created_at);
CREATE INDEX idx_orders_status ON orders(status);

-- User packages table
CREATE TABLE user_packages (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    package_id BIGINT NOT NULL REFERENCES packages(id) ON DELETE RESTRICT,
    order_id BIGINT NOT NULL REFERENCES orders(id) ON DELETE RESTRICT,
    traffic_quota BIGINT NOT NULL CHECK (traffic_quota > 0),
    traffic_used BIGINT DEFAULT 0 CHECK (traffic_used >= 0),
    expires_at TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) DEFAULT 'active' CHECK (status IN ('active', 'expired', 'exhausted')),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_user_packages_user_id ON user_packages(user_id);
CREATE INDEX idx_user_packages_expires_at ON user_packages(expires_at);
CREATE INDEX idx_user_packages_status ON user_packages(status);

-- Nodes table
CREATE TABLE nodes (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    host VARCHAR(255) NOT NULL,
    port INT NOT NULL CHECK (port > 0 AND port <= 65535),
    protocol VARCHAR(20) NOT NULL CHECK (protocol IN ('shadowsocks', 'vmess', 'trojan', 'hysteria2', 'vless')),
    secret VARCHAR(255) NOT NULL,
    config JSONB NOT NULL,
    status VARCHAR(20) DEFAULT 'offline' CHECK (status IN ('online', 'offline', 'maintenance')),
    max_users INT DEFAULT 1000 CHECK (max_users > 0),
    current_users INT DEFAULT 0 CHECK (current_users >= 0),
    total_upload BIGINT DEFAULT 0 CHECK (total_upload >= 0),
    total_download BIGINT DEFAULT 0 CHECK (total_download >= 0),
    last_heartbeat TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_nodes_status ON nodes(status);
CREATE INDEX idx_nodes_protocol ON nodes(protocol);

-- Traffic logs table
CREATE TABLE traffic_logs (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    node_id BIGINT NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    upload BIGINT NOT NULL CHECK (upload >= 0),
    download BIGINT NOT NULL CHECK (download >= 0),
    recorded_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_traffic_logs_user_id ON traffic_logs(user_id);
CREATE INDEX idx_traffic_logs_node_id ON traffic_logs(node_id);
CREATE INDEX idx_traffic_logs_recorded_at ON traffic_logs(recorded_at);

-- Subscriptions table
CREATE TABLE subscriptions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    token VARCHAR(64) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_accessed TIMESTAMPTZ
);

CREATE INDEX idx_subscriptions_token ON subscriptions(token);
CREATE INDEX idx_subscriptions_user_id ON subscriptions(user_id);

-- Coin transactions table
CREATE TABLE coin_transactions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount BIGINT NOT NULL,
    type VARCHAR(20) NOT NULL CHECK (type IN ('recharge', 'purchase', 'referral', 'refund', 'adjustment')),
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_coin_transactions_user_id ON coin_transactions(user_id);
CREATE INDEX idx_coin_transactions_type ON coin_transactions(type);
CREATE INDEX idx_coin_transactions_created_at ON coin_transactions(created_at);

-- Admin logs table
CREATE TABLE admin_logs (
    id BIGSERIAL PRIMARY KEY,
    admin_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action VARCHAR(50) NOT NULL,
    target_type VARCHAR(50),
    target_id BIGINT,
    details JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_admin_logs_admin_id ON admin_logs(admin_id);
CREATE INDEX idx_admin_logs_created_at ON admin_logs(created_at);
CREATE INDEX idx_admin_logs_action ON admin_logs(action);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$ language 'plpgsql';

-- Triggers for updated_at
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_packages_updated_at BEFORE UPDATE ON packages
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_nodes_updated_at BEFORE UPDATE ON nodes
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default packages
INSERT INTO packages (name, traffic_amount, price, duration_days, description) VALUES
    ('体验套餐', 10737418240, 100, 30, '10GB 流量，有效期 30 天'),
    ('标准套餐', 53687091200, 500, 30, '50GB 流量，有效期 30 天'),
    ('高级套餐', 107374182400, 900, 30, '100GB 流量，有效期 30 天'),
    ('旗舰套餐', 536870912000, 4000, 90, '500GB 流量，有效期 90 天');

-- Create default admin user (password: admin123)
-- IMPORTANT: Change this password immediately after deployment!
INSERT INTO users (email, password_hash, is_admin, status) VALUES
    ('admin@example.com', '$argon2id$v=19$m=19456,t=2,p=1$6HqspZKtuGhEhGzqaKfWvA$vh5qa/0HFo6HIhbywr0nkr/voSPNNsdbqM6vA6o2XKU', true, 'active');

COMMENT ON TABLE users IS '用户表';
COMMENT ON TABLE packages IS '流量套餐表';
COMMENT ON TABLE orders IS '订单表';
COMMENT ON TABLE user_packages IS '用户套餐关联表';
COMMENT ON TABLE nodes IS 'VPN 节点表';
COMMENT ON TABLE traffic_logs IS '流量日志表';
COMMENT ON TABLE subscriptions IS '订阅表';
COMMENT ON TABLE coin_transactions IS '金币交易记录表';
COMMENT ON TABLE admin_logs IS '管理员操作日志表';

-- ========================================
-- MIGRATION 003: Clash Configuration Management
-- ========================================

-- Clash proxy groups table (manages proxy group configurations)
CREATE TABLE clash_proxy_groups (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL UNIQUE,
    type VARCHAR(20) NOT NULL CHECK (type IN ('select', 'url-test', 'fallback', 'load-balance', 'relay')),
    proxies TEXT[] NOT NULL DEFAULT '{}',
    url VARCHAR(255),
    interval INT,
    tolerance INT,
    is_active BOOLEAN DEFAULT true,
    sort_order INT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_clash_proxy_groups_is_active ON clash_proxy_groups(is_active);
CREATE INDEX idx_clash_proxy_groups_sort_order ON clash_proxy_groups(sort_order);
CREATE INDEX idx_clash_proxy_groups_type ON clash_proxy_groups(type);

-- Clash rules table (manages routing rules)
CREATE TABLE clash_rules (
    id BIGSERIAL PRIMARY KEY,
    rule_type VARCHAR(50) NOT NULL CHECK (rule_type IN (
        'DOMAIN', 'DOMAIN-SUFFIX', 'DOMAIN-KEYWORD', 
        'IP-CIDR', 'IP-CIDR6', 'SRC-IP-CIDR', 
        'GEOIP', 'DST-PORT', 'SRC-PORT',
        'PROCESS-NAME', 'MATCH'
    )),
    rule_value VARCHAR(255),
    proxy_group VARCHAR(100) NOT NULL,
    no_resolve BOOLEAN DEFAULT false,
    is_active BOOLEAN DEFAULT true,
    sort_order INT DEFAULT 0,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_clash_rules_is_active ON clash_rules(is_active);
CREATE INDEX idx_clash_rules_sort_order ON clash_rules(sort_order);
CREATE INDEX idx_clash_rules_rule_type ON clash_rules(rule_type);
CREATE INDEX idx_clash_rules_proxy_group ON clash_rules(proxy_group);

-- Triggers for updated_at
CREATE TRIGGER update_clash_proxy_groups_updated_at BEFORE UPDATE ON clash_proxy_groups
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_clash_rules_updated_at BEFORE UPDATE ON clash_rules
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default proxy groups
INSERT INTO clash_proxy_groups (name, type, proxies, sort_order) VALUES
    ('直接连接', 'select', ARRAY['DIRECT'], 0),
    ('国外流量', 'select', ARRAY['DIRECT'], 1),
    ('其他流量', 'select', ARRAY['国外流量', '直接连接'], 2),
    ('Telegram', 'select', ARRAY['国外流量'], 3),
    ('Youtube', 'select', ARRAY['国外流量'], 4),
    ('Netflix', 'select', ARRAY['国外流量'], 5),
    ('哔哩哔哩', 'select', ARRAY['直接连接'], 6),
    ('ChatGPT及其他AI', 'select', ARRAY['国外流量'], 7),
    ('Steam', 'select', ARRAY['直接连接'], 8),
    ('国外媒体', 'select', ARRAY['国外流量'], 9),
    ('苹果服务', 'select', ARRAY['直接连接', '国外流量'], 10);

-- Insert default rules
INSERT INTO clash_rules (rule_type, rule_value, proxy_group, sort_order, description) VALUES
    ('PROCESS-NAME', 'com.ximalaya.ting.himalaya', '国外流量', 0, 'Himalaya Podcast'),
    ('DOMAIN-SUFFIX', 'himalaya.com', '国外流量', 1, 'Himalaya'),
    ('DOMAIN-SUFFIX', 'baidu.com', '直接连接', 2, 'Baidu'),
    ('DOMAIN-SUFFIX', 'baidubcr.com', '直接连接', 3, 'Baidu BCR'),
    ('DOMAIN-SUFFIX', 'baidupan.com', '直接连接', 4, 'Baidu Pan'),
    ('DOMAIN-SUFFIX', 'baidupcs.com', '直接连接', 5, 'Baidu PCS'),
    ('DOMAIN-SUFFIX', 'bdimg.com', '直接连接', 6, 'Baidu Images'),
    ('DOMAIN-SUFFIX', 'bdstatic.com', '直接连接', 7, 'Baidu Static'),
    ('DOMAIN-SUFFIX', 'yunjiasu-cdn.net', '直接连接', 8, 'Yunjiasu CDN'),
    ('IP-CIDR', '185.25.183.179/25', '直接连接', 9, 'Specific IP range'),
    ('GEOIP', 'CN', '直接连接', 10, 'China IP'),
    ('MATCH', '', '其他流量', 999, 'Default match all');

COMMENT ON TABLE clash_proxy_groups IS 'Clash代理组配置表';
COMMENT ON TABLE clash_rules IS 'Clash路由规则表';

-- ========================================
-- MIGRATION 004: Clash Access Logs
-- ========================================

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

-- ========================================
-- MIGRATION 005: Node-Proxy Unification
-- ========================================

-- Add new columns to nodes table for Clash integration
ALTER TABLE nodes ADD COLUMN include_in_clash BOOLEAN DEFAULT false;
ALTER TABLE nodes ADD COLUMN sort_order INTEGER DEFAULT 0;

-- Create index for Clash-related queries
CREATE INDEX idx_nodes_clash_inclusion ON nodes(include_in_clash, sort_order);

-- ========================================
-- END OF MIGRATIONS
-- ========================================
