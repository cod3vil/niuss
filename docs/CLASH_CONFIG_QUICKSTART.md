# Clash Configuration Management - Quick Start Guide

## 1. Apply Database Migration

```bash
# Connect to your database and run the migration
psql -U postgres -d vpn_platform < migrations/003_clash_config_management.sql
```

This creates three tables and populates them with default proxy groups and rules.

## 2. Get Admin Token

First, login as admin to get your JWT token:

```bash
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "admin123"
  }'
```

Save the token from the response:
```bash
export ADMIN_TOKEN="your_jwt_token_here"
```

## 3. Create Your First Proxy

```bash
curl -X POST http://localhost:8080/api/admin/clash/proxies \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Hong Kong-01",
    "type": "trojan",
    "server": "hk.example.com",
    "port": 443,
    "config": {
      "password": "your-password-here",
      "udp": true,
      "skip-cert-verify": true
    },
    "is_active": true,
    "sort_order": 0
  }'
```

## 4. Update Proxy Group to Include Your Proxy

```bash
# First, get the list of proxy groups to find the ID
curl -X GET "http://localhost:8080/api/admin/clash/proxy-groups" \
  -H "Authorization: Bearer $ADMIN_TOKEN"

# Update the "国外流量" group (usually ID 2)
curl -X PUT http://localhost:8080/api/admin/clash/proxy-groups/2 \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "国外流量",
    "type": "select",
    "proxies": ["Hong Kong-01", "直接连接"],
    "is_active": true,
    "sort_order": 1
  }'
```

## 5. Generate Configuration

```bash
curl -X GET http://localhost:8080/api/admin/clash/generate \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  > clash_config.yaml
```

## 6. View Your Configuration

```bash
cat clash_config.yaml
```

You should see a complete Clash configuration with:
- Your proxy (Hong Kong-01)
- Default proxy groups
- Default routing rules

## 7. Test User Subscription

Users will automatically get this configuration when they access their subscription URL:

```bash
# Get subscription link (as a regular user)
curl -X GET http://localhost:8080/api/subscription/link \
  -H "Authorization: Bearer $USER_TOKEN"

# Access the subscription (public endpoint)
curl http://localhost:8080/sub/SUBSCRIPTION_TOKEN
```

## Common Operations

### Add More Proxies

```bash
# VMess example
curl -X POST http://localhost:8080/api/admin/clash/proxies \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "US-VMess-01",
    "type": "vmess",
    "server": "us.example.com",
    "port": 443,
    "config": {
      "uuid": "12345678-1234-1234-1234-123456789012",
      "alterId": 0,
      "cipher": "auto",
      "udp": true,
      "network": "tcp"
    }
  }'

# VLESS with Reality example
curl -X POST http://localhost:8080/api/admin/clash/proxies \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "SG-VLESS-01",
    "type": "vless",
    "server": "sg.example.com",
    "port": 443,
    "config": {
      "uuid": "12345678-1234-1234-1234-123456789012",
      "flow": "xtls-rprx-vision",
      "network": "tcp",
      "reality-opts": {
        "public-key": "your-public-key",
        "short-id": "short-id"
      },
      "client-fingerprint": "chrome"
    }
  }'
```

### Add Custom Rules

```bash
# Block ads
curl -X POST http://localhost:8080/api/admin/clash/rules \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "rule_type": "DOMAIN-SUFFIX",
    "rule_value": "doubleclick.net",
    "proxy_group": "REJECT",
    "sort_order": 5,
    "description": "Block ads"
  }'

# Route specific app
curl -X POST http://localhost:8080/api/admin/clash/rules \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "rule_type": "PROCESS-NAME",
    "rule_value": "Telegram",
    "proxy_group": "Telegram",
    "sort_order": 3,
    "description": "Telegram app"
  }'
```

### Create Custom Proxy Group

```bash
# Auto-select fastest proxy
curl -X POST http://localhost:8080/api/admin/clash/proxy-groups \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Auto-Select",
    "type": "url-test",
    "proxies": ["Hong Kong-01", "US-VMess-01", "SG-VLESS-01"],
    "url": "http://www.gstatic.com/generate_204",
    "interval": 300,
    "tolerance": 150,
    "is_active": true,
    "sort_order": 2
  }'
```

### List All Configurations

```bash
# List proxies
curl -X GET "http://localhost:8080/api/admin/clash/proxies?active_only=true" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq '.'

# List proxy groups
curl -X GET "http://localhost:8080/api/admin/clash/proxy-groups?active_only=true" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq '.'

# List rules
curl -X GET "http://localhost:8080/api/admin/clash/rules?active_only=true" \
  -H "Authorization: Bearer $ADMIN_TOKEN" | jq '.'
```

### Disable/Enable Items

```bash
# Disable a proxy (set is_active to false)
curl -X PUT http://localhost:8080/api/admin/clash/proxies/1 \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Hong Kong-01",
    "type": "trojan",
    "server": "hk.example.com",
    "port": 443,
    "config": {
      "password": "your-password-here",
      "udp": true,
      "skip-cert-verify": true
    },
    "is_active": false,
    "sort_order": 0
  }'
```

### Delete Items

```bash
# Delete a proxy
curl -X DELETE http://localhost:8080/api/admin/clash/proxies/1 \
  -H "Authorization: Bearer $ADMIN_TOKEN"

# Delete a rule
curl -X DELETE http://localhost:8080/api/admin/clash/rules/5 \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

## Using the Example Script

We've provided a complete example script:

```bash
# Set your admin token
export ADMIN_TOKEN="your_jwt_token_here"

# Run the example
./examples/clash_config_example.sh
```

This script will:
1. Create 2 proxies
2. Create 2 proxy groups
3. Create 4 rules
4. List all configurations
5. Generate YAML config
6. Update a proxy
7. Save the generated config to `clash_generated.yaml`

## Troubleshooting

### "Admin access required" error
- Make sure you're using an admin account token
- Check that the token hasn't expired

### "Proxy not found" error
- Verify the proxy ID exists by listing all proxies first
- Check that you're using the correct ID in the URL

### Empty configuration generated
- Make sure you have at least one active proxy, proxy group, and rule
- Check that `is_active` is set to `true` for items you want included

### Configuration not updating for users
- The subscription endpoint caches configs for 5 minutes
- Wait a few minutes or clear the Redis cache

## Next Steps

- Read the full [API Documentation](CLASH_CONFIG_MANAGEMENT.md)
- Check the [Implementation Summary](CLASH_CONFIG_SUMMARY.md)
- Explore proxy configuration examples for different protocols
- Set up automated configuration backups

## Support

For issues or questions:
1. Check the logs: `docker-compose logs api`
2. Verify database connection: `psql -U postgres -d vpn_platform -c "SELECT COUNT(*) FROM clash_proxies;"`
3. Test API endpoints with curl
4. Review admin logs: `SELECT * FROM admin_logs ORDER BY created_at DESC LIMIT 10;`
