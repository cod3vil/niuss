# Clash Configuration Management

This document describes the Clash configuration management system that allows administrators to manage proxies, proxy groups, and routing rules through the backend API.

## Overview

The Clash configuration management system provides three main components:

1. **Proxies** - Individual proxy server configurations (Shadowsocks, VMess, Trojan, Hysteria2, VLESS)
2. **Proxy Groups** - Groups that organize proxies with selection strategies (select, url-test, fallback, load-balance, relay)
3. **Rules** - Traffic routing rules that determine which proxy group handles specific traffic

## Database Schema

### clash_proxies Table

Stores individual proxy configurations.

| Column | Type | Description |
|--------|------|-------------|
| id | BIGSERIAL | Primary key |
| name | VARCHAR(100) | Unique proxy name |
| type | VARCHAR(20) | Proxy type: ss, vmess, trojan, hysteria2, vless |
| server | VARCHAR(255) | Server address |
| port | INT | Server port (1-65535) |
| config | JSONB | Protocol-specific configuration |
| is_active | BOOLEAN | Whether proxy is active |
| sort_order | INT | Display order |
| created_at | TIMESTAMPTZ | Creation timestamp |
| updated_at | TIMESTAMPTZ | Last update timestamp |

### clash_proxy_groups Table

Stores proxy group configurations.

| Column | Type | Description |
|--------|------|-------------|
| id | BIGSERIAL | Primary key |
| name | VARCHAR(100) | Unique group name |
| type | VARCHAR(20) | Group type: select, url-test, fallback, load-balance, relay |
| proxies | TEXT[] | Array of proxy/group names |
| url | VARCHAR(255) | Test URL (for url-test, fallback) |
| interval | INT | Test interval in seconds |
| tolerance | INT | Tolerance in milliseconds |
| is_active | BOOLEAN | Whether group is active |
| sort_order | INT | Display order |
| created_at | TIMESTAMPTZ | Creation timestamp |
| updated_at | TIMESTAMPTZ | Last update timestamp |

### clash_rules Table

Stores routing rules.

| Column | Type | Description |
|--------|------|-------------|
| id | BIGSERIAL | Primary key |
| rule_type | VARCHAR(50) | Rule type (DOMAIN, DOMAIN-SUFFIX, IP-CIDR, GEOIP, etc.) |
| rule_value | VARCHAR(255) | Rule value (domain, IP range, etc.) |
| proxy_group | VARCHAR(100) | Target proxy group name |
| no_resolve | BOOLEAN | Skip DNS resolution |
| is_active | BOOLEAN | Whether rule is active |
| sort_order | INT | Rule priority (lower = higher priority) |
| description | TEXT | Rule description |
| created_at | TIMESTAMPTZ | Creation timestamp |
| updated_at | TIMESTAMPTZ | Last update timestamp |

## API Endpoints

All endpoints require admin authentication via JWT token in the `Authorization: Bearer <token>` header.

### Proxy Management

#### List Proxies
```
GET /api/admin/clash/proxies?active_only=true
```

Response:
```json
[
  {
    "id": 1,
    "name": "Hong Kong-01",
    "type": "trojan",
    "server": "hk.example.com",
    "port": 56201,
    "config": {
      "password": "password123",
      "udp": true,
      "skip-cert-verify": true
    },
    "is_active": true,
    "sort_order": 0,
    "created_at": "2026-01-27T00:00:00Z",
    "updated_at": "2026-01-27T00:00:00Z"
  }
]
```

#### Create Proxy
```
POST /api/admin/clash/proxies
Content-Type: application/json

{
  "name": "Hong Kong-01",
  "type": "trojan",
  "server": "hk.example.com",
  "port": 56201,
  "config": {
    "password": "password123",
    "udp": true,
    "skip-cert-verify": true
  },
  "is_active": true,
  "sort_order": 0
}
```

#### Update Proxy
```
PUT /api/admin/clash/proxies/:id
Content-Type: application/json

{
  "name": "Hong Kong-01-Updated",
  "type": "trojan",
  "server": "hk.example.com",
  "port": 56201,
  "config": {
    "password": "newpassword",
    "udp": true,
    "skip-cert-verify": false
  },
  "is_active": true,
  "sort_order": 0
}
```

#### Delete Proxy
```
DELETE /api/admin/clash/proxies/:id
```

### Proxy Group Management

#### List Proxy Groups
```
GET /api/admin/clash/proxy-groups?active_only=true
```

Response:
```json
[
  {
    "id": 1,
    "name": "国外流量",
    "type": "select",
    "proxies": ["Hong Kong-01", "Hong Kong-02", "直接连接"],
    "url": null,
    "interval": null,
    "tolerance": null,
    "is_active": true,
    "sort_order": 1,
    "created_at": "2026-01-27T00:00:00Z",
    "updated_at": "2026-01-27T00:00:00Z"
  }
]
```

#### Create Proxy Group
```
POST /api/admin/clash/proxy-groups
Content-Type: application/json

{
  "name": "国外流量",
  "type": "select",
  "proxies": ["Hong Kong-01", "Hong Kong-02", "直接连接"],
  "is_active": true,
  "sort_order": 1
}
```

#### Update Proxy Group
```
PUT /api/admin/clash/proxy-groups/:id
Content-Type: application/json

{
  "name": "国外流量",
  "type": "select",
  "proxies": ["Hong Kong-01", "Hong Kong-02", "Hong Kong-03", "直接连接"],
  "is_active": true,
  "sort_order": 1
}
```

#### Delete Proxy Group
```
DELETE /api/admin/clash/proxy-groups/:id
```

### Rule Management

#### List Rules
```
GET /api/admin/clash/rules?active_only=true
```

