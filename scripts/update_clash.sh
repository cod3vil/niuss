#!/bin/bash

# Clash 配置管理功能更新脚本
# 适用于 docker compose (新版本)

set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}=== Clash 配置管理功能更新 ===${NC}\n"

# 检测 docker compose 命令
if command -v docker &> /dev/null && docker compose version &> /dev/null; then
    DOCKER_COMPOSE="docker compose"
elif command -v docker-compose &> /dev/null; then
    DOCKER_COMPOSE="docker-compose"
else
    echo -e "${RED}错误: 未找到 docker compose 或 docker-compose 命令${NC}"
    exit 1
fi

echo "使用命令: $DOCKER_COMPOSE"
echo ""

# 1. 拉取代码
echo -e "${BLUE}[1/4] 拉取最新代码...${NC}"
git pull origin main
echo -e "${GREEN}✓ 完成${NC}\n"

# 2. 数据库迁移
echo -e "${BLUE}[2/4] 数据库迁移...${NC}"
echo -e "${YELLOW}将创建 3 个新表: clash_proxies, clash_proxy_groups, clash_rules${NC}"
read -p "继续? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "已取消"
    exit 1
fi

POSTGRES_CONTAINER=$($DOCKER_COMPOSE ps -q postgres)
if [ -z "$POSTGRES_CONTAINER" ]; then
    echo -e "${RED}✗ 未找到 postgres 容器${NC}"
    exit 1
fi

docker cp migrations/003_clash_config_management.sql ${POSTGRES_CONTAINER}:/tmp/migration.sql
$DOCKER_COMPOSE exec -T postgres psql -U vpn_user -d vpn_platform -f /tmp/migration.sql

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ 数据库迁移完成${NC}\n"
    $DOCKER_COMPOSE exec -T postgres rm /tmp/migration.sql 2>/dev/null || true
else
    echo -e "${RED}✗ 数据库迁移失败${NC}"
    exit 1
fi

# 3. 重新构建和重启
echo -e "${BLUE}[3/4] 重新构建 API 服务...${NC}"
echo -e "${YELLOW}提示: 如果网络慢，可以按 Ctrl+C 取消，稍后手动执行${NC}"
sleep 2

$DOCKER_COMPOSE build api
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ 构建完成${NC}\n"
    $DOCKER_COMPOSE up -d
    echo -e "${GREEN}✓ 服务已重启${NC}\n"
else
    echo -e "${YELLOW}⚠ 构建失败，尝试只重启服务...${NC}"
    $DOCKER_COMPOSE restart api
    echo -e "${YELLOW}⚠ 服务已重启，但新功能需要重新构建才能使用${NC}\n"
fi

# 4. 验证
echo -e "${BLUE}[4/4] 验证更新...${NC}"
sleep 3

# 检查服务状态
if $DOCKER_COMPOSE ps | grep -q "api.*Up"; then
    echo -e "${GREEN}✓ API 服务运行中${NC}"
else
    echo -e "${RED}✗ API 服务未运行${NC}"
    echo "查看日志: $DOCKER_COMPOSE logs api"
    exit 1
fi

# 检查数据库表
TABLE_COUNT=$($DOCKER_COMPOSE exec -T postgres psql -U vpn_user -d vpn_platform -t -c "SELECT COUNT(*) FROM information_schema.tables WHERE table_name IN ('clash_proxies', 'clash_proxy_groups', 'clash_rules');")
if [ "$TABLE_COUNT" -eq 3 ]; then
    echo -e "${GREEN}✓ 数据库表创建成功 (3/3)${NC}"
else
    echo -e "${RED}✗ 数据库表不完整 ($TABLE_COUNT/3)${NC}"
fi

# 测试 API
if curl -s -f http://localhost:8080/health > /dev/null 2>&1; then
    echo -e "${GREEN}✓ API 健康检查通过${NC}"
else
    echo -e "${YELLOW}⚠ API 健康检查失败${NC}"
fi

echo ""
echo -e "${GREEN}=== 更新完成 ===${NC}\n"
echo "新功能:"
echo "  • Clash 代理管理"
echo "  • Clash 代理组管理"
echo "  • Clash 规则管理"
echo "  • 动态 YAML 配置生成"
echo ""
echo "API 端点:"
echo "  GET/POST/PUT/DELETE  /api/admin/clash/proxies"
echo "  GET/POST/PUT/DELETE  /api/admin/clash/proxy-groups"
echo "  GET/POST/PUT/DELETE  /api/admin/clash/rules"
echo "  GET                  /api/admin/clash/generate"
echo ""
echo "文档: docs/CLASH_CONFIG_QUICKSTART.md"
echo ""
