# Clash Configuration Management Feature

## Overview

The Clash Configuration Management system provides a complete backend solution for dynamically managing Clash proxy configurations through a RESTful API. Instead of manually editing YAML files, administrators can now manage proxies, proxy groups, and routing rules through database-driven CRUD operations.

## Key Benefits

### 🚀 Dynamic Management
- Add, update, or remove proxies without restarting services
- Real-time configuration updates for all users
- No manual YAML file editing required

### 📊 Database-Driven
- All configurations stored in PostgreSQL
- Version control through timestamps
- Easy backup and restore

### 🔒 Secure & Audited
- Admin-only access with JWT authentication
- All changes logged in admin_logs table
- Complete audit trail for compliance

### 🎯 Flexible & Scalable
- Support for 5 proxy protocols (Shadowsocks, VMess, Trojan, Hysteria2, VLESS)
- 5 proxy group types (select, url-test, fallback, load-balance, relay)
- 11 rule types for traffic routing
- Unlimited proxies, groups, and rules

### 🔄 Backward Compatible
- Seamlessly integrates with existing node-based system
- Falls back to node configuration if database is empty
- No breaking changes to existing functionality

## Architecture

```
┌─────────────────┐
│  Admin Panel    │
│   (Frontend)    │
└────────┬────────┘
         │ HTTP/REST
         ▼
┌─────────────────┐
│   API Server    │
│  (Rust/Axum)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐      ┌──────────────┐
│   PostgreSQL    │◄────►│    Redis     │
│   (Database)    │      │   (Cache)    │
└─────────────────┘      └──────────────┘
         │
         ▼
┌─────────────────┐
│  YAML Generator │
│  (Clash Config) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Subscription   │
│    Endpoint     │
└─────────────────┘
```

## Components

### 1. Database Tables

#### clash_proxies
Stores individual proxy server configurations.

**Fields:**
- `id`: Unique identifier
- `name`: Proxy name (unique)
- `type`: Protocol type (ss, vmess, trojan, hysteria2, vless)
- `server`: Server address
- `port`: Server port
- `config`: Protocol-specific configuration (JSONB)
- `is_active`: Enable/disable flag
- `sort_order`: Display order
- `created_at`, `updated_at`: Timestamps

#### clash_proxy_groups
Stores proxy group configurations.

**Fields:**
- `id`: Unique identifier
- `name`: Group name (unique)
- `type`: Group type (select, url-test, fallback, load-balance, relay)
- `proxies`: Array of proxy/group names
- `url`: Test URL (for url-test, fallback)
- `interval`: Test interval in seconds
- `tolerance`: Tolerance in milliseconds
- `is_active`: Enable/disable flag
- `sort_order`: Display order
- `created_at`, `updated_at`: Timestamps

#### clash_rules
Stores routing rules.

**Fields:**
- `id`: Unique identifier
- `rule_type`: Rule type (DOMAIN, DOMAIN-SUFFIX, IP-CIDR, GEOIP, etc.)
- `rule_value`: Rule value (domain, IP range, etc.)
- `proxy_group`: Target proxy group
- `no_resolve`: Skip DNS resolution flag
- `is_active`: Enable/disable flag
- `sort_order`: Rule priority (lower = higher priority)
- `description`: Rule description
- `created_at`, `updated_at`: Timestamps

### 2. API Endpoints

All endpoints require admin authentication.

#### Proxy Management
- `GET /api/admin/clash/proxies` - List proxies
- `POST /api/admin/clash/proxies` - Create proxy
- `PUT /api/admin/clash/proxies/:id` - Update proxy
- `DELETE /api/admin/clash/proxies/:id` - Delete proxy

#### Proxy Group Management
- `GET /api/admin/clash/proxy-groups` - List groups
- `POST /api/admin/clash/proxy-groups` - Create group
- `PUT /api/admin/clash/proxy-groups/:id` - Update group
- `DELETE /api/admin/clash/proxy-groups/:id` - Delete group

#### Rule Management
- `GET /api/admin/clash/rules` - List rules
- `POST /api/admin/clash/rules` - Create rule
- `PUT /api/admin/clash/rules/:id` - Update rule
- `DELETE /api/admin/clash/rules/:id` - Delete rule

#### Configuration Generation
- `GET /api/admin/clash/generate` - Generate YAML config

### 3. Configuration Generator

The system includes a sophisticated YAML generator that:
- Converts database models to Clash format
- Supports all proxy protocols with proper field mapping
- Generates valid Clash YAML configuration
- Handles nested configurations (e.g., VLESS Reality)

### 4. Subscription Integration

User subscription endpoint automatically:
- Checks for database configuration first
- Falls back to node-based config if database is empty
- Caches generated configurations in Redis
- Updates user subscriptions in real-time

## Supported Proxy Types

### Shadowsocks (ss)
```json
{
  "cipher": "aes-256-gcm",
  "password": "password",
  "udp": true
}
```

