-- Seed Test Data for Development
-- This migration adds test data for development and testing purposes
-- DO NOT run this in production!

-- Note: This file is optional and should only be used in development environments

BEGIN;

-- Insert test users
-- Password for all test users: "password123"
-- Hash generated with argon2: $argon2id$v=19$m=19456,t=2,p=1$test$test
INSERT INTO users (email, password_hash, coin_balance, traffic_quota, traffic_used, status, is_admin) VALUES
    ('user1@test.com', '$argon2id$v=19$m=19456,t=2,p=1$test$test', 1000, 10737418240, 1073741824, 'active', false),
    ('user2@test.com', '$argon2id$v=19$m=19456,t=2,p=1$test$test', 500, 53687091200, 5368709120, 'active', false),
    ('user3@test.com', '$argon2id$v=19$m=19456,t=2,p=1$test$test', 0, 0, 0, 'active', false),
    ('disabled@test.com', '$argon2id$v=19$m=19456,t=2,p=1$test$test', 100, 10737418240, 0, 'disabled', false)
ON CONFLICT (email) DO NOTHING;

-- Generate referral codes for test users
UPDATE users SET referral_code = 'REF' || LPAD(id::text, 6, '0') WHERE referral_code IS NULL;

-- Insert test nodes
INSERT INTO nodes (name, host, port, protocol, secret, config, status, max_users) VALUES
    (
        'Test Node 1 - Tokyo',
        'node1.test.com',
        443,
        'vless',
        'test-secret-1',
        '{
            "flow": "xtls-rprx-vision",
            "encryption": "none",
            "network": "tcp",
            "security": "reality",
            "reality_config": {
                "dest": "www.microsoft.com:443",
                "server_names": ["www.microsoft.com"],
                "private_key": "test-private-key-1",
                "public_key": "test-public-key-1",
                "short_ids": [""]
            }
        }'::jsonb,
        'online',
        1000
    ),
    (
        'Test Node 2 - Singapore',
        'node2.test.com',
        443,
        'vmess',
        'test-secret-2',
        '{
            "alter_id": 0,
            "network": "ws",
            "path": "/vmess",
            "security": "tls"
        }'::jsonb,
        'online',
        1000
    ),
    (
        'Test Node 3 - US',
        'node3.test.com',
        443,
        'trojan',
        'test-secret-3',
        '{
            "network": "tcp",
            "security": "tls",
            "sni": "node3.test.com"
        }'::jsonb,
        'offline',
        500
    )
ON CONFLICT DO NOTHING;

-- Update node heartbeats
UPDATE nodes SET last_heartbeat = NOW() WHERE status = 'online';

-- Insert test orders for user1
INSERT INTO orders (order_no, user_id, package_id, amount, status, created_at, completed_at)
SELECT 
    'TEST-' || TO_CHAR(NOW(), 'YYYYMMDD') || '-' || LPAD(seq::text, 6, '0'),
    (SELECT id FROM users WHERE email = 'user1@test.com'),
    p.id,
    p.price,
    'completed',
    NOW() - (seq || ' days')::interval,
    NOW() - (seq || ' days')::interval + interval '1 minute'
FROM packages p, generate_series(1, 2) seq
WHERE p.name IN ('体验套餐', '标准套餐')
ON CONFLICT (order_no) DO NOTHING;

-- Insert user packages for user1
INSERT INTO user_packages (user_id, package_id, order_id, traffic_quota, traffic_used, expires_at, status)
SELECT 
    (SELECT id FROM users WHERE email = 'user1@test.com'),
    o.package_id,
    o.id,
    p.traffic_amount,
    CASE 
        WHEN p.name = '体验套餐' THEN p.traffic_amount / 2
        ELSE 0
    END,
    o.completed_at + (p.duration_days || ' days')::interval,
    CASE 
        WHEN o.completed_at + (p.duration_days || ' days')::interval > NOW() THEN 'active'
        ELSE 'expired'
    END
