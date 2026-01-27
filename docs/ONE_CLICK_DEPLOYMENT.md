# 一键部署指南

本文档介绍如何使用一键部署脚本快速部署 VPN 订阅平台的管理平台和节点服务器。

## 目录

- [管理平台部署](#管理平台部署)
- [节点部署](#节点部署)
- [批量节点部署](#批量节点部署)
- [常见问题](#常见问题)

## 管理平台部署

管理平台包括 API 服务、用户前端、管理后台、PostgreSQL 数据库和 Redis 缓存。

### 系统要求

- **操作系统**: Ubuntu 20.04+, Debian 11+, CentOS 8+, RHEL 8+
- **CPU**: 2 核心（推荐 4 核心）
- **内存**: 4GB（推荐 8GB）
- **磁盘**: 20GB（推荐 50GB SSD）
- **权限**: Root 或 sudo

### 快速部署

部署脚本会自动完成以下操作：
1. 检查并安装 Docker 和 Docker Compose
2. 配置环境变量
3. 构建并启动所有服务
4. **自动运行数据库迁移和初始化**
5. 配置防火墙（可选）
6. 配置 SSL 证书（可选）

#### 1. 基础部署（不配置 SSL）

```bash
# 下载脚本
curl -sSL https://raw.githubusercontent.com/your-org/vpn-platform/main/scripts/deploy_platform.sh -o deploy_platform.sh

# 执行部署
sudo bash deploy_platform.sh
```

#### 2. 完整部署（包含 SSL 证书）

```bash
sudo bash deploy_platform.sh \
  --domain yourdomain.com \
  --email your@email.com \
  --enable-ssl
```

### 部署选项

| 选项 | 说明 | 示例 |
|------|------|------|
| `--domain` | 设置域名 | `--domain api.example.com` |
| `--email` | 设置邮箱（用于 Let's Encrypt） | `--email admin@example.com` |
| `--enable-ssl` | 启用 SSL 证书自动配置 | `--enable-ssl` |
| `--skip-docker` | 跳过 Docker 安装 | `--skip-docker` |
| `--skip-firewall` | 跳过防火墙配置 | `--skip-firewall` |

### 部署后访问

部署完成后，可以通过以下地址访问：

- **用户前端**: `http://服务器IP` 或 `https://yourdomain.com`
- **管理后台**: `http://服务器IP:8081` 或 `https://admin.yourdomain.com`
- **API 服务**: `http://服务器IP:8080` 或 `https://yourdomain.com/api`

### 默认管理员账号

```
邮箱: admin@example.com
密码: admin123
```

⚠️ **重要**: 请立即登录管理后台并修改默认密码！


## 节点部署

节点服务器运行 Xray-core 和 Node Agent，负责处理用户流量。

### 系统要求

- **操作系统**: Linux（Ubuntu, Debian, CentOS, RHEL）
- **网络**: 公网 IP 地址
- **端口**: 根据协议开放相应端口（默认 443）
- **权限**: Root 或 sudo

### 快速部署

#### 方式 1: 交互式部署

```bash
# 下载脚本
curl -sSL https://raw.githubusercontent.com/your-org/vpn-platform/main/scripts/quick_deploy_node.sh -o quick_deploy_node.sh

# 执行部署（会提示输入配置）
sudo bash quick_deploy_node.sh
```

#### 方式 2: 命令行参数部署

```bash
sudo bash quick_deploy_node.sh \
  --api-url https://api.yourdomain.com \
  --admin-token eyJhbGci... \
  --node-name node-hk-01
```

#### 方式 3: 指定协议和端口

```bash
sudo bash quick_deploy_node.sh \
  --api-url https://api.yourdomain.com \
  --admin-token eyJhbGci... \
  --node-name node-us-01 \
  --node-protocol vmess \
  --node-port 8443
```

### 获取管理员 Token

1. 登录管理后台
2. 进入"系统设置"或"API 密钥"页面
3. 生成或复制管理员 JWT Token

### 支持的协议

| 协议 | 说明 | 默认端口 |
|------|------|----------|
| `vless` | VLESS 协议（推荐） | 443 |
| `vmess` | VMess 协议 | 443 |
| `trojan` | Trojan 协议 | 443 |
| `shadowsocks` | Shadowsocks 协议 | 443 |
| `hysteria2` | Hysteria2 协议 | 443 |


## 批量节点部署

使用 YAML 配置文件可以一次部署多个节点。

### 1. 创建配置文件

创建 `nodes.yaml` 文件：

```yaml
# API 配置
api_url: https://api.yourdomain.com
admin_token: your-admin-jwt-token

# 节点列表
nodes:
  # 香港节点
  - name: node-hk-01
    host: auto  # 自动检测公网 IP
    port: 443
    protocol: vless

  # 美国节点
  - name: node-us-01
    host: auto
    port: 8443
    protocol: vmess

  # 日本节点
  - name: node-jp-01
    host: auto
    port: 443
    protocol: trojan
```

### 2. 执行批量部署

```bash
sudo bash quick_deploy_node.sh --batch-config nodes.yaml
```

### 3. 查看部署结果

脚本会依次部署每个节点，并显示部署结果：

```
部署节点 1/3: node-hk-01
✓ node-hk-01 部署成功

部署节点 2/3: node-us-01
✓ node-us-01 部署成功

部署节点 3/3: node-jp-01
✓ node-jp-01 部署成功

批量部署完成
```


## 常见问题

### 管理平台相关

#### Q: 部署后无法访问前端？

A: 检查以下几点：
1. 防火墙是否开放 80 和 443 端口
2. Docker 容器是否正常运行：`docker compose ps`
3. 查看日志：`docker compose logs frontend`

#### Q: API 服务启动失败？

A: 常见原因：
1. 数据库连接失败 - 检查 `.env` 中的 `DB_PASSWORD`
2. Redis 连接失败 - 检查 Redis 容器状态
3. 端口被占用 - 修改 `docker-compose.yml` 中的端口映射

#### Q: 如何修改管理员密码？

A: 
1. 登录管理后台
2. 进入"个人设置"
3. 修改密码并保存

#### Q: 如何备份数据？

A: 使用以下命令备份数据库：

```bash
docker compose exec postgres pg_dump -U vpn_user vpn_platform > backup.sql
```

### 节点部署相关

#### Q: 节点部署失败，提示认证错误？

A: 检查以下几点：
1. Admin Token 是否正确
2. Token 是否已过期
3. 是否有管理员权限

#### Q: 节点创建成功但服务无法启动？

A: 查看服务日志：

```bash
# 查看 Xray 日志
journalctl -u xray -n 50

# 查看 Node Agent 日志
journalctl -u node-agent -n 50
```

#### Q: 如何更新节点配置？

A: 
1. 修改配置文件：`/etc/node-agent/config.env`
2. 重启服务：`systemctl restart node-agent`

#### Q: 节点无法连接到 API？

A: 检查以下几点：
1. 网络连接是否正常
2. API URL 是否正确
3. 防火墙是否阻止了出站连接
4. Node Secret 是否正确

#### Q: 如何卸载节点？

A: 使用卸载脚本：

```bash
# 停止服务
systemctl stop node-agent xray
systemctl disable node-agent xray

# 删除文件
rm -rf /usr/local/bin/node-agent
rm -rf /etc/node-agent
rm -rf /etc/systemd/system/node-agent.service

# 卸载 Xray
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ remove
```


### 网络和安全相关

#### Q: 如何配置防火墙？

A: 管理平台需要开放：
- 80 (HTTP)
- 443 (HTTPS)
- 22 (SSH，用于管理)

节点服务器需要开放：
- 配置的节点端口（如 443, 8443）
- 22 (SSH，用于管理)

Ubuntu/Debian:
```bash
sudo ufw allow 22/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable
```

CentOS/RHEL:
```bash
sudo firewall-cmd --permanent --add-service=ssh
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --reload
```

#### Q: 如何启用 HTTPS？

A: 部署时使用 `--enable-ssl` 选项：

```bash
sudo bash deploy_platform.sh \
  --domain yourdomain.com \
  --email your@email.com \
  --enable-ssl
```

或手动配置 Nginx + Let's Encrypt：

```bash
sudo certbot --nginx -d yourdomain.com -d admin.yourdomain.com
```

#### Q: SSL 证书如何自动续期？

A: 部署脚本会自动配置 cron 任务。手动配置：

```bash
# 添加到 crontab
(crontab -l 2>/dev/null; echo "0 0 * * * certbot renew --quiet") | crontab -
```

### 性能优化

#### Q: 如何提升性能？

A: 
1. 使用 SSD 硬盘
2. 增加内存和 CPU 核心数
3. 启用 Redis 缓存
4. 配置 CDN
5. 使用负载均衡

#### Q: 数据库性能优化？

A: 在 `.env` 中配置：

```bash
DATABASE_MAX_CONNECTIONS=50
DATABASE_POOL_SIZE=20
```

#### Q: 如何监控系统资源？

A: 使用以下命令：

```bash
# 查看容器资源使用
docker stats

# 查看磁盘使用
df -h

# 查看内存使用
free -h

# 查看 CPU 使用
top
```

## 技术支持

如需帮助，请：

1. 查看日志文件
2. 检查 [GitHub Issues](https://github.com/your-org/vpn-platform/issues)
3. 阅读完整文档
4. 联系技术支持

## 相关文档

- [完整部署指南](../DEPLOYMENT.md)
- [快速开始指南](../QUICKSTART.md)
- [API 文档](./API.md)
- [故障排查指南](./TROUBLESHOOTING.md)
