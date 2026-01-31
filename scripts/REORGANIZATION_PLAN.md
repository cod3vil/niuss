# Scripts 目录整理方案

## 当前问题

1. **功能重复**: `deploy_node.sh` 和 `quick_deploy_node.sh` 功能高度重叠
2. **过时脚本**: `update_clash.sh` 已过时（Clash 代理已合并到节点表）
3. **文档不同步**: README.md 需要更新以反映新的架构
4. **脚本过于复杂**: `deploy_node.sh` 有 4748 行，过于庞大

## 整理方案

### 1. 合并节点部署脚本

**保留**: `deploy_node.sh` (重命名为 `node.sh`)
**删除**: `quick_deploy_node.sh`, `install_node.sh`

**原因**:
- `deploy_node.sh` 功能最完整，包含错误处理、日志记录、回滚机制
- `quick_deploy_node.sh` 和 `install_node.sh` 的功能都被 `deploy_node.sh` 包含
- 合并后提供统一的节点管理入口

**新的 `node.sh` 功能**:
```bash
# 部署节点
./node.sh deploy --api-url <URL> --admin-token <TOKEN> --node-name <NAME>

# 卸载节点
./node.sh uninstall

# 更新节点
./node.sh update

# 检查节点状态
./node.sh status
```

### 2. 简化平台部署脚本

**保留**: `deploy_platform.sh` (重命名为 `platform.sh`)

**新的 `platform.sh` 功能**:
```bash
# 部署平台
./platform.sh deploy

# 更新平台
./platform.sh update

# 停止平台
./platform.sh stop

# 重启平台
./platform.sh restart

# 查看状态
./platform.sh status
```

### 3. 删除过时脚本

**删除**: `update_clash.sh`

**原因**:
- Clash 代理管理已合并到节点表（node-proxy-unification）
- 不再需要单独的 Clash 更新脚本
- 相关功能已集成到平台更新流程中

### 4. 保留独立工具脚本

**保留**: 
- `db_manage.sh` - 数据库管理工具
- `update_admin_password.sh` - 管理员密码更新工具

**原因**: 这些是独立的维护工具，不与部署流程耦合

### 5. 更新文档

**更新**: `README.md`
- 反映新的脚本结构
- 更新使用示例
- 添加 node-proxy-unification 相关说明

## 最终目录结构

```
scripts/
├── platform.sh              # 平台管理（部署/更新/重启）
├── node.sh                  # 节点管理（部署/卸载/更新）
├── db_manage.sh             # 数据库管理工具
├── update_admin_password.sh # 管理员密码工具
└── README.md                # 更新后的文档
```

## 实施步骤

1. ✅ 创建整理方案文档
2. 创建新的 `platform.sh`（基于 `deploy_platform.sh`）
3. 创建新的 `node.sh`（基于 `deploy_node.sh` + `uninstall_node.sh`）
4. 删除过时和重复的脚本
5. 更新 `README.md`
6. 测试新脚本

## 向后兼容性

为了不破坏现有用户的使用习惯，可以考虑：
- 保留旧脚本名称作为符号链接
- 在旧脚本中添加弃用警告
- 提供迁移指南

## 预期收益

1. **简化维护**: 从 8 个脚本减少到 4 个核心脚本
2. **统一接口**: 所有操作通过子命令完成
3. **减少混淆**: 清晰的职责划分
4. **易于扩展**: 模块化设计便于添加新功能
5. **文档同步**: 架构变更反映在脚本中
