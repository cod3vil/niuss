# 脚本迁移指南

本指南帮助您从旧版脚本迁移到 v2.0.0 的新脚本。

## 📋 变更概览

### 删除的脚本
- ❌ `update_clash.sh` - 已过时（Clash 功能已合并到节点表）

### 保留但建议迁移的脚本
- ⚠️ `deploy_platform.sh` → 建议使用 `platform.sh`
- ⚠️ `deploy_node.sh` → 建议使用 `node.sh`
- ⚠️ `quick_deploy_node.sh` → 建议使用 `node.sh`
- ⚠️ `install_node.sh` → 建议使用 `node.sh`
- ⚠️ `uninstall_node.sh` → 建议使用 `node.sh uninstall`

### 保持不变的脚本
- ✅ `db_manage.sh` - 无变更
- ✅ `update_admin_password.sh` - 无变更

## 🔄 命令对照表

### 平台管理

| 旧命令 | 新命令 |
|--------|--------|
| `bash deploy_platform.sh` | `./platform.sh deploy` |
| `bash deploy_platform.sh --domain example.com --email admin@example.com --enable-ssl` | `./platform.sh deploy --domain example.com --email admin@example.com --enable-ssl` |
| `docker compose restart` | `./platform.sh restart` |
| `docker compose ps` | `./platform.sh status` |
| `docker compose logs -f` | `./platform.sh logs` |
| N/A | `./platform.sh update` |

### 节点管理

| 旧命令 | 新命令 |
|--------|--------|
| `bash deploy_node.sh --api-url <URL> --admin-token <TOKEN> --node-name <NAME>` | `./node.sh deploy --api-url <URL> --admin-token <TOKEN> --node-name <NAME>` |
| `bash quick_deploy_node.sh --api-url <URL> --admin-token <TOKEN> --node-name <NAME>` | `./node.sh deploy --api-url <URL> --admin-token <TOKEN> --node-name <NAME>` |
| `bash install_node.sh` | `./node.sh deploy` (需要提供 API 参数) |
| `bash uninstall_node.sh` | `./node.sh uninstall` |
| `systemctl status node-agent` | `./node.sh status` |

### Clash 更新

| 旧命令 | 新命令 |
|--------|--------|
| `bash update_clash.sh` | `./platform.sh update` |

**说明**: Clash 配置管理已集成到节点表中，不再需要单独的更新脚本。

## 📝 迁移步骤

### 1. 更新代码

```bash
cd /path/to/vpn-subscription-platform
git pull origin main
```

### 2. 运行数据库迁移（如果从旧版本升级）

```bash
sudo ./scripts/db_manage.sh migrate migrations/005_node_proxy_unification.sql
```

这个迁移会：
- 在 `nodes` 表中添加 `include_in_clash` 和 `sort_order` 字段
- 将 `clash_proxies` 表的数据迁移到 `nodes` 表
- 删除 `clash_proxies` 表

### 3. 更新自动化脚本

如果您有自动化脚本或 CI/CD 流程使用旧脚本，请更新它们：

**旧的部署脚本**:
```bash
#!/bin/bash
bash /path/to/scripts/deploy_platform.sh
bash /path/to/scripts/deploy_node.sh --api-url $API_URL --admin-token $TOKEN --node-name $NAME
```

**新的部署脚本**:
```bash
#!/bin/bash
/path/to/scripts/platform.sh deploy
/path/to/scripts/node.sh deploy --api-url $API_URL --admin-token $TOKEN --node-name $NAME
```

### 4. 更新文档和 README

如果您有自己的文档引用了旧脚本，请更新它们。

## ⚠️ 重要注意事项

### Node-Proxy 统一架构

从 v2.0.0 开始，系统采用了新的架构：

**旧架构**:
```
nodes 表 (节点信息)
clash_proxies 表 (Clash 代理配置)
```

**新架构**:
```
nodes 表 (节点信息 + Clash 配置)
  - include_in_clash: 是否包含在 Clash 配置中
  - sort_order: Clash 配置中的排序
```

**影响**:
- ✅ 不再需要手动同步节点和代理
- ✅ 创建节点时自动包含 Clash 配置
- ✅ 管理界面更简洁
- ⚠️ 需要运行数据库迁移

### API 变更

如果您直接调用 API，请注意以下变更：

**删除的端点**:
```
GET/POST/PUT/DELETE /api/admin/clash/proxies
```

**新的节点字段**:
```json
{
  "name": "node-01",
  "host": "1.2.3.4",
  "port": 443,
  "protocol": "vless",
  "secret": "...",
  "config": {},
  "include_in_clash": true,    // 新增
  "sort_order": 1               // 新增
}
```

## 🧪 测试迁移

在生产环境迁移前，建议先在测试环境验证：

### 1. 测试平台部署

```bash
# 在测试服务器上
sudo ./scripts/platform.sh deploy
sudo ./scripts/platform.sh status
```

### 2. 测试节点部署

```bash
# 在测试节点上
sudo ./scripts/node.sh deploy \
  --api-url https://test-api.yourdomain.com \
  --admin-token test-token \
  --node-name test-node-01

sudo ./scripts/node.sh status
```

### 3. 测试数据库迁移

```bash
# 备份数据库
sudo ./scripts/db_manage.sh backup

# 运行迁移
sudo ./scripts/db_manage.sh migrate migrations/005_node_proxy_unification.sql

# 验证迁移
sudo ./scripts/db_manage.sh shell
# 在 psql 中执行:
# \d nodes
# SELECT * FROM nodes WHERE include_in_clash = true;
```

## 🔙 回滚方案

如果迁移后遇到问题，可以回滚：

### 回滚数据库迁移

```bash
sudo ./scripts/db_manage.sh migrate migrations/005_node_proxy_unification_rollback.sql
```

### 回滚代码

```bash
git checkout v1.0.0  # 或之前的稳定版本
```

### 恢复数据库备份

```bash
sudo ./scripts/db_manage.sh restore backups/vpn_platform_YYYYMMDD_HHMMSS.sql.gz
```

## 📞 获取帮助

如果在迁移过程中遇到问题：

1. 查看日志:
   ```bash
   sudo ./scripts/platform.sh logs
   sudo ./scripts/node.sh status
   ```

2. 查看迁移文档:
   - [Node-Proxy 统一架构说明](../migrations/README_NODE_PROXY_UNIFICATION.md)
   - [脚本整理方案](./REORGANIZATION_PLAN.md)

3. 提交 Issue:
   - [GitHub Issues](https://github.com/your-org/vpn-platform/issues)

## ✅ 迁移检查清单

完成迁移后，请检查以下项目：

- [ ] 数据库迁移成功运行
- [ ] 所有节点在 `nodes` 表中有 `include_in_clash` 和 `sort_order` 字段
- [ ] `clash_proxies` 表已删除
- [ ] 平台服务正常运行
- [ ] 节点服务正常运行
- [ ] Clash 配置生成正常
- [ ] 管理后台可以正常访问
- [ ] 用户前端可以正常访问
- [ ] 自动化脚本已更新
- [ ] 文档已更新

## 🎉 迁移完成

恭喜！您已成功迁移到 v2.0.0。

新版本的优势：
- ✨ 更简洁的脚本接口
- 🚀 更快的部署流程
- 🔧 更容易的维护
- 📊 统一的数据模型
- 🎯 更好的用户体验