Response:
```json
[
  {
    "id": 1,
    "rule_type": "DOMAIN-SUFFIX",
    "rule_value": "google.com",
    "proxy_group": "国外流量",
    "no_resolve": false,
    "is_active": true,
    "sort_order": 0,
    "description": "Google services",
    "created_at": "2026-01-27T00:00:00Z",
    "updated_at": "2026-01-27T00:00:00Z"
  }
]
```

#### Create Rule
```
POST /api/admin/clash/rules
Content-Type: application/json

{
  "rule_type": "DOMAIN-SUFFIX",
  "rule_value": "google.com",
  "proxy_group": "国外流量",
  "no_resolve": false,
  "is_active": true,
  "sort_order": 0,
  "description": "Google services"
}
```

#### Update Rule
```
PUT /api/admin/clash/rules/:id
Content-Type: application/json

{
  "rule_type": "DOMAIN-SUFFIX",
  "rule_value": "google.com",
  "proxy_group": "国外流量",
  "no_resolve": true,
  "is_active": true,
  "sort_order": 0,
  "description": "Google services (updated)"
}
```

#### Delete Rule
```
DELETE /api/admin/clash/rules/:id
```

### Generate Configuration

#### Generate Clash YAML
```
GET /api/admin/clash/generate
```

Response: YAML configuration file
```yaml
proxies:
  - name: Hong Kong-01
    type: trojan
    server: hk.example.com
    port: 56201
    password: password123
    udp: true
    skip-cert-verify: true

proxy-groups:
  - name: 国外流量
    type: select
    proxies:
      - Hong Kong-01
      - Hong Kong-02
      - 直接连接

rules:
  - DOMAIN-SUFFIX,google.com,国外流量
  - GEOIP,CN,直接连接
  - MATCH,其他流量
```

## Proxy Configuration Examples

### Shadowsocks (ss)
```json
{
  "name": "SS-Server",
  "type": "ss",
  "server": "example.com",
  "port": 8388,
  "config": {
    "cipher": "aes-256-gcm",
    "password": "password123",
    "udp": true
  }
}
```

### VMess
```json
{
  "name": "VMess-Server",
  "type": "vmess",
  "server": "example.com",
  "port": 443,
  "config": {
    "uuid": "12345678-1234-1234-1234-123456789012",
    "alterId": 0,
    "cipher": "auto",
    "udp": true,
    "network": "tcp"
  }
}
```

### Trojan
```json
{
  "name": "Trojan-Server",
  "type": "trojan",
  "server": "example.com",
  "port": 443,
  "config": {
    "password": "password123",
    "udp": true,
    "sni": "example.com",
    "skip-cert-verify": false
  }
}
```

### Hysteria2
```json
{
  "name": "Hysteria2-Server",
  "type": "hysteria2",
  "server": "example.com",
  "port": 443,
  "config": {
    "password": "password123",
    "obfs": "salamander",
    "obfs-password": "obfspass",
    "sni": "example.com",
    "skip-cert-verify": false
  }
}
```

### VLESS with Reality
```json
{
  "name": "VLESS-Reality-Server",
  "type": "vless",
  "server": "example.com",
  "port": 443,
  "config": {
    "uuid": "12345678-1234-1234-1234-123456789012",
    "flow": "xtls-rprx-vision",
    "network": "tcp",
    "reality-opts": {
      "public-key": "publickey123",
      "short-id": "shortid"
    },
    "client-fingerprint": "chrome"
  }
}
```

## Rule Types

| Rule Type | Description | Example |
|-----------|-------------|---------|
| DOMAIN | Exact domain match | `DOMAIN,google.com,Proxy` |
| DOMAIN-SUFFIX | Domain suffix match | `DOMAIN-SUFFIX,google.com,Proxy` |
| DOMAIN-KEYWORD | Domain keyword match | `DOMAIN-KEYWORD,google,Proxy` |
| IP-CIDR | IPv4 CIDR match | `IP-CIDR,192.168.0.0/16,DIRECT` |
| IP-CIDR6 | IPv6 CIDR match | `IP-CIDR6,2001:db8::/32,DIRECT` |
| SRC-IP-CIDR | Source IP CIDR match | `SRC-IP-CIDR,192.168.1.0/24,DIRECT` |
| GEOIP | GeoIP country match | `GEOIP,CN,DIRECT` |
| DST-PORT | Destination port match | `DST-PORT,80,Proxy` |
| SRC-PORT | Source port match | `SRC-PORT,7777,DIRECT` |
| PROCESS-NAME | Process name match | `PROCESS-NAME,chrome.exe,Proxy` |
| MATCH | Match all (final rule) | `MATCH,Proxy` |

## Proxy Group Types

| Type | Description |
|------|-------------|
| select | Manual selection by user |
| url-test | Automatic selection based on URL test |
| fallback | Use first available proxy |
| load-balance | Load balance across proxies |
| relay | Chain proxies together |

## Migration

To apply the database schema:

```bash
psql -U postgres -d vpn_platform < migrations/003_clash_config_management.sql
```

## Workflow

1. **Create Proxies**: Add individual proxy servers with their configurations
2. **Create Proxy Groups**: Organize proxies into groups with selection strategies
3. **Create Rules**: Define routing rules to direct traffic to appropriate proxy groups
4. **Generate Config**: Use the generate endpoint to create a complete Clash YAML configuration
5. **Auto-Update**: The subscription endpoint will automatically use the database configuration when generating user configs

## Notes

- All changes are logged in the `admin_logs` table for audit purposes
- The `sort_order` field determines the order of items in the generated configuration
- Rules are processed in order of `sort_order` (lower values = higher priority)
- The `MATCH` rule should always be last (highest sort_order)
- Inactive items (`is_active = false`) are excluded from generated configurations
