-- VPN Subscription Platform Database Schema
-- This migration creates all necessary tables for the system

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
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

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
-- Note: This is a placeholder hash, will be replaced with proper hash in production
INSERT INTO users (email, password_hash, is_admin, status) VALUES
    ('admin@example.com', '$argon2id$v=19$m=19456,t=2,p=1$placeholder$placeholder', true, 'active');

COMMENT ON TABLE users IS '用户表';
COMMENT ON TABLE packages IS '流量套餐表';
COMMENT ON TABLE orders IS '订单表';
COMMENT ON TABLE user_packages IS '用户套餐关联表';
COMMENT ON TABLE nodes IS 'VPN 节点表';
COMMENT ON TABLE traffic_logs IS '流量日志表';
COMMENT ON TABLE subscriptions IS '订阅表';
COMMENT ON TABLE coin_transactions IS '金币交易记录表';
COMMENT ON TABLE admin_logs IS '管理员操作日志表';
