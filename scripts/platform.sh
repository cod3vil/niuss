#!/usr/bin/env bash

################################################################################
# VPN 订阅平台管理脚本
# 
# 功能：
# - 部署完整的管理平台
# - 更新平台服务
# - 管理平台状态（启动/停止/重启）
# - 查看服务状态和日志
#
# 使用方法：
#   sudo ./platform.sh deploy [--domain DOMAIN] [--email EMAIL] [--enable-ssl]
#   sudo ./platform.sh update
#   sudo ./platform.sh restart
#   sudo ./platform.sh status
################################################################################

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 脚本版本
readonly VERSION="2.0.0"

# 日志文件
LOG_FILE="/var/log/vpn-platform.log"

# 配置变量
DOMAIN=""
EMAIL=""
ENABLE_SSL=false
SKIP_DOCKER_INSTALL=false
SKIP_FIREWALL=false

################################################################################
# 工具函数
################################################################################

log_info() {
    local msg="$1"
    echo -e "${GREEN}[INFO]${NC} $msg"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [INFO] $msg" >> "$LOG_FILE" 2>/dev/null || true
}

log_warn() {
    local msg="$1"
    echo -e "${YELLOW}[WARN]${NC} $msg"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [WARN] $msg" >> "$LOG_FILE" 2>/dev/null || true
}

log_error() {
    local msg="$1"
    echo -e "${RED}[ERROR]${NC} $msg" >&2
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [ERROR] $msg" >> "$LOG_FILE" 2>/dev/null || true
}

print_banner() {
    echo ""
    echo "=========================================="
    echo "  VPN 订阅平台管理脚本 v${VERSION}"
    echo "=========================================="
    echo ""
}

print_usage() {
    cat <<EOF
使用方法: $0 <命令> [选项]

命令:
  deploy              部署平台
  update              更新平台
  start               启动所有服务
  stop                停止所有服务
  restart             重启所有服务
  status              查看服务状态
  logs [service]      查看日志
  version             显示版本信息
  help                显示此帮助信息

部署选项:
  --domain DOMAIN     设置域名（用于 SSL 证书）
  --email EMAIL       设置邮箱（用于 Let's Encrypt）
  --enable-ssl        启用 SSL 证书自动配置
  --skip-docker       跳过 Docker 安装
  --skip-firewall     跳过防火墙配置

示例:
  # 基础部署
  sudo $0 deploy

  # 完整部署（包含 SSL）
  sudo $0 deploy --domain yourdomain.com --email your@email.com --enable-ssl

  # 更新平台
  sudo $0 update

  # 查看服务状态
  sudo $0 status

  # 查看 API 日志
  sudo $0 logs api
EOF
}

################################################################################
# 环境检测
################################################################################

check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "此脚本需要 root 权限运行"
        log_error "请使用: sudo $0 $*"
        exit 1
    fi
}

detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        OS_VERSION=$VERSION_ID
        log_info "检测到操作系统: $OS $OS_VERSION"
    else
        log_error "无法检测操作系统"
        exit 1
    fi
}

check_docker_compose() {
    if ! [ -f "docker-compose.yml" ]; then
        log_error "未找到 docker-compose.yml 文件"
        log_error "请在项目根目录运行此脚本"
        exit 1
    fi
}

################################################################################
# 命令实现
################################################################################

cmd_deploy() {
    log_info "开始部署平台..."
    
    check_root
    detect_os
    check_docker_compose
    
    # 安装 Docker（如果需要）
    if [ "$SKIP_DOCKER_INSTALL" != true ]; then
        install_docker
    fi
    
    # 配置项目
    setup_project
    
    # 部署服务
    deploy_services
    
    # 配置防火墙
    if [ "$SKIP_FIREWALL" != true ]; then
        configure_firewall
    fi
    
    # 配置 SSL
    if [ "$ENABLE_SSL" = true ]; then
        configure_ssl
    fi
    
    # 验证部署
    if verify_deployment; then
        show_deployment_info
        log_info "平台部署完成"
    else
        log_error "部署验证失败"
        exit 1
    fi
}

