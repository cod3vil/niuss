# Clash 配置管理功能 - 更新指南

## 一键更新

```bash
sudo ./scripts/update_clash.sh
```

脚本会自动：
1. 拉取最新代码
2. 运行数据库迁移
3. 重新构建 API 服务
4. 验证更新结果

## 手动更新（3 步）

如果脚本失败，可以手动执行：

```bash
# 1. 拉取代码
git pull origin main

# 2. 数据库迁移
docker cp migrations/003_clash_config_management.sql $(docker compose ps -q postgres):/tmp/migration.sql
docker compose exec postgres psql -U vpn_user -d vpn_platform -f /tmp/migration.sql

# 3. 重启服务
docker compose restart api
```

## 验证更新

```bash
# 检查服务
docker compose ps

# 检查数据库表
docker compose exec postgres psql -U vpn_user -d vpn_platform -c "\dt clash_*"

# 测试 API
curl http://localhost:8080/health
```

## 新功能

- ✅ Clash 代理管理 API
- ✅ Clash 代理组管理 API
- ✅ Clash 规则管理 API
- ✅ 动态 YAML 配置生成

## 文档

- 快速入门: `docs/CLASH_CONFIG_QUICKSTART.md`
- API 文档: `docs/CLASH_CONFIG_MANAGEMENT.md`
- 功能说明: `docs/FEATURES_CLASH_CONFIG.md`

## 故障排除

### 网络超时
如果 Docker 镜像拉取超时，配置镜像加速：

```bash
sudo tee /etc/docker/daemon.json <<-'EOF'
{
  "registry-mirrors": [
    "https://docker.mirrors.ustc.edu.cn",
    "https://mirror.ccs.tencentyun.com"
  ]
}
EOF

sudo systemctl restart docker
```

### 查看日志
```bash
docker compose logs -f api
```

### 重新构建
```bash
docker compose build --no-cache api
docker compose up -d
```
