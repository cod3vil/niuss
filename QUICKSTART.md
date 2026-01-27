# Quick Start Guide

Get your VPN Subscription Platform up and running in minutes!

## Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- 4GB RAM minimum
- 20GB disk space

## 🚀 One-Click Deployment (Recommended)

### Deploy Management Platform

```bash
# Download and run deployment script
curl -sSL https://raw.githubusercontent.com/your-org/vpn-platform/main/scripts/deploy_platform.sh | sudo bash

# Or with SSL (recommended for production)
curl -sSL https://raw.githubusercontent.com/your-org/vpn-platform/main/scripts/deploy_platform.sh -o deploy_platform.sh
sudo bash deploy_platform.sh --domain yourdomain.com --email your@email.com --enable-ssl
```

### Deploy Node Server

```bash
# Interactive deployment (easiest)
curl -sSL https://raw.githubusercontent.com/your-org/vpn-platform/main/scripts/quick_deploy_node.sh | sudo bash

# Or with parameters
sudo bash quick_deploy_node.sh \
  --api-url https://api.yourdomain.com \
  --admin-token your-jwt-token \
  --node-name node-hk-01
```

See [One-Click Deployment Guide](docs/ONE_CLICK_DEPLOYMENT.md) for detailed instructions.

## Manual Deployment (5 minutes)

### 1. Clone and Setup

```bash
# Clone the repository
git clone <repository-url>
cd vpn-subscription-platform

# Initialize environment
make init-env

# Edit .env file with your settings
nano .env
```

**Important**: Change these values in `.env`:
- `DB_PASSWORD` - Use a strong password
- `JWT_SECRET` - Use a long random string (at least 32 characters)

### 2. Deploy

```bash
# Build and start all services
make deploy

# Or use the test script to verify deployment
./scripts/test_deployment.sh
```

### 3. Access the Platform

Once deployed, access:

- **User Frontend**: http://localhost
- **Admin Panel**: http://localhost:8081
- **API**: http://localhost:8080

**Default Admin Credentials**:
- Email: `admin@example.com`
- Password: `admin123`

⚠️ **Change the admin password immediately after first login!**

## What's Running?

After deployment, you'll have:

- ✅ PostgreSQL database with initialized schema
- ✅ Redis cache and message queue
- ✅ API service (Rust/Axum)
- ✅ User frontend (Vue 3)
- ✅ Admin panel (Vue 3)

## Next Steps

### 1. Create Your First Node

1. Log in to admin panel: http://localhost:8081
2. Navigate to "Nodes" section
3. Click "Add Node"
4. Fill in node details:
   - Name: e.g., "Tokyo Node 1"
   - Host: Your node server IP/domain
   - Port: 443 (or your chosen port)
   - Protocol: Choose from vless, vmess, trojan, hysteria2, shadowsocks
   - Secret: Generate a strong secret key
   - Config: Protocol-specific configuration (JSON)

### 2. Deploy Node Agent

On your VPN node server:

```bash
# Set environment variables
export API_URL="http://your-api-server:8080"
export NODE_ID="your-node-id"
export NODE_SECRET="your-node-secret"

# Download and run installation script
curl -sSL https://raw.githubusercontent.com/your-org/vpn-platform/main/scripts/install_node.sh | bash
```

Or manually:

```bash
# Copy the script to your node server
scp scripts/install_node.sh user@node-server:/tmp/

# SSH to node server and run
ssh user@node-server
sudo bash /tmp/install_node.sh
```

### 3. Test User Registration

1. Open user frontend: http://localhost
2. Click "Register"
3. Create a test account
4. Purchase a package (you'll need to add coins first)
5. Get your subscription link
6. Import to Clash client

## Common Commands

```bash
# View logs
make logs

# Check service status
make status

# Restart services
make restart

# Stop services
make down

# Database backup
make db-backup

# Database shell
make db-shell

# Redis shell
make redis-shell

# Health check
make health-check
```

## Troubleshooting

### Services won't start

```bash
# Check Docker daemon
docker info

# Check logs
make logs

# Restart services
make restart
```

### Database connection errors

```bash
# Check PostgreSQL status
docker-compose ps postgres

# Check PostgreSQL logs
make logs-postgres

# Reset database (WARNING: deletes all data)
make db-reset
```

### API not responding

```bash
# Check API logs
make logs-api

# Verify environment variables
docker-compose exec api env | grep -E "DATABASE_URL|REDIS_URL|JWT_SECRET"

# Restart API
docker-compose restart api
```

### Frontend can't connect to API

1. Check CORS settings in `.env`:
   ```
   CORS_ORIGINS=http://localhost:3000,http://localhost:3001
   ```

2. Verify API is running:
   ```bash
   curl http://localhost:8080/health
   ```

3. Check browser console for errors

## Development Mode

### Run API locally

```bash
# Set environment variables
export DATABASE_URL="postgres://vpn_user:vpn_password@localhost:5434/vpn_platform"
export REDIS_URL="redis://localhost:6380"
export JWT_SECRET="your-secret"

# Run API
make dev-api
```

### Run Frontend locally

```bash
# Install dependencies
make install-frontend

# Run dev server
make dev-frontend
```

### Run Admin locally

```bash
# Install dependencies
make install-admin

# Run dev server
make dev-admin
```

## Production Deployment

For production deployment, see [DEPLOYMENT.md](DEPLOYMENT.md) for:

- SSL/TLS configuration
- Reverse proxy setup (Nginx)
- Security hardening
- Backup strategies
- Monitoring setup
- Performance optimization

## Database Management

### Backup

```bash
# Create backup
make db-backup

# Backups are stored in ./backups/
```

### Restore

```bash
# Restore from backup
make db-restore FILE=backups/vpn_platform_20240101_120000.sql.gz
```

### View Statistics

```bash
# Show database stats
make db-stats
```

## Testing

### Run all tests

```bash
make test
```

### Run specific tests

```bash
# API tests only
make test-api

# Node agent tests only
make test-node-agent
```

### Test deployment

```bash
# Comprehensive deployment test
./scripts/test_deployment.sh
```

## Architecture Overview

```
┌─────────────┐     ┌─────────────┐
│   Frontend  │────▶│     API     │
│   (Vue 3)   │     │  (Rust/Axum)│
└─────────────┘     └──────┬──────┘
                           │
                    ┌──────┴──────┐
                    │             │
              ┌─────▼─────┐ ┌────▼────┐
              │ PostgreSQL│ │  Redis  │
              └───────────┘ └─────────┘
                           │
                    ┌──────┴──────┐
                    │             │
              ┌─────▼─────┐ ┌────▼────┐
              │Node Agent │ │Node Agent│
              │+ Xray-core│ │+ Xray-core│
              └───────────┘ └──────────┘
```

## Support

- Documentation: [DEPLOYMENT.md](DEPLOYMENT.md)
- Issues: GitHub Issues
- Logs: `make logs`

## Security Checklist

Before going to production:

- [ ] Change default admin password
- [ ] Set strong `JWT_SECRET` (32+ characters)
- [ ] Set strong `DB_PASSWORD`
- [ ] Enable HTTPS/SSL
- [ ] Configure firewall rules
- [ ] Set up regular backups
- [ ] Review CORS settings
- [ ] Enable rate limiting
- [ ] Set up monitoring
- [ ] Review security logs

## What's Next?

1. **Customize**: Modify packages, pricing, and features
2. **Scale**: Add more nodes for better performance
3. **Monitor**: Set up logging and monitoring
4. **Secure**: Enable HTTPS and security features
5. **Optimize**: Tune database and cache settings

Happy deploying! 🚀
