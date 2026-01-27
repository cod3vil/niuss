# 部署指南

本文档提供 VPN 订阅服务平台的详细部署说明。

## 系统要求

### 硬件要求

**最低配置**：
- CPU: 2 核
- 内存: 4GB
- 磁盘: 20GB

**推荐配置**：
- CPU: 4 核
- 内存: 8GB
- 磁盘: 50GB SSD

### 软件要求

- Docker 20.10+
- Docker Compose 2.0+
- 操作系统: Linux (Ubuntu 20.04+, CentOS 8+, Debian 11+)

## 部署步骤

### 1. 准备服务器

```bash
# 更新系统
sudo apt update && sudo apt upgrade -y

# 安装 Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# 安装 Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# 验证安装
docker --version
docker-compose --version
```

### 2. 克隆项目

```bash
git clone <repository-url>
cd vpn-subscription-platform
```

### 3. 配置环境变量

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑环境变量
nano .env
```

**重要配置项**：

```bash
# 数据库密码（必须修改）
DB_PASSWORD=your-strong-password-here

# JWT 密钥（必须修改，使用长随机字符串）
JWT_SECRET=your-very-long-random-secret-key-at-least-32-characters

# JWT 过期时间（秒，默认 24 小时）
JWT_EXPIRATION=86400

# CORS 允许的源（根据实际域名修改）
CORS_ORIGINS=https://yourdomain.com,https://admin.yourdomain.com
```

### 4. 生成强密钥

```bash
# 生成 JWT 密钥
openssl rand -base64 48

# 生成数据库密码
openssl rand -base64 32
```

### 5. 启动服务

```bash
# 构建镜像
docker-compose build

# 启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f
```

### 6. 验证部署

```bash
# 检查服务状态
docker-compose ps

# 测试 API 健康检查
curl http://localhost:8080/health

# 测试前端
curl http://localhost