### VMess
```json
{
  "uuid": "uuid-here",
  "alterId": 0,
  "cipher": "auto",
  "udp": true,
  "network": "tcp"
}
```

### Trojan
```json
{
  "password": "password",
  "udp": true,
  "sni": "example.com",
  "skip-cert-verify": false
}
```

### Hysteria2
```json
{
  "password": "password",
  "obfs": "salamander",
  "obfs-password": "obfs-pass",
  "sni": "example.com",
  "skip-cert-verify": false
}
```

### VLESS with Reality
```json
{
  "uuid": "uuid-here",
  "flow": "xtls-rprx-vision",
  "network": "tcp",
  "reality-opts": {
    "public-key": "public-key",
    "short-id": "short-id"
  },
  "client-fingerprint": "chrome"
}
```

## Proxy Group Types

| Type | Description | Use Case |
|------|-------------|----------|
| select | Manual selection | User chooses proxy |
| url-test | Auto-select fastest | Best performance |
| fallback | Use first available | High availability |
| load-balance | Distribute load | Load balancing |
| relay | Chain proxies | Multi-hop routing |

## Rule Types

| Type | Example | Description |
|------|---------|-------------|
| DOMAIN | `google.com` | Exact domain match |
| DOMAIN-SUFFIX | `google.com` | Domain and subdomains |
| DOMAIN-KEYWORD | `google` | Domain contains keyword |
| IP-CIDR | `192.168.0.0/16` | IPv4 CIDR match |
| IP-CIDR6 | `2001:db8::/32` | IPv6 CIDR match |
| GEOIP | `CN` | Country code match |
| DST-PORT | `80` | Destination port |
| SRC-PORT | `7777` | Source port |
| PROCESS-NAME | `chrome.exe` | Process name |
| MATCH | - | Match all (final rule) |

## Default Configuration

The migration includes sensible defaults:

### Proxy Groups (11)
- 直接连接 (Direct)
- 国外流量 (Foreign Traffic)
- 其他流量 (Other Traffic)
- Telegram
- Youtube
- Netflix
- 哔哩哔哩 (Bilibili)
- ChatGPT及其他AI (ChatGPT & AI)
- Steam
- 国外媒体 (Foreign Media)
- 苹果服务 (Apple Services)

### Rules (12)
- Himalaya Podcast → 国外流量
- Baidu services → 直接连接
- China IP (GEOIP CN) → 直接连接
- Default (MATCH) → 其他流量

## Usage Example

```bash
# 1. Create a proxy
curl -X POST http://localhost:8080/api/admin/clash/proxies \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "HK-01",
    "type": "trojan",
    "server": "hk.example.com",
    "port": 443,
    "config": {"password": "pass", "udp": true}
  }'

# 2. Add to proxy group
curl -X PUT http://localhost:8080/api/admin/clash/proxy-groups/2 \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "name": "国外流量",
    "type": "select",
    "proxies": ["HK-01", "直接连接"]
  }'

# 3. Generate config
curl http://localhost:8080/api/admin/clash/generate \
  -H "Authorization: Bearer $TOKEN"
```

## Documentation

- **[Quick Start Guide](CLASH_CONFIG_QUICKSTART.md)** - Get started in 5 minutes
- **[API Reference](CLASH_CONFIG_MANAGEMENT.md)** - Complete API documentation
- **[Implementation Summary](CLASH_CONFIG_SUMMARY.md)** - Technical details

## Migration

```bash
psql -U postgres -d vpn_platform < migrations/003_clash_config_management.sql
```

## Testing

Use the provided example script:

```bash
export ADMIN_TOKEN="your-token"
./examples/clash_config_example.sh
```

## Performance

- Database queries are optimized with indexes
- Redis caching for generated configurations
- Efficient JSONB storage for proxy configs
- Minimal overhead on subscription endpoint

## Security

- Admin-only access with JWT authentication
- All actions logged for audit trail
- Input validation on all endpoints
- SQL injection protection via parameterized queries
- No sensitive data in logs

## Future Enhancements

Potential improvements:
- Web UI for configuration management
- Configuration templates
- Bulk import/export
- Configuration versioning
- A/B testing for proxy groups
- Analytics and monitoring
- Automated proxy health checks

## Troubleshooting

### Configuration not updating
- Check Redis cache TTL (5 minutes)
- Verify `is_active` flag is true
- Check admin logs for errors

### Empty configuration
- Ensure at least one active proxy, group, and rule
- Verify database migration completed
- Check database connectivity

### Permission denied
- Verify admin JWT token
- Check token expiration
- Confirm user has `is_admin = true`

## Support

For issues:
1. Check logs: `docker-compose logs api`
2. Verify database: `SELECT COUNT(*) FROM clash_proxies;`
3. Test endpoints with curl
4. Review admin logs: `SELECT * FROM admin_logs ORDER BY created_at DESC;`

## License

Same as the main project.