FROM orders o
JOIN packages p ON o.package_id = p.id
WHERE o.user_id = (SELECT id FROM users WHERE email = 'user1@test.com')
ON CONFLICT DO NOTHING;

-- Insert subscriptions for test users
INSERT INTO subscriptions (user_id, token, created_at, last_accessed)
SELECT 
    id,
    'TEST-TOKEN-' || LPAD(id::text, 10, '0'),
    created_at,
    NOW() - interval '1 hour'
FROM users
WHERE email LIKE '%@test.com'
ON CONFLICT (user_id) DO NOTHING;

-- Insert coin transactions
INSERT INTO coin_transactions (user_id, amount, type, description, created_at)
SELECT 
    (SELECT id FROM users WHERE email = 'user1@test.com'),
    amount,
    type,
    description,
    NOW() - (seq || ' days')::interval
FROM (VALUES
    (1000, 'recharge', 'Initial recharge', 5),
    (-100, 'purchase', 'Purchased 体验套餐', 4),
    (-500, 'purchase', 'Purchased 标准套餐', 2),
    (50, 'referral', 'Referral bonus from user2', 1)
) AS t(amount, type, description, seq)
ON CONFLICT DO NOTHING;

-- Insert traffic logs
INSERT INTO traffic_logs (user_id, node_id, upload, download, recorded_at)
SELECT 
    u.id,
    n.id,
    (random() * 1000000000)::bigint,
    (random() * 5000000000)::bigint,
    NOW() - (seq || ' hours')::interval
FROM 
    users u,
    nodes n,
    generate_series(1, 24) seq
WHERE 
    u.email = 'user1@test.com'
    AND n.status = 'online'
LIMIT 48
ON CONFLICT DO NOTHING;

-- Insert admin logs
INSERT INTO admin_logs (admin_id, action, target_type, target_id, details, created_at)
SELECT 
    (SELECT id FROM users WHERE email = 'admin@example.com'),
    action,
    target_type,
    target_id,
    details::jsonb,
    NOW() - (seq || ' days')::interval
FROM (VALUES
    ('create_node', 'node', 1, '{"name": "Test Node 1 - Tokyo"}', 5),
    ('create_node', 'node', 2, '{"name": "Test Node 2 - Singapore"}', 4),
    ('disable_user', 'user', (SELECT id FROM users WHERE email = 'disabled@test.com'), '{"reason": "Test disable"}', 3),
    ('adjust_balance', 'user', (SELECT id FROM users WHERE email = 'user1@test.com'), '{"old_balance": 0, "new_balance": 1000, "reason": "Test adjustment"}', 2)
) AS t(action, target_type, target_id, details, seq)
ON CONFLICT DO NOTHING;

COMMIT;

-- Display summary
DO $$
DECLARE
    user_count INTEGER;
    node_count INTEGER;
    order_count INTEGER;
    package_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO user_count FROM users WHERE email LIKE '%@test.com';
    SELECT COUNT(*) INTO node_count FROM nodes WHERE name LIKE 'Test Node%';
    SELECT COUNT(*) INTO order_count FROM orders WHERE order_no LIKE 'TEST-%';
    SELECT COUNT(*) INTO package_count FROM packages;
    
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Test Data Seeded Successfully!';
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Test Users: %', user_count;
    RAISE NOTICE 'Test Nodes: %', node_count;
    RAISE NOTICE 'Test Orders: %', order_count;
    RAISE NOTICE 'Total Packages: %', package_count;
    RAISE NOTICE '========================================';
    RAISE NOTICE 'Test User Credentials:';
    RAISE NOTICE '  Email: user1@test.com';
    RAISE NOTICE '  Email: user2@test.com';
    RAISE NOTICE '  Email: user3@test.com';
    RAISE NOTICE '  Password: password123';
    RAISE NOTICE '========================================';
END $$;