# 测试管理后台
curl http://localhost:8081
```

### 7. 配置反向代理（可选但推荐）

使用 Nginx 作为反向代理并配置 SSL：

```nginx
# /etc/nginx/sites-available/vpn-platform
server {
    listen 80;
    server_name yourdomain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name yourdomain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:80;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

# 管理后台
server {
    listen 443 ssl http2;
    server_name admin.yourdomain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:8081;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

启用配置：
```bash
sudo ln -s /etc/nginx/sites-available/vpn-platform /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

### 8. 配置 SSL 证书（使用 Let's Encrypt）

```bash
# 安装 Certbot
sudo apt install certbot python3-certbot-nginx

# 获取证书
sudo certbot --nginx -d yourdomain.com -d admin.yourdomain.com

# 自动续期
sudo certbot renew --dry-run
```

## 节点部署

### 节点服务器要求

- 操作系统: Linux
- 网络: 公网 IP
- 端口: 根据协议开放相应端口（如 443）

### 节点部署脚本

创建 `install_node.sh` 脚本：

```bash
#!/bin/bash

set -e

# 配置变量
API_URL="${API_URL:-https://api.yourdomain.com}"
NODE_ID="${NODE_ID}"
NODE_SECRET="${NODE_SECRET}"

if [ -z "$NODE_ID" ] || [ -z "$NODE_SECRET" ]; then
    echo "Error: NODE_ID and NODE_SECRET must be set"
    exit 1
fi

echo "Installing Xray-core..."
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install

echo "Downloading Node Agent..."
wget https://github.com/your-org/vpn-platform/releases/latest/download/node-agent -O /usr/local/bin/node-agent
chmod +x /usr/local/bin/node-agent

echo "Creating configuration directory..."
mkdir -p /etc/node-agent

echo "Creating configuration file..."
cat > /etc/node-agent/config.env <<EOF
API_URL=${API_URL}
NODE_ID=${NODE_ID}
NODE_SECRET=${NODE_SECRET}
XRAY_API_PORT=10085
TRAFFIC_REPORT_INTERVAL=30
HEARTBEAT_INTERVAL=60
EOF

echo "Creating systemd service..."
cat > /etc/systemd/system/node-agent.service <<EOF
[Unit]
Description=VPN Node Agent
After=network.target

[Service]
Type=simple
EnvironmentFile=/etc/node-agent/config.env
ExecStart=/usr/local/bin/node-agent
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

echo "Starting services..."
systemctl daemon-reload
systemctl enable node-agent
systemctl start node-agent

echo "Node Agent installed successfully!"
echo "Check status: systemctl status node-agent"
echo "View logs: journalctl -u node-agent -f"
```

使用脚本：

```bash
# 在节点服务器上执行
export NODE_ID="node-001"
export NODE_SECRET="your-node-secret"
export API_URL="https://api.yourdomain.com"

curl -sSL https://raw.githubusercontent.com/your-org/vpn-platform/main/install_node.sh | bash
```

## 数据库管理

### 备份数据库

```bash
# 创建备份
docker-compose exec postgres pg_dump -U vpn_user vpn_platform > backup_$(date +%Y%m%d_%H%M%S).sql

# 或使用 Docker 卷备份
docker run --rm -v vpn-subscription-platform_postgres_data:/data -v $(pwd):/backup alpine tar czf /backup/postgres_backup_$(date +%Y%m%d_%H%M%S).tar.gz /data
```

### 恢复数据库

```bash
# 从 SQL 文件恢复
docker-compose exec -T postgres psql -U vpn_user vpn_platform < backup.sql

# 从卷备份恢复
docker run --rm -v vpn-subscription-platform_postgres_data:/data -v $(pwd):/backup alpine tar xzf /backup/postgres_backup.tar.gz -C /
```

### 数据库维护

```bash
# 连接到数据库
docker-compose exec postgres psql -U vpn_user vpn_platform

# 查看表大小
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

# 清理旧数据（示例：删除 90 天前的流量日志）
DELETE FROM traffic_logs WHERE recorded_at < NOW() - INTERVAL '90 days';

# 重建索引
REINDEX DATABASE vpn_platform;

# 分析表
ANALYZE;
```

## 监控和日志

### 查看日志

```bash
# 查看所有服务日志
docker-compose logs -f

# 查看特定服务日志
docker-compose logs -f api
docker-compose logs -f postgres
docker-compose logs -f redis

# 查看最近 100 行日志
docker-compose logs --tail=100 api
```

### 监控资源使用

```bash
# 查看容器资源使用
docker stats

# 查看磁盘使用
df -h
docker system df

# 查看数据库连接数
docker-compose exec postgres psql -U vpn_user vpn_platform -c "SELECT count(*) FROM pg_stat_activity;"
```

## 更新和维护

### 更新应用

```bash
# 拉取最新代码
git pull

# 重新构建镜像
docker-compose build

# 重启服务（零停机时间）
docker-compose up -d

# 清理旧镜像
docker image prune -f
```

### 数据库迁移

```bash
# 执行新的迁移脚本
docker-compose exec postgres psql -U vpn_user vpn_platform -f /docker-entrypoint-initdb.d/002_new_migration.sql
```

## 故障排查

### API 服务无法启动

```bash
# 检查日志
docker-compose logs api

# 常见问题：
# 1. 数据库连接失败 - 检查 DATABASE_URL
# 2. Redis 连接失败 - 检查 REDIS_URL
# 3. 端口被占用 - 修改 docker-compose.yml 中的端口映射
```

### 数据库连接问题

```bash
# 测试数据库连接
docker-compose exec postgres psql -U vpn_user vpn_platform -c "SELECT 1;"

# 检查数据库日志
docker-compose logs postgres

# 重启数据库
docker-compose restart postgres
```

### Redis 连接问题

```bash
# 测试 Redis 连接
docker-compose exec redis redis-cli ping

# 检查 Redis 日志
docker-compose logs redis

# 重启 Redis
docker-compose restart redis
```

### 前端无法访问 API

```bash
# 检查 CORS 配置
# 确保 .env 中的 CORS_ORIGINS 包含前端域名

# 检查网络连接
docker-compose exec frontend ping api

# 检查 Nginx 配置
docker-compose exec frontend cat /etc/nginx/conf.d/default.conf
```

## 安全建议

1. **修改默认密码**：立即修改默认管理员密码
2. **使用强密钥**：JWT_SECRET 至少 32 字符
3. **启用 HTTPS**：生产环境必须使用 SSL/TLS
4. **防火墙配置**：只开放必要端口（80, 443）
5. **定期备份**：设置自动备份任务
6. **更新系统**：定期更新 Docker 镜像和系统包
7. **监控日志**：定期检查异常访问和错误日志
8. **限制访问**：使用 IP 白名单限制管理后台访问

## 性能优化

### 数据库优化

```sql
-- 创建额外索引（根据查询模式）
CREATE INDEX idx_orders_user_status ON orders(user_id, status);
CREATE INDEX idx_traffic_logs_user_recorded ON traffic_logs(user_id, recorded_at);

-- 配置连接池
-- 在 .env 中设置
DATABASE_MAX_CONNECTIONS=20
```

### Redis 优化

```bash
# 在 docker-compose.yml 中配置 Redis
redis:
  command: redis-server --maxmemory 256mb --maxmemory-policy allkeys-lru
```

### 应用优化

- 启用 HTTP/2
- 配置 CDN
- 使用缓存头
- 压缩静态资源

## 扩展部署

### 水平扩展 API 服务

```yaml
# docker-compose.yml
api:
  deploy:
    replicas: 3
  # 添加负载均衡器
```

### 数据库主从复制

参考 PostgreSQL 官方文档配置主从复制。

### Redis 集群

参考 Redis 官方文档配置 Redis Cluster。

## 支持

如有问题，请：
1. 查看日志文件
2. 检查 GitHub Issues
3. 联系技术支持

## 附录

### 常用命令

```bash
# 启动服务
make up

# 停止服务
make down

# 查看日志
make logs

# 运行测试
make test

# 清理所有数据
make clean

# 数据库 Shell
make db-shell

# Redis Shell
make redis-shell
```

### 端口说明

| 服务 | 端口 | 说明 |
|------|------|------|
| 用户前端 | 80 | HTTP |
| 管理后台 | 8081 | HTTP |
| API 服务 | 8080 | HTTP |
| PostgreSQL | 5432 | 数据库 |
| Redis | 6379 | 缓存 |

### 目录说明

| 目录 | 说明 |
|------|------|
| /var/lib/docker/volumes/vpn-subscription-platform_postgres_data | PostgreSQL 数据 |
| /var/lib/docker/volumes/vpn-subscription-platform_redis_data | Redis 数据 |
