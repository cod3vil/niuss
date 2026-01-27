# 部署脚本说明

本目录包含 VPN 订阅平台的一键部署脚本。

## 脚本列表

### 1. deploy_platform.sh - 管理平台部署脚本

自动部署完整的管理平台，包括：
- API 服务
- 用户前端
- 管理后台
- PostgreSQL 数据库
- Redis 缓存
- Nginx 反向代理（可选）
- SSL 证书配置（可选）

**使用方法**:
```bash
# 基础部署
sudo bash deploy_platform.sh

# 完整部署（包含 SSL）
sudo bash deploy_platform.sh --domain yourdomain.com --email your@email.com --enable-ssl
```

### 2. quick_deploy_node.sh - 节点快速部署脚本

快速部署 VPN 节点服务器，包括：
- 通过 API 自动创建节点
- 安装 Xray-core
- 安装和配置 Node Agent
- 启动服务并验证

**使用方法**:
```bash
# 交互式部署
sudo bash quick_deploy_node.sh

# 命令行参数部署
sudo bash quick_deploy_node.sh \
  --api-url https://api.yourdomain.com \
  --admin-token your-jwt-token \
  --node-name node-hk-01

# 批量部署
sudo bash quick_deploy_node.sh --batch-config nodes.yaml
```

### 3. deploy_node.sh - 节点完整部署脚本

功能更完整的节点部署脚本，支持：
- 所有 quick_deploy_node.sh 的功能
- 更详细的日志记录
- 更完善的错误处理
- 幂等性支持
- 回滚机制
- 批量部署

**使用方法**:
```bash
sudo bash deploy_node.sh --api-url <URL> --admin-token <TOKEN> --node-name <NAME>
```

### 4. install_node.sh - 节点安装脚本

仅安装节点组件，不创建节点记录（需要手动在管理后台创建）。

### 5. uninstall_node.sh - 节点卸载脚本

完全卸载节点服务和配置。

## 快速开始

### 部署管理平台

```bash
# 1. 克隆项目
git clone <repository-url>
cd vpn-subscription-platform

# 2. 执行部署
sudo bash scripts/deploy_platform.sh
```

### 部署节点

```bash
# 1. 获取管理员 Token（登录管理后台获取）

# 2. 执行部署
sudo bash scripts/quick_deploy_node.sh \
  --api-url https://api.yourdomain.com \
  --admin-token your-jwt-token \
  --node-name node-01
```

## 系统要求

### 管理平台
- 操作系统: Ubuntu 20.04+, Debian 11+, CentOS 8+
- CPU: 2 核心（推荐 4 核心）
- 内存: 4GB（推荐 8GB）
- 磁盘: 20GB（推荐 50GB SSD）

### 节点服务器
- 操作系统: Linux
- 网络: 公网 IP
- 端口: 根据协议开放相应端口

## 详细文档

- [一键部署指南](../docs/ONE_CLICK_DEPLOYMENT.md)
- [完整部署文档](../DEPLOYMENT.md)
- [快速开始](../QUICKSTART.md)

## 故障排查

### 管理平台

查看日志：
```bash
docker compose logs -f
```

重启服务：
```bash
docker compose restart
```

### 节点

查看服务状态：
```bash
systemctl status node-agent
systemctl status xray
```

查看日志：
```bash
journalctl -u node-agent -f
journalctl -u xray -f
```

## 技术支持

如有问题，请查看：
1. [常见问题](../docs/ONE_CLICK_DEPLOYMENT.md#常见问题)
2. [GitHub Issues](https://github.com/your-org/vpn-platform/issues)
3. 完整文档