cmd_update() {
    log_info "开始更新平台..."
    
    check_root
    check_docker_compose
    
    # 拉取最新代码
    log_info "拉取最新代码..."
    if [ -d ".git" ]; then
        git pull origin main || log_warn "代码拉取失败，继续使用本地代码"
    else
        log_warn "不是 Git 仓库，跳过代码拉取"
    fi
    
    # 运行数据库迁移
    log_info "检查数据库迁移..."
    run_migrations
    
    # 重新构建镜像
    log_info "重新构建镜像..."
    docker compose build
    
    # 重启服务
    log_info "重启服务..."
    docker compose up -d
    
    # 等待服务就绪
    sleep 5
    
    # 验证更新
    if verify_deployment; then
        log_info "平台更新完成"
    else
        log_error "更新验证失败"
        exit 1
    fi
}

cmd_start() {
    log_info "启动所有服务..."
    check_docker_compose
    docker compose up -d
    log_info "服务已启动"
}

cmd_stop() {
    log_info "停止所有服务..."
    check_docker_compose
    docker compose down
    log_info "服务已停止"
}

cmd_restart() {
    log_info "重启所有服务..."
    check_docker_compose
    docker compose restart
    log_info "服务已重启"
}

cmd_status() {
    log_info "服务状态:"
    check_docker_compose
    docker compose ps
}

cmd_logs() {
    local service="$1"
    check_docker_compose
    
    if [ -n "$service" ]; then
        log_info "查看 $service 日志..."
        docker compose logs -f "$service"
    else
        log_info "查看所有服务日志..."
        docker compose logs -f
    fi
}

cmd_version() {
    echo "VPN 订阅平台管理脚本 v${VERSION}"
}

################################################################################
# 辅助函数（从 deploy_platform.sh 提取）
################################################################################

install_docker() {
    log_info "检查 Docker 安装状态..."
    
    if command -v docker &> /dev/null; then
        log_info "Docker 已安装"
        return 0
    fi
    
    log_info "开始安装 Docker..."
    
    case $OS in
        ubuntu|debian)
            apt-get update
            apt-get install -y ca-certificates curl gnupg lsb-release
            mkdir -p /etc/apt/keyrings
            curl -fsSL https://download.docker.com/linux/$OS/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
            echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/$OS $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null
            apt-get update
            apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
            ;;
        centos|rhel|rocky|almalinux)
            yum install -y yum-utils
            yum-config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo
            yum install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
            ;;
        *)
            log_error "不支持的操作系统: $OS"
            exit 1
            ;;
    esac
    
    systemctl start docker
    systemctl enable docker
    log_info "Docker 安装完成"
}

setup_project() {
    log_info "配置项目..."
    
    if [ ! -f ".env" ]; then
        log_info "创建 .env 文件..."
        local db_password=$(openssl rand -base64 32 | tr -d '/+=' | head -c 32)
        local jwt_secret=$(openssl rand -base64 48 | tr -d '/+=' | head -c 64)
        
        cat > .env <<EOF
DB_PASSWORD=${db_password}
JWT_SECRET=${jwt_secret}
JWT_EXPIRATION=86400
API_HOST=0.0.0.0
API_PORT=8080
CORS_ORIGINS=http://localhost,http://localhost:80,http://localhost:8081
RUST_LOG=info
VITE_API_URL=http://localhost:8080
EOF
        log_info ".env 文件创建完成"
    else
        log_info ".env 文件已存在"
    fi
    
    if [ -n "$DOMAIN" ]; then
        log_info "更新域名配置: $DOMAIN"
        sed -i "s|CORS_ORIGINS=.*|CORS_ORIGINS=https://${DOMAIN},https://admin.${DOMAIN}|" .env
        sed -i "s|VITE_API_URL=.*|VITE_API_URL=https://${DOMAIN}|" .env
    fi
}

deploy_services() {
    log_info "部署服务..."
    
    docker compose build
    docker compose up -d
    
    log_info "等待服务启动..."
    sleep 10
    
    # 等待数据库就绪
    local db_ready=false
    for i in {1..30}; do
        if docker compose exec -T postgres pg_isready -U vpn_user > /dev/null 2>&1; then
            log_info "数据库已就绪"
            db_ready=true
            break
        fi
        sleep 2
    done
    
    if [ "$db_ready" = false ]; then
        log_error "数据库启动超时"
        exit 1
    fi
    
    # 运行数据库迁移
    run_migrations
}

