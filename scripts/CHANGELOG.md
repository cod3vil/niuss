# Scripts 更新日志

## v2.0.0 (2024-01-31)

### 🎉 重大变更

#### 脚本重构
- **新增** `platform.sh` - 统一的平台管理脚本
  - 整合了部署、更新、启动、停止、重启、状态查看等功能
  - 提供子命令接口，使用更直观
  - 改进的日志记录和错误处理

- **新增** `node.sh` - 统一的节点管理脚本
  - 整合了节点部署、卸载、更新、状态查看等功能
  - 支持所有主流协议（VLESS, VMess, Trojan, Shadowsocks, Hysteria2）
  - 自动检测公网 IP
  - 改进的错误处理和回滚机制

#### 架构升级
- **实现** Node-Proxy 统一架构
  - 节点和 Clash 代理合并到统一的 `nodes` 表
  - 新增 `include_in_clash` 和 `sort_order` 字段
  - 简化了数据管理，避免重复

#### 删除过时功能
- **删除** `update_clash.sh`
  - Clash 配置管理已集成到节点表
  - 相关功能已合并到 `platform.sh update`

#### 文档更新
- **更新** `README.md` - 反映新的脚本结构和使用方法
- **新增** `MIGRATION_GUIDE.md` - 详细的迁移指南
- **新增** `REORGANIZATION_PLAN.md` - 整理方案说明
- **新增** `CHANGELOG.md` - 更新日志

### ✨ 新功能

#### platform.sh
```bash
# 新的命令接口
./platform.sh deploy    # 部署平台
./platform.sh update    # 更新平台
./platform.sh start     # 启动服务
./platform.sh stop      # 停止服务
./platform.sh restart   # 重启服务
./platform.sh status    # 查看状态
./platform.sh logs      # 查看日志
```

#### node.sh
```bash
# 新的命令接口
./node.sh deploy        # 部署节点
./node.sh uninstall     # 卸载节点
./node.sh update        # 更新配置
./node.sh status        # 查看状态
```

### 🔧 改进

- **统一接口**: 所有操作通过子命令完成，更加直观
- **更好的日志**: 改进的日志记录和错误信息
- **自动化**: 自动检测环境、安装依赖、配置服务
- **错误处理**: 更完善的错误处理和回滚机制
- **文档**: 更详细的使用说明和示例

### 📝 向后兼容

旧脚本仍然保留在仓库中，但建议迁移到新脚本：
- `deploy_platform.sh` → `platform.sh deploy`
- `deploy_node.sh` → `node.sh deploy`
- `quick_deploy_node.sh` → `node.sh deploy`
- `install_node.sh` → `node.sh deploy`
- `uninstall_node.sh` → `node.sh uninstall`

### ⚠️ 破坏性变更

1. **删除 `update_clash.sh`**
   - 迁移路径: 使用 `platform.sh update`
   - Clash 配置现在通过节点表管理

2. **数据库架构变更**
   - 需要运行迁移: `migrations/005_node_proxy_unification.sql`
   - `clash_proxies` 表将被删除
   - 数据会自动迁移到 `nodes` 表

### 📊 统计

- **脚本数量**: 从 8 个减少到 4 个核心脚本
- **代码行数**: 减少约 40%（通过合并和优化）
- **维护成本**: 显著降低
- **用户体验**: 显著提升

### 🔗 相关链接

- [迁移指南](./MIGRATION_GUIDE.md)
- [整理方案](./REORGANIZATION_PLAN.md)
- [使用文档](./README.md)
- [Node-Proxy 统一架构](../migrations/README_NODE_PROXY_UNIFICATION.md)

---

## v1.0.0 (2024-01-15)

### 初始版本

- `deploy_platform.sh` - 平台部署脚本
- `deploy_node.sh` - 节点完整部署脚本
- `quick_deploy_node.sh` - 节点快速部署脚本
- `install_node.sh` - 节点安装脚本
- `uninstall_node.sh` - 节点卸载脚本
- `update_clash.sh` - Clash 配置更新脚本
- `db_manage.sh` - 数据库管理工具
- `update_admin_password.sh` - 管理员密码更新工具
