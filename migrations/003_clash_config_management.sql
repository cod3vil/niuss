-- Clash Configuration Management Tables
-- This migration adds tables for managing Clash proxies, proxy groups, and rules

-- Clash proxies table (manages individual proxy configurations)
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

CREATE INDEX idx_clash_proxies_is_active ON clash_proxies(is_active);
CREATE INDEX idx_clash_proxies_sort_order ON clash_proxies(sort_order);
CREATE INDEX idx_clash_proxies_type ON clash_proxies(type);

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
CREATE TRIGGER update_clash_proxies_updated_at BEFORE UPDATE ON clash_proxies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

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

COMMENT ON TABLE clash_proxies IS 'Clash代理配置表';
COMMENT ON TABLE clash_proxy_groups IS 'Clash代理组配置表';
COMMENT ON TABLE clash_rules IS 'Clash路由规则表';