run_migrations() {
    log_info "运行数据库迁移..."
    
    local tables_exist=$(docker compose exec -T postgres psql -U vpn_user -d vpn_platform -tAc "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='public' AND table_name='users';" 2>/dev/null || echo "0")
    
    if [ "$tables_exist" = "0" ]; then
        log_info "初始化数据库..."
        
        for migration in migrations/*.sql; do
            if [[ "$migration" == *"rollback"* ]]; then
                continue
            fi
            
            log_info "运行迁移: $(basename "$migration")"
            docker compose exec -T postgres psql -U vpn_user -d vpn_platform < "$migration"
        done
        
        log_info "数据库初始化完成"
    else
        log_info "数据库已存在，检查新迁移..."
        # 这里可以添加更智能的迁移检测逻辑
    fi
}

configure_firewall() {
    log_info "配置防火墙..."
    
    if command -v ufw &> /dev/null; then
        ufw allow 22/tcp comment 'SSH'
        ufw allow 80/tcp comment 'HTTP'
        ufw allow 443/tcp comment 'HTTPS'
        ufw --force enable
        log_info "ufw 防火墙配置完成"
    elif command -v firewall-cmd &> /dev/null; then
        firewall-cmd --permanent --add-service=ssh
        firewall-cmd --permanent --add-service=http
        firewall-cmd --permanent --add-service=https
        firewall-cmd --reload
        log_info "firewalld 防火墙配置完成"
    else
        log_warn "未检测到防火墙工具"
    fi
}

configure_ssl() {
    log_info "配置 SSL 证书..."
    log_warn "SSL 配置功能待实现"
    # SSL 配置逻辑可以从 deploy_platform.sh 复制
}

verify_deployment() {
    log_info "验证部署..."
    
    local errors=0
    
    # 检查容器状态
    local containers=("vpn-postgres" "vpn-redis" "vpn-api" "vpn-frontend" "vpn-admin")
    for container in "${containers[@]}"; do
        if docker ps | grep -q "$container"; then
            log_info "✓ $container 运行中"
        else
            log_error "✗ $container 未运行"
            ((errors++))
        fi
    done
    
    # 检查 API 健康
    if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
        log_info "✓ API 健康检查通过"
    else
        log_error "✗ API 健康检查失败"
        ((errors++))
    fi
    
    return $errors
}

show_deployment_info() {
    echo ""
    echo "=========================================="
    echo "  部署完成！"
    echo "=========================================="
    echo ""
    echo "访问地址:"
    echo "  用户前端: http://$(hostname -I | awk '{print $1}'):50080"
    echo "  管理后台: http://$(hostname -I | awk '{print $1}'):50081"
    echo "  API 服务: http://$(hostname -I | awk '{print $1}'):50082"
    echo ""
    echo "默认管理员账号:"
    echo "  邮箱: admin@example.com"
    echo "  密码: admin123"
    echo "  ⚠️  请立即登录并修改密码！"
    echo ""
    echo "常用命令:"
    echo "  查看状态: $0 status"
    echo "  查看日志: $0 logs"
    echo "  重启服务: $0 restart"
    echo "  更新平台: $0 update"
    echo ""
}

################################################################################
# 主函数
################################################################################

main() {
    local command="${1:-help}"
    shift || true
    
    # 解析选项
    while [[ $# -gt 0 ]]; do
        case $1 in
            --domain)
                DOMAIN="$2"
                shift 2
                ;;
            --email)
                EMAIL="$2"
                shift 2
                ;;
            --enable-ssl)
                ENABLE_SSL=true
                shift
                ;;
            --skip-docker)
                SKIP_DOCKER_INSTALL=true
                shift
                ;;
            --skip-firewall)
                SKIP_FIREWALL=true
                shift
                ;;
            *)
                break
                ;;
        esac
    done
    
    # 执行命令
    case $command in
        deploy)
            print_banner
            cmd_deploy
            ;;
        update)
            print_banner
            cmd_update
            ;;
        start)
            cmd_start
            ;;
        stop)
            cmd_stop
            ;;
        restart)
            cmd_restart
            ;;
        status)
            cmd_status
            ;;
        logs)
            cmd_logs "$1"
            ;;
        version)
            cmd_version
            ;;
        help|--help|-h)
            print_usage
            ;;
        *)
            log_error "未知命令: $command"
            print_usage
            exit 1
            ;;
    esac
}

# 执行主函数
main "$@"
