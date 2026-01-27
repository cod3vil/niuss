# 更新 Clash 配置管理功能

## 快速更新（3 步）

### 1. 拉取代码
```bash
cd /path/to/your/project
git pull origin main
```

### 2. 运行数据库迁移
```bash
# 方式 A: 使用 Docker（推荐）
docker-compose exec postgres psql -U postgres -d vpn_platform -f /migrations/003_clash_config_management.sql

# 方式 B: 直接连接数据库
psql -U postgres -d vpn_platform < migrations/003_clash_config_management.sql
```

### 3. 重启服务
```bash
docker-compose down
docker-compose build api
docker-compose up -d
```

## 一键更新脚本

如果你想自动化整个过程：

```bash
sudo ./scripts/update_clash_feature.sh
```

## 验证更新

### 检查服务状态
```bash
docker-compose ps
```

### 检查数据库表
```bash
docker-compose exec postgres psql -U postgres -d vpn_platform -c "\dt clash_*"
```

应该看到 3 个新表：
- `clash_proxies`
- `clash_proxy_groups`
- `clash_rules`

### 测试 API
```bash
# 健康检查
curl http://localhost:8080/health

# 登录获取 token
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@example.com","password":"admin123"}'

# 测试新端点（需要替换 YOUR_TOKEN）
curl http://localhost:8080/api/admin/clash/proxies \
  -H "Authorization: Bearer YOUR_TOKEN"
```

## 如果遇到问题

### 查看日志
```bash
docker-compose logs api
docker-compose logs postgres
```

### 回滚数据库（如果需要）
```bash
docker-compose exec postgres psql -U postgres -d vpn_platform -c "
DROP TABLE IF EXISTS clash_rules CASCADE;
DROP TABLE IF EXISTS clash_proxy_groups CASCADE;
DROP TABLE IF EXISTS clash_proxies CASCADE;
"
```

### 重新构建
```bash
docker-compose down -v
docker-compose build --no-cache
docker-compose up -d
```

## 更新后的新功能

✅ **动态管理 Clash 配置**
- 通过 API 管理代理服务器
- 管理代理组
- 管理路由规则
- 自动生成 YAML 配置

✅ **支持的代理类型**
- Shadowsocks (ss)
- VMess
- Trojan
- Hysteria2
- VLESS (含 Reality)

✅ **API 端点**
```
GET/POST/PUT/DELETE  /api/admin/clash/proxies
GET/POST/PUT/DELETE  /api/admin/clash/proxy-groups
GET/POST/PUT/DELETE  /api/admin/clash/rules
GET                  /api/admin/clash/generate
```

## 快速开始

查看快速入门指南：
```bash
cat docs/CLASH_CONFIG_QUICKSTART.md
```

运行示例脚本：
```bash
export ADMIN_TOKEN="your_jwt_token"
./examples/clash_config_example.sh
```

## 文档

- 📖 [快速入门](docs/CLASH_CONFIG_QUICKSTART.md)
- 📚 [API 文档](docs/CLASH_CONFIG_MANAGEMENT.md)
- 🎯 [功能说明](docs/FEATURES_CLASH_CONFIG.md)
- 📝 [实现总结](docs/CLASH_CONFIG_SUMMARY.md)

## 需要帮助？

1. 查看日志：`docker-compose logs -f api`
2. 检查数据库：`docker-compose exec postgres psql -U postgres -d vpn_platform`
3. 查看文档：`docs/` 目录
4. 运行示例：`examples/clash_config_example.sh`
