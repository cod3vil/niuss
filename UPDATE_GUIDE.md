# 服务器更新指南

## 快速更新（推荐）

如果你的服务器已经部署，使用以下命令快速更新：

```bash
# 方法 1: 使用平台管理脚本（推荐）
sudo ./scripts/platform.sh update

# 方法 2: 使用 Makefile
make deploy
```

## 详细更新步骤

### 1. 连接到服务器

```bash
ssh your-server
cd /path/to/your/project
```

### 2. 备份数据库（重要！）

```bash
# 备份数据库
make db-backup

# 或使用脚本
./scripts/db_manage.sh backup
```

### 3. 拉取最新代码

```bash
git pull origin main
```

### 4. 运行数据库迁移

如果你更新了 SQL 文件，需要运行迁移：

```bash
# 检查当前数据库状态
make db-shell
# 在 psql 中执行: \dt 查看表

# 运行新的迁移（如果有）
docker compose exec -T postgres psql -U vpn_user -d vpn_platform < migrations/your_new_migration.sql
```

### 5. 重新构建并重启服务

```bash
# 重新构建镜像
docker compose build

# 重启所有服务
docker compose up -d

# 或者使用 Makefile
make build
make restart
```

### 6. 验证更新

```bash
# 检查服务状态
make status

# 检查健康状态
make health-check

# 查看日志
make logs-api
```

## 针对本次更新

本次更新包含：
1. **后端代码修复**：修复推荐链接显示问题
2. **数据库迁移**：如果你整合了 migrations

### 更新步骤：

```bash
# 1. 备份数据库
make db-backup

# 2. 拉取代码（如果使用 Git）
git pull

# 3. 重新构建 API 服务
docker compose build api

# 4. 重启 API 服务
docker compose restart api

# 5. 检查 API 日志
docker compose logs -f api

# 6. 验证推荐链接功能
# 登录前端，访问推荐页面，确认链接显示正常
```

## 环境变量检查

确保 `.env` 文件包含正确的 `FRONTEND_URL`：

```bash
# 编辑 .env 文件
nano .env

# 添加或修改以下行
FRONTEND_URL=https://your-domain.com
# 或者如果没有域名
FRONTEND_URL=http://your-server-ip
```

修改后重启服务：

```bash
docker compose restart api
```

## 回滚（如果出现问题）

```bash
# 1. 停止服务
docker compose down

# 2. 恢复数据库备份
./scripts/db_manage.sh restore backup_file.sql.gz

# 3. 切换到之前的代码版本
git checkout previous-commit-hash

# 4. 重新构建并启动
docker compose build
docker compose up -d
```

## 常见问题

### 问题 1: 推荐链接仍然不显示

**解决方案：**
```bash
# 1. 检查 API 日志
docker compose logs api | grep -i referral

# 2. 确认环境变量
docker compose exec api env | grep FRONTEND_URL

# 3. 重启 API
docker compose restart api
```

### 问题 2: 数据库连接失败

**解决方案：**
```bash
# 检查数据库状态
docker compose ps postgres

# 查看数据库日志
docker compose logs postgres

# 重启数据库
docker compose restart postgres
```

### 问题 3: 服务无法启动

**解决方案：**
```bash
# 查看所有容器状态
docker compose ps

# 查看失败服务的日志
docker compose logs <service-name>

# 完全重启
docker compose down
docker compose up -d
```

## 监控和维护

### 查看实时日志

```bash
# 所有服务
make logs

# 特定服务
make logs-api
make logs-postgres
```

### 检查资源使用

```bash
# 容器资源使用
docker stats

# 磁盘使用
df -h

# 数据库大小
make db-stats
```

## 联系支持

如果遇到问题：
1. 查看日志文件：`/var/log/vpn-platform.log`
2. 检查容器日志：`docker compose logs`
3. 查看数据库状态：`make db-shell`
