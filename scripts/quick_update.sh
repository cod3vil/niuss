#!/usr/bin/env bash

################################################################################
# 快速更新脚本
# 用于快速更新平台代码和服务
################################################################################

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_banner() {
    echo ""
    echo "=========================================="
    echo "  VPN 平台快速更新脚本"
    echo "=========================================="
    echo ""
}

# 检查是否在项目根目录
check_directory() {
    if [ ! -f "docker-compose.yml" ]; then
        log_error "请在项目根目录运行此脚本"
        exit 1
    fi
}

# 备份数据库
backup_database() {
    log_info "备份数据库..."
    
    local backup_dir="backups"
    mkdir -p "$backup_dir"
    
    local backup_file="${backup_dir}/backup_$(date +%Y%m%d_%H%M%S).sql.gz"
    
    if docker compose exec -T postgres pg_dump -U vpn_user vpn_platform | gzip > "$backup_file"; then
        log_info "数据库备份完成: $backup_file"
        echo "$backup_file"
    else
        log_error "数据库备份失败"
        exit 1
    fi
}

# 拉取最新代码
pull_code() {
    log_info "拉取最新代码..."
    
    if [ -d ".git" ]; then
        git pull origin main || {
            log_warn "代码拉取失败，继续使用本地代码"
        }
    else
        log_warn "不是 Git 仓库，跳过代码拉取"
    fi
}

# 检查并运行数据库迁移
run_migrations() {
    log_info "检查数据库迁移..."
    
    # 检查是否有新的迁移文件
    if [ -d "migrations" ]; then
        log_info "发现迁移文件目录"
        
        # 这里可以添加更智能的迁移检测逻辑
        # 目前简单地跳过已存在的表
        
        local tables_exist=$(docker compose exec -T postgres psql -U vpn_user -d vpn_platform -tAc "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='public' AND table_name='users';" 2>/dev/null || echo "0")
        
        if [ "$tables_exist" != "0" ]; then
            log_info "数据库已初始化，跳过基础迁移"
            
            # 检查是否需要运行特定迁移
            # 例如：clash_access_logs 表
            local clash_logs_exist=$(docker compose exec -T postgres psql -U vpn_user -d vpn_platform -tAc "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='public' AND table_name='clash_access_logs';" 2>/dev/null || echo "0")
            
            if [ "$clash_logs_exist" = "0" ] && [ -f "migrations/004_clash_access_logs.sql" ]; then
                log_info "运行 Clash 访问日志迁移..."
                docker compose exec -T postgres psql -U vpn_user -d vpn_platform < migrations/004_clash_access_logs.sql
            fi
            
            # 检查 node_proxy_unification 迁移
            local include_in_clash_exists=$(docker compose exec -T postgres psql -U vpn_user -d vpn_platform -tAc "SELECT COUNT(*) FROM information_schema.columns WHERE table_name='nodes' AND column_name='include_in_clash';" 2>/dev/null || echo "0")
            
            if [ "$include_in_clash_exists" = "0" ] && [ -f "migrations/005_node_proxy_unification.sql" ]; then
                log_info "运行节点代理统一迁移..."
                docker compose exec -T postgres psql -U vpn_user -d vpn_platform < migrations/005_node_proxy_unification.sql
            fi
        fi
    fi
}

# 重新构建镜像
rebuild_images() {
    log_info "重新构建镜像..."
    
    # 只重新构建 API（通常是最常更新的）
    docker compose build api
    
    # 如果前端也有更新，取消注释以下行
    # docker compose build frontend admin
}

# 重启服务
restart_services() {
    log_info "重启服务..."
    
    # 重启 API
    docker compose up -d api
    
    # 等待服务就绪
    log_info "等待服务启动..."
    sleep 5
    
    # 如果需要重启其他服务
    # docker compose up -d frontend admin
}

# 验证更新
verify_update() {
    log_info "验证更新..."
    
    local errors=0
    
    # 检查 API 健康
    if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
        log_info "✓ API 健康检查通过"
    else
        log_error "✗ API 健康检查失败"
        ((errors++))
    fi
    
    # 检查容器状态
    if docker compose ps api | grep -q "Up"; then
        log_info "✓ API 容器运行中"
    else
        log_error "✗ API 容器未运行"
        ((errors++))
    fi
    
    return $errors
}

# 显示更新信息
show_update_info() {
    echo ""
    echo "=========================================="
    echo "  更新完成！"
    echo "=========================================="
    echo ""
    echo "服务状态:"
    docker compose ps
    echo ""
    echo "查看日志:"
    echo "  docker compose logs -f api"
    echo ""
    echo "如果遇到问题，可以查看备份文件："
    echo "  $1"
    echo ""
}

# 主函数
main() {
    print_banner
    
    # 检查目录
    check_directory
    
    # 询问是否备份
    read -p "是否备份数据库？(y/n) [y]: " backup_choice
    backup_choice=${backup_choice:-y}
    
    local backup_file=""
    if [[ "$backup_choice" =~ ^[Yy]$ ]]; then
        backup_file=$(backup_database)
    else
        log_warn "跳过数据库备份"
    fi
    
    # 拉取代码
    pull_code
    
    # 运行迁移
    run_migrations
    
    # 重新构建
    rebuild_images
    
    # 重启服务
    restart_services
    
    # 验证
    if verify_update; then
        show_update_info "$backup_file"
        log_info "更新成功完成！"
    else
        log_error "更新验证失败，请检查日志"
        log_info "如需回滚，使用备份文件: $backup_file"
        exit 1
    fi
}

# 执行主函数
main "$@"
