# 部署脚本说明

本目录包含 VPN 订阅平台的管理和部署脚本。

## 📋 脚本列表

### 核心脚本

#### 1. platform.sh - 平台管理脚本

统一管理平台的部署、更新和运维操作。

**功能**:
- 部署完整的管理平台（API、前端、管理后台、数据库、Redis）
- 更新平台服务
- 启动/停止/重启服务
- 查看服务状态和日志
- 配置 SSL 证书（可选）

**使用方法**:
```bash
# 部署平台
sudo ./platform.sh deploy

# 部署平台（包含 SSL）
sudo ./platform.sh deploy --domain yourdomain.com --email your@email.com --enable-ssl

# 更新平台
sudo ./platform.sh update

# 重启服务
sudo ./platform.sh restart

# 查看状态
sudo ./platform.sh status

# 查看日志
sudo ./platform.sh logs [service]
```

#### 2. node.sh - 节点管理脚本

统一管理节点的部署、卸载和运维操作。

**功能**:
- 通过 API 自动创建节点记录
- 安装 Xray-core 和 Node Agent
- 配置和启动服务
- 卸载节点
- 查看节点状态

**使用方法**:
```bash
# 部署节点
sudo ./node.sh deploy \
  --api-url https://api.yourdomain.com \
  --admin-token your-jwt-token \
  --node-name node-hk-01

# 指定协议和端口
sudo ./node.sh deploy \
  --api-url https://api.yourdomain.com \
  --admin-token your-jwt-token \
  --node-name node-us-01 \
  --node-protocol vmess \
  --node-port 8443

# 卸载节点
sudo ./node.sh uninstall

# 查看节点状态
sudo ./node.sh status
```

### 工具脚本

#### 3. db_manage.sh - 数据库管理工具

提供数据库的备份、恢复、迁移等操作。

**使用方法**:
```bash
# 备份数据库
sudo ./db_manage.sh backup

# 恢复数据库
sudo ./db_manage.sh restore backups/vpn_platform_20240101_120000.sql.gz

# 运行迁移
sudo ./db_manage.sh migrate migrations/005_node_proxy_unification.sql

# 查看数据库统计
sudo ./db_manage.sh stats

# 打开数据库 shell
sudo ./db_manage.sh shell
```

#### 4. update_admin_password.sh - 管理员密码更新工具

快速更新管理员密码。

**使用方法**:
```bash
sudo ./update_admin_password.sh
```

## 🚀 快速开始

### 部署管理平台

```bash
# 1. 克隆项目
git clone <repository-url>
cd vpn-subscription-platform

# 2. 执行部署
sudo ./scripts/platform.sh deploy
```

### 部署节点

```bash
# 1. 获取管理员 Token（登录管理后台获取）

# 2. 执行部署
sudo ./scripts/node.sh deploy \
  --api-url https://api.yourdomain.com \
  --admin-token your-jwt-token \
  --node-name node-01
```

## 📊 系统要求

### 管理平台
- **操作系统**: Ubuntu 20.04+, Debian 11+, CentOS 8+
- **CPU**: 2 核心（推荐 4 核心）
- **内存**: 4GB（推荐 8GB）
- **磁盘**: 20GB（推荐 50GB SSD）
- **软件**: Docker, Docker Compose

### 节点服务器
- **操作系统**: Linux（Ubuntu/Debian/CentOS）
- **网络**: 公网 IP
- **端口**: 根据协议开放相应端口（默认 443）
- **软件**: curl, jq, systemctl, openssl

## 🔧 支持的协议

节点部署支持以下协议:
- **VLESS** (默认)
- **VMess**
- **Trojan**
- **Shadowsocks**
- **Hysteria2**

## 📝 重要说明

### Node-Proxy 统一架构

从 v2.0.0 开始，系统采用了新的 **Node-Proxy 统一架构**:

- ✅ 节点和 Clash 代理已合并到统一的 `nodes` 表
- ✅ 不再需要单独管理 `clash_proxies` 表
- ✅ 节点创建时自动包含 Clash 配置字段
- ✅ 简化了管理流程，避免数据重复

**迁移说明**: 如果从旧版本升级，请运行数据库迁移:
```bash
sudo ./scripts/db_manage.sh migrate migrations/005_node_proxy_unification.sql
```

### 脚本版本变更

**v2.0.0 重大变更**:
- ✅ 合并了 `deploy_node.sh`, `quick_deploy_node.sh`, `install_node.sh` → `node.sh`
- ✅ 合并了 `deploy_platform.sh` → `platform.sh`
- ✅ 删除了过时的 `update_clash.sh`（功能已集成到平台更新中）
- ✅ 统一了命令接口，使用子命令模式

**向后兼容**: 旧脚本仍然保留在仓库中，但建议迁移到新脚本。

## 🔍 故障排查

### 管理平台

**查看日志**:
```bash
# 所有服务
sudo ./scripts/platform.sh logs

# 特定服务
sudo ./scripts/platform.sh logs api
sudo ./scripts/platform.sh logs postgres
```

**重启服务**:
```bash
sudo ./scripts/platform.sh restart
```

**检查服务状态**:
```bash
sudo ./scripts/platform.sh status
```

### 节点

**查看服务状态**:
```bash
sudo ./scripts/node.sh status
```

**查看日志**:
```bash
# Node Agent 日志
sudo journalctl -u node-agent -f

# Xray 日志
sudo journalctl -u xray -f
```

**重启服务**:
```bash
sudo systemctl restart node-agent
sudo systemctl restart xray
```

## 📚 详细文档

- [一键部署指南](../docs/ONE_CLICK_DEPLOYMENT.md)
- [Clash 配置管理](../docs/CLASH_CONFIG_MANAGEMENT.md)
- [Node-Proxy 统一架构](../migrations/README_NODE_PROXY_UNIFICATION.md)
- [快速开始](../QUICKSTART.md)

## 🆘 技术支持

如有问题，请查看:
1. [常见问题](../docs/ONE_CLICK_DEPLOYMENT.md#常见问题)
2. [GitHub Issues](https://github.com/your-org/vpn-platform/issues)
3. 完整文档

## 📜 更新日志

### v2.0.0 (2024-01-31)
- 🎉 重大重构：统一脚本接口
- ✨ 新增 `platform.sh` 和 `node.sh` 统一管理脚本
- 🔄 实现 Node-Proxy 统一架构
- 🗑️ 删除过时的 `update_clash.sh`
- 📝 更新文档以反映新架构

### v1.0.0
- 初始版本
- 基础部署脚本
